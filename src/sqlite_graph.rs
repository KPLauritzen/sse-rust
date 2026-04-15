#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, Transaction};
use serde_json::json;

use crate::matrix::DynMatrix;
use crate::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchEvent, SearchFinishedRecord, SearchObserver,
    SearchRootRecord, SearchStartRecord,
};
use crate::types::{
    FrontierMode, MoveFamilyPolicy, SearchConfig, SearchDirection, SearchRunResult, SearchStage,
    SearchTelemetry,
};

#[cfg(not(target_arch = "wasm32"))]
pub struct SqliteGraphRecorder {
    conn: Connection,
    run_id: i64,
    matrix_ids: HashMap<String, i64>,
    disabled: bool,
    error: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SqliteGraphRecorder {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path.as_ref())
            .map_err(|err| format!("failed to open {}: {err}", path.as_ref().display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .map_err(|err| format!("failed to configure sqlite busy timeout: {err}"))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|err| format!("failed to configure sqlite pragmas: {err}"))?;

        initialise_schema(&conn)?;

        Ok(Self {
            conn,
            run_id: 0,
            matrix_ids: HashMap::new(),
            disabled: false,
            error: None,
        })
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn with_connection<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self) -> Result<(), String>,
    {
        if self.disabled {
            return;
        }
        if let Err(err) = f(self) {
            self.disabled = true;
            self.error = Some(err);
        }
    }

    fn ensure_matrix_id(&mut self, matrix: &DynMatrix) -> Result<i64, String> {
        let key = matrix_key(matrix);
        if let Some(&id) = self.matrix_ids.get(&key) {
            return Ok(id);
        }

        let data_json = matrix_json(matrix)?;
        let trace = if matrix.is_square() {
            Some(matrix.trace() as i64)
        } else {
            None
        };
        self.conn
            .execute(
                "INSERT INTO matrices (
                    matrix_key,
                    rows,
                    cols,
                    data_json,
                    entry_sum,
                    max_entry,
                    trace
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(matrix_key) DO NOTHING",
                params![
                    key,
                    matrix.rows as i64,
                    matrix.cols as i64,
                    data_json,
                    matrix.entry_sum() as i64,
                    matrix.max_entry() as i64,
                    trace,
                ],
            )
            .map_err(|err| format!("failed to insert matrix {key}: {err}"))?;

        let id = self
            .conn
            .query_row(
                "SELECT id FROM matrices WHERE matrix_key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to load matrix id for {key}: {err}"))?;
        self.matrix_ids.insert(key, id);
        Ok(id)
    }

    fn insert_run(&mut self, start: &SearchStartRecord) -> Result<(), String> {
        let a_id = self.ensure_matrix_id(&start.request.source)?;
        let b_id = self.ensure_matrix_id(&start.request.target)?;
        let a_canonical_id = self.ensure_matrix_id(&start.source_canonical)?;
        let b_canonical_id = self.ensure_matrix_id(&start.target_canonical)?;
        let started_unix_ms = unix_timestamp_ms();

        self.conn
            .execute(
                "INSERT INTO search_runs (
                    started_unix_ms,
                    source_matrix_id,
                    target_matrix_id,
                    source_canonical_matrix_id,
                    target_canonical_matrix_id,
                    max_lag,
                    max_intermediate_dim,
                    max_entry,
                    search_mode,
                    frontier_mode,
                    move_family_policy,
                    stage
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    started_unix_ms,
                    a_id,
                    b_id,
                    a_canonical_id,
                    b_canonical_id,
                    start.request.config.max_lag as i64,
                    start.request.config.max_intermediate_dim as i64,
                    start.request.config.max_entry as i64,
                    search_mode_label(&start.request.config),
                    frontier_mode_label(start.request.config.frontier_mode),
                    move_family_policy_label(start.request.config.move_family_policy),
                    search_stage_label(start.request.stage),
                ],
            )
            .map_err(|err| format!("failed to insert search run: {err}"))?;
        self.run_id = self.conn.last_insert_rowid();
        Ok(())
    }

    fn upsert_root(&mut self, root: &SearchRootRecord) -> Result<(), String> {
        let canonical_id = self.ensure_matrix_id(&root.canonical)?;
        let orig_id = self.ensure_matrix_id(&root.orig)?;
        let (seen_from_forward, seen_from_backward, forward_depth, backward_depth) =
            match root.direction {
                SearchDirection::Forward => (1, 0, Some(root.depth as i64), None),
                SearchDirection::Backward => (0, 1, None, Some(root.depth as i64)),
            };

        self.conn
            .execute(
                "INSERT INTO run_nodes (
                    run_id,
                    canonical_matrix_id,
                    first_orig_matrix_id,
                    forward_depth,
                    backward_depth,
                    seen_from_forward,
                    seen_from_backward
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(run_id, canonical_matrix_id) DO UPDATE SET
                    first_orig_matrix_id = run_nodes.first_orig_matrix_id,
                    forward_depth = COALESCE(run_nodes.forward_depth, excluded.forward_depth),
                    backward_depth = COALESCE(run_nodes.backward_depth, excluded.backward_depth),
                    seen_from_forward = MAX(run_nodes.seen_from_forward, excluded.seen_from_forward),
                    seen_from_backward = MAX(run_nodes.seen_from_backward, excluded.seen_from_backward)",
                params![
                    self.run_id,
                    canonical_id,
                    orig_id,
                    forward_depth,
                    backward_depth,
                    seen_from_forward,
                    seen_from_backward,
                ],
            )
            .map_err(|err| format!("failed to insert root node: {err}"))?;
        Ok(())
    }

    fn upsert_edge_target(
        tx: &Transaction<'_>,
        run_id: i64,
        canonical_id: i64,
        orig_id: i64,
        direction: SearchDirection,
        depth: i64,
    ) -> Result<(), String> {
        let (seen_from_forward, seen_from_backward, forward_depth, backward_depth) = match direction
        {
            SearchDirection::Forward => (1, 0, Some(depth), None),
            SearchDirection::Backward => (0, 1, None, Some(depth)),
        };
        tx.execute(
            "INSERT INTO run_nodes (
                run_id,
                canonical_matrix_id,
                first_orig_matrix_id,
                forward_depth,
                backward_depth,
                seen_from_forward,
                seen_from_backward
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(run_id, canonical_matrix_id) DO UPDATE SET
                forward_depth = COALESCE(run_nodes.forward_depth, excluded.forward_depth),
                backward_depth = COALESCE(run_nodes.backward_depth, excluded.backward_depth),
                seen_from_forward = MAX(run_nodes.seen_from_forward, excluded.seen_from_forward),
                seen_from_backward = MAX(run_nodes.seen_from_backward, excluded.seen_from_backward)",
            params![
                run_id,
                canonical_id,
                orig_id,
                forward_depth,
                backward_depth,
                seen_from_forward,
                seen_from_backward,
            ],
        )
        .map_err(|err| format!("failed to upsert edge target node: {err}"))?;
        Ok(())
    }

    fn insert_edges(&mut self, edges: &[SearchEdgeRecord]) -> Result<(), String> {
        if edges.is_empty() {
            return Ok(());
        }

        let mut edge_rows = Vec::with_capacity(edges.len());
        for edge in edges {
            edge_rows.push(PendingEdgeRow {
                layer_index: edge.layer_index as i64,
                direction: search_direction_label(edge.direction),
                move_family: edge.move_family,
                from_canonical_id: self.ensure_matrix_id(&edge.from_canonical)?,
                from_orig_id: self.ensure_matrix_id(&edge.from_orig)?,
                to_canonical_id: self.ensure_matrix_id(&edge.to_canonical)?,
                to_orig_id: self.ensure_matrix_id(&edge.to_orig)?,
                from_depth: edge.from_depth as i64,
                to_depth: edge.to_depth as i64,
                status: edge_status_label(edge.status),
                approximate_other_side_hit: edge.approximate_other_side_hit as i64,
                enqueued: edge.enqueued as i64,
                step_u_id: self.ensure_matrix_id(&edge.step.u)?,
                step_v_id: self.ensure_matrix_id(&edge.step.v)?,
                edge_direction: edge.direction,
            });
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|err| format!("failed to start sqlite edge transaction: {err}"))?;
        for edge in &edge_rows {
            Self::upsert_edge_target(
                &tx,
                self.run_id,
                edge.to_canonical_id,
                edge.to_orig_id,
                edge.edge_direction,
                edge.to_depth,
            )?;
            tx.execute(
                "INSERT INTO run_edges (
                    run_id,
                    layer_index,
                    direction,
                    move_family,
                    from_canonical_matrix_id,
                    from_orig_matrix_id,
                    to_canonical_matrix_id,
                    to_orig_matrix_id,
                    from_depth,
                    to_depth,
                    status,
                    approximate_other_side_hit,
                    enqueued,
                    step_u_matrix_id,
                    step_v_matrix_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    self.run_id,
                    edge.layer_index,
                    edge.direction,
                    edge.move_family,
                    edge.from_canonical_id,
                    edge.from_orig_id,
                    edge.to_canonical_id,
                    edge.to_orig_id,
                    edge.from_depth,
                    edge.to_depth,
                    edge.status,
                    edge.approximate_other_side_hit,
                    edge.enqueued,
                    edge.step_u_id,
                    edge.step_v_id,
                ],
            )
            .map_err(|err| format!("failed to insert edge row: {err}"))?;
        }
        tx.commit()
            .map_err(|err| format!("failed to commit sqlite edge transaction: {err}"))?;
        Ok(())
    }

    fn finish_run(
        &mut self,
        result: &SearchRunResult,
        telemetry: &SearchTelemetry,
    ) -> Result<(), String> {
        let (outcome, reason, path_steps) = match result {
            SearchRunResult::Equivalent(path) => {
                ("equivalent", None, Some(path.steps.len() as i64))
            }
            SearchRunResult::EquivalentByConcreteShift(proof) => (
                "equivalent_by_concrete_shift",
                Some(proof.description()),
                None,
            ),
            SearchRunResult::NotEquivalent(reason) => {
                ("not_equivalent", Some(reason.clone()), None)
            }
            SearchRunResult::Unknown => ("unknown", None, None),
        };
        let telemetry_json = serde_json::to_string(telemetry)
            .map_err(|err| format!("failed to serialise telemetry: {err}"))?;
        let result_json = result_json(result)?;
        self.conn
            .execute(
                "UPDATE search_runs
                 SET finished_unix_ms = ?1,
                     outcome = ?2,
                     reason = ?3,
                     path_steps = ?4,
                     telemetry_json = ?5,
                     result_json = ?6
                 WHERE id = ?7",
                params![
                    unix_timestamp_ms(),
                    outcome,
                    reason,
                    path_steps,
                    telemetry_json,
                    result_json,
                    self.run_id,
                ],
            )
            .map_err(|err| format!("failed to update completed run: {err}"))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SearchObserver for SqliteGraphRecorder {
    fn on_event(&mut self, event: &SearchEvent) {
        match event {
            SearchEvent::Started(start) => {
                self.with_connection(|this| this.insert_run(start));
            }
            SearchEvent::Roots(roots) => {
                self.with_connection(|this| {
                    for root in roots {
                        this.upsert_root(root)?;
                    }
                    Ok(())
                });
            }
            SearchEvent::Layer(edges) => {
                self.with_connection(|this| this.insert_edges(edges));
            }
            SearchEvent::Finished(SearchFinishedRecord {
                result, telemetry, ..
            }) => {
                self.with_connection(|this| this.finish_run(result, telemetry));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PendingEdgeRow<'a> {
    layer_index: i64,
    direction: &'a str,
    move_family: &'a str,
    from_canonical_id: i64,
    from_orig_id: i64,
    to_canonical_id: i64,
    to_orig_id: i64,
    from_depth: i64,
    to_depth: i64,
    status: &'a str,
    approximate_other_side_hit: i64,
    enqueued: i64,
    step_u_id: i64,
    step_v_id: i64,
    edge_direction: SearchDirection,
}

#[cfg(not(target_arch = "wasm32"))]
fn initialise_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS matrices (
            id INTEGER PRIMARY KEY,
            matrix_key TEXT NOT NULL UNIQUE,
            rows INTEGER NOT NULL,
            cols INTEGER NOT NULL,
            data_json TEXT NOT NULL,
            entry_sum INTEGER NOT NULL,
            max_entry INTEGER NOT NULL,
            trace INTEGER
        );
        CREATE TABLE IF NOT EXISTS search_runs (
            id INTEGER PRIMARY KEY,
            started_unix_ms INTEGER NOT NULL,
            finished_unix_ms INTEGER,
            source_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            target_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            source_canonical_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            target_canonical_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            max_lag INTEGER NOT NULL,
            max_intermediate_dim INTEGER NOT NULL,
            max_entry INTEGER NOT NULL,
            search_mode TEXT NOT NULL,
            frontier_mode TEXT,
            move_family_policy TEXT,
            stage TEXT NOT NULL,
            outcome TEXT,
            reason TEXT,
            path_steps INTEGER,
            telemetry_json TEXT,
            result_json TEXT
        );
        CREATE TABLE IF NOT EXISTS run_nodes (
            run_id INTEGER NOT NULL REFERENCES search_runs(id) ON DELETE CASCADE,
            canonical_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            first_orig_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            forward_depth INTEGER,
            backward_depth INTEGER,
            seen_from_forward INTEGER NOT NULL DEFAULT 0,
            seen_from_backward INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (run_id, canonical_matrix_id)
        );
        CREATE TABLE IF NOT EXISTS run_edges (
            id INTEGER PRIMARY KEY,
            run_id INTEGER NOT NULL REFERENCES search_runs(id) ON DELETE CASCADE,
            layer_index INTEGER NOT NULL,
            direction TEXT NOT NULL,
            move_family TEXT NOT NULL,
            from_canonical_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            from_orig_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            to_canonical_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            to_orig_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            from_depth INTEGER NOT NULL,
            to_depth INTEGER NOT NULL,
            status TEXT NOT NULL,
            approximate_other_side_hit INTEGER NOT NULL,
            enqueued INTEGER NOT NULL,
            step_u_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            step_v_matrix_id INTEGER NOT NULL REFERENCES matrices(id)
        );
        CREATE INDEX IF NOT EXISTS idx_run_nodes_run ON run_nodes(run_id);
        CREATE INDEX IF NOT EXISTS idx_run_edges_run_layer ON run_edges(run_id, layer_index, direction);
        CREATE INDEX IF NOT EXISTS idx_run_edges_run_from ON run_edges(run_id, from_canonical_matrix_id);
        CREATE INDEX IF NOT EXISTS idx_run_edges_run_to ON run_edges(run_id, to_canonical_matrix_id);",
    )
    .map_err(|err| format!("failed to initialise sqlite schema: {err}"))?;

    let mut has_stage_column = false;
    let mut has_frontier_mode_column = false;
    let mut has_move_family_policy_column = false;
    let mut stmt = conn
        .prepare("PRAGMA table_info(search_runs)")
        .map_err(|err| format!("failed to inspect search_runs schema: {err}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("failed to query search_runs schema: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read search_runs schema row: {err}"))?
    {
        let column_name: String = row
            .get(1)
            .map_err(|err| format!("failed to read search_runs column name: {err}"))?;
        if column_name == "stage" {
            has_stage_column = true;
        } else if column_name == "frontier_mode" {
            has_frontier_mode_column = true;
        } else if column_name == "move_family_policy" {
            has_move_family_policy_column = true;
        }
    }
    if !has_stage_column {
        conn.execute(
            "ALTER TABLE search_runs ADD COLUMN stage TEXT NOT NULL DEFAULT 'endpoint_search'",
            [],
        )
        .map_err(|err| format!("failed to add stage column to search_runs: {err}"))?;
    }
    if !has_frontier_mode_column {
        conn.execute("ALTER TABLE search_runs ADD COLUMN frontier_mode TEXT", [])
            .map_err(|err| format!("failed to add frontier_mode column to search_runs: {err}"))?;
    }
    if !has_move_family_policy_column {
        conn.execute(
            "ALTER TABLE search_runs ADD COLUMN move_family_policy TEXT",
            [],
        )
        .map_err(|err| format!("failed to add move_family_policy column to search_runs: {err}"))?;
    }
    Ok(())
}

fn matrix_key(matrix: &DynMatrix) -> String {
    let mut key = format!("{}x{}:", matrix.rows, matrix.cols);
    for (index, value) in matrix.data.iter().enumerate() {
        if index > 0 {
            key.push(',');
        }
        key.push_str(&value.to_string());
    }
    key
}

fn matrix_json(matrix: &DynMatrix) -> Result<String, String> {
    let rows: Vec<Vec<u32>> = (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix.get(row, col))
                .collect::<Vec<_>>()
        })
        .collect();
    serde_json::to_string(&rows).map_err(|err| format!("failed to serialise matrix: {err}"))
}

fn result_json(result: &SearchRunResult) -> Result<String, String> {
    let json = match result {
        SearchRunResult::Equivalent(path) => json!({
            "outcome": "equivalent",
            "matrices": path
                .matrices
                .iter()
                .map(|matrix| matrix.data.clone())
                .collect::<Vec<_>>(),
            "steps": path
                .steps
                .iter()
                .map(|step| {
                    json!({
                        "u": dyn_matrix_rows(&step.u),
                        "v": dyn_matrix_rows(&step.v),
                    })
                })
                .collect::<Vec<_>>(),
        }),
        SearchRunResult::EquivalentByConcreteShift(proof) => json!({
            "outcome": "equivalent_by_concrete_shift",
            "relation": proof.relation.as_str(),
            "witness": {
                "lag": proof.witness.shift.lag,
                "r": proof.witness.shift.r.data,
                "s": proof.witness.shift.s.data,
                "sigma_g": proof.witness.sigma_g.mapping,
                "sigma_h": proof.witness.sigma_h.mapping,
                "omega_e": proof.witness.omega_e.mapping,
                "omega_f": proof.witness.omega_f.mapping,
            },
        }),
        SearchRunResult::NotEquivalent(reason) => json!({
            "outcome": "not_equivalent",
            "reason": reason,
        }),
        SearchRunResult::Unknown => json!({
            "outcome": "unknown",
        }),
    };
    serde_json::to_string(&json).map_err(|err| format!("failed to serialise search result: {err}"))
}

fn dyn_matrix_rows(matrix: &DynMatrix) -> Vec<Vec<u32>> {
    (0..matrix.rows)
        .map(|row| (0..matrix.cols).map(|col| matrix.get(row, col)).collect())
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn unix_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn search_mode_label(config: &SearchConfig) -> &'static str {
    match config.frontier_mode {
        FrontierMode::Bfs => config.move_family_policy.snake_case_label(),
        FrontierMode::Beam => "beam",
        FrontierMode::BeamBfsHandoff => "beam_bfs_handoff",
    }
}

fn frontier_mode_label(mode: FrontierMode) -> &'static str {
    match mode {
        FrontierMode::Bfs => "bfs",
        FrontierMode::Beam => "beam",
        FrontierMode::BeamBfsHandoff => "beam_bfs_handoff",
    }
}

fn move_family_policy_label(policy: MoveFamilyPolicy) -> &'static str {
    policy.snake_case_label()
}

fn search_stage_label(stage: SearchStage) -> &'static str {
    match stage {
        SearchStage::EndpointSearch => "endpoint_search",
        SearchStage::GuidedRefinement => "guided_refinement",
        SearchStage::ShortcutSearch => "shortcut_search",
    }
}

fn search_direction_label(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "forward",
        SearchDirection::Backward => "backward",
    }
}

fn edge_status_label(status: SearchEdgeStatus) -> &'static str {
    match status {
        SearchEdgeStatus::SeenCollision => "seen_collision",
        SearchEdgeStatus::Discovered => "discovered",
        SearchEdgeStatus::ExactMeet => "exact_meet",
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use std::fs;

    use crate::aligned::{
        canonical_module_shift_witness_2x2, ConcreteShiftRelation2x2, ShiftEquivalenceWitness2x2,
    };
    use crate::matrix::SqMatrix;
    use crate::search::search_sse_2x2_with_telemetry_and_observer;
    use crate::types::{ConcreteShiftProof2x2, SearchConfig, SearchRunResult};

    fn temp_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sse-recorder-{}-{}.sqlite",
            std::process::id(),
            unix_timestamp_ms()
        ))
    }

    #[test]
    fn sqlite_graph_recorder_persists_nodes_and_edges() {
        let path = temp_db_path();
        {
            let mut recorder = SqliteGraphRecorder::new(&path).unwrap();
            let a = SqMatrix::new([[1, 3], [2, 1]]);
            let b = SqMatrix::new([[1, 6], [1, 1]]);
            let config = SearchConfig {
                max_lag: 4,
                max_intermediate_dim: 3,
                max_entry: 4,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::Mixed,
                beam_width: None,
            };

            let (_result, _telemetry) =
                search_sse_2x2_with_telemetry_and_observer(&a, &b, &config, Some(&mut recorder));
            assert_eq!(recorder.error(), None);
        }

        let conn = Connection::open(&path).unwrap();
        let run_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM search_runs", [], |row| row.get(0))
            .unwrap();
        let matrix_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM matrices", [], |row| row.get(0))
            .unwrap();
        let node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM run_nodes", [], |row| row.get(0))
            .unwrap();
        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM run_edges", [], |row| row.get(0))
            .unwrap();
        let outcome: String = conn
            .query_row("SELECT outcome FROM search_runs LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        let (search_mode, frontier_mode, move_family_policy): (
            String,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT search_mode, frontier_mode, move_family_policy FROM search_runs LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(run_count, 1);
        assert!(matrix_count >= 2);
        assert!(node_count >= 1);
        assert!(edge_count >= 1);
        assert!(!outcome.is_empty());
        assert_eq!(search_mode, "mixed");
        assert_eq!(frontier_mode.as_deref(), Some("bfs"));
        assert_eq!(move_family_policy.as_deref(), Some("mixed"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn sqlite_graph_legacy_search_mode_label_stays_backward_compatible() {
        let beam_graph_only = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::Beam,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: Some(8),
        };
        let beam_bfs_handoff = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: Some(8),
        };

        assert_eq!(search_mode_label(&SearchConfig::default()), "mixed");
        assert_eq!(
            search_mode_label(&SearchConfig {
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                ..SearchConfig::default()
            }),
            "graph_only"
        );
        assert_eq!(
            search_mode_label(&SearchConfig {
                move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
                ..SearchConfig::default()
            }),
            "graph_plus_structured"
        );
        assert_eq!(search_mode_label(&beam_graph_only), "beam");
        assert_eq!(frontier_mode_label(beam_graph_only.frontier_mode), "beam");
        assert_eq!(search_mode_label(&beam_bfs_handoff), "beam_bfs_handoff");
        assert_eq!(
            frontier_mode_label(beam_bfs_handoff.frontier_mode),
            "beam_bfs_handoff"
        );
        assert_eq!(
            move_family_policy_label(beam_graph_only.move_family_policy),
            "graph_only"
        );
        assert_eq!(
            move_family_policy_label(MoveFamilyPolicy::GraphPlusStructured),
            "graph_plus_structured"
        );
    }

    #[test]
    fn sqlite_graph_serialises_concrete_shift_relation() {
        let a = SqMatrix::identity();
        let witness = canonical_module_shift_witness_2x2(
            &a,
            &a,
            ShiftEquivalenceWitness2x2 {
                lag: 1,
                r: SqMatrix::identity(),
                s: SqMatrix::identity(),
            },
        )
        .unwrap();
        let result = SearchRunResult::EquivalentByConcreteShift(ConcreteShiftProof2x2 {
            relation: ConcreteShiftRelation2x2::Balanced,
            witness,
        });

        let json = result_json(&result).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["outcome"], "equivalent_by_concrete_shift");
        assert_eq!(value["relation"], "balanced");
        assert_eq!(value["witness"]["lag"], 1);
    }

    #[test]
    fn sqlite_graph_finish_run_persists_concrete_shift_reason() {
        let path = temp_db_path();
        {
            let mut recorder = SqliteGraphRecorder::new(&path).unwrap();
            let matrix = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
            let request = crate::types::SearchRequest {
                source: matrix.clone(),
                target: matrix.clone(),
                config: SearchConfig::default(),
                stage: SearchStage::EndpointSearch,
                guide_artifacts: Vec::new(),
                guided_refinement: crate::types::GuidedRefinementConfig::default(),
                shortcut_search: crate::types::ShortcutSearchConfig::default(),
            };
            let start = SearchStartRecord {
                request,
                source_canonical: matrix.clone(),
                target_canonical: matrix,
            };
            recorder.insert_run(&start).unwrap();

            let a = SqMatrix::identity();
            let witness = canonical_module_shift_witness_2x2(
                &a,
                &a,
                ShiftEquivalenceWitness2x2 {
                    lag: 1,
                    r: SqMatrix::identity(),
                    s: SqMatrix::identity(),
                },
            )
            .unwrap();
            let result = SearchRunResult::EquivalentByConcreteShift(ConcreteShiftProof2x2 {
                relation: ConcreteShiftRelation2x2::Compatible,
                witness,
            });
            recorder
                .finish_run(&result, &SearchTelemetry::default())
                .unwrap();
        }

        let conn = Connection::open(&path).unwrap();
        let (outcome, reason): (String, Option<String>) = conn
            .query_row(
                "SELECT outcome, reason FROM search_runs LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(outcome, "equivalent_by_concrete_shift");
        assert_eq!(reason.as_deref(), Some("compatible concrete-shift witness"));

        let _ = fs::remove_file(&path);
    }
}
