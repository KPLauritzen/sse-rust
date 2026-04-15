use sse_core::conjugacy::{
    derive_positive_conjugacy_proposals_2x2, find_positive_conjugacy_2x2,
    PositiveConjugacyProposal2x2, PositiveConjugacyProposalConfig2x2,
    PositiveConjugacySearchConfig2x2, PositiveConjugacySearchResult2x2,
};
use sse_core::matrix::SqMatrix;
use sse_core::structured_surface::StructuredSurfaceDescriptor2x2;

#[derive(Clone)]
struct Case2x2 {
    name: &'static str,
    description: &'static str,
    source: SqMatrix<2>,
    target: SqMatrix<2>,
}

fn main() {
    let descriptor = StructuredSurfaceDescriptor2x2::sampled_positive_conjugacy();
    let mut case = String::from("brix_k3");
    let mut max_conjugator_entry = 8u32;
    let mut sample_points = 64usize;
    let mut top_k = 6usize;
    let mut include_endpoints = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--max-conjugator-entry" => {
                max_conjugator_entry = args
                    .next()
                    .expect("--max-conjugator-entry requires a value")
                    .parse()
                    .expect("invalid max conjugator entry");
            }
            "--sample-points" => {
                sample_points = args
                    .next()
                    .expect("--sample-points requires a value")
                    .parse()
                    .expect("invalid sample point count");
            }
            "--top-k" => {
                top_k = args
                    .next()
                    .expect("--top-k requires a value")
                    .parse()
                    .expect("invalid top-k");
            }
            "--include-endpoints" => {
                include_endpoints = true;
            }
            "--help" | "-h" => {
                println!(
                    "usage: evaluate_positive_conjugacy_proposals [--case brix_k3|brix_k4|simple_diag|constant_positive] [--max-conjugator-entry N] [--sample-points N] [--top-k N] [--include-endpoints]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let case = load_case(&case);
    let search_config = PositiveConjugacySearchConfig2x2 {
        max_conjugator_entry,
        sample_points,
    };
    let proposal_config = PositiveConjugacyProposalConfig2x2 {
        max_proposals: usize::MAX,
        include_endpoints,
    };

    println!("Positive-conjugacy proposal evaluation");
    println!("Case: {} ({})", case.name, case.description);
    println!("A = {:?}", case.source);
    println!("B = {:?}", case.target);
    println!(
        "Witness search: max_conjugator_entry={}, sample_points={}",
        search_config.max_conjugator_entry, search_config.sample_points
    );
    println!(
        "Proposal model: rounded_sample_waypoint (entrywise floor/ceil shadows of sampled positive matrices)"
    );
    println!();

    match find_positive_conjugacy_2x2(&case.source, &case.target, &search_config) {
        PositiveConjugacySearchResult2x2::Equivalent(witness) => {
            let mut all_proposals = derive_positive_conjugacy_proposals_2x2(
                &case.source,
                &case.target,
                &witness,
                &proposal_config,
            );
            let exact_shadow_count = all_proposals
                .iter()
                .filter(|proposal| proposal.shadow_l1_distance == 0.0)
                .count();
            let corridor_count = all_proposals
                .iter()
                .filter(|proposal| {
                    proposal.preserves_endpoint_diagonal && proposal.stays_within_endpoint_box
                })
                .count();

            println!("Found {}", descriptor.reporting_label());
            println!("G = {:?}", witness.conjugator);
            println!("sampled matrices = {}", witness.sampled_path.len());
            println!("unique proposal candidates = {}", all_proposals.len());
            println!("exact integer shadows = {}", exact_shadow_count);
            println!(
                "same-diagonal monotone-corridor proposals = {}",
                corridor_count
            );
            if all_proposals.is_empty() {
                println!();
                println!(
                    "No interior positive integer shadows survived the current phase-1 filters."
                );
                return;
            }

            all_proposals.truncate(top_k.min(all_proposals.len()));
            println!();
            println!("Top {} proposals:", all_proposals.len());
            for (index, proposal) in all_proposals.iter().enumerate() {
                print_proposal(index + 1, proposal, &case.source, &case.target);
            }
        }
        PositiveConjugacySearchResult2x2::Exhausted => {
            println!(
                "No {} found under the requested bounds.",
                descriptor.reporting_label()
            );
        }
    }
}

fn load_case(case: &str) -> Case2x2 {
    match case {
        "brix_k3" => Case2x2 {
            name: "brix_k3",
            description: "Brix-Ruiz witness-known calibration, k=3",
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 6], [1, 1]]),
        },
        "brix_k4" => Case2x2 {
            name: "brix_k4",
            description: "Brix-Ruiz witness-known calibration, k=4",
            source: SqMatrix::new([[1, 4], [3, 1]]),
            target: SqMatrix::new([[1, 12], [1, 1]]),
        },
        "simple_diag" => Case2x2 {
            name: "simple_diag",
            description: "simple diagonal scaling calibration",
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 4], [1, 1]]),
        },
        "constant_positive" => Case2x2 {
            name: "constant_positive",
            description: "constant positive sanity case",
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 2], [2, 1]]),
        },
        _ => panic!("unsupported case: {case}"),
    }
}

fn print_proposal(
    rank: usize,
    proposal: &PositiveConjugacyProposal2x2,
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) {
    println!(
        "{}. {:?}  t={:.3}  shadow_l1={:.3}  endpoint_l1={}  offdiag={}  dA=({:+},{:+})  dB=({:+},{:+})",
        rank,
        proposal.matrix,
        proposal.nearest_sample_t,
        proposal.shadow_l1_distance,
        proposal.endpoint_l1_distance,
        offdiag_string(&proposal.matrix),
        proposal.matrix.data[0][1] as i32 - source.data[0][1] as i32,
        proposal.matrix.data[1][0] as i32 - source.data[1][0] as i32,
        proposal.matrix.data[0][1] as i32 - target.data[0][1] as i32,
        proposal.matrix.data[1][0] as i32 - target.data[1][0] as i32,
    );
    println!("   proxy: {}", proxy_summary(proposal));
}

fn offdiag_string(matrix: &SqMatrix<2>) -> String {
    format!("({}, {})", matrix.data[0][1], matrix.data[1][0])
}

fn proxy_summary(proposal: &PositiveConjugacyProposal2x2) -> String {
    let proximity_label = if proposal.shadow_l1_distance == 0.0 {
        "exact sampled integer"
    } else if proposal.shadow_l1_distance <= 0.5 {
        "very close witness shadow"
    } else if proposal.shadow_l1_distance <= 1.0 {
        "coarse nearby witness shadow"
    } else {
        "loose witness shadow"
    };

    let corridor_label =
        if proposal.preserves_endpoint_diagonal && proposal.stays_within_endpoint_box {
            "same-diagonal monotone waypoint candidate"
        } else if proposal.preserves_endpoint_diagonal {
            "same-diagonal off-corridor seed"
        } else {
            "off-diagonal shadow seed"
        };

    let locality_label = match proposal.endpoint_l1_distance {
        0 => "endpoint replay",
        1 => "one lattice step from an endpoint",
        2 => "two lattice steps from an endpoint",
        _ => "multi-step nearby discrete object",
    };

    format!("{proximity_label}; {corridor_label}; {locality_label}")
}
