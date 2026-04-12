use crate::matrix::DynMatrix;
use crate::types::{EsseStep, SearchConfig, SearchDirection, SearchTelemetry, SseResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchEdgeStatus {
    SeenCollision,
    Discovered,
    ExactMeet,
}

#[derive(Clone, Debug)]
pub struct SearchRootRecord {
    pub direction: SearchDirection,
    pub canonical: DynMatrix,
    pub orig: DynMatrix,
    pub depth: usize,
}

#[derive(Clone, Debug)]
pub struct SearchEdgeRecord {
    pub layer_index: usize,
    pub direction: SearchDirection,
    pub move_family: &'static str,
    pub from_canonical: DynMatrix,
    pub from_orig: DynMatrix,
    pub to_canonical: DynMatrix,
    pub to_orig: DynMatrix,
    pub from_depth: usize,
    pub to_depth: usize,
    pub step: EsseStep,
    pub status: SearchEdgeStatus,
    pub approximate_other_side_hit: bool,
    pub enqueued: bool,
}

pub trait SearchObserver {
    fn on_search_started(
        &mut self,
        _a: &DynMatrix,
        _b: &DynMatrix,
        _a_canonical: &DynMatrix,
        _b_canonical: &DynMatrix,
        _config: &SearchConfig,
    ) {
    }

    fn on_roots(&mut self, _roots: &[SearchRootRecord]) {}

    fn on_layer(&mut self, _edges: &[SearchEdgeRecord]) {}

    fn on_search_finished(&mut self, _result: &SseResult<2>, _telemetry: &SearchTelemetry) {}
}
