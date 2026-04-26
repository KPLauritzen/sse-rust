use crate::matrix::DynMatrix;
use crate::types::{
    DynSsePath, EndpointExactMeetSurface, EndpointExactMeetWitness, SearchConfig, SearchTelemetry,
};

#[derive(Clone, Debug)]
struct RetainedExactMeetCandidate<M> {
    canonical: M,
    path_depth: usize,
    discovery_order: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ExactMeetRetention<M> {
    pub(super) requested_cap: usize,
    next_discovery_order: usize,
    retained: Vec<RetainedExactMeetCandidate<M>>,
}

impl<M: Clone + PartialEq> ExactMeetRetention<M> {
    pub(super) fn from_config(config: &SearchConfig) -> Option<Self> {
        config
            .endpoint_multi_meet_cap
            .filter(|cap| *cap > 0)
            .map(Self::new)
    }

    fn new(requested_cap: usize) -> Self {
        Self {
            requested_cap,
            next_discovery_order: 0,
            retained: Vec::new(),
        }
    }

    pub(super) fn has_retained(&self) -> bool {
        !self.retained.is_empty()
    }

    pub(super) fn retain(&mut self, canonical: &M, path_depth: usize) {
        let candidate = RetainedExactMeetCandidate {
            canonical: canonical.clone(),
            path_depth,
            discovery_order: self.next_discovery_order,
        };
        self.next_discovery_order += 1;
        if let Some(existing) = self
            .retained
            .iter_mut()
            .find(|existing| existing.canonical == candidate.canonical)
        {
            if candidate.path_depth < existing.path_depth
                || (candidate.path_depth == existing.path_depth
                    && candidate.discovery_order < existing.discovery_order)
            {
                *existing = candidate;
            }
            self.retained.sort_by(|left, right| {
                left.path_depth
                    .cmp(&right.path_depth)
                    .then(left.discovery_order.cmp(&right.discovery_order))
            });
            return;
        }
        self.retained.push(candidate);
        self.retained.sort_by(|left, right| {
            left.path_depth
                .cmp(&right.path_depth)
                .then(left.discovery_order.cmp(&right.discovery_order))
        });
        if self.retained.len() > self.requested_cap {
            self.retained.truncate(self.requested_cap);
        }
    }

    pub(super) fn first(&self) -> Option<&M> {
        self.retained.first().map(|candidate| &candidate.canonical)
    }
}

pub(super) fn best_retained_exact_meet_path<M, FPath>(
    retention: &ExactMeetRetention<M>,
    mut reconstruct_path: FPath,
) -> Option<DynSsePath>
where
    M: Clone + PartialEq,
    FPath: FnMut(&M) -> DynSsePath,
{
    retention.first().map(&mut reconstruct_path)
}

pub(super) fn store_endpoint_exact_meets<M, FPath, FCanon>(
    telemetry: &mut SearchTelemetry,
    retention: &ExactMeetRetention<M>,
    mut reconstruct_path: FPath,
    mut to_dyn_matrix: FCanon,
) where
    M: Clone + PartialEq,
    FPath: FnMut(&M) -> DynSsePath,
    FCanon: FnMut(&M) -> DynMatrix,
{
    if !retention.has_retained() {
        return;
    }

    telemetry.endpoint_exact_meets = Some(EndpointExactMeetSurface {
        requested_cap: retention.requested_cap,
        retained: retention
            .retained
            .iter()
            .map(|candidate| EndpointExactMeetWitness {
                path_lag: candidate.path_depth,
                meeting_canonical: to_dyn_matrix(&candidate.canonical),
                path: reconstruct_path(&candidate.canonical),
            })
            .collect(),
    });
}

pub(super) fn maybe_store_endpoint_exact_meets<M, FPath, FCanon>(
    telemetry: &mut SearchTelemetry,
    retention: &ExactMeetRetention<M>,
    timed_out: bool,
    reconstruct_path: FPath,
    to_dyn_matrix: FCanon,
) where
    M: Clone + PartialEq,
    FPath: FnMut(&M) -> DynSsePath,
    FCanon: FnMut(&M) -> DynMatrix,
{
    if timed_out {
        return;
    }
    store_endpoint_exact_meets(telemetry, retention, reconstruct_path, to_dyn_matrix);
}
