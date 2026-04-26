use crate::matrix::DynMatrix;
use crate::types::MoveFamilyPolicy;

type FactorisationEnumerator = fn(&DynMatrix, u32, &mut dyn FnMut(DynMatrix, DynMatrix));
type FactorisationFamilyEnabled = fn(usize, usize, MoveFamilyPolicy) -> bool;

#[derive(Clone, Copy)]
struct FactorisationFamilyDescriptor {
    label: &'static str,
    enabled: FactorisationFamilyEnabled,
    enumerate: FactorisationEnumerator,
}

impl FactorisationFamilyDescriptor {
    const fn new(
        label: &'static str,
        enabled: FactorisationFamilyEnabled,
        enumerate: FactorisationEnumerator,
    ) -> Self {
        Self {
            label,
            enabled,
            enumerate,
        }
    }

    fn is_enabled(
        &self,
        input_dim: usize,
        max_intermediate_dim: usize,
        move_family_policy: MoveFamilyPolicy,
    ) -> bool {
        (self.enabled)(input_dim, max_intermediate_dim, move_family_policy)
    }

    fn visit<F>(&self, a: &DynMatrix, max_entry: u32, visit: &mut F)
    where
        F: FnMut(&'static str, DynMatrix, DynMatrix),
    {
        let label = self.label;
        (self.enumerate)(a, max_entry, &mut |u, v| visit(label, u, v));
    }
}

const TWO_BY_TWO_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 2] = [
    FactorisationFamilyDescriptor::new(
        "square_factorisation_2x2",
        enabled_square_factorisation_2x2,
        super::enumerate_square_factorisation_2x2_family,
    ),
    FactorisationFamilyDescriptor::new(
        "rectangular_factorisation_2x3",
        enabled_rectangular_factorisation_2x3,
        super::enumerate_rectangular_factorisation_2x3_family,
    ),
];

const THREE_BY_THREE_RECTANGULAR_FAMILIES: [FactorisationFamilyDescriptor; 4] = [
    FactorisationFamilyDescriptor::new(
        "rectangular_factorisation_3x3_to_2",
        enabled_rectangular_factorisation_3x3_to_2,
        super::enumerate_rectangular_factorisation_3x3_to_2_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_row_split_3x3_to_4x4",
        enabled_single_row_split_3x3_to_4x4,
        super::enumerate_single_row_split_3x3_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_split_3x3_to_4x4",
        enabled_single_column_split_3x3_to_4x4,
        super::enumerate_single_column_split_3x3_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_3x3_to_4",
        enabled_binary_sparse_factorisation_3x3_to_4,
        super::enumerate_binary_sparse_factorisation_3x3_to_4_family,
    ),
];

const THREE_BY_THREE_SAME_DIMENSION_FAMILIES: [FactorisationFamilyDescriptor; 6] = [
    FactorisationFamilyDescriptor::new(
        "square_factorisation_3x3",
        enabled_square_factorisation_3x3,
        super::enumerate_square_factorisation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "diagonal_refactorization_3x3",
        enabled_three_by_three_same_dimension_family,
        super::enumerate_diagonal_refactorization_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "elementary_conjugation_3x3",
        enabled_elementary_conjugation_3x3,
        super::enumerate_elementary_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "opposite_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        super::enumerate_opposite_shear_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "parallel_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        super::enumerate_parallel_shear_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "convergent_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        super::enumerate_convergent_shear_conjugation_3x3_family,
    ),
];

const FOUR_BY_FOUR_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 7] = [
    FactorisationFamilyDescriptor::new(
        "single_row_amalgamation_4x4_to_3x3",
        enabled_single_row_amalgamation_4x4_to_3x3,
        super::enumerate_single_row_amalgamation_4x4_to_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_amalgamation_4x4_to_3x3",
        enabled_single_column_amalgamation_4x4_to_3x3,
        super::enumerate_single_column_amalgamation_4x4_to_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_4x3_to_3",
        enabled_binary_sparse_factorisation_4x4_to_3,
        super::enumerate_binary_sparse_factorisation_4x4_to_3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_row_split_4x4_to_5x5",
        enabled_single_row_split_4x4_to_5x5,
        super::enumerate_single_row_split_4x4_to_5x5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_split_4x4_to_5x5",
        enabled_single_column_split_4x4_to_5x5,
        super::enumerate_single_column_split_4x4_to_5x5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_4x4_to_5",
        enabled_binary_sparse_factorisation_4x4_to_5,
        super::enumerate_binary_sparse_factorisation_4x4_to_5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "diagonal_refactorization_4x4",
        enabled_four_by_four_same_dimension_family,
        super::enumerate_diagonal_refactorization_4x4_family,
    ),
];

const FIVE_BY_FIVE_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 3] = [
    FactorisationFamilyDescriptor::new(
        "single_row_amalgamation_5x5_to_4x4",
        enabled_single_row_amalgamation_5x5_to_4x4,
        super::enumerate_single_row_amalgamation_5x5_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_amalgamation_5x5_to_4x4",
        enabled_single_column_amalgamation_5x5_to_4x4,
        super::enumerate_single_column_amalgamation_5x5_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_5x5_to_4",
        enabled_binary_sparse_factorisation_5x5_to_4,
        super::enumerate_binary_sparse_factorisation_5x5_to_4_family,
    ),
];

const GENERIC_SAME_DIMENSION_CONJUGATION_FAMILIES: [FactorisationFamilyDescriptor; 1] =
    [FactorisationFamilyDescriptor::new(
        "elementary_conjugation",
        enabled_generic_same_dimension_conjugation,
        super::enumerate_generic_same_dimension_conjugation_family,
    )];

fn enabled_square_factorisation_2x2(
    input_dim: usize,
    _max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 2 && move_family_policy.permits_factorisations()
}

fn enabled_rectangular_factorisation_2x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 2
        && max_intermediate_dim >= 3
        && match move_family_policy {
            MoveFamilyPolicy::GraphOnly => max_intermediate_dim == 3,
            MoveFamilyPolicy::Mixed | MoveFamilyPolicy::GraphPlusStructured => true,
        }
}

fn enabled_rectangular_factorisation_3x3_to_2(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
        && match move_family_policy {
            MoveFamilyPolicy::GraphOnly => max_intermediate_dim == 3,
            MoveFamilyPolicy::Mixed | MoveFamilyPolicy::GraphPlusStructured => true,
        }
}

fn enabled_binary_sparse_factorisation_3x3_to_4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 4 && move_family_policy.permits_factorisations()
}

fn enabled_single_row_split_3x3_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    // The graph_plus_structured lane keeps the broader binary-sparse 3x3->4 lift
    // and drops these explicit split families after they benchmarked as pure
    // duplicate volume on the retained Brix-Ruiz k=4 dim4 surface.
    input_dim == 3
        && max_intermediate_dim >= 4
        && matches!(move_family_policy, MoveFamilyPolicy::Mixed)
}

fn enabled_single_column_split_3x3_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
        && max_intermediate_dim >= 4
        && matches!(move_family_policy, MoveFamilyPolicy::Mixed)
}

fn enabled_square_factorisation_3x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
        && max_intermediate_dim >= 3
        && move_family_policy.includes_square_factorisation_3x3()
}

fn enabled_three_by_three_same_dimension_family(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 3 && move_family_policy.permits_factorisations()
}

fn enabled_elementary_conjugation_3x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
        && max_intermediate_dim >= 3
        && match move_family_policy {
            MoveFamilyPolicy::GraphOnly => max_intermediate_dim == 3,
            MoveFamilyPolicy::Mixed | MoveFamilyPolicy::GraphPlusStructured => true,
        }
}

fn enabled_binary_sparse_factorisation_4x4_to_3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 4 && move_family_policy.permits_factorisations()
}

fn enabled_single_row_amalgamation_4x4_to_3x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    // On the retained Brix-Ruiz k=4 dim4 lane, the broader binary-sparse
    // 4x4->3 lift stays enabled while the explicit row/column amalgamation
    // siblings are tested as duplicate-volume cuts for GraphPlusStructured.
    input_dim == 4
        && max_intermediate_dim >= 4
        && matches!(move_family_policy, MoveFamilyPolicy::Mixed)
}

fn enabled_single_column_amalgamation_4x4_to_3x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4
        && max_intermediate_dim >= 4
        && matches!(move_family_policy, MoveFamilyPolicy::Mixed)
}

fn enabled_single_row_split_4x4_to_5x5(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_single_column_split_4x4_to_5x5(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_binary_sparse_factorisation_4x4_to_5(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_four_by_four_same_dimension_family(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 4 && move_family_policy.permits_factorisations()
}

fn enabled_single_row_amalgamation_5x5_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_single_column_amalgamation_5x5_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_binary_sparse_factorisation_5x5_to_4(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5 && move_family_policy.permits_factorisations()
}

fn enabled_generic_same_dimension_conjugation(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim >= 4
        && max_intermediate_dim >= input_dim
        && move_family_policy.permits_factorisations()
}

fn visit_enabled_factorisation_family_descriptors<F>(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
    mut visit: F,
) where
    F: FnMut(&FactorisationFamilyDescriptor),
{
    let mut visit_group = |families: &[FactorisationFamilyDescriptor]| {
        for family in families {
            if family.is_enabled(input_dim, max_intermediate_dim, move_family_policy) {
                visit(family);
            }
        }
    };

    match input_dim {
        2 => visit_group(&TWO_BY_TWO_FACTORISATION_FAMILIES),
        3 => {
            visit_group(&THREE_BY_THREE_RECTANGULAR_FAMILIES);
            visit_group(&THREE_BY_THREE_SAME_DIMENSION_FAMILIES);
        }
        4 => visit_group(&FOUR_BY_FOUR_FACTORISATION_FAMILIES),
        5 => visit_group(&FIVE_BY_FIVE_FACTORISATION_FAMILIES),
        _ => {}
    }

    if input_dim >= 4 {
        visit_group(&GENERIC_SAME_DIMENSION_CONJUGATION_FAMILIES);
    }
}

pub(super) fn visit_selected_factorisation_families<F>(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
    visit: &mut F,
) where
    F: FnMut(&'static str, DynMatrix, DynMatrix),
{
    visit_enabled_factorisation_family_descriptors(
        a.rows,
        max_intermediate_dim,
        move_family_policy,
        |family| family.visit(a, max_entry, visit),
    );
}

#[cfg(test)]
pub(super) fn selected_factorisation_family_labels(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    visit_enabled_factorisation_family_descriptors(
        input_dim,
        max_intermediate_dim,
        move_family_policy,
        |family| labels.push(family.label),
    );
    labels
}
