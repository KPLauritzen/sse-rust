use crate::matrix::DynMatrix;
use crate::types::{EsseStep, SearchDirection, SearchRequest, SearchRunResult, SearchTelemetry};

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
pub struct SearchStartRecord {
    pub request: SearchRequest,
    pub source_canonical: DynMatrix,
    pub target_canonical: DynMatrix,
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

#[derive(Clone, Debug)]
pub struct SearchFinishedRecord {
    pub request: SearchRequest,
    pub result: SearchRunResult,
    pub telemetry: SearchTelemetry,
}

#[derive(Clone, Debug)]
pub enum SearchEvent {
    Started(SearchStartRecord),
    Roots(Vec<SearchRootRecord>),
    Layer(Vec<SearchEdgeRecord>),
    Finished(SearchFinishedRecord),
}

pub trait SearchObserver {
    fn on_event(&mut self, _event: &SearchEvent) {}
}
