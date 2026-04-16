use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::matrix::DynMatrix;

type EndpointKey = (DynMatrix, DynMatrix);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NamedPath {
    pub label: String,
    pub matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathQuotientConfig {
    pub max_suffix_lag: usize,
    pub max_rewrite_states: usize,
    pub max_samples: usize,
}

impl Default for PathQuotientConfig {
    fn default() -> Self {
        Self {
            max_suffix_lag: 4,
            max_rewrite_states: 1024,
            max_samples: 12,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRewriteKind {
    Triangle,
    CommutingSquare,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PathQuotientCorpusSummary {
    pub source_paths: usize,
    pub suffix_window_occurrences: usize,
    pub unique_suffix_windows: usize,
    pub terminal_state_collision_groups: usize,
    pub endpoint_collision_groups: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LocalRewriteCatalogSummary {
    pub triangle_endpoint_pairs: usize,
    pub triangle_two_step_windows: usize,
    pub commuting_square_endpoint_pairs: usize,
    pub commuting_square_two_step_windows: usize,
    pub endpoint_collision_groups_with_local_rewrites: usize,
    pub endpoint_collision_groups_without_local_rewrites: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PathQuotientCanonicalizationSummary {
    pub collapsed_window_occurrences: usize,
    pub collapsed_unique_windows: usize,
    pub lag_reduced_window_occurrences: usize,
    pub lag_reduced_unique_windows: usize,
    pub triangle_rewritten_window_occurrences: usize,
    pub commuting_square_rewritten_window_occurrences: usize,
    pub exploration_truncated_window_occurrences: usize,
    pub exploration_truncated_unique_windows: usize,
    pub unique_raw_windows: usize,
    pub unique_canonical_windows: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathQuotientSample {
    pub source_label: String,
    pub start_index: usize,
    pub end_index: usize,
    pub occurrence_count: usize,
    pub original_lag: usize,
    pub canonical_lag: usize,
    pub rewrite_kinds: Vec<LocalRewriteKind>,
    pub original_matrices: Vec<DynMatrix>,
    pub canonical_matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PathQuotientAnalysis {
    pub config: PathQuotientConfig,
    pub corpus: PathQuotientCorpusSummary,
    pub local_rewrites: LocalRewriteCatalogSummary,
    pub canonicalization: PathQuotientCanonicalizationSummary,
    pub samples: Vec<PathQuotientSample>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct GuidePoolPathQuotientSummary {
    pub source_guides: usize,
    pub unique_raw_guides: usize,
    pub quotient_retained_guides: usize,
    pub raw_guides_removed_by_dedup: usize,
    pub canonical_collision_groups: usize,
    pub raw_guides_in_collision_groups: usize,
    pub guides_changed: usize,
    pub guides_lag_reduced: usize,
    pub exploration_truncated_guides: usize,
    pub raw_total_lag: usize,
    pub quotient_total_lag: usize,
    pub quotient_retained_total_lag: usize,
    pub raw_total_matrices: usize,
    pub quotient_total_matrices: usize,
    pub quotient_retained_total_matrices: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct GuidePoolLocalRedundancySummary {
    pub raw_duplicate_suffix_window_occurrences: usize,
    pub quotient_duplicate_suffix_window_occurrences: usize,
    pub duplicate_suffix_window_occurrences_removed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QuotientGuideRepresentative {
    pub label: String,
    pub occurrence_count: usize,
    pub lag: usize,
    pub source_labels: Vec<String>,
    pub matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GuidePoolQuotientSample {
    pub source_label: String,
    pub occurrence_count: usize,
    pub original_lag: usize,
    pub canonical_lag: usize,
    pub rewrite_kinds: Vec<LocalRewriteKind>,
    pub truncated: bool,
    pub original_matrices: Vec<DynMatrix>,
    pub canonical_matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GuidePoolQuotientAnalysis {
    pub config: PathQuotientConfig,
    pub guide_pool: GuidePoolPathQuotientSummary,
    pub local_suffix_redundancy: GuidePoolLocalRedundancySummary,
    pub raw_window_analysis: PathQuotientAnalysis,
    pub quotient_window_analysis: PathQuotientAnalysis,
    pub retained_guides: Vec<QuotientGuideRepresentative>,
    pub samples: Vec<GuidePoolQuotientSample>,
}

#[derive(Clone, Debug)]
struct SuffixWindowOccurrence {
    source_label: String,
    start_index: usize,
    end_index: usize,
    matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug)]
struct CanonicalizationResult {
    canonical_matrices: Vec<DynMatrix>,
    rewrite_kinds: BTreeSet<LocalRewriteKind>,
    truncated: bool,
}

#[derive(Clone, Debug, Default)]
struct RewriteCatalog {
    direct_windows: BTreeMap<EndpointKey, Vec<DynMatrix>>,
    two_step_windows: BTreeMap<EndpointKey, BTreeSet<Vec<DynMatrix>>>,
}

#[derive(Clone, Debug)]
struct RewrittenWindow {
    kind: LocalRewriteKind,
    matrices: Vec<DynMatrix>,
}

#[derive(Clone, Debug)]
struct StateInfo {
    predecessor: Option<Vec<DynMatrix>>,
    via: Option<LocalRewriteKind>,
}

pub fn analyze_path_quotient(
    paths: &[NamedPath],
    config: &PathQuotientConfig,
) -> PathQuotientAnalysis {
    let windows = collect_suffix_windows(paths, config.max_suffix_lag);
    let unique_windows = windows.iter().fold(
        BTreeMap::<Vec<DynMatrix>, Vec<&SuffixWindowOccurrence>>::new(),
        |mut acc, window| {
            acc.entry(window.matrices.clone()).or_default().push(window);
            acc
        },
    );

    let corpus = summarize_corpus(paths, &windows, &unique_windows);
    let catalog = build_rewrite_catalog(&windows);
    let local_rewrites =
        summarize_local_rewrites(&windows, &catalog, corpus.endpoint_collision_groups);

    let mut canonical_by_window = BTreeMap::new();
    for window in unique_windows.keys() {
        canonical_by_window.insert(
            window.clone(),
            canonicalize_window(window, &catalog, config.max_rewrite_states),
        );
    }

    let canonicalization = summarize_canonicalization(&unique_windows, &canonical_by_window);
    let samples = build_samples(&unique_windows, &canonical_by_window, config.max_samples);

    PathQuotientAnalysis {
        config: config.clone(),
        corpus,
        local_rewrites,
        canonicalization,
        samples,
    }
}

pub fn analyze_guide_pool_quotient(
    paths: &[NamedPath],
    config: &PathQuotientConfig,
) -> GuidePoolQuotientAnalysis {
    let raw_window_analysis = analyze_path_quotient(paths, config);
    let windows = collect_suffix_windows(paths, config.max_suffix_lag);
    let catalog = build_rewrite_catalog(&windows);
    let unique_raw_paths = paths.iter().fold(
        BTreeMap::<Vec<DynMatrix>, Vec<&NamedPath>>::new(),
        |mut acc, path| {
            acc.entry(path.matrices.clone()).or_default().push(path);
            acc
        },
    );

    let mut canonical_by_path = BTreeMap::new();
    for path in unique_raw_paths.keys() {
        canonical_by_path.insert(
            path.clone(),
            canonicalize_window(path, &catalog, config.max_rewrite_states),
        );
    }

    let (guide_pool, retained_guides) =
        summarize_guide_pool_paths(paths, &unique_raw_paths, &canonical_by_path);
    let quotient_paths = retained_guides
        .iter()
        .map(|guide| NamedPath {
            label: guide.label.clone(),
            matrices: guide.matrices.clone(),
        })
        .collect::<Vec<_>>();
    let quotient_window_analysis = analyze_path_quotient(&quotient_paths, config);

    let raw_duplicate_suffix_window_occurrences = raw_window_analysis
        .corpus
        .suffix_window_occurrences
        .saturating_sub(raw_window_analysis.corpus.unique_suffix_windows);
    let quotient_duplicate_suffix_window_occurrences = quotient_window_analysis
        .corpus
        .suffix_window_occurrences
        .saturating_sub(quotient_window_analysis.corpus.unique_suffix_windows);
    let local_suffix_redundancy = GuidePoolLocalRedundancySummary {
        raw_duplicate_suffix_window_occurrences,
        quotient_duplicate_suffix_window_occurrences,
        duplicate_suffix_window_occurrences_removed: raw_duplicate_suffix_window_occurrences
            .saturating_sub(quotient_duplicate_suffix_window_occurrences),
    };
    let samples =
        build_guide_pool_samples(&unique_raw_paths, &canonical_by_path, config.max_samples);

    GuidePoolQuotientAnalysis {
        config: config.clone(),
        guide_pool,
        local_suffix_redundancy,
        raw_window_analysis,
        quotient_window_analysis,
        retained_guides,
        samples,
    }
}

fn collect_suffix_windows(
    paths: &[NamedPath],
    max_suffix_lag: usize,
) -> Vec<SuffixWindowOccurrence> {
    let mut windows = Vec::new();
    for path in paths {
        if path.matrices.len() < 2 {
            continue;
        }
        for end_index in 1..path.matrices.len() {
            let min_start = end_index.saturating_sub(max_suffix_lag);
            for start_index in min_start..end_index {
                windows.push(SuffixWindowOccurrence {
                    source_label: path.label.clone(),
                    start_index,
                    end_index,
                    matrices: path.matrices[start_index..=end_index].to_vec(),
                });
            }
        }
    }
    windows
}

fn summarize_corpus(
    paths: &[NamedPath],
    windows: &[SuffixWindowOccurrence],
    unique_windows: &BTreeMap<Vec<DynMatrix>, Vec<&SuffixWindowOccurrence>>,
) -> PathQuotientCorpusSummary {
    let mut terminal_groups = BTreeMap::<DynMatrix, BTreeSet<Vec<DynMatrix>>>::new();
    let mut endpoint_groups = BTreeMap::<EndpointKey, BTreeSet<Vec<DynMatrix>>>::new();

    for window in unique_windows.keys() {
        let endpoints = endpoints(window);
        terminal_groups
            .entry(endpoints.1.clone())
            .or_default()
            .insert(window.clone());
        endpoint_groups
            .entry(endpoints)
            .or_default()
            .insert(window.clone());
    }

    PathQuotientCorpusSummary {
        source_paths: paths.len(),
        suffix_window_occurrences: windows.len(),
        unique_suffix_windows: unique_windows.len(),
        terminal_state_collision_groups: terminal_groups
            .values()
            .filter(|group| group.len() > 1)
            .count(),
        endpoint_collision_groups: endpoint_groups
            .values()
            .filter(|group| group.len() > 1)
            .count(),
    }
}

fn build_rewrite_catalog(windows: &[SuffixWindowOccurrence]) -> RewriteCatalog {
    let mut direct_windows = BTreeMap::<EndpointKey, Vec<DynMatrix>>::new();
    let mut two_step_windows = BTreeMap::<EndpointKey, BTreeSet<Vec<DynMatrix>>>::new();

    for window in windows {
        match lag(&window.matrices) {
            1 => {
                direct_windows
                    .entry(endpoints(&window.matrices))
                    .or_insert_with(|| window.matrices.clone());
            }
            2 => {
                two_step_windows
                    .entry(endpoints(&window.matrices))
                    .or_default()
                    .insert(window.matrices.clone());
            }
            _ => {}
        }
    }

    RewriteCatalog {
        direct_windows,
        two_step_windows,
    }
}

fn summarize_local_rewrites(
    windows: &[SuffixWindowOccurrence],
    catalog: &RewriteCatalog,
    endpoint_collision_groups: usize,
) -> LocalRewriteCatalogSummary {
    let mut triangle_endpoint_pairs = 0usize;
    let mut triangle_two_step_windows = 0usize;
    let mut commuting_square_endpoint_pairs = 0usize;
    let mut commuting_square_two_step_windows = 0usize;

    let mut endpoint_pairs_with_rewrites = BTreeSet::new();

    for (endpoints, two_step_group) in &catalog.two_step_windows {
        let has_triangle = catalog.direct_windows.contains_key(endpoints);
        if has_triangle {
            triangle_endpoint_pairs += 1;
            triangle_two_step_windows += two_step_group.len();
            endpoint_pairs_with_rewrites.insert(endpoints.clone());
        }
        if two_step_group.len() > 1 {
            commuting_square_endpoint_pairs += 1;
            commuting_square_two_step_windows += two_step_group.len();
            endpoint_pairs_with_rewrites.insert(endpoints.clone());
        }
    }

    let endpoint_collision_groups_without_local_rewrites =
        endpoint_collision_groups.saturating_sub(endpoint_pairs_with_rewrites.len());

    // Touch `windows` so the caller can evolve the summary to use occurrence-based
    // counts without needing a separate catalog walk.
    let _ = windows;

    LocalRewriteCatalogSummary {
        triangle_endpoint_pairs,
        triangle_two_step_windows,
        commuting_square_endpoint_pairs,
        commuting_square_two_step_windows,
        endpoint_collision_groups_with_local_rewrites: endpoint_pairs_with_rewrites.len(),
        endpoint_collision_groups_without_local_rewrites,
    }
}

fn summarize_canonicalization(
    unique_windows: &BTreeMap<Vec<DynMatrix>, Vec<&SuffixWindowOccurrence>>,
    canonical_by_window: &BTreeMap<Vec<DynMatrix>, CanonicalizationResult>,
) -> PathQuotientCanonicalizationSummary {
    let mut summary = PathQuotientCanonicalizationSummary {
        unique_raw_windows: unique_windows.len(),
        unique_canonical_windows: canonical_by_window
            .values()
            .map(|result| result.canonical_matrices.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        ..Default::default()
    };

    for (window, occurrences) in unique_windows {
        let occurrence_count = occurrences.len();
        let result = canonical_by_window
            .get(window)
            .expect("every unique window should have a canonicalization result");
        let collapsed = result.canonical_matrices != *window;
        let lag_reduced = lag(&result.canonical_matrices) < lag(window);

        if collapsed {
            summary.collapsed_unique_windows += 1;
            summary.collapsed_window_occurrences += occurrence_count;
        }
        if lag_reduced {
            summary.lag_reduced_unique_windows += 1;
            summary.lag_reduced_window_occurrences += occurrence_count;
        }
        if result.rewrite_kinds.contains(&LocalRewriteKind::Triangle) {
            summary.triangle_rewritten_window_occurrences += occurrence_count;
        }
        if result
            .rewrite_kinds
            .contains(&LocalRewriteKind::CommutingSquare)
        {
            summary.commuting_square_rewritten_window_occurrences += occurrence_count;
        }
        if result.truncated {
            summary.exploration_truncated_unique_windows += 1;
            summary.exploration_truncated_window_occurrences += occurrence_count;
        }
    }

    summary
}

fn build_samples(
    unique_windows: &BTreeMap<Vec<DynMatrix>, Vec<&SuffixWindowOccurrence>>,
    canonical_by_window: &BTreeMap<Vec<DynMatrix>, CanonicalizationResult>,
    max_samples: usize,
) -> Vec<PathQuotientSample> {
    let mut samples = unique_windows
        .iter()
        .filter_map(|(window, occurrences)| {
            let result = canonical_by_window
                .get(window)
                .expect("every unique window should have a canonicalization result");
            if result.canonical_matrices == *window {
                return None;
            }
            let exemplar = occurrences
                .first()
                .expect("collapsed window should have at least one occurrence");
            Some(PathQuotientSample {
                source_label: exemplar.source_label.clone(),
                start_index: exemplar.start_index,
                end_index: exemplar.end_index,
                occurrence_count: occurrences.len(),
                original_lag: lag(window),
                canonical_lag: lag(&result.canonical_matrices),
                rewrite_kinds: result.rewrite_kinds.iter().copied().collect(),
                original_matrices: window.clone(),
                canonical_matrices: result.canonical_matrices.clone(),
            })
        })
        .collect::<Vec<_>>();

    samples.sort_by(|left, right| {
        right
            .occurrence_count
            .cmp(&left.occurrence_count)
            .then(
                (left.original_lag - left.canonical_lag)
                    .cmp(&(right.original_lag - right.canonical_lag))
                    .reverse(),
            )
            .then(left.original_matrices.cmp(&right.original_matrices))
    });
    samples.truncate(max_samples);
    samples
}

fn summarize_guide_pool_paths(
    paths: &[NamedPath],
    unique_raw_paths: &BTreeMap<Vec<DynMatrix>, Vec<&NamedPath>>,
    canonical_by_path: &BTreeMap<Vec<DynMatrix>, CanonicalizationResult>,
) -> (
    GuidePoolPathQuotientSummary,
    Vec<QuotientGuideRepresentative>,
) {
    let mut summary = GuidePoolPathQuotientSummary {
        source_guides: paths.len(),
        unique_raw_guides: unique_raw_paths.len(),
        ..Default::default()
    };
    let mut canonical_groups = BTreeMap::<Vec<DynMatrix>, Vec<&NamedPath>>::new();

    for path in paths {
        let result = canonical_by_path
            .get(&path.matrices)
            .expect("every unique path should have a canonicalization result");
        summary.raw_total_lag += lag(&path.matrices);
        summary.quotient_total_lag += lag(&result.canonical_matrices);
        summary.raw_total_matrices += path.matrices.len();
        summary.quotient_total_matrices += result.canonical_matrices.len();

        if result.canonical_matrices != path.matrices {
            summary.guides_changed += 1;
        }
        if lag(&result.canonical_matrices) < lag(&path.matrices) {
            summary.guides_lag_reduced += 1;
        }
        if result.truncated {
            summary.exploration_truncated_guides += 1;
        }

        canonical_groups
            .entry(result.canonical_matrices.clone())
            .or_default()
            .push(path);
    }

    summary.quotient_retained_guides = canonical_groups.len();
    summary.raw_guides_removed_by_dedup = summary
        .unique_raw_guides
        .saturating_sub(summary.quotient_retained_guides);
    summary.canonical_collision_groups = canonical_groups
        .values()
        .filter(|group| group.len() > 1)
        .count();
    summary.raw_guides_in_collision_groups = canonical_groups
        .values()
        .filter(|group| group.len() > 1)
        .map(|group| group.len())
        .sum();

    let mut retained_guides = Vec::new();
    for (matrices, group) in canonical_groups {
        let mut source_labels = group
            .into_iter()
            .map(|path| path.label.clone())
            .collect::<Vec<_>>();
        source_labels.sort();
        source_labels.dedup();
        summary.quotient_retained_total_lag += lag(&matrices);
        summary.quotient_retained_total_matrices += matrices.len();
        retained_guides.push(QuotientGuideRepresentative {
            label: source_labels
                .first()
                .cloned()
                .unwrap_or_else(|| "quotient_guide".to_string()),
            occurrence_count: source_labels.len(),
            lag: lag(&matrices),
            source_labels,
            matrices,
        });
    }

    retained_guides.sort_by(|left, right| {
        left.lag
            .cmp(&right.lag)
            .then(left.matrices.cmp(&right.matrices))
            .then(left.label.cmp(&right.label))
    });

    (summary, retained_guides)
}

fn build_guide_pool_samples(
    unique_raw_paths: &BTreeMap<Vec<DynMatrix>, Vec<&NamedPath>>,
    canonical_by_path: &BTreeMap<Vec<DynMatrix>, CanonicalizationResult>,
    max_samples: usize,
) -> Vec<GuidePoolQuotientSample> {
    let mut samples = unique_raw_paths
        .iter()
        .filter_map(|(path, occurrences)| {
            let result = canonical_by_path
                .get(path)
                .expect("every unique path should have a canonicalization result");
            if result.canonical_matrices == *path {
                return None;
            }
            let exemplar = occurrences
                .first()
                .expect("canonicalized guide should have at least one source occurrence");
            Some(GuidePoolQuotientSample {
                source_label: exemplar.label.clone(),
                occurrence_count: occurrences.len(),
                original_lag: lag(path),
                canonical_lag: lag(&result.canonical_matrices),
                rewrite_kinds: result.rewrite_kinds.iter().copied().collect(),
                truncated: result.truncated,
                original_matrices: path.clone(),
                canonical_matrices: result.canonical_matrices.clone(),
            })
        })
        .collect::<Vec<_>>();

    samples.sort_by(|left, right| {
        right
            .occurrence_count
            .cmp(&left.occurrence_count)
            .then(
                (left.original_lag - left.canonical_lag)
                    .cmp(&(right.original_lag - right.canonical_lag))
                    .reverse(),
            )
            .then(left.original_matrices.cmp(&right.original_matrices))
    });
    samples.truncate(max_samples);
    samples
}

fn canonicalize_window(
    window: &[DynMatrix],
    catalog: &RewriteCatalog,
    max_rewrite_states: usize,
) -> CanonicalizationResult {
    let start = window.to_vec();
    let mut visited = BTreeMap::<Vec<DynMatrix>, StateInfo>::new();
    let mut queue = VecDeque::new();
    let mut truncated = false;

    visited.insert(
        start.clone(),
        StateInfo {
            predecessor: None,
            via: None,
        },
    );
    queue.push_back(start.clone());

    while let Some(current) = queue.pop_front() {
        for rewritten in enumerate_local_rewrites(&current, catalog) {
            if visited.contains_key(&rewritten.matrices) {
                continue;
            }
            if visited.len() >= max_rewrite_states {
                truncated = true;
                queue.clear();
                break;
            }
            visited.insert(
                rewritten.matrices.clone(),
                StateInfo {
                    predecessor: Some(current.clone()),
                    via: Some(rewritten.kind),
                },
            );
            queue.push_back(rewritten.matrices);
        }
    }

    let canonical_matrices = visited
        .keys()
        .min_by(|left, right| compare_windows(left, right))
        .expect("at least the start state should be present")
        .clone();

    let mut rewrite_kinds = BTreeSet::new();
    let mut current = canonical_matrices.clone();
    while let Some(info) = visited.get(&current) {
        match (&info.predecessor, info.via) {
            (Some(previous), Some(kind)) => {
                rewrite_kinds.insert(kind);
                current = previous.clone();
            }
            _ => break,
        }
    }

    CanonicalizationResult {
        canonical_matrices,
        rewrite_kinds,
        truncated,
    }
}

fn enumerate_local_rewrites(
    window: &[DynMatrix],
    catalog: &RewriteCatalog,
) -> Vec<RewrittenWindow> {
    let mut rewrites = Vec::new();
    if window.len() < 3 {
        return rewrites;
    }

    for start in 0..window.len() - 2 {
        let slice = &window[start..start + 3];
        let key = endpoints(slice);

        if let Some(direct) = catalog.direct_windows.get(&key) {
            rewrites.push(RewrittenWindow {
                kind: LocalRewriteKind::Triangle,
                matrices: replace_window(window, start, direct),
            });
        }

        if let Some(alternatives) = catalog.two_step_windows.get(&key) {
            for alternative in alternatives {
                if alternative == slice {
                    continue;
                }
                rewrites.push(RewrittenWindow {
                    kind: LocalRewriteKind::CommutingSquare,
                    matrices: replace_window(window, start, alternative),
                });
            }
        }
    }

    rewrites
}

fn replace_window(window: &[DynMatrix], start: usize, replacement: &[DynMatrix]) -> Vec<DynMatrix> {
    let mut next = Vec::with_capacity(window.len() - 3 + replacement.len());
    next.extend_from_slice(&window[..start]);
    next.extend_from_slice(replacement);
    next.extend_from_slice(&window[start + 3..]);
    next
}

fn endpoints(window: &[DynMatrix]) -> EndpointKey {
    (
        window
            .first()
            .expect("non-empty window should have a source")
            .clone(),
        window
            .last()
            .expect("non-empty window should have a target")
            .clone(),
    )
}

fn lag(window: &[DynMatrix]) -> usize {
    window.len().saturating_sub(1)
}

fn compare_windows(left: &[DynMatrix], right: &[DynMatrix]) -> std::cmp::Ordering {
    lag(left).cmp(&lag(right)).then(left.cmp(right))
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_guide_pool_quotient, analyze_path_quotient, lag, LocalRewriteKind, NamedPath,
        PathQuotientConfig,
    };
    use crate::matrix::DynMatrix;

    #[test]
    fn triangle_collapse_prefers_direct_segment() {
        let a = matrix(1);
        let b = matrix(2);
        let c = matrix(3);

        let analysis = analyze_path_quotient(
            &[
                NamedPath {
                    label: "direct".to_string(),
                    matrices: vec![a.clone(), c.clone()],
                },
                NamedPath {
                    label: "two-hop".to_string(),
                    matrices: vec![a.clone(), b, c.clone()],
                },
            ],
            &PathQuotientConfig {
                max_suffix_lag: 3,
                max_rewrite_states: 32,
                max_samples: 8,
            },
        );

        assert_eq!(analysis.local_rewrites.triangle_endpoint_pairs, 1);
        assert_eq!(analysis.local_rewrites.triangle_two_step_windows, 1);
        assert_eq!(analysis.canonicalization.collapsed_unique_windows, 1);
        assert_eq!(analysis.canonicalization.lag_reduced_unique_windows, 1);

        let sample = analysis
            .samples
            .first()
            .expect("expected a collapse sample");
        assert_eq!(sample.original_lag, 2);
        assert_eq!(sample.canonical_lag, 1);
        assert_eq!(sample.rewrite_kinds, vec![LocalRewriteKind::Triangle]);
        assert_eq!(sample.canonical_matrices, vec![a, c]);
    }

    #[test]
    fn commuting_square_rewrites_choose_stable_two_step_representative() {
        let a = matrix(1);
        let b = matrix(5);
        let c = matrix(3);
        let d = matrix(4);

        let analysis = analyze_path_quotient(
            &[
                NamedPath {
                    label: "left".to_string(),
                    matrices: vec![a.clone(), b.clone(), c.clone()],
                },
                NamedPath {
                    label: "right".to_string(),
                    matrices: vec![a.clone(), d.clone(), c.clone()],
                },
            ],
            &PathQuotientConfig {
                max_suffix_lag: 3,
                max_rewrite_states: 32,
                max_samples: 8,
            },
        );

        assert_eq!(analysis.local_rewrites.commuting_square_endpoint_pairs, 1);
        assert_eq!(analysis.local_rewrites.commuting_square_two_step_windows, 2);
        assert_eq!(analysis.canonicalization.collapsed_unique_windows, 1);
        assert_eq!(analysis.canonicalization.lag_reduced_unique_windows, 0);

        let sample = analysis
            .samples
            .first()
            .expect("expected a collapse sample");
        assert_eq!(sample.original_lag, 2);
        assert_eq!(sample.canonical_lag, 2);
        assert_eq!(
            sample.rewrite_kinds,
            vec![LocalRewriteKind::CommutingSquare]
        );
        assert_eq!(lag(&sample.canonical_matrices), 2);
        assert_eq!(sample.canonical_matrices, vec![a, d, c]);
    }

    #[test]
    fn guide_pool_triangle_quotient_removes_redundant_two_hop_guide() {
        let a = matrix(1);
        let b = matrix(2);
        let c = matrix(3);

        let analysis = analyze_guide_pool_quotient(
            &[
                NamedPath {
                    label: "direct".to_string(),
                    matrices: vec![a.clone(), c.clone()],
                },
                NamedPath {
                    label: "two-hop".to_string(),
                    matrices: vec![a.clone(), b, c.clone()],
                },
            ],
            &PathQuotientConfig {
                max_suffix_lag: 3,
                max_rewrite_states: 32,
                max_samples: 8,
            },
        );

        assert_eq!(analysis.guide_pool.source_guides, 2);
        assert_eq!(analysis.guide_pool.quotient_retained_guides, 1);
        assert_eq!(analysis.guide_pool.raw_guides_removed_by_dedup, 1);
        assert_eq!(analysis.guide_pool.guides_changed, 1);
        assert_eq!(analysis.guide_pool.guides_lag_reduced, 1);
        assert_eq!(analysis.guide_pool.raw_total_lag, 3);
        assert_eq!(analysis.guide_pool.quotient_retained_total_lag, 1);
        assert_eq!(analysis.retained_guides.len(), 1);
        assert_eq!(
            analysis.retained_guides[0].source_labels,
            vec!["direct", "two-hop"]
        );
        assert_eq!(analysis.samples.len(), 1);
        assert_eq!(
            analysis.samples[0].rewrite_kinds,
            vec![LocalRewriteKind::Triangle]
        );
    }

    #[test]
    fn guide_pool_square_quotient_deduplicates_to_one_two_step_representative() {
        let a = matrix(1);
        let b = matrix(5);
        let c = matrix(3);
        let d = matrix(4);

        let analysis = analyze_guide_pool_quotient(
            &[
                NamedPath {
                    label: "left".to_string(),
                    matrices: vec![a.clone(), b, c.clone()],
                },
                NamedPath {
                    label: "right".to_string(),
                    matrices: vec![a.clone(), d.clone(), c.clone()],
                },
            ],
            &PathQuotientConfig {
                max_suffix_lag: 3,
                max_rewrite_states: 32,
                max_samples: 8,
            },
        );

        assert_eq!(analysis.guide_pool.source_guides, 2);
        assert_eq!(analysis.guide_pool.quotient_retained_guides, 1);
        assert_eq!(analysis.guide_pool.raw_guides_removed_by_dedup, 1);
        assert_eq!(analysis.guide_pool.guides_changed, 1);
        assert_eq!(analysis.guide_pool.guides_lag_reduced, 0);
        assert_eq!(analysis.retained_guides[0].lag, 2);
        assert_eq!(
            analysis.samples[0].rewrite_kinds,
            vec![LocalRewriteKind::CommutingSquare]
        );
    }

    fn matrix(value: u32) -> DynMatrix {
        DynMatrix::new(1, 1, vec![value])
    }
}
