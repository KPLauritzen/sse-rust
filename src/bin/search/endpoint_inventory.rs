use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::search::build_full_path_guide_artifact;
use sse_core::types::{
    DynSsePath, EndpointExactMeetSurface, EndpointExactMeetWitness, GuideArtifactCompatibility,
    GuideArtifactPayload, GuideArtifactProvenance, SearchDirection, SearchRequest, SearchStage,
    SearchTelemetry,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ControlGuideSpec {
    pub(super) class: String,
    pub(super) path: String,
    pub(super) artifact_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) struct EndpointWitnessInventory {
    pub(super) artifact_kind: &'static str,
    pub(super) source: Vec<Vec<u32>>,
    pub(super) target: Vec<Vec<u32>>,
    pub(super) requested_cap: usize,
    pub(super) retained_count: usize,
    pub(super) orientation_status: &'static str,
    pub(super) orientation_note: &'static str,
    pub(super) controls_loaded: Vec<EndpointWitnessControlSummary>,
    pub(super) rows: Vec<EndpointWitnessInventoryRow>,
    pub(super) emitted_guide_artifacts: Vec<EndpointWitnessGuideArtifactOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) struct EndpointWitnessControlSummary {
    pub(super) class: String,
    pub(super) artifact_id: Option<String>,
    pub(super) label: Option<String>,
    pub(super) source_ref: String,
    pub(super) reconstructed_path_length: usize,
    pub(super) full_path_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) struct EndpointWitnessInventoryRow {
    pub(super) retained_rank: usize,
    pub(super) retained_index: usize,
    pub(super) meet_lag: usize,
    pub(super) reconstructed_path_length: usize,
    pub(super) endpoint_orientation: &'static str,
    pub(super) meeting_state_signature: String,
    pub(super) full_path_signature: String,
    pub(super) full_path_hash: String,
    pub(super) control_matches: Vec<EndpointWitnessControlMatch>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) struct EndpointWitnessControlMatch {
    pub(super) class: String,
    pub(super) artifact_id: Option<String>,
    pub(super) label: Option<String>,
    pub(super) source_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub(super) struct EndpointWitnessGuideArtifactOutput {
    pub(super) retained_rank: usize,
    pub(super) path: String,
    pub(super) artifact_id: String,
}

pub(super) fn maybe_write_endpoint_witness_inventory(
    request: &SearchRequest,
    telemetry: &SearchTelemetry,
    inventory_path: Option<&str>,
    control_specs: &[ControlGuideSpec],
    guide_dir: Option<&str>,
    guide_ranks: Option<&[usize]>,
) -> Result<(), String> {
    if inventory_path.is_none() && guide_dir.is_none() {
        return Ok(());
    }
    let surface = telemetry.endpoint_exact_meets.as_ref().ok_or_else(|| {
        "endpoint witness inventory requested, but no retained endpoint_exact_meets surface was produced"
            .to_string()
    })?;
    let controls = load_endpoint_witness_controls(control_specs)?;
    let mut emitted_guide_artifacts = Vec::new();
    if let Some(dir) = guide_dir {
        emitted_guide_artifacts =
            write_endpoint_witness_guide_artifacts(request, surface, dir, guide_ranks)?;
    }
    let inventory =
        build_endpoint_witness_inventory(request, surface, &controls, emitted_guide_artifacts);
    if let Some(path) = inventory_path {
        let json = serde_json::to_string_pretty(&inventory)
            .map_err(|err| format!("failed to serialize endpoint witness inventory: {err}"))?;
        fs::write(path, format!("{json}\n")).map_err(|err| {
            format!("failed to write endpoint witness inventory to {path}: {err}")
        })?;
    }
    Ok(())
}

fn load_endpoint_witness_controls(
    specs: &[ControlGuideSpec],
) -> Result<Vec<EndpointWitnessLoadedControl>, String> {
    let mut controls = Vec::new();
    for spec in specs {
        let artifacts = load_guide_artifacts_from_path(&spec.path)?;
        let mut matched = 0usize;
        for artifact in artifacts {
            if let Some(expected_id) = spec.artifact_id.as_deref() {
                if artifact.artifact_id.as_deref() != Some(expected_id) {
                    continue;
                }
            }
            let GuideArtifactPayload::FullPath { path } = &artifact.payload;
            matched += 1;
            controls.push(EndpointWitnessLoadedControl {
                class: spec.class.clone(),
                artifact_id: artifact.artifact_id.clone(),
                label: artifact.provenance.label.clone(),
                source_ref: control_source_ref(spec),
                reconstructed_path_length: path.steps.len(),
                full_path_signature: witness_matrix_signature(path),
                full_path_hash: stable_path_hash(path),
            });
        }
        if spec.artifact_id.is_some() && matched == 0 {
            return Err(format!(
                "control guide {} did not contain artifact_id {}",
                spec.path,
                spec.artifact_id.as_deref().unwrap_or("")
            ));
        }
    }
    Ok(controls)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EndpointWitnessLoadedControl {
    pub(super) class: String,
    pub(super) artifact_id: Option<String>,
    pub(super) label: Option<String>,
    pub(super) source_ref: String,
    pub(super) reconstructed_path_length: usize,
    pub(super) full_path_signature: String,
    pub(super) full_path_hash: String,
}

fn control_source_ref(spec: &ControlGuideSpec) -> String {
    match spec.artifact_id.as_deref() {
        Some(artifact_id) => format!("{}#{}", spec.path, artifact_id),
        None => spec.path.clone(),
    }
}

pub(super) fn write_endpoint_witness_guide_artifacts(
    request: &SearchRequest,
    surface: &EndpointExactMeetSurface,
    dir: &str,
    selected_ranks: Option<&[usize]>,
) -> Result<Vec<EndpointWitnessGuideArtifactOutput>, String> {
    fs::create_dir_all(dir)
        .map_err(|err| format!("failed to create endpoint witness guide dir {dir}: {err}"))?;
    let selected = selected_ranks
        .map(|ranks| ranks.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_else(|| (1..=surface.retained.len()).collect::<BTreeSet<_>>());
    for rank in &selected {
        if *rank > surface.retained.len() {
            return Err(format!(
                "requested retained rank {rank}, but only {} exact meets were retained",
                surface.retained.len()
            ));
        }
    }
    let mut outputs = Vec::new();
    for (index, witness) in surface.retained.iter().enumerate() {
        let rank = index + 1;
        if !selected.contains(&rank) {
            continue;
        }
        let path_hash = stable_path_hash(&witness.path);
        let artifact_id = format!(
            "endpoint-exact-meet-rank-{rank}-meet-lag-{}-reconstructed-lag-{}-path-{}",
            witness.path_lag,
            witness.path.steps.len(),
            path_hash.trim_start_matches("fnv1a64:")
        );
        let mut artifact =
            build_full_path_guide_artifact(&request.source, &request.target, &witness.path)
                .map_err(|err| {
                    format!("failed to build retained exact-meet guide artifact rank {rank}: {err}")
                })?;
        artifact.artifact_id = Some(artifact_id.clone());
        artifact.provenance = GuideArtifactProvenance {
            source_kind: Some("endpoint_exact_meet_inventory".to_string()),
            label: Some(format!("retained endpoint exact meet rank {rank}")),
            source_ref: Some(format!("endpoint_exact_meet_rank:{rank}")),
        };
        artifact.compatibility = GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch],
            max_endpoint_dim: Some(request.source.rows.max(request.target.rows)),
        };
        let output_path = Path::new(dir).join(format!("rank-{rank:03}-{artifact_id}.json"));
        let json = serde_json::to_string_pretty(&artifact).map_err(|err| {
            format!("failed to serialize retained exact-meet guide artifact rank {rank}: {err}")
        })?;
        fs::write(&output_path, format!("{json}\n")).map_err(|err| {
            format!(
                "failed to write retained exact-meet guide artifact rank {rank} to {}: {err}",
                output_path.display()
            )
        })?;
        outputs.push(EndpointWitnessGuideArtifactOutput {
            retained_rank: rank,
            path: output_path.display().to_string(),
            artifact_id,
        });
    }
    Ok(outputs)
}

pub(super) fn build_endpoint_witness_inventory(
    request: &SearchRequest,
    surface: &EndpointExactMeetSurface,
    controls: &[EndpointWitnessLoadedControl],
    emitted_guide_artifacts: Vec<EndpointWitnessGuideArtifactOutput>,
) -> EndpointWitnessInventory {
    EndpointWitnessInventory {
        artifact_kind: "endpoint_exact_meet_witness_inventory",
        source: dyn_matrix_to_vecs(&request.source),
        target: dyn_matrix_to_vecs(&request.target),
        requested_cap: surface.requested_cap,
        retained_count: surface.retained.len(),
        orientation_status: endpoint_witness_orientation_status(surface),
        orientation_note: endpoint_witness_orientation_note(surface),
        controls_loaded: controls
            .iter()
            .map(|control| EndpointWitnessControlSummary {
                class: control.class.clone(),
                artifact_id: control.artifact_id.clone(),
                label: control.label.clone(),
                source_ref: control.source_ref.clone(),
                reconstructed_path_length: control.reconstructed_path_length,
                full_path_hash: control.full_path_hash.clone(),
            })
            .collect(),
        rows: surface
            .retained
            .iter()
            .enumerate()
            .map(|(index, witness)| endpoint_witness_inventory_row(index, witness, controls))
            .collect(),
        emitted_guide_artifacts,
    }
}

fn endpoint_witness_inventory_row(
    index: usize,
    witness: &EndpointExactMeetWitness,
    controls: &[EndpointWitnessLoadedControl],
) -> EndpointWitnessInventoryRow {
    let full_path_signature = witness_matrix_signature(&witness.path);
    let full_path_hash = stable_signature_hash(&full_path_signature);
    EndpointWitnessInventoryRow {
        retained_rank: index + 1,
        retained_index: index,
        meet_lag: witness.path_lag,
        reconstructed_path_length: witness.path.steps.len(),
        endpoint_orientation: endpoint_orientation_label(witness.meet_direction),
        meeting_state_signature: matrix_signature(&witness.meeting_canonical),
        control_matches: controls
            .iter()
            .filter(|control| control.full_path_signature == full_path_signature)
            .map(|control| EndpointWitnessControlMatch {
                class: control.class.clone(),
                artifact_id: control.artifact_id.clone(),
                label: control.label.clone(),
                source_ref: control.source_ref.clone(),
            })
            .collect(),
        full_path_signature,
        full_path_hash,
    }
}

fn endpoint_witness_orientation_status(surface: &EndpointExactMeetSurface) -> &'static str {
    if !surface.retained.is_empty()
        && surface
            .retained
            .iter()
            .all(|witness| witness.meet_direction.is_some())
    {
        "recorded"
    } else {
        "not_recorded"
    }
}

fn endpoint_witness_orientation_note(surface: &EndpointExactMeetSurface) -> &'static str {
    if surface.retained.is_empty() {
        "no retained exact-meet rows were present, so endpoint orientation is not applicable"
    } else if endpoint_witness_orientation_status(surface) == "recorded" {
        "endpoint_orientation records the frontier expansion direction that produced each retained exact meet: forward from the source frontier or backward from the target frontier"
    } else {
        "at least one retained exact-meet row does not record the frontier expansion direction that produced the meet"
    }
}

fn endpoint_orientation_label(direction: Option<SearchDirection>) -> &'static str {
    match direction {
        Some(SearchDirection::Forward) => "forward",
        Some(SearchDirection::Backward) => "backward",
        None => "not_recorded",
    }
}

pub(super) fn witness_matrix_signature(path: &DynSsePath) -> String {
    path.matrices
        .iter()
        .map(matrix_signature)
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn matrix_signature(matrix: &DynMatrix) -> String {
    let data = matrix
        .data
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}x{}:{data}", matrix.rows, matrix.cols)
}

pub(super) fn stable_path_hash(path: &DynSsePath) -> String {
    stable_signature_hash(&witness_matrix_signature(path))
}

fn stable_signature_hash(signature: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn dyn_matrix_to_vecs(m: &DynMatrix) -> Vec<Vec<u32>> {
    (0..m.rows)
        .map(|r| (0..m.cols).map(|c| m.data[r * m.cols + c]).collect())
        .collect()
}
