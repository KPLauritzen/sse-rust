use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;
use sse_core::search::{execute_search_request, validate_sse_path_dyn};
use sse_core::types::{
    DynSsePath, EsseStep, FrontierMode, GuideArtifact, GuideArtifactCompatibility,
    GuideArtifactEndpoints, GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality,
    GuideArtifactValidation, GuidedRefinementConfig, MoveFamilyPolicy, SearchConfig, SearchRequest,
    SearchRunResult, SearchStage, ShortcutSearchConfig,
};

#[derive(Debug)]
struct Cli {
    fixture_ref: String,
    seeded_guide_ids: Vec<String>,
    paths_db: Option<PathBuf>,
    output_path: PathBuf,
    k: i64,
    include_shortcut_sqlite: bool,
    include_graph_sqlite: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct EndpointFixtureCollection {
    fixtures: Vec<EndpointFixture>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum EndpointFixtureFile {
    Collection(EndpointFixtureCollection),
    Single(EndpointFixture),
}

#[derive(Clone, Debug, Deserialize)]
struct EndpointFixture {
    id: String,
    #[serde(default)]
    a: Vec<Vec<u32>>,
    #[serde(default)]
    b: Vec<Vec<u32>>,
    #[serde(default)]
    seeded_guides: Vec<SeededGuideFixture>,
}

#[derive(Clone, Debug, Deserialize)]
struct SeededGuideFixture {
    id: String,
    matrices: Vec<Vec<Vec<u32>>>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    source_ref: Option<String>,
}

#[derive(Clone, Debug)]
struct PathCandidate {
    artifact_id_hint: String,
    label: String,
    source_kind: String,
    source_ref: String,
    max_dim: usize,
    max_entry: u32,
    matrices: Vec<DynMatrix>,
}

#[derive(Debug, Serialize)]
struct GuideArtifactEnvelope {
    artifacts: Vec<GuideArtifact>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    println!("assembling normalized k={} guide pool", cli.k);
    println!("  fixture_ref: {}", cli.fixture_ref);
    println!(
        "  paths_db: {}",
        display_optional_path(cli.paths_db.as_deref())
    );
    println!("  include_shortcut_sqlite: {}", cli.include_shortcut_sqlite);
    println!("  include_graph_sqlite: {}", cli.include_graph_sqlite);
    println!("  output: {}", cli.output_path.display());

    let fixture = load_endpoint_fixture(&cli.fixture_ref)?;
    let endpoint_source = case_matrix(&fixture.a)
        .map_err(|err| format!("fixture {} has invalid source matrix: {err}", fixture.id))?;
    let endpoint_target = case_matrix(&fixture.b)
        .map_err(|err| format!("fixture {} has invalid target matrix: {err}", fixture.id))?;
    let source_canonical = endpoint_source.canonical_perm();
    let target_canonical = endpoint_target.canonical_perm();

    let mut candidates = Vec::new();
    let seeded_ids = if cli.seeded_guide_ids.is_empty() {
        fixture
            .seeded_guides
            .iter()
            .map(|guide| guide.id.clone())
            .collect::<Vec<_>>()
    } else {
        cli.seeded_guide_ids.clone()
    };

    for seeded_id in seeded_ids {
        let Some(guide) = fixture
            .seeded_guides
            .iter()
            .find(|guide| guide.id == seeded_id)
        else {
            return Err(format!(
                "seeded guide {} not found in fixture {}",
                seeded_id, fixture.id
            ));
        };
        candidates.push(candidate_from_seeded_guide(&cli.fixture_ref, guide)?);
    }

    if let Some(db_path) = &cli.paths_db {
        if db_path.exists() {
            let conn = Connection::open(db_path)
                .map_err(|err| format!("failed to open {}: {err}", db_path.display()))?;
            if cli.include_shortcut_sqlite {
                let shortcut = load_shortcut_candidates(&conn, cli.k)?;
                println!(
                    "loaded {} shortcut sqlite candidate path(s)",
                    shortcut.len()
                );
                candidates.extend(shortcut);
            }
            if cli.include_graph_sqlite {
                let graph = load_graph_candidates(&conn, cli.k)?;
                println!("loaded {} graph sqlite candidate path(s)", graph.len());
                candidates.extend(graph);
            }
        } else {
            println!(
                "paths db {} does not exist; continuing without sqlite sources",
                db_path.display()
            );
        }
    }

    println!("loaded {} total candidate path(s)", candidates.len());

    let mut accepted = 0usize;
    let mut skipped_endpoint_mismatch = 0usize;
    let mut reconstruction_failures = 0usize;
    let mut reference_guides = 0usize;

    let mut by_path_key: HashMap<String, GuideArtifact> = HashMap::new();
    let reference = build_lind_marcus_reference_guide()?;
    match orient_guide_artifact_to_endpoints(reference, &endpoint_source, &endpoint_target)? {
        Some(reference) => {
            let reference_key = path_key(match &reference.payload {
                GuideArtifactPayload::FullPath { path } => path,
            });
            by_path_key.insert(reference_key, reference);
            reference_guides += 1;
        }
        None => {
            println!(
                "skipping Lind-Marcus/Baker reference guide for non-matching fixture endpoints"
            );
        }
    }

    for candidate in candidates {
        let Some(reverse) =
            candidate_endpoint_orientation(&candidate, &source_canonical, &target_canonical)
        else {
            skipped_endpoint_mismatch += 1;
            continue;
        };

        let mut oriented = candidate.clone();
        if reverse {
            oriented.matrices.reverse();
            oriented.source_ref = format!("{}#reversed", oriented.source_ref);
            oriented.label = format!("{} [reversed]", oriented.label);
        }

        match materialize_candidate(&oriented) {
            Ok(path) => {
                accepted += 1;
                let key = path_key(&path);
                let artifact = build_artifact_from_candidate(&oriented, path);
                match by_path_key.get(&key) {
                    Some(existing)
                        if existing.quality.lag.unwrap_or(usize::MAX)
                            <= artifact.quality.lag.unwrap_or(usize::MAX) => {}
                    _ => {
                        by_path_key.insert(key, artifact);
                    }
                }
            }
            Err(err) => {
                reconstruction_failures += 1;
                eprintln!(
                    "warn: failed to reconstruct candidate {} ({}): {}",
                    candidate.artifact_id_hint, candidate.source_kind, err
                );
            }
        }
    }

    let mut artifacts = by_path_key.into_values().collect::<Vec<_>>();
    artifacts.sort_by(|left, right| {
        left.quality
            .lag
            .unwrap_or(usize::MAX)
            .cmp(&right.quality.lag.unwrap_or(usize::MAX))
            .then_with(|| {
                left.artifact_id
                    .as_deref()
                    .unwrap_or("")
                    .cmp(right.artifact_id.as_deref().unwrap_or(""))
            })
    });

    for (index, artifact) in artifacts.iter_mut().enumerate() {
        if artifact.artifact_id.is_none() {
            artifact.artifact_id = Some(format!("k3-normalized-{index:03}"));
        }
    }

    if artifacts.is_empty() {
        return Err("no guide artifacts were materialized".to_string());
    }

    let envelope = GuideArtifactEnvelope { artifacts };
    let json = serde_json::to_string_pretty(&envelope)
        .map_err(|err| format!("failed to serialize output JSON: {err}"))?;

    if let Some(parent) = cli.output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create output directory {}: {err}",
                parent.display()
            )
        })?;
    }

    fs::write(&cli.output_path, format!("{json}\n"))
        .map_err(|err| format!("failed to write {}: {err}", cli.output_path.display()))?;

    println!();
    println!("materialization summary:");
    println!("  built-in reference guides: {}", reference_guides);
    println!("  accepted candidates: {}", accepted);
    println!("  endpoint mismatch skipped: {}", skipped_endpoint_mismatch);
    println!("  reconstruction failures: {}", reconstruction_failures);
    println!(
        "  unique normalized guides written: {}",
        envelope.artifacts.len()
    );
    println!("  best lag in pool: {}", best_lag(&envelope.artifacts));

    Ok(())
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut fixture_ref = "research/fixtures/brix_ruiz_family.json#brix_ruiz_k3".to_string();
    let mut seeded_guide_ids = Vec::new();
    let mut paths_db = Some(PathBuf::from("research/k3-graph-paths.sqlite"));
    let mut output_path = PathBuf::from("research/guide_artifacts/k3_normalized_guide_pool.json");
    let mut k = 3i64;
    let mut include_shortcut_sqlite = true;
    let mut include_graph_sqlite = true;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixture-ref" => {
                fixture_ref = args.next().ok_or("--fixture-ref requires a value")?;
            }
            "--seeded-guide-id" => {
                seeded_guide_ids.push(args.next().ok_or("--seeded-guide-id requires a value")?);
            }
            "--paths-db" => {
                paths_db = Some(PathBuf::from(
                    args.next().ok_or("--paths-db requires a value")?,
                ));
            }
            "--no-paths-db" => {
                paths_db = None;
            }
            "--out" => {
                output_path = PathBuf::from(args.next().ok_or("--out requires a value")?);
            }
            "--k" => {
                k = args
                    .next()
                    .ok_or("--k requires a value")?
                    .parse()
                    .map_err(|err| format!("invalid --k value: {err}"))?;
            }
            "--no-shortcut-sqlite" => {
                include_shortcut_sqlite = false;
            }
            "--no-graph-sqlite" => {
                include_graph_sqlite = false;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: assemble_k3_guide_pool [options]\n\n\
                     Options:\n\
                       --fixture-ref REF         fixture file/id reference (default: research/fixtures/brix_ruiz_family.json#brix_ruiz_k3)\n\
                       --seeded-guide-id ID      seeded fixture guide id to include (repeatable; default: all from fixture)\n\
                       --paths-db PATH           sqlite db with graph/shortcut paths (default: research/k3-graph-paths.sqlite)\n\
                       --no-paths-db             disable sqlite ingestion\n\
                       --no-shortcut-sqlite      skip shortcut_path_results ingestion\n\
                       --no-graph-sqlite         skip graph_path_results ingestion\n\
                       --k N                     sqlite run family k filter (default: 3)\n\
                       --out PATH                output JSON path (default: research/guide_artifacts/k3_normalized_guide_pool.json)"
                        .to_string(),
                );
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                return Err(format!("unexpected positional argument: {other}"));
            }
        }
    }

    if !include_shortcut_sqlite && !include_graph_sqlite && paths_db.is_some() {
        return Err(
            "sqlite ingestion disabled for both graph and shortcut; use --no-paths-db instead"
                .to_string(),
        );
    }

    Ok(Cli {
        fixture_ref,
        seeded_guide_ids,
        paths_db,
        output_path,
        k,
        include_shortcut_sqlite,
        include_graph_sqlite,
    })
}

fn display_optional_path(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn load_endpoint_fixture(fixture_ref: &str) -> Result<EndpointFixture, String> {
    let (path, fixture_id) = split_fixture_ref(fixture_ref);
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read fixture {}: {err}", path.display()))?;
    let parsed: EndpointFixtureFile = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse fixture {}: {err}", path.display()))?;

    let mut fixtures = match parsed {
        EndpointFixtureFile::Single(fixture) => vec![fixture],
        EndpointFixtureFile::Collection(collection) => collection.fixtures,
    };

    match fixture_id {
        Some(id) => fixtures
            .into_iter()
            .find(|fixture| fixture.id == id)
            .ok_or_else(|| format!("fixture {} not found in {}", id, path.display())),
        None => {
            if fixtures.len() != 1 {
                return Err(format!(
                    "fixture {} contains {} fixtures; specify path#fixture_id",
                    path.display(),
                    fixtures.len()
                ));
            }
            Ok(fixtures.remove(0))
        }
    }
}

fn split_fixture_ref(fixture_ref: &str) -> (PathBuf, Option<String>) {
    match fixture_ref.split_once('#') {
        Some((path, fixture_id)) if !fixture_id.is_empty() => {
            (PathBuf::from(path), Some(fixture_id.to_string()))
        }
        _ => (PathBuf::from(fixture_ref), None),
    }
}

fn candidate_from_seeded_guide(
    fixture_ref: &str,
    guide: &SeededGuideFixture,
) -> Result<PathCandidate, String> {
    if guide.matrices.len() < 2 {
        return Err(format!(
            "seeded guide {} must have at least 2 matrices",
            guide.id
        ));
    }

    let matrices = guide
        .matrices
        .iter()
        .map(|rows| case_matrix(rows))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("seeded guide {} contains invalid matrix: {err}", guide.id))?;

    let max_dim = matrices.iter().map(|matrix| matrix.rows).max().unwrap_or(0);
    let max_entry = matrices
        .iter()
        .flat_map(|matrix| matrix.data.iter().copied())
        .max()
        .unwrap_or(0);

    Ok(PathCandidate {
        artifact_id_hint: format!("seeded-{}", guide.id),
        label: guide
            .label
            .clone()
            .unwrap_or_else(|| format!("seeded-guide-{}", guide.id)),
        source_kind: guide
            .source_kind
            .clone()
            .unwrap_or_else(|| "seeded_fixture".to_string()),
        source_ref: guide
            .source_ref
            .clone()
            .unwrap_or_else(|| format!("{}#{}", fixture_ref, guide.id)),
        max_dim,
        max_entry,
        matrices,
    })
}

fn load_shortcut_candidates(conn: &Connection, k: i64) -> Result<Vec<PathCandidate>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.guide_label, r.source_kind, r.path_signature, rr.max_dim, rr.max_entry, m.step_index, x.data_json
             FROM shortcut_path_results r
             JOIN shortcut_path_runs rr ON rr.id = r.run_id
             JOIN shortcut_path_matrices m ON m.result_id = r.id
             JOIN matrices x ON x.id = m.matrix_id
             WHERE rr.k = ?1
             ORDER BY r.id, m.step_index",
        )
        .map_err(|err| format!("failed to prepare shortcut query: {err}"))?;

    let mut rows = stmt
        .query([k])
        .map_err(|err| format!("failed to query shortcut paths: {err}"))?;

    let mut out = Vec::new();

    let mut current_id: Option<i64> = None;
    let mut current_label = String::new();
    let mut current_kind = String::new();
    let mut current_ref = String::new();
    let mut current_max_dim = 0usize;
    let mut current_max_entry = 0u32;
    let mut current_matrices: Vec<DynMatrix> = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read shortcut row: {err}"))?
    {
        let result_id: i64 = row.get(0).map_err(sqlite_value_err)?;
        let guide_label: String = row.get(1).map_err(sqlite_value_err)?;
        let source_kind: String = row.get(2).map_err(sqlite_value_err)?;
        let path_signature: String = row.get(3).map_err(sqlite_value_err)?;
        let max_dim_raw: i64 = row.get(4).map_err(sqlite_value_err)?;
        let max_entry_raw: i64 = row.get(5).map_err(sqlite_value_err)?;
        let data_json: String = row.get(7).map_err(sqlite_value_err)?;

        if current_id != Some(result_id) {
            if let Some(previous_id) = current_id {
                out.push(PathCandidate {
                    artifact_id_hint: format!("sqlite-shortcut-{previous_id}"),
                    label: current_label.clone(),
                    source_kind: current_kind.clone(),
                    source_ref: current_ref.clone(),
                    max_dim: current_max_dim,
                    max_entry: current_max_entry,
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_id = Some(result_id);
            current_label = format!("shortcut_result_{result_id}:{guide_label}");
            current_kind = format!("sqlite_{}", source_kind.replace(' ', "_"));
            current_ref = format!("sqlite:shortcut_path_results:{result_id}:{path_signature}");
            current_max_dim = usize::try_from(max_dim_raw)
                .map_err(|_| format!("invalid shortcut max_dim: {max_dim_raw}"))?;
            current_max_entry = u32::try_from(max_entry_raw)
                .map_err(|_| format!("invalid shortcut max_entry: {max_entry_raw}"))?;
            current_matrices.clear();
        }

        current_matrices.push(sqlite_matrix_from_json(&data_json)?);
    }

    if let Some(previous_id) = current_id {
        out.push(PathCandidate {
            artifact_id_hint: format!("sqlite-shortcut-{previous_id}"),
            label: current_label,
            source_kind: current_kind,
            source_ref: current_ref,
            max_dim: current_max_dim,
            max_entry: current_max_entry,
            matrices: current_matrices,
        });
    }

    Ok(out)
}

fn load_graph_candidates(conn: &Connection, k: i64) -> Result<Vec<PathCandidate>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.ordinal, r.path_signature, rr.max_dim, rr.max_entry, s.step_index, from_m.data_json, to_m.data_json
             FROM graph_path_results r
             JOIN graph_path_runs rr ON rr.id = r.run_id
             JOIN graph_path_steps s ON s.result_id = r.id
             JOIN matrices from_m ON from_m.id = s.from_matrix_id
             JOIN matrices to_m ON to_m.id = s.to_matrix_id
             WHERE rr.k = ?1
             ORDER BY r.id, s.step_index",
        )
        .map_err(|err| format!("failed to prepare graph query: {err}"))?;

    let mut rows = stmt
        .query([k])
        .map_err(|err| format!("failed to query graph paths: {err}"))?;

    let mut out = Vec::new();

    let mut current_id: Option<i64> = None;
    let mut current_ordinal = 0i64;
    let mut current_ref = String::new();
    let mut current_max_dim = 0usize;
    let mut current_max_entry = 0u32;
    let mut current_matrices: Vec<DynMatrix> = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read graph row: {err}"))?
    {
        let result_id: i64 = row.get(0).map_err(sqlite_value_err)?;
        let ordinal: i64 = row.get(1).map_err(sqlite_value_err)?;
        let path_signature: String = row.get(2).map_err(sqlite_value_err)?;
        let max_dim_raw: i64 = row.get(3).map_err(sqlite_value_err)?;
        let max_entry_raw: i64 = row.get(4).map_err(sqlite_value_err)?;
        let step_index: i64 = row.get(5).map_err(sqlite_value_err)?;
        let from_data_json: String = row.get(6).map_err(sqlite_value_err)?;
        let to_data_json: String = row.get(7).map_err(sqlite_value_err)?;

        if current_id != Some(result_id) {
            if let Some(previous_id) = current_id {
                out.push(PathCandidate {
                    artifact_id_hint: format!("sqlite-graph-{previous_id}"),
                    label: format!("graph_result_{previous_id}:ordinal_{current_ordinal}"),
                    source_kind: "sqlite_graph".to_string(),
                    source_ref: current_ref.clone(),
                    max_dim: current_max_dim,
                    max_entry: current_max_entry,
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_id = Some(result_id);
            current_ordinal = ordinal;
            current_ref = format!("sqlite:graph_path_results:{result_id}:{path_signature}");
            current_max_dim = usize::try_from(max_dim_raw)
                .map_err(|_| format!("invalid graph max_dim: {max_dim_raw}"))?;
            current_max_entry = u32::try_from(max_entry_raw)
                .map_err(|_| format!("invalid graph max_entry: {max_entry_raw}"))?;
            current_matrices.clear();
        }

        if step_index == 0 {
            current_matrices.push(sqlite_matrix_from_json(&from_data_json)?);
        }
        current_matrices.push(sqlite_matrix_from_json(&to_data_json)?);
    }

    if let Some(previous_id) = current_id {
        out.push(PathCandidate {
            artifact_id_hint: format!("sqlite-graph-{previous_id}"),
            label: format!("graph_result_{previous_id}:ordinal_{current_ordinal}"),
            source_kind: "sqlite_graph".to_string(),
            source_ref: current_ref,
            max_dim: current_max_dim,
            max_entry: current_max_entry,
            matrices: current_matrices,
        });
    }

    Ok(out)
}

fn sqlite_matrix_from_json(data_json: &str) -> Result<DynMatrix, String> {
    let rows = serde_json::from_str::<Vec<Vec<u32>>>(data_json)
        .map_err(|err| format!("failed to parse matrix JSON: {err}"))?;
    case_matrix(&rows)
}

fn sqlite_value_err(err: rusqlite::Error) -> String {
    format!("sqlite value decode error: {err}")
}

fn case_matrix(rows: &[Vec<u32>]) -> Result<DynMatrix, String> {
    if rows.is_empty() {
        return Err("matrix must have at least one row".to_string());
    }
    let dim = rows.len();
    if rows.iter().any(|row| row.len() != dim) {
        return Err("matrix must be square".to_string());
    }
    Ok(DynMatrix::new(
        dim,
        dim,
        rows.iter().flat_map(|row| row.iter().copied()).collect(),
    ))
}

fn candidate_endpoint_orientation(
    candidate: &PathCandidate,
    source_canonical: &DynMatrix,
    target_canonical: &DynMatrix,
) -> Option<bool> {
    let first = candidate.matrices.first()?.canonical_perm();
    let last = candidate.matrices.last()?.canonical_perm();
    if first == *source_canonical && last == *target_canonical {
        return Some(false);
    }
    if first == *target_canonical && last == *source_canonical {
        return Some(true);
    }
    None
}

fn endpoint_orientation(
    source: &DynMatrix,
    target: &DynMatrix,
    requested_source: &DynMatrix,
    requested_target: &DynMatrix,
) -> Option<bool> {
    let source_canonical = source.canonical_perm();
    let target_canonical = target.canonical_perm();
    let requested_source_canonical = requested_source.canonical_perm();
    let requested_target_canonical = requested_target.canonical_perm();
    if source_canonical == requested_source_canonical
        && target_canonical == requested_target_canonical
    {
        return Some(false);
    }
    if source_canonical == requested_target_canonical
        && target_canonical == requested_source_canonical
    {
        return Some(true);
    }
    None
}

fn orient_guide_artifact_to_endpoints(
    mut artifact: GuideArtifact,
    requested_source: &DynMatrix,
    requested_target: &DynMatrix,
) -> Result<Option<GuideArtifact>, String> {
    let Some(reverse) = endpoint_orientation(
        &artifact.endpoints.source,
        &artifact.endpoints.target,
        requested_source,
        requested_target,
    ) else {
        return Ok(None);
    };

    if reverse {
        let GuideArtifactPayload::FullPath { path } = &artifact.payload;
        let reversed = reverse_dyn_sse_path(path);
        validate_sse_path_dyn(requested_source, requested_target, &reversed).map_err(|err| {
            format!(
                "failed to orient guide artifact {}: {err}",
                artifact_label(&artifact)
            )
        })?;
        artifact.endpoints = GuideArtifactEndpoints {
            source: requested_source.clone(),
            target: requested_target.clone(),
        };
        artifact.payload = GuideArtifactPayload::FullPath { path: reversed };
        artifact.provenance.label = artifact
            .provenance
            .label
            .as_ref()
            .map(|label| format!("{label} [reversed]"));
        artifact.provenance.source_ref = artifact
            .provenance
            .source_ref
            .as_ref()
            .map(|source_ref| format!("{source_ref}#reversed"));
    }

    Ok(Some(artifact))
}

fn artifact_label(artifact: &GuideArtifact) -> &str {
    artifact
        .artifact_id
        .as_deref()
        .or(artifact.provenance.label.as_deref())
        .unwrap_or("<unnamed>")
}

fn reverse_dyn_sse_path(path: &DynSsePath) -> DynSsePath {
    DynSsePath {
        matrices: path.matrices.iter().cloned().rev().collect(),
        steps: path
            .steps
            .iter()
            .rev()
            .map(|step| EsseStep {
                u: step.v.clone(),
                v: step.u.clone(),
            })
            .collect(),
    }
}

fn materialize_candidate(candidate: &PathCandidate) -> Result<DynSsePath, String> {
    if candidate.matrices.len() < 2 {
        return Err("candidate path has fewer than 2 matrices".to_string());
    }

    let path_max_dim = candidate
        .matrices
        .iter()
        .map(|matrix| matrix.rows)
        .max()
        .unwrap_or(0);
    let path_max_entry = candidate
        .matrices
        .iter()
        .flat_map(|matrix| matrix.data.iter().copied())
        .max()
        .unwrap_or(0);

    let max_dim = candidate.max_dim.max(path_max_dim);
    let max_entry = candidate.max_entry.max(path_max_entry).max(7);

    let mut full_path = DynSsePath {
        matrices: vec![candidate.matrices[0].clone()],
        steps: Vec::new(),
    };

    for window in candidate.matrices.windows(2) {
        if window[0] == window[1] {
            continue;
        }
        let segment = match search_segment(&window[0], &window[1], max_dim, max_entry, 1)? {
            SearchRunResult::Equivalent(path) => path,
            SearchRunResult::EquivalentByConcreteShift(_) => {
                let fallback_lag = 3usize;
                match search_segment(&window[0], &window[1], max_dim, max_entry, fallback_lag)? {
                    SearchRunResult::Equivalent(path) => path,
                    SearchRunResult::EquivalentByConcreteShift(_) => {
                        return Err(
                            "segment only resolved via concrete-shift witness; explicit path replay unavailable"
                                .to_string(),
                        )
                    }
                    SearchRunResult::NotEquivalent(reason) => {
                        return Err(format!(
                            "segment fallback unexpectedly not-equivalent: {reason}"
                        ))
                    }
                    SearchRunResult::Unknown => {
                        return Err(
                            "segment fallback search returned unknown after concrete-shift replay"
                                .to_string(),
                        )
                    }
                }
            }
            SearchRunResult::NotEquivalent(reason) => {
                return Err(format!("segment unexpectedly not-equivalent: {reason}"))
            }
            SearchRunResult::Unknown => return Err("segment search returned unknown".to_string()),
        };

        if segment.matrices.first() != Some(&window[0])
            || segment.matrices.last() != Some(&window[1])
        {
            return Err("segment witness endpoints do not match candidate segment".to_string());
        }

        if full_path.matrices.last() != segment.matrices.first() {
            return Err("segment witness does not stitch with accumulated path".to_string());
        }

        full_path.steps.extend(segment.steps);
        full_path
            .matrices
            .extend(segment.matrices.into_iter().skip(1));
    }

    validate_sse_path_dyn(
        full_path
            .matrices
            .first()
            .expect("full path should include source matrix"),
        full_path
            .matrices
            .last()
            .expect("full path should include target matrix"),
        &full_path,
    )
    .map_err(|err| format!("materialized candidate path failed validation: {err}"))?;

    Ok(full_path)
}

fn search_segment(
    source: &DynMatrix,
    target: &DynMatrix,
    max_dim: usize,
    max_entry: u32,
    max_lag: usize,
) -> Result<SearchRunResult, String> {
    let request = SearchRequest {
        source: source.clone(),
        target: target.clone(),
        config: SearchConfig {
            max_lag,
            max_intermediate_dim: max_dim,
            max_entry,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: GuidedRefinementConfig::default(),
        shortcut_search: ShortcutSearchConfig::default(),
    };

    let (result, _telemetry) = execute_search_request(&request).map_err(|err| {
        format!(
            "segment {}x{} -> {}x{} search (max_lag={max_lag}) failed: {err}",
            source.rows, source.cols, target.rows, target.cols
        )
    })?;
    Ok(result)
}

fn build_artifact_from_candidate(candidate: &PathCandidate, path: DynSsePath) -> GuideArtifact {
    let lag = path.steps.len();
    let source = path
        .matrices
        .first()
        .expect("materialized path should have source")
        .clone();
    let target = path
        .matrices
        .last()
        .expect("materialized path should have target")
        .clone();
    let endpoint_max_dim = source.rows.max(target.rows);
    GuideArtifact {
        artifact_id: Some(slugify(&format!("k3-{}", candidate.artifact_id_hint))),
        endpoints: GuideArtifactEndpoints { source, target },
        payload: GuideArtifactPayload::FullPath { path },
        provenance: GuideArtifactProvenance {
            source_kind: Some(candidate.source_kind.clone()),
            label: Some(candidate.label.clone()),
            source_ref: Some(candidate.source_ref.clone()),
        },
        validation: GuideArtifactValidation::WitnessValidated,
        compatibility: GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch],
            max_endpoint_dim: Some(endpoint_max_dim),
        },
        quality: GuideArtifactQuality {
            lag: Some(lag),
            cost: Some(lag),
            score: None,
        },
    }
}

fn path_key(path: &DynSsePath) -> String {
    path.matrices
        .iter()
        .map(|matrix| {
            format!(
                "{}x{}:{}",
                matrix.rows,
                matrix.cols,
                matrix
                    .data
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn build_lind_marcus_reference_guide() -> Result<GuideArtifact, String> {
    let steps = lind_marcus_baker_steps();
    if steps.is_empty() {
        return Err("Lind-Marcus/Baker guide has no steps".to_string());
    }

    let source = steps[0].u.mul(&steps[0].v);
    let mut matrices = vec![source.clone()];
    for (index, step) in steps.iter().enumerate() {
        let expected = matrices
            .last()
            .expect("matrix sequence should include current matrix");
        if step.u.mul(&step.v) != *expected {
            return Err(format!(
                "Lind-Marcus/Baker step {} does not start at the current matrix",
                index + 1
            ));
        }
        matrices.push(step.v.mul(&step.u));
    }
    let target = matrices
        .last()
        .expect("matrix sequence should include target matrix")
        .clone();

    let path = DynSsePath {
        matrices,
        steps: steps.clone(),
    };
    validate_sse_path_dyn(&source, &target, &path)
        .map_err(|err| format!("Lind-Marcus/Baker reference witness failed validation: {err}"))?;

    Ok(GuideArtifact {
        artifact_id: Some("k3-lind-marcus-baker-lag7".to_string()),
        endpoints: GuideArtifactEndpoints { source, target },
        payload: GuideArtifactPayload::FullPath { path },
        provenance: GuideArtifactProvenance {
            source_kind: Some("literature_reference".to_string()),
            label: Some("Lind-Marcus/Baker lag-7 witness".to_string()),
            source_ref: Some("src/bin/check_lind_marcus_path.rs".to_string()),
        },
        validation: GuideArtifactValidation::WitnessValidated,
        compatibility: GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch],
            max_endpoint_dim: Some(2),
        },
        quality: GuideArtifactQuality {
            lag: Some(steps.len()),
            cost: Some(steps.len()),
            score: None,
        },
    })
}

fn lind_marcus_baker_steps() -> Vec<EsseStep> {
    vec![
        esse_step(
            DynMatrix::new(2, 3, vec![0, 1, 1, 1, 0, 0]),
            DynMatrix::new(3, 2, vec![2, 1, 1, 2, 0, 1]),
        ),
        esse_step(
            DynMatrix::new(3, 4, vec![1, 0, 2, 0, 0, 1, 1, 1, 0, 1, 0, 0]),
            DynMatrix::new(4, 3, vec![1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1]),
        ),
        esse_step(
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 0, 2, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1]),
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0]),
        ),
        esse_step(
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0]),
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0]),
        ),
        esse_step(
            DynMatrix::new(4, 4, vec![0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0]),
            DynMatrix::new(4, 4, vec![0, 1, 0, 1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0]),
        ),
        esse_step(
            DynMatrix::new(4, 3, vec![1, 0, 1, 0, 1, 0, 0, 0, 1, 1, 0, 0]),
            DynMatrix::new(3, 4, vec![0, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0]),
        ),
        esse_step(
            DynMatrix::new(3, 2, vec![1, 0, 0, 5, 0, 1]),
            DynMatrix::new(2, 3, vec![1, 1, 1, 1, 0, 1]),
        ),
    ]
}

fn esse_step(u: DynMatrix, v: DynMatrix) -> EsseStep {
    EsseStep { u, v }
}

fn best_lag(artifacts: &[GuideArtifact]) -> usize {
    artifacts
        .iter()
        .map(|artifact| artifact.quality.lag.unwrap_or(usize::MAX))
        .min()
        .unwrap_or(usize::MAX)
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}
