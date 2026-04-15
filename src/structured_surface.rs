use crate::concrete_shift::ConcreteShiftRelation2x2;

/// Canonical descriptor vocabulary for the repo's structured 2x2 surfaces.
///
/// This stays descriptive on purpose: it records what family a surface belongs
/// to, what sort of semantics it has, and how the current product uses it,
/// without forcing the concrete implementations into a unified execution trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuredSurfaceDescriptor2x2 {
    pub family: StructuredSurfaceFamily2x2,
    pub semantics: StructuredSurfaceSemantics,
    pub usage: StructuredSurfaceUsage,
}

impl StructuredSurfaceDescriptor2x2 {
    pub const fn concrete_shift(relation: ConcreteShiftRelation2x2) -> Self {
        Self {
            family: StructuredSurfaceFamily2x2::ConcreteShift(relation),
            semantics: StructuredSurfaceSemantics::CertifiedProofSearch,
            usage: StructuredSurfaceUsage::MainSolverFallback,
        }
    }

    pub const fn balanced_elementary_equivalence() -> Self {
        Self {
            family: StructuredSurfaceFamily2x2::BalancedElementaryEquivalence,
            semantics: StructuredSurfaceSemantics::CertifiedProofSearch,
            usage: StructuredSurfaceUsage::SidecarProofSearch,
        }
    }

    pub const fn sampled_positive_conjugacy() -> Self {
        Self {
            family: StructuredSurfaceFamily2x2::SampledPositiveConjugacy,
            semantics: StructuredSurfaceSemantics::ProposalSource,
            usage: StructuredSurfaceUsage::SidecarProposalSearch,
        }
    }

    pub const fn family_label(self) -> &'static str {
        match self.family {
            StructuredSurfaceFamily2x2::ConcreteShift(_) => "concrete shift",
            StructuredSurfaceFamily2x2::BalancedElementaryEquivalence => {
                "balanced elementary equivalence"
            }
            StructuredSurfaceFamily2x2::SampledPositiveConjugacy => "sampled positive conjugacy",
        }
    }

    /// Stable human-facing witness label used by current reporting surfaces.
    ///
    /// The balanced-elementary and positive-conjugacy labels intentionally keep
    /// the existing CLI wording so adopting the shared descriptor layer remains
    /// behavior-preserving for current outputs.
    pub const fn reporting_label(self) -> &'static str {
        match self.family {
            StructuredSurfaceFamily2x2::ConcreteShift(relation) => match relation {
                ConcreteShiftRelation2x2::Aligned => "aligned concrete-shift witness",
                ConcreteShiftRelation2x2::Balanced => "balanced concrete-shift witness",
                ConcreteShiftRelation2x2::Compatible => "compatible concrete-shift witness",
            },
            StructuredSurfaceFamily2x2::BalancedElementaryEquivalence => {
                "balanced elementary witness"
            }
            StructuredSurfaceFamily2x2::SampledPositiveConjugacy => "positive conjugacy witness",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredSurfaceFamily2x2 {
    ConcreteShift(ConcreteShiftRelation2x2),
    BalancedElementaryEquivalence,
    SampledPositiveConjugacy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredSurfaceSemantics {
    CertifiedProofSearch,
    ProposalSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuredSurfaceUsage {
    MainSolverFallback,
    SidecarProofSearch,
    SidecarProposalSearch,
}

#[cfg(test)]
mod tests {
    use super::{
        StructuredSurfaceDescriptor2x2, StructuredSurfaceFamily2x2, StructuredSurfaceSemantics,
        StructuredSurfaceUsage,
    };
    use crate::concrete_shift::ConcreteShiftRelation2x2;

    #[test]
    fn descriptors_capture_rfc_surface_families() {
        let concrete =
            StructuredSurfaceDescriptor2x2::concrete_shift(ConcreteShiftRelation2x2::Balanced);
        assert_eq!(
            concrete.family,
            StructuredSurfaceFamily2x2::ConcreteShift(ConcreteShiftRelation2x2::Balanced)
        );
        assert_eq!(concrete.family_label(), "concrete shift");
        assert_eq!(
            concrete.reporting_label(),
            "balanced concrete-shift witness"
        );
        assert_eq!(
            concrete.semantics,
            StructuredSurfaceSemantics::CertifiedProofSearch
        );
        assert_eq!(concrete.usage, StructuredSurfaceUsage::MainSolverFallback);

        let balanced = StructuredSurfaceDescriptor2x2::balanced_elementary_equivalence();
        assert_eq!(
            balanced.family,
            StructuredSurfaceFamily2x2::BalancedElementaryEquivalence
        );
        assert_eq!(balanced.family_label(), "balanced elementary equivalence");
        assert_eq!(balanced.reporting_label(), "balanced elementary witness");
        assert_eq!(
            balanced.semantics,
            StructuredSurfaceSemantics::CertifiedProofSearch
        );
        assert_eq!(balanced.usage, StructuredSurfaceUsage::SidecarProofSearch);

        let conjugacy = StructuredSurfaceDescriptor2x2::sampled_positive_conjugacy();
        assert_eq!(
            conjugacy.family,
            StructuredSurfaceFamily2x2::SampledPositiveConjugacy
        );
        assert_eq!(conjugacy.family_label(), "sampled positive conjugacy");
        assert_eq!(conjugacy.reporting_label(), "positive conjugacy witness");
        assert_eq!(
            conjugacy.semantics,
            StructuredSurfaceSemantics::ProposalSource
        );
        assert_eq!(
            conjugacy.usage,
            StructuredSurfaceUsage::SidecarProposalSearch
        );
    }
}
