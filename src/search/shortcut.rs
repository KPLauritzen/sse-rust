use crate::concrete_shift::{
    search_concrete_shift_equivalence_with_lag_2x2, ConcreteShiftRelation2x2,
    ConcreteShiftSearchResult2x2,
};
use crate::matrix::SqMatrix;
use crate::types::{ConcreteShiftProof2x2, SearchConfig};

fn is_essential_matrix_2x2(m: &SqMatrix<2>) -> bool {
    let row0 = m.data[0][0] + m.data[0][1];
    let row1 = m.data[1][0] + m.data[1][1];
    let col0 = m.data[0][0] + m.data[1][0];
    let col1 = m.data[0][1] + m.data[1][1];
    row0 > 0 && row1 > 0 && col0 > 0 && col1 > 0
}

fn concrete_shift_witness_budget(config: &SearchConfig) -> usize {
    if config.max_lag <= 4 && config.max_entry <= 6 {
        10_000
    } else {
        25_000
    }
}

fn should_try_concrete_shift_fallback(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> bool {
    is_essential_matrix_2x2(a)
        && is_essential_matrix_2x2(b)
        && config.max_lag <= 4
        && config.max_entry <= 6
}

pub(super) fn try_concrete_shift_shortcut_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> Option<ConcreteShiftProof2x2> {
    if !should_try_concrete_shift_fallback(a, b, config) {
        return None;
    }

    let max_witnesses = concrete_shift_witness_budget(config);
    find_concrete_shift_shortcut_proof(config.max_lag as u32, |lag, relation| {
        search_concrete_shift_equivalence_with_lag_2x2(
            a,
            b,
            lag,
            config.max_entry,
            max_witnesses,
            relation,
        )
    })
}

pub(super) fn find_concrete_shift_shortcut_proof<F>(
    max_lag: u32,
    mut probe: F,
) -> Option<ConcreteShiftProof2x2>
where
    F: FnMut(u32, ConcreteShiftRelation2x2) -> ConcreteShiftSearchResult2x2,
{
    for lag in 1..=max_lag {
        let mut any_limit = false;
        for relation in [
            ConcreteShiftRelation2x2::Aligned,
            ConcreteShiftRelation2x2::Balanced,
            ConcreteShiftRelation2x2::Compatible,
        ] {
            match probe(lag, relation) {
                ConcreteShiftSearchResult2x2::Equivalent(witness) => {
                    return Some(ConcreteShiftProof2x2 { relation, witness });
                }
                ConcreteShiftSearchResult2x2::Exhausted => {}
                ConcreteShiftSearchResult2x2::SearchLimitReached => any_limit = true,
            }
        }

        if any_limit {
            return None;
        }
    }

    None
}
