use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::EndpointSummary;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OutcomePoints {
    pub(crate) equivalent: i64,
    pub(crate) not_equivalent: i64,
    pub(crate) unknown: i64,
    pub(crate) timeout: i64,
    pub(crate) panic: i64,
}

impl OutcomePoints {
    pub(crate) fn for_outcome(&self, outcome: &str) -> i64 {
        match outcome {
            "equivalent" => self.equivalent,
            "not_equivalent" => self.not_equivalent,
            "unknown" => self.unknown,
            "timeout" => self.timeout,
            "panic" => self.panic,
            _ => self.panic,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct BestKnownWitness {
    pub(crate) lag: usize,
    pub(crate) elapsed_ms: u128,
    pub(crate) source: String,
}

#[derive(Debug, Default)]
pub(crate) struct ReusedResults {
    pub(crate) endpoint_best_witness: BTreeMap<String, BestKnownWitness>,
    pub(crate) sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PersistedHarnessSummary {
    #[serde(default)]
    cases: Vec<PersistedCaseSummary>,
}

#[derive(Debug, Deserialize)]
struct PersistedCaseSummary {
    endpoint: EndpointSummary,
    elapsed_ms: u128,
    result_model: PersistedResultModel,
}

#[derive(Debug, Deserialize)]
struct PersistedResultModel {
    witness_lag: Option<usize>,
}

pub(crate) fn load_reused_results(
    reuse_runs: &[PathBuf],
    reuse_dirs: &[PathBuf],
) -> Result<ReusedResults, String> {
    let mut sources = reuse_runs.to_vec();
    for dir in reuse_dirs {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(format!(
                    "failed to read reuse directory {}: {err}",
                    dir.display()
                ))
            }
        };
        let mut dir_sources = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed to read entry in {}: {err}", dir.display()))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                dir_sources.push(path);
            }
        }
        dir_sources.sort();
        sources.extend(dir_sources);
    }

    let mut reused = ReusedResults::default();
    for source in sources {
        let raw = fs::read_to_string(&source)
            .map_err(|err| format!("failed to read reuse artifact {}: {err}", source.display()))?;
        let parsed: PersistedHarnessSummary = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse reuse artifact {}: {err}", source.display()))?;
        let source_label = source.display().to_string();
        reused.sources.push(source_label.clone());

        for case in parsed.cases {
            let Some(lag) = case.result_model.witness_lag else {
                continue;
            };
            let endpoint_key = endpoint_identity_key(&case.endpoint);
            let candidate = BestKnownWitness {
                lag,
                elapsed_ms: case.elapsed_ms,
                source: source_label.clone(),
            };
            match reused.endpoint_best_witness.get(&endpoint_key) {
                Some(existing) if !best_known_witness_beats(&candidate, existing) => {}
                _ => {
                    reused.endpoint_best_witness.insert(endpoint_key, candidate);
                }
            }
        }
    }

    Ok(reused)
}

pub(crate) fn endpoint_identity_key(endpoint: &EndpointSummary) -> String {
    serde_json::to_string(&(
        endpoint.source_dim,
        endpoint.target_dim,
        &endpoint.a,
        &endpoint.b,
    ))
    .expect("endpoint identity key should serialise")
}

fn best_known_witness_beats(candidate: &BestKnownWitness, existing: &BestKnownWitness) -> bool {
    candidate.lag < existing.lag
        || (candidate.lag == existing.lag && candidate.elapsed_ms < existing.elapsed_ms)
        || (candidate.lag == existing.lag
            && candidate.elapsed_ms == existing.elapsed_ms
            && candidate.source < existing.source)
}

pub(crate) fn merge_best_known_witness(
    current: Option<BestKnownWitness>,
    historical: Option<&BestKnownWitness>,
) -> (Option<BestKnownWitness>, bool) {
    match (current, historical) {
        (Some(current), Some(historical)) => {
            if best_known_witness_beats(&current, historical) {
                (Some(current), true)
            } else {
                (Some(historical.clone()), false)
            }
        }
        (Some(current), None) => (Some(current), true),
        (None, Some(historical)) => (Some(historical.clone()), false),
        (None, None) => (None, false),
    }
}
