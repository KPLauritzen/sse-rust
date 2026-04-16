use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use sse_core::graph_moves::{
    enumerate_in_amalgamations, enumerate_one_step_insplits, enumerate_one_step_outsplits,
    enumerate_out_amalgamations,
};
use sse_core::matrix::DynMatrix;

fn main() {
    let mut paths_db: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from("research/neo4j-k3-export");
    let mut k = 3u32;
    let mut max_dim = 5usize;
    let mut max_entry = 6u32;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--paths-db" | "--sqlite" => {
                paths_db = Some(PathBuf::from(
                    args.next().expect("--paths-db requires a value"),
                ));
            }
            "--out-dir" => {
                out_dir = PathBuf::from(args.next().expect("--out-dir requires a value"));
            }
            "--k" => {
                k = args
                    .next()
                    .expect("--k requires a value")
                    .parse()
                    .expect("invalid k");
            }
            "--max-dim" => {
                max_dim = args
                    .next()
                    .expect("--max-dim requires a value")
                    .parse()
                    .expect("invalid max dim");
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .expect("--max-entry requires a value")
                    .parse()
                    .expect("invalid max entry");
            }
            "--help" | "-h" => {
                println!(
                    "usage: export_k3_paths_neo4j --paths-db PATH [--out-dir DIR] [--k N] [--max-dim N] [--max-entry N]"
                );
                println!();
                println!("Exports sqlite-recorded graph and shortcut paths to Neo4j-friendly CSV.");
                println!("Writes nodes.csv, edges.csv, and import.cypher into --out-dir.");
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let paths_db = paths_db.expect("--paths-db is required");
    let conn = Connection::open(&paths_db)
        .unwrap_or_else(|err| panic!("failed to open {}: {err}", paths_db.display()));
    let export = load_export(&conn, k, max_dim, max_entry).unwrap_or_else(|err| panic!("{err}"));
    write_export(&out_dir, &export).unwrap_or_else(|err| panic!("{err}"));

    println!("exported k={k} path graph");
    println!("  db: {}", paths_db.display());
    println!("  out_dir: {}", out_dir.display());
    println!("  nodes: {}", export.nodes.len());
    println!("  edges: {}", export.edges.len());
    println!("  recorded graph edges: {}", export.recorded_graph_edges);
    println!(
        "  inferred shortcut edges: {}",
        export.inferred_shortcut_edges
    );
    println!(
        "  ambiguous shortcut edges: {}",
        export.ambiguous_shortcut_edges
    );
    println!(
        "  unknown shortcut edges: {}",
        export.unknown_shortcut_edges
    );
}

#[derive(Clone, Debug)]
struct NodeRecord {
    node_id: String,
    matrix_key: String,
    dim: usize,
    entry_sum: u64,
    max_entry: u32,
    trace: u64,
    data_json: String,
}

#[derive(Clone, Debug)]
struct EdgeRecord {
    source_id: String,
    target_id: String,
    move_families: String,
    family_inference: String,
    path_kind: String,
    path_label: String,
    path_signature: String,
    run_id: i64,
    result_id: i64,
    step_index: usize,
}

#[derive(Clone, Debug)]
struct ExportGraph {
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    recorded_graph_edges: usize,
    inferred_shortcut_edges: usize,
    ambiguous_shortcut_edges: usize,
    unknown_shortcut_edges: usize,
}

#[derive(Clone, Debug)]
struct ShortcutPathRecord {
    run_id: i64,
    result_id: i64,
    path_label: String,
    path_signature: String,
    matrices: Vec<DynMatrix>,
}

fn load_export(
    conn: &Connection,
    k: u32,
    max_dim: usize,
    _max_entry: u32,
) -> Result<ExportGraph, String> {
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let mut graph_successor_family_cache: BTreeMap<String, BTreeMap<String, Vec<String>>> =
        BTreeMap::new();

    let graph_edges = load_graph_edges(conn, k, &mut nodes)?;
    let shortcut_paths = load_shortcut_paths(conn, k)?;

    let mut ambiguous_shortcut_edges = 0usize;
    let mut unknown_shortcut_edges = 0usize;
    for path in shortcut_paths {
        for (step_index, window) in path.matrices.windows(2).enumerate() {
            let from = window[0].canonical_perm();
            let to = window[1].canonical_perm();
            register_node(&mut nodes, &from)?;
            register_node(&mut nodes, &to)?;

            let graph_families = graph_successor_family_cache
                .entry(matrix_key(&from))
                .or_insert_with(|| build_graph_successor_family_map(&from, max_dim))
                .get(&matrix_key(&to))
                .cloned()
                .unwrap_or_default();
            let (families, family_inference) = if graph_families.is_empty() {
                let fallback = fallback_factorisation_families(&from, &to);
                if fallback.is_empty() {
                    unknown_shortcut_edges += 1;
                    (vec!["unknown".to_string()], "unknown")
                } else if fallback.len() == 1 {
                    (fallback, "dim_fallback")
                } else {
                    ambiguous_shortcut_edges += 1;
                    (fallback, "dim_fallback_ambiguous")
                }
            } else if graph_families.len() == 1 {
                (graph_families, "graph_inferred")
            } else {
                ambiguous_shortcut_edges += 1;
                (graph_families, "graph_ambiguous")
            };

            edges.push(EdgeRecord {
                source_id: matrix_key(&from),
                target_id: matrix_key(&to),
                move_families: families.join("|"),
                family_inference: family_inference.to_string(),
                path_kind: "shortcut".to_string(),
                path_label: path.path_label.clone(),
                path_signature: path.path_signature.clone(),
                run_id: path.run_id,
                result_id: path.result_id,
                step_index,
            });
        }
    }

    let recorded_graph_edges = graph_edges.len();
    let inferred_shortcut_edges = edges.len();
    edges.extend(graph_edges);
    edges.sort_by(|left, right| {
        (
            left.path_kind.as_str(),
            left.run_id,
            left.result_id,
            left.step_index,
            left.source_id.as_str(),
            left.target_id.as_str(),
        )
            .cmp(&(
                right.path_kind.as_str(),
                right.run_id,
                right.result_id,
                right.step_index,
                right.source_id.as_str(),
                right.target_id.as_str(),
            ))
    });

    Ok(ExportGraph {
        nodes: nodes.into_values().collect(),
        edges,
        recorded_graph_edges,
        inferred_shortcut_edges,
        ambiguous_shortcut_edges,
        unknown_shortcut_edges,
    })
}

fn load_graph_edges(
    conn: &Connection,
    k: u32,
    nodes: &mut BTreeMap<String, NodeRecord>,
) -> Result<Vec<EdgeRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                r.run_id,
                r.id,
                r.ordinal,
                r.path_signature,
                s.step_index,
                s.family,
                mf.data_json,
                mt.data_json
             FROM graph_path_results r
             JOIN graph_path_runs rr ON rr.id = r.run_id
             JOIN graph_path_steps s ON s.result_id = r.id
             JOIN matrices mf ON mf.id = s.from_matrix_id
             JOIN matrices mt ON mt.id = s.to_matrix_id
             WHERE rr.k = ?1
             ORDER BY r.run_id, r.id, s.step_index",
        )
        .map_err(|err| format!("failed to prepare graph edge query: {err}"))?;
    let mut rows = stmt
        .query(params![k as i64])
        .map_err(|err| format!("failed to query graph edges: {err}"))?;

    let mut edges = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read graph edge row: {err}"))?
    {
        let run_id: i64 = row
            .get(0)
            .map_err(|err| format!("bad graph run id: {err}"))?;
        let result_id: i64 = row
            .get(1)
            .map_err(|err| format!("bad graph result id: {err}"))?;
        let ordinal: i64 = row
            .get(2)
            .map_err(|err| format!("bad graph ordinal: {err}"))?;
        let path_signature: String = row
            .get(3)
            .map_err(|err| format!("bad graph path signature: {err}"))?;
        let step_index: i64 = row
            .get(4)
            .map_err(|err| format!("bad graph step index: {err}"))?;
        let family: String = row
            .get(5)
            .map_err(|err| format!("bad graph family: {err}"))?;
        let from = parse_matrix_json(
            &row.get::<_, String>(6)
                .map_err(|err| format!("bad graph from matrix json: {err}"))?,
        )?
        .canonical_perm();
        let to = parse_matrix_json(
            &row.get::<_, String>(7)
                .map_err(|err| format!("bad graph to matrix json: {err}"))?,
        )?
        .canonical_perm();

        register_node(nodes, &from)?;
        register_node(nodes, &to)?;
        edges.push(EdgeRecord {
            source_id: matrix_key(&from),
            target_id: matrix_key(&to),
            move_families: family,
            family_inference: "recorded".to_string(),
            path_kind: "graph".to_string(),
            path_label: format!("graph_path_result_{result_id}_ordinal_{ordinal}"),
            path_signature,
            run_id,
            result_id,
            step_index: step_index as usize,
        });
    }

    Ok(edges)
}

fn load_shortcut_paths(conn: &Connection, k: u32) -> Result<Vec<ShortcutPathRecord>, String> {
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'shortcut_path_results'",
            [],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to probe shortcut tables: {err}"))?;
    if table_exists == 0 {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT
                r.run_id,
                r.id,
                r.guide_label,
                r.path_signature,
                m.step_index,
                mm.data_json
             FROM shortcut_path_results r
             JOIN shortcut_path_runs rr ON rr.id = r.run_id
             JOIN shortcut_path_matrices m ON m.result_id = r.id
             JOIN matrices mm ON mm.id = m.matrix_id
             WHERE rr.k = ?1
             ORDER BY r.run_id, r.id, m.step_index",
        )
        .map_err(|err| format!("failed to prepare shortcut path query: {err}"))?;
    let mut rows = stmt
        .query(params![k as i64])
        .map_err(|err| format!("failed to query shortcut paths: {err}"))?;

    let mut paths = Vec::new();
    let mut current_run_id = None;
    let mut current_result_id = None;
    let mut current_label = String::new();
    let mut current_signature = String::new();
    let mut current_matrices = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read shortcut path row: {err}"))?
    {
        let run_id: i64 = row
            .get(0)
            .map_err(|err| format!("bad shortcut run id: {err}"))?;
        let result_id: i64 = row
            .get(1)
            .map_err(|err| format!("bad shortcut result id: {err}"))?;
        let guide_label: String = row
            .get(2)
            .map_err(|err| format!("bad shortcut guide label: {err}"))?;
        let path_signature: String = row
            .get(3)
            .map_err(|err| format!("bad shortcut path signature: {err}"))?;
        let matrix = parse_matrix_json(
            &row.get::<_, String>(5)
                .map_err(|err| format!("bad shortcut matrix json: {err}"))?,
        )?;

        if current_result_id != Some(result_id) {
            if let Some(prev_result_id) = current_result_id {
                paths.push(ShortcutPathRecord {
                    run_id: current_run_id.expect("current_run_id must exist"),
                    result_id: prev_result_id,
                    path_label: format!(
                        "shortcut_path_result_{prev_result_id}_from_{current_label}"
                    ),
                    path_signature: current_signature.clone(),
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_run_id = Some(run_id);
            current_result_id = Some(result_id);
            current_label = guide_label;
            current_signature = path_signature;
        }

        current_matrices.push(matrix);
    }

    if let Some(result_id) = current_result_id {
        paths.push(ShortcutPathRecord {
            run_id: current_run_id.expect("current_run_id must exist"),
            result_id,
            path_label: format!("shortcut_path_result_{result_id}_from_{current_label}"),
            path_signature: current_signature,
            matrices: current_matrices,
        });
    }

    Ok(paths)
}

fn register_node(
    nodes: &mut BTreeMap<String, NodeRecord>,
    matrix: &DynMatrix,
) -> Result<(), String> {
    let canon = matrix.canonical_perm();
    let node_id = matrix_key(&canon);
    if nodes.contains_key(&node_id) {
        return Ok(());
    }

    nodes.insert(
        node_id.clone(),
        NodeRecord {
            node_id,
            matrix_key: matrix_key(&canon),
            dim: canon.rows,
            entry_sum: canon.entry_sum(),
            max_entry: canon.max_entry(),
            trace: canon.trace(),
            data_json: matrix_json(&canon)?,
        },
    );
    Ok(())
}

fn build_graph_successor_family_map(
    current: &DynMatrix,
    max_dim: usize,
) -> BTreeMap<String, Vec<String>> {
    let current = current.canonical_perm();
    let mut families_by_target: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    if current.rows < max_dim {
        for witness in enumerate_one_step_outsplits(&current) {
            let next = witness.outsplit.canonical_perm();
            families_by_target
                .entry(matrix_key(&next))
                .or_default()
                .insert("outsplit".to_string());
        }
        for witness in enumerate_one_step_insplits(&current) {
            let next = witness.outsplit.canonical_perm();
            families_by_target
                .entry(matrix_key(&next))
                .or_default()
                .insert("insplit".to_string());
        }
    }

    if current.rows > 2 {
        for witness in enumerate_out_amalgamations(&current) {
            let next = witness.outsplit.canonical_perm();
            families_by_target
                .entry(matrix_key(&next))
                .or_default()
                .insert("out_amalgamation".to_string());
        }
        for witness in enumerate_in_amalgamations(&current) {
            let next = witness.outsplit.canonical_perm();
            families_by_target
                .entry(matrix_key(&next))
                .or_default()
                .insert("in_amalgamation".to_string());
        }
    }

    families_by_target
        .into_iter()
        .map(|(target, families)| (target, families.into_iter().collect()))
        .collect()
}

fn fallback_factorisation_families(current: &DynMatrix, target: &DynMatrix) -> Vec<String> {
    match (current.rows, target.rows) {
        (2, 2) => vec!["square_factorisation_2x2".to_string()],
        (2, 3) => vec!["rectangular_factorisation_2x3".to_string()],
        (3, 2) => vec!["rectangular_factorisation_3x3_to_2".to_string()],
        (3, 3) => vec![
            "square_factorisation_3x3".to_string(),
            "elementary_conjugation_3x3".to_string(),
            "opposite_shear_conjugation_3x3".to_string(),
            "parallel_shear_conjugation_3x3".to_string(),
            "convergent_shear_conjugation_3x3".to_string(),
        ],
        (3, 4) => vec!["binary_sparse_rectangular_factorisation_3x3_to_4".to_string()],
        (4, 3) => vec!["binary_sparse_rectangular_factorisation_4x3_to_3".to_string()],
        (4, 4) => vec!["elementary_conjugation".to_string()],
        (4, 5) => vec![
            "single_row_split_4x4_to_5x5".to_string(),
            "single_column_split_4x4_to_5x5".to_string(),
            "binary_sparse_rectangular_factorisation_4x4_to_5".to_string(),
        ],
        (5, 4) => vec![
            "single_row_amalgamation_5x5_to_4x4".to_string(),
            "binary_sparse_rectangular_factorisation_5x5_to_4".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn parse_matrix_json(raw: &str) -> Result<DynMatrix, String> {
    let rows: Vec<Vec<u32>> =
        serde_json::from_str(raw).map_err(|err| format!("failed to parse matrix json: {err}"))?;
    let dim = rows.len();
    if dim == 0 {
        return Err("matrix json must not be empty".to_string());
    }
    if rows.iter().any(|row| row.len() != dim) {
        return Err("matrix json must be square".to_string());
    }
    Ok(DynMatrix::new(
        dim,
        dim,
        rows.into_iter().flatten().collect(),
    ))
}

fn matrix_json(matrix: &DynMatrix) -> Result<String, String> {
    let rows: Vec<Vec<u32>> = (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix.get(row, col))
                .collect::<Vec<_>>()
        })
        .collect();
    serde_json::to_string(&rows).map_err(|err| format!("failed to serialize matrix json: {err}"))
}

fn matrix_key(matrix: &DynMatrix) -> String {
    let mut key = format!("{}x{}:", matrix.rows, matrix.cols);
    for (idx, value) in matrix.data.iter().enumerate() {
        if idx > 0 {
            key.push(',');
        }
        key.push_str(&value.to_string());
    }
    key
}

fn write_export(out_dir: &Path, export: &ExportGraph) -> Result<(), String> {
    fs::create_dir_all(out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;
    write_nodes_csv(&out_dir.join("nodes.csv"), &export.nodes)?;
    write_edges_csv(&out_dir.join("edges.csv"), &export.edges)?;
    write_import_cypher(&out_dir.join("import.cypher"))?;
    Ok(())
}

fn write_nodes_csv(path: &Path, nodes: &[NodeRecord]) -> Result<(), String> {
    let file =
        File::create(path).map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(
        out,
        "node_id,matrix_key,dim,entry_sum,max_entry,trace,data_json"
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    for node in nodes {
        write_csv_row(
            &mut out,
            &[
                &node.node_id,
                &node.matrix_key,
                &node.dim.to_string(),
                &node.entry_sum.to_string(),
                &node.max_entry.to_string(),
                &node.trace.to_string(),
                &node.data_json,
            ],
        )?;
    }
    out.flush()
        .map_err(|err| format!("failed to flush {}: {err}", path.display()))
}

fn write_edges_csv(path: &Path, edges: &[EdgeRecord]) -> Result<(), String> {
    let file =
        File::create(path).map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    let mut out = BufWriter::new(file);
    writeln!(
        out,
        "source_id,target_id,move_families,family_inference,path_kind,path_label,path_signature,run_id,result_id,step_index"
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    for edge in edges {
        write_csv_row(
            &mut out,
            &[
                &edge.source_id,
                &edge.target_id,
                &edge.move_families,
                &edge.family_inference,
                &edge.path_kind,
                &edge.path_label,
                &edge.path_signature,
                &edge.run_id.to_string(),
                &edge.result_id.to_string(),
                &edge.step_index.to_string(),
            ],
        )?;
    }
    out.flush()
        .map_err(|err| format!("failed to flush {}: {err}", path.display()))
}

fn write_import_cypher(path: &Path) -> Result<(), String> {
    let file =
        File::create(path).map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    let mut out = BufWriter::new(file);
    out.write_all(
        br#"CREATE CONSTRAINT matrix_node_id IF NOT EXISTS
FOR (m:Matrix)
REQUIRE m.node_id IS UNIQUE;

LOAD CSV WITH HEADERS FROM 'file:///nodes.csv' AS row
MERGE (m:Matrix {node_id: row.node_id})
SET m.matrix_key = row.matrix_key,
    m.dim = toInteger(row.dim),
    m.entry_sum = toInteger(row.entry_sum),
    m.max_entry = toInteger(row.max_entry),
    m.trace = toInteger(row.trace),
    m.data_json = row.data_json;

LOAD CSV WITH HEADERS FROM 'file:///edges.csv' AS row
MATCH (src:Matrix {node_id: row.source_id})
MATCH (dst:Matrix {node_id: row.target_id})
CREATE (src)-[:MOVE {
    move_families: split(row.move_families, '|'),
    family_inference: row.family_inference,
    path_kind: row.path_kind,
    path_label: row.path_label,
    path_signature: row.path_signature,
    run_id: toInteger(row.run_id),
    result_id: toInteger(row.result_id),
    step_index: toInteger(row.step_index)
}]->(dst);
"#,
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    out.flush()
        .map_err(|err| format!("failed to flush {}: {err}", path.display()))
}

fn write_csv_row(out: &mut impl Write, fields: &[&str]) -> Result<(), String> {
    for (idx, field) in fields.iter().enumerate() {
        if idx > 0 {
            write!(out, ",").map_err(|err| format!("failed to write csv delimiter: {err}"))?;
        }
        write!(out, "{}", csv_escape(field))
            .map_err(|err| format!("failed to write csv field: {err}"))?;
    }
    writeln!(out).map_err(|err| format!("failed to terminate csv row: {err}"))
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{csv_escape, fallback_factorisation_families};
    use sse_core::matrix::DynMatrix;

    #[test]
    fn csv_escape_quotes_commas_and_newlines() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn fallback_factorisation_families_keep_explicit_4x4_to_5x5_labels_ahead_of_sparse() {
        let current = DynMatrix::new(4, 4, vec![1; 16]);
        let target = DynMatrix::new(5, 5, vec![1; 25]);

        assert_eq!(
            fallback_factorisation_families(&current, &target),
            vec![
                "single_row_split_4x4_to_5x5".to_string(),
                "single_column_split_4x4_to_5x5".to_string(),
                "binary_sparse_rectangular_factorisation_4x4_to_5".to_string(),
            ]
        );
    }

    #[test]
    fn fallback_factorisation_families_keep_explicit_5x5_to_4x4_labels_ahead_of_sparse() {
        let current = DynMatrix::new(5, 5, vec![1; 25]);
        let target = DynMatrix::new(4, 4, vec![1; 16]);

        assert_eq!(
            fallback_factorisation_families(&current, &target),
            vec![
                "single_row_amalgamation_5x5_to_4x4".to_string(),
                "binary_sparse_rectangular_factorisation_5x5_to_4".to_string(),
            ]
        );
    }
}
