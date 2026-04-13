use std::fs;
use std::path::Path;

use crate::types::GuideArtifact;

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum GuideArtifactFile {
    Artifact(GuideArtifact),
    Artifacts(Vec<GuideArtifact>),
    Envelope { artifacts: Vec<GuideArtifact> },
}

pub fn load_guide_artifacts_from_path(
    path: impl AsRef<Path>,
) -> Result<Vec<GuideArtifact>, String> {
    let path = path.as_ref();
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|err| {
                format!(
                    "failed to read guide artifact directory {}: {err}",
                    path.display()
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                format!(
                    "failed to read guide artifact directory {}: {err}",
                    path.display()
                )
            })?;
        entries.sort_by(|left, right| left.path().cmp(&right.path()));

        let mut artifacts = Vec::new();
        for entry in entries {
            let entry_path = entry.path();
            if !entry_path.is_file() {
                continue;
            }
            if entry_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            artifacts.extend(load_guide_artifacts_file(&entry_path)?);
        }
        return Ok(artifacts);
    }

    load_guide_artifacts_file(path)
}

fn load_guide_artifacts_file(path: &Path) -> Result<Vec<GuideArtifact>, String> {
    let json = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read guide artifacts from {}: {err}",
            path.display()
        )
    })?;
    let parsed: GuideArtifactFile = serde_json::from_str(&json).map_err(|err| {
        format!(
            "failed to parse guide artifacts from {} as JSON: {err}",
            path.display()
        )
    })?;
    Ok(match parsed {
        GuideArtifactFile::Artifact(artifact) => vec![artifact],
        GuideArtifactFile::Artifacts(artifacts) => artifacts,
        GuideArtifactFile::Envelope { artifacts } => artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::load_guide_artifacts_from_path;
    use crate::search::build_full_path_guide_artifact;
    use crate::types::SearchStage;
    use crate::{matrix::DynMatrix, types::GuideArtifact};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_guide_artifacts_from_directory_collects_sorted_json_files() {
        let dir = temp_dir_path("guide-artifact-dir");
        fs::create_dir_all(&dir).unwrap();

        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let path = crate::types::DynSsePath {
            matrices: vec![source.clone()],
            steps: vec![],
        };
        let mut artifact_a = build_full_path_guide_artifact(&source, &source, &path).unwrap();
        artifact_a.artifact_id = Some("a".to_string());
        artifact_a.compatibility.supported_stages = vec![SearchStage::GuidedRefinement];
        let mut artifact_b = artifact_a.clone();
        artifact_b.artifact_id = Some("b".to_string());

        fs::write(
            dir.join("b.json"),
            format!("{}\n", serde_json::to_string_pretty(&artifact_b).unwrap()),
        )
        .unwrap();
        fs::write(
            dir.join("a.json"),
            format!("{}\n", serde_json::to_string_pretty(&artifact_a).unwrap()),
        )
        .unwrap();
        fs::write(dir.join("ignored.txt"), "not json\n").unwrap();

        let artifacts = load_guide_artifacts_from_path(&dir).unwrap();
        let artifact_ids = artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_deref().unwrap_or(""))
            .collect::<Vec<_>>();
        assert_eq!(artifact_ids, vec!["a", "b"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_guide_artifacts_from_file_accepts_envelope() {
        let path = temp_dir_path("guide-artifact-envelope.json");

        let artifact: GuideArtifact = serde_json::from_str(
            r#"{
              "artifact_id":"fixture",
              "endpoints":{"source":{"rows":2,"cols":2,"data":[1,0,0,1]},"target":{"rows":2,"cols":2,"data":[1,0,0,1]}},
              "kind":"full_path",
              "path":{"matrices":[{"rows":2,"cols":2,"data":[1,0,0,1]}],"steps":[]},
              "compatibility":{"supported_stages":["guided_refinement"]},
              "quality":{"lag":0,"cost":0}
            }"#,
        )
        .unwrap();
        fs::write(
            &path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({ "artifacts": [artifact] }))
                    .unwrap()
            ),
        )
        .unwrap();

        let artifacts = load_guide_artifacts_from_path(&path).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_id.as_deref(), Some("fixture"));

        let _ = fs::remove_file(path);
    }

    fn temp_dir_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("sse-core-{label}-{}-{nonce}", std::process::id()))
    }
}
