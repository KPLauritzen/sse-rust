use std::cmp::Ordering;
use std::collections::VecDeque;

use ahash::AHashSet as HashSet;

use crate::matrix::DynMatrix;
use crate::path_scoring::score_node;
use crate::types::SearchConfig;

use super::frontier::{choose_next_layer, FrontierLayerChoiceInputs, FrontierOverlapSignal};
use super::{approx_signature, ApproxSignature};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BeamFrontierEntry {
    pub(super) canonical: DynMatrix,
    pub(super) depth: usize,
    pub(super) score: i64,
    pub(super) approximate_hit: bool,
    pub(super) serial: usize,
}

#[derive(Clone, Debug)]
pub(super) struct BeamFrontier {
    beam_width: usize,
    entries: Vec<BeamFrontierEntry>,
}

impl BeamFrontier {
    pub(super) fn new(beam_width: usize) -> Self {
        Self {
            beam_width: beam_width.max(1),
            entries: Vec::new(),
        }
    }

    pub(super) fn push(&mut self, entry: BeamFrontierEntry) -> Option<BeamFrontierEntry> {
        self.entries.push(entry);
        self.entries.sort_by(compare_beam_frontier_entries);
        if self.entries.len() > self.beam_width {
            self.entries.pop()
        } else {
            None
        }
    }

    pub(super) fn pop_best(&mut self) -> Option<BeamFrontierEntry> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entries.remove(0))
        }
    }

    pub(super) fn pop_batch_same_depth(&mut self, max_batch: usize) -> Vec<BeamFrontierEntry> {
        let Some(first) = self.pop_best() else {
            return Vec::new();
        };

        let target_depth = first.depth;
        let mut batch = vec![first];
        let mut index = 0usize;
        while batch.len() < max_batch && index < self.entries.len() {
            if self.entries[index].depth == target_depth {
                batch.push(self.entries.remove(index));
            } else {
                index += 1;
            }
        }
        batch
    }

    fn peek(&self) -> Option<&BeamFrontierEntry> {
        self.entries.first()
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn expansion_batch_size(&self) -> usize {
        self.beam_width.max(1)
    }

    pub(super) fn refresh_approximate_hits(&mut self, other_signatures: &HashSet<ApproxSignature>) {
        let mut changed = false;
        for entry in &mut self.entries {
            if !entry.approximate_hit
                && other_signatures.contains(&approx_signature(&entry.canonical))
            {
                entry.approximate_hit = true;
                changed = true;
            }
        }
        if changed {
            self.entries.sort_by(compare_beam_frontier_entries);
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BeamBfsHandoffFrontier {
    active: BeamFrontier,
    deferred: VecDeque<DeferredBeamFrontierEntry>,
    deferred_cap: Option<usize>,
    deferred_overflow_len: usize,
}

#[derive(Clone, Debug)]
pub(super) struct BeamBfsHandoffExactMeet {
    pub(super) canonical: DynMatrix,
    pub(super) path_depth: usize,
}

#[derive(Clone, Debug)]
struct DeferredBeamFrontierEntry {
    entry: BeamFrontierEntry,
    retained_overflow: bool,
}

pub(super) const DEFAULT_BEAM_BFS_HANDOFF_DEPTH: usize = 4;

pub(super) fn effective_beam_bfs_handoff_depth(config: &SearchConfig) -> usize {
    config
        .beam_bfs_handoff_depth
        .unwrap_or(DEFAULT_BEAM_BFS_HANDOFF_DEPTH)
        .min(config.max_lag)
}

impl BeamBfsHandoffFrontier {
    pub(super) fn new(beam_width: usize, deferred_cap: Option<usize>) -> Self {
        Self {
            active: BeamFrontier::new(beam_width),
            deferred: VecDeque::new(),
            deferred_cap,
            deferred_overflow_len: 0,
        }
    }

    pub(super) fn push_beam(&mut self, entry: BeamFrontierEntry) {
        if let Some(overflow) = self.active.push(entry) {
            self.defer_entry(overflow, true);
            self.enforce_deferred_overflow_cap();
        }
    }

    pub(super) fn push_bfs(&mut self, entry: BeamFrontierEntry) {
        self.defer_entry(entry, false);
    }

    fn defer_entry(&mut self, entry: BeamFrontierEntry, retained_overflow: bool) {
        let insert_at = self
            .deferred
            .iter()
            .position(|pending| {
                compare_deferred_beam_entries(&entry, &pending.entry) == Ordering::Less
            })
            .unwrap_or(self.deferred.len());
        self.deferred.insert(
            insert_at,
            DeferredBeamFrontierEntry {
                entry,
                retained_overflow,
            },
        );
        if retained_overflow {
            self.deferred_overflow_len += 1;
        }
    }

    fn enforce_deferred_overflow_cap(&mut self) {
        let Some(cap) = self.deferred_cap else {
            return;
        };
        while self.deferred_overflow_len > cap {
            let Some(index) = self
                .deferred
                .iter()
                .rposition(|pending| pending.retained_overflow)
            else {
                break;
            };
            let removed = self
                .deferred
                .remove(index)
                .expect("overflow entry index should be valid");
            debug_assert!(removed.retained_overflow);
            self.deferred_overflow_len -= 1;
        }
    }

    pub(super) fn pop_beam_batch(&mut self) -> Vec<BeamFrontierEntry> {
        self.active
            .pop_batch_same_depth(self.active.expansion_batch_size())
    }

    pub(super) fn pop_bfs_batch(&mut self) -> Vec<BeamFrontierEntry> {
        let Some(first) = self.deferred.pop_front() else {
            return Vec::new();
        };
        if first.retained_overflow {
            self.deferred_overflow_len -= 1;
        }
        let target_depth = first.entry.depth;
        let mut batch = vec![first.entry];
        while self
            .deferred
            .front()
            .is_some_and(|entry| entry.entry.depth == target_depth)
        {
            if let Some(entry) = self.deferred.pop_front() {
                if entry.retained_overflow {
                    self.deferred_overflow_len -= 1;
                }
                batch.push(entry.entry);
            }
        }
        batch
    }

    fn peek_active(&self) -> Option<&BeamFrontierEntry> {
        self.active.peek()
    }

    fn peek_deferred(&self) -> Option<&BeamFrontierEntry> {
        self.deferred.front().map(|entry| &entry.entry)
    }

    pub(super) fn refresh_approximate_hits(&mut self, other_signatures: &HashSet<ApproxSignature>) {
        self.active.refresh_approximate_hits(other_signatures);
        for entry in &mut self.deferred {
            if !entry.entry.approximate_hit
                && other_signatures.contains(&approx_signature(&entry.entry.canonical))
            {
                entry.entry.approximate_hit = true;
            }
        }
    }

    pub(super) fn active_len(&self) -> usize {
        self.active.len()
    }

    pub(super) fn pending_len(&self) -> usize {
        self.active.len() + self.deferred.len()
    }
}

fn compare_beam_frontier_entries(left: &BeamFrontierEntry, right: &BeamFrontierEntry) -> Ordering {
    right
        .approximate_hit
        .cmp(&left.approximate_hit)
        .then_with(|| left.score.cmp(&right.score))
        .then_with(|| left.depth.cmp(&right.depth))
        .then_with(|| left.serial.cmp(&right.serial))
}

fn compare_deferred_beam_entries(left: &BeamFrontierEntry, right: &BeamFrontierEntry) -> Ordering {
    left.depth
        .cmp(&right.depth)
        .then_with(|| left.serial.cmp(&right.serial))
}

pub(super) fn choose_next_beam_direction(
    fwd_frontier: &BeamFrontier,
    bwd_frontier: &BeamFrontier,
) -> Option<bool> {
    match (fwd_frontier.peek(), bwd_frontier.peek()) {
        (Some(fwd), Some(bwd)) => match compare_beam_frontier_entries(fwd, bwd) {
            Ordering::Less => Some(true),
            Ordering::Greater => Some(false),
            Ordering::Equal => Some(fwd_frontier.len() <= bwd_frontier.len()),
        },
        (Some(_), None) => Some(true),
        (None, Some(_)) => Some(false),
        (None, None) => None,
    }
}

pub(super) fn choose_next_beam_bfs_handoff_direction(
    fwd_frontier: &BeamBfsHandoffFrontier,
    bwd_frontier: &BeamBfsHandoffFrontier,
    beam_phase: bool,
) -> Option<bool> {
    if beam_phase {
        match (fwd_frontier.peek_active(), bwd_frontier.peek_active()) {
            (Some(_), Some(_)) => {
                return choose_next_beam_direction(&fwd_frontier.active, &bwd_frontier.active);
            }
            (Some(_), None) => return Some(true),
            (None, Some(_)) => return Some(false),
            (None, None) => return None,
        }
    }

    let next_fwd_depth = fwd_frontier.peek_deferred().map(|entry| entry.depth);
    let next_bwd_depth = bwd_frontier.peek_deferred().map(|entry| entry.depth);
    choose_next_layer(FrontierLayerChoiceInputs {
        fwd_depth: next_fwd_depth,
        bwd_depth: next_bwd_depth,
        fwd_frontier_len: fwd_frontier.pending_len(),
        bwd_frontier_len: bwd_frontier.pending_len(),
        fwd_factorisations_per_node: 1.0,
        bwd_factorisations_per_node: 1.0,
        fwd_cost_sample_nodes: 0,
        bwd_cost_sample_nodes: 0,
        fwd_overlap_signal: FrontierOverlapSignal::default(),
        bwd_overlap_signal: FrontierOverlapSignal::default(),
    })
    .map(|(expand_forward, _)| expand_forward)
}

pub(super) fn should_use_beam_bfs_handoff_phase(
    beam_phase: bool,
    next_depth: usize,
    beam_handoff_depth: usize,
) -> bool {
    beam_phase && next_depth <= beam_handoff_depth
}

pub(super) fn record_best_beam_bfs_handoff_exact_meet(
    best_exact_meet: &mut Option<BeamBfsHandoffExactMeet>,
    canonical: &DynMatrix,
    path_depth: usize,
) {
    let should_replace = match best_exact_meet {
        Some(best) => path_depth < best.path_depth,
        None => true,
    };
    if should_replace {
        *best_exact_meet = Some(BeamBfsHandoffExactMeet {
            canonical: canonical.clone(),
            path_depth,
        });
    }
}

fn beam_candidate_score(
    matrix: &DynMatrix,
    other_signatures: &HashSet<ApproxSignature>,
    target: &DynMatrix,
) -> (i64, bool) {
    let signature = approx_signature(matrix);
    let approximate_hit = other_signatures.contains(&signature);
    let score = (score_node(matrix, target) * 4.0).round() as i64;
    (score, approximate_hit)
}

fn build_beam_frontier_entry(
    canonical: &DynMatrix,
    depth: usize,
    other_signatures: &HashSet<ApproxSignature>,
    target: &DynMatrix,
    serial: &mut usize,
) -> BeamFrontierEntry {
    let (score, approximate_hit) = beam_candidate_score(canonical, other_signatures, target);
    let entry = BeamFrontierEntry {
        canonical: canonical.clone(),
        depth,
        score,
        approximate_hit,
        serial: *serial,
    };
    *serial += 1;
    entry
}

pub(super) fn push_beam_frontier_entry(
    frontier: &mut BeamFrontier,
    canonical: &DynMatrix,
    depth: usize,
    other_signatures: &HashSet<ApproxSignature>,
    target: &DynMatrix,
    serial: &mut usize,
) {
    let _ = frontier.push(build_beam_frontier_entry(
        canonical,
        depth,
        other_signatures,
        target,
        serial,
    ));
}

pub(super) fn push_beam_bfs_handoff_entry(
    frontier: &mut BeamBfsHandoffFrontier,
    canonical: &DynMatrix,
    depth: usize,
    other_signatures: &HashSet<ApproxSignature>,
    target: &DynMatrix,
    serial: &mut usize,
    use_beam_phase: bool,
) {
    let entry = build_beam_frontier_entry(canonical, depth, other_signatures, target, serial);
    if use_beam_phase {
        frontier.push_beam(entry);
    } else {
        frontier.push_bfs(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beam_entry(
        value: u32,
        depth: usize,
        score: i64,
        approximate_hit: bool,
        serial: usize,
    ) -> BeamFrontierEntry {
        BeamFrontierEntry {
            canonical: DynMatrix::new(1, 1, vec![value]),
            depth,
            score,
            approximate_hit,
            serial,
        }
    }

    #[test]
    fn test_beam_frontier_enforces_width_cap() {
        let mut frontier = BeamFrontier::new(2);
        frontier.push(beam_entry(1, 1, 1, false, 2));
        frontier.push(beam_entry(2, 1, 3, false, 1));
        frontier.push(beam_entry(3, 1, 2, false, 0));

        assert_eq!(frontier.len(), 2);
        assert_eq!(frontier.pop_best().unwrap().score, 1);
        assert_eq!(frontier.pop_best().unwrap().score, 2);
    }

    #[test]
    fn test_beam_frontier_batches_same_depth_entries() {
        let mut frontier = BeamFrontier::new(4);
        frontier.push(beam_entry(1, 1, 0, false, 0));
        frontier.push(beam_entry(2, 2, 1, false, 1));
        frontier.push(beam_entry(3, 1, 2, false, 2));
        frontier.push(beam_entry(4, 3, 3, false, 3));

        let batch = frontier.pop_batch_same_depth(4);
        assert_eq!(batch.len(), 2);
        assert!(batch.iter().all(|entry| entry.depth == 1));
        assert_eq!(frontier.pop_best().unwrap().depth, 2);
        assert_eq!(frontier.pop_best().unwrap().depth, 3);
    }

    #[test]
    fn test_beam_frontier_refreshes_approximate_hits() {
        let mut frontier = BeamFrontier::new(2);
        let exact = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let other = DynMatrix::new(2, 2, vec![1, 1, 1, 1]);

        frontier.push(BeamFrontierEntry {
            canonical: other.clone(),
            depth: 1,
            score: 0,
            approximate_hit: false,
            serial: 0,
        });
        frontier.push(BeamFrontierEntry {
            canonical: exact.clone(),
            depth: 1,
            score: 5,
            approximate_hit: false,
            serial: 1,
        });

        let mut other_signatures = HashSet::new();
        other_signatures.insert(approx_signature(&exact));
        frontier.refresh_approximate_hits(&other_signatures);

        let best = frontier.pop_best().unwrap();
        assert!(best.approximate_hit);
        assert_eq!(best.canonical, exact);
    }

    #[test]
    fn test_beam_bfs_handoff_frontier_retains_overflow_for_bfs_phase() {
        let mut frontier = BeamBfsHandoffFrontier::new(2, None);
        frontier.push_beam(beam_entry(1, 1, 1, false, 2));
        frontier.push_beam(beam_entry(2, 1, 3, false, 1));
        frontier.push_beam(beam_entry(3, 1, 2, false, 0));

        assert_eq!(frontier.active_len(), 2);
        assert_eq!(frontier.pending_len(), 3);

        let beam_batch = frontier.pop_beam_batch();
        assert_eq!(beam_batch.len(), 2);
        assert_eq!(beam_batch[0].score, 1);
        assert_eq!(beam_batch[1].score, 2);

        let bfs_batch = frontier.pop_bfs_batch();
        assert_eq!(bfs_batch.len(), 1);
        assert_eq!(bfs_batch[0].score, 3);
        assert_eq!(bfs_batch[0].depth, 1);
    }

    #[test]
    fn test_beam_bfs_handoff_frontier_caps_deferred_overflow() {
        let mut frontier = BeamBfsHandoffFrontier::new(1, Some(1));
        frontier.push_beam(beam_entry(1, 1, 1, false, 0));
        frontier.push_beam(beam_entry(2, 1, 2, false, 1));
        frontier.push_beam(beam_entry(3, 1, 3, false, 2));

        assert_eq!(frontier.active_len(), 1);
        assert_eq!(frontier.pending_len(), 2);

        let bfs_batch = frontier.pop_bfs_batch();
        assert_eq!(bfs_batch.len(), 1);
        assert_eq!(bfs_batch[0].canonical, DynMatrix::new(1, 1, vec![2]));
        assert_eq!(bfs_batch[0].depth, 1);
        assert!(frontier.pop_bfs_batch().is_empty());
    }

    #[test]
    fn test_beam_bfs_handoff_frontier_does_not_cap_bfs_insertions() {
        let mut frontier = BeamBfsHandoffFrontier::new(1, Some(1));
        frontier.push_beam(beam_entry(1, 1, 1, false, 0));
        frontier.push_beam(beam_entry(2, 1, 2, false, 1));
        frontier.push_bfs(beam_entry(4, 2, 4, false, 3));
        frontier.push_bfs(beam_entry(5, 3, 5, false, 4));

        assert_eq!(frontier.active_len(), 1);
        assert_eq!(frontier.pending_len(), 4);

        let first_batch = frontier.pop_bfs_batch();
        assert_eq!(first_batch.len(), 1);
        assert_eq!(first_batch[0].canonical, DynMatrix::new(1, 1, vec![2]));
        assert_eq!(first_batch[0].depth, 1);

        let second_batch = frontier.pop_bfs_batch();
        assert_eq!(second_batch.len(), 1);
        assert_eq!(second_batch[0].canonical, DynMatrix::new(1, 1, vec![4]));
        assert_eq!(second_batch[0].depth, 2);

        let third_batch = frontier.pop_bfs_batch();
        assert_eq!(third_batch.len(), 1);
        assert_eq!(third_batch[0].canonical, DynMatrix::new(1, 1, vec![5]));
        assert_eq!(third_batch[0].depth, 3);

        assert!(frontier.pop_bfs_batch().is_empty());
    }

    #[test]
    fn test_beam_bfs_handoff_overflow_cap_does_not_evict_bfs_entries() {
        let mut frontier = BeamBfsHandoffFrontier::new(1, Some(1));
        frontier.push_beam(beam_entry(1, 1, 1, false, 0));
        frontier.push_beam(beam_entry(2, 1, 2, false, 1));
        frontier.push_bfs(beam_entry(3, 2, 3, false, 2));
        frontier.push_beam(beam_entry(4, 1, 4, false, 3));

        assert_eq!(frontier.active_len(), 1);
        assert_eq!(frontier.pending_len(), 3);

        let first_batch = frontier.pop_bfs_batch();
        assert_eq!(first_batch.len(), 1);
        assert_eq!(first_batch[0].canonical, DynMatrix::new(1, 1, vec![2]));
        assert_eq!(first_batch[0].depth, 1);

        let second_batch = frontier.pop_bfs_batch();
        assert_eq!(second_batch.len(), 1);
        assert_eq!(second_batch[0].canonical, DynMatrix::new(1, 1, vec![3]));
        assert_eq!(second_batch[0].depth, 2);

        assert!(frontier.pop_bfs_batch().is_empty());
    }

    #[test]
    fn test_beam_direction_prefers_approximate_hits() {
        let mut fwd_frontier = BeamFrontier::new(1);
        let mut bwd_frontier = BeamFrontier::new(1);
        fwd_frontier.push(beam_entry(1, 1, 0, false, 0));
        bwd_frontier.push(beam_entry(2, 1, 10, true, 1));

        assert_eq!(
            choose_next_beam_direction(&fwd_frontier, &bwd_frontier),
            Some(false)
        );
    }

    #[test]
    fn test_beam_bfs_handoff_depth_boundary_is_inclusive() {
        assert!(should_use_beam_bfs_handoff_phase(true, 4, 4));
        assert!(!should_use_beam_bfs_handoff_phase(true, 5, 4));
        assert!(!should_use_beam_bfs_handoff_phase(false, 4, 4));
    }

    #[test]
    fn test_effective_beam_bfs_handoff_depth_defaults_and_clamps() {
        let default_config = SearchConfig {
            max_lag: 3,
            ..SearchConfig::default()
        };
        let configured = SearchConfig {
            max_lag: 10,
            beam_bfs_handoff_depth: Some(6),
            ..SearchConfig::default()
        };
        let clamped = SearchConfig {
            max_lag: 5,
            beam_bfs_handoff_depth: Some(7),
            ..SearchConfig::default()
        };

        assert_eq!(effective_beam_bfs_handoff_depth(&default_config), 3);
        assert_eq!(effective_beam_bfs_handoff_depth(&configured), 6);
        assert_eq!(effective_beam_bfs_handoff_depth(&clamped), 5);
    }
}
