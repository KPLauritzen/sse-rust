use crate::types::{FrontierMode, MoveFamilyPolicy, SearchConfig};

pub(super) fn default_config() -> SearchConfig {
    SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 2,
        max_entry: 10,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::Mixed,
        beam_width: None,
        beam_bfs_handoff_depth: None,
        beam_bfs_handoff_deferred_cap: None,
        endpoint_multi_meet_cap: None,
    }
}
