use sse_core::invariants::{
    gl2z_similarity_profile_2x2, DeterminantBand2x2, Gl2zSimilarityAnalysis2x2,
};
use sse_core::matrix::SqMatrix;

#[derive(Clone)]
struct Case2x2 {
    name: String,
    description: String,
    source: SqMatrix<2>,
    target: SqMatrix<2>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut case_name = String::from("riedel_baker_k3");
    let mut source_override = None;
    let mut target_override = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case_name = args.next().ok_or("--case requires a value")?;
            }
            "--source" => {
                source_override = Some(parse_matrix_arg(
                    &args.next().ok_or("--source requires a value")?,
                )?);
            }
            "--target" => {
                target_override = Some(parse_matrix_arg(
                    &args.next().ok_or("--target requires a value")?,
                )?);
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let case = match (source_override, target_override) {
        (Some(source), Some(target)) => Case2x2 {
            name: "explicit".to_string(),
            description: "explicit command-line matrices".to_string(),
            source,
            target,
        },
        (None, None) => {
            load_case(&case_name).ok_or_else(|| format!("unsupported case: {case_name}"))?
        }
        _ => {
            return Err(
                "either provide both --source and --target, or use only --case".to_string(),
            );
        }
    };

    let profile = gl2z_similarity_profile_2x2(&case.source, &case.target);

    println!("GL(2,Z) similarity profile (2x2)");
    println!("Case: {} ({})", case.name, case.description);
    println!("A = {:?}", case.source);
    println!("B = {:?}", case.target);
    println!();

    print_matrix_profile("A", &profile.source);
    print_matrix_profile("B", &profile.target);
    println!();

    match profile.pair_determinant_band {
        Some(band) => println!("Pair determinant band: {}", band.label()),
        None => println!("Pair determinant band: n/a (trace/determinant mismatch)"),
    }
    println!(
        "GL(2,Z) similar: {}",
        if profile.gl2z_similar { "yes" } else { "no" }
    );
    print_similarity_analysis(&profile.analysis);
    println!(
        "Theorem-backed positive territory: {}",
        theorem_positive_summary(profile.gl2z_similar, profile.pair_determinant_band)
    );

    Ok(())
}

fn print_help() {
    println!(
        "usage: profile_gl2z_similarity_2x2 [--case NAME] [--source a,b,c,d --target e,f,g,h]\n\n\
         cases:\n\
           riedel_baker_kN      Boyle-Schmieding/Riedel-Baker literature family (default: k=3)\n\
           brix_k3              Brix-Ruiz k=3 calibration pair\n\
           brix_k4              Brix-Ruiz k=4 calibration pair\n\
           simple_diag          simple diagonal-scaling calibration\n\
           constant_positive    identical positive sanity case\n\
           eilers_kiming_14_2   classical non-SSE arithmetic obstruction pair\n\
           choe_shin_swap       nonnegative permutation-similar pair in the Choe-Shin band\n\n\
         explicit matrices use nonnegative entries in row-major order, e.g.\n\
           --source 3,2,1,3 --target 2,1,1,4"
    );
}

fn print_matrix_profile(label: &str, profile: &sse_core::invariants::ArithmeticProfile2x2) {
    println!(
        "{label}: trace={} det={} discriminant={} band={}",
        profile.trace,
        profile.determinant,
        profile.discriminant,
        profile.determinant_band.label()
    );
    match profile.quadratic_arithmetic {
        Some(quadratic) => println!(
            "  quadratic order: field_disc={} conductor={} maximal={} principal_class={}",
            quadratic.order.field_discriminant,
            quadratic.order.conductor,
            if quadratic.order.maximal_order() {
                "yes"
            } else {
                "no"
            },
            if quadratic.principal_ideal_class {
                "yes"
            } else {
                "no"
            }
        ),
        None => println!("  quadratic order: n/a (split or rational characteristic polynomial)"),
    }
}

fn print_similarity_analysis(analysis: &Gl2zSimilarityAnalysis2x2) {
    match analysis {
        Gl2zSimilarityAnalysis2x2::CharacteristicPolynomialMismatch => {
            println!("Similarity analysis: rejected immediately by trace/determinant mismatch");
        }
        Gl2zSimilarityAnalysis2x2::Scalar { eigenvalue } => {
            println!(
                "Similarity analysis: both endpoints are the scalar matrix {}I",
                eigenvalue
            );
        }
        Gl2zSimilarityAnalysis2x2::Split {
            low_eigenvalue,
            high_eigenvalue,
            source_content,
            target_content,
        } => {
            println!(
                "Similarity analysis: split characteristic polynomial with eigenvalues ({low_eigenvalue}, {high_eigenvalue})"
            );
            println!(
                "  split content gcd(A-λI): source={} target={}",
                source_content, target_content
            );
        }
        Gl2zSimilarityAnalysis2x2::Irreducible {
            source_order_ideal_class,
            target_order_ideal_class,
        } => {
            println!(
                "Similarity analysis: irreducible characteristic polynomial, comparing order ideal classes in Z[λ]"
            );
            println!("  source class: {:?}", source_order_ideal_class);
            println!("  target class: {:?}", target_order_ideal_class);
        }
    }
}

fn theorem_positive_summary(
    gl2z_similar: bool,
    pair_determinant_band: Option<DeterminantBand2x2>,
) -> &'static str {
    match (gl2z_similar, pair_determinant_band) {
        (true, Some(DeterminantBand2x2::Baker)) => "yes (Baker band + integer similarity)",
        (true, Some(DeterminantBand2x2::ChoeShin)) => "yes (Choe-Shin band + integer similarity)",
        (true, Some(DeterminantBand2x2::Neither)) => "no (outside Baker/Choe-Shin bands)",
        (true, None) => "no (pair does not share trace/determinant)",
        (false, Some(_)) => "no (band matches, but the pair is not GL(2,Z)-similar)",
        (false, None) => "no (pair does not share trace/determinant)",
    }
}

fn parse_matrix_arg(raw: &str) -> Result<SqMatrix<2>, String> {
    let values: Vec<u32> = raw
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| format!("invalid matrix entry in `{raw}`"))
        })
        .collect::<Result<_, _>>()?;

    if values.len() != 4 {
        return Err(format!(
            "expected four comma-separated entries, got {} in `{raw}`",
            values.len()
        ));
    }

    Ok(SqMatrix::new([
        [values[0], values[1]],
        [values[2], values[3]],
    ]))
}

fn load_case(case: &str) -> Option<Case2x2> {
    match case {
        "brix_k3" => Some(Case2x2 {
            name: "brix_k3".to_string(),
            description: "Brix-Ruiz witness-known calibration, k=3".to_string(),
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 6], [1, 1]]),
        }),
        "brix_k4" => Some(Case2x2 {
            name: "brix_k4".to_string(),
            description: "Brix-Ruiz witness-known calibration, k=4".to_string(),
            source: SqMatrix::new([[1, 4], [3, 1]]),
            target: SqMatrix::new([[1, 12], [1, 1]]),
        }),
        "simple_diag" => Some(Case2x2 {
            name: "simple_diag".to_string(),
            description: "simple diagonal-scaling calibration".to_string(),
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 4], [1, 1]]),
        }),
        "constant_positive" => Some(Case2x2 {
            name: "constant_positive".to_string(),
            description: "constant positive sanity case".to_string(),
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 2], [2, 1]]),
        }),
        "eilers_kiming_14_2" => Some(Case2x2 {
            name: "eilers_kiming_14_2".to_string(),
            description: "Eilers-Kiming arithmetic obstruction pair".to_string(),
            source: SqMatrix::new([[14, 2], [1, 0]]),
            target: SqMatrix::new([[13, 5], [3, 1]]),
        }),
        "choe_shin_swap" => Some(Case2x2 {
            name: "choe_shin_swap".to_string(),
            description: "nonnegative permutation-similar pair in the Choe-Shin determinant band"
                .to_string(),
            source: SqMatrix::new([[1, 3], [3, 3]]),
            target: SqMatrix::new([[3, 3], [3, 1]]),
        }),
        _ => load_riedel_baker_case(case),
    }
}

fn load_riedel_baker_case(case: &str) -> Option<Case2x2> {
    let k = case.strip_prefix("riedel_baker_k")?.parse::<u32>().ok()?;
    if k < 2 {
        return None;
    }

    Some(Case2x2 {
        name: case.to_string(),
        description: format!(
            "Riedel/Baker literature family, k={} (Boyle-Schmieding Example `riedelexample`)",
            k
        ),
        source: SqMatrix::new([[k, 2], [1, k]]),
        target: SqMatrix::new([[k - 1, 1], [1, k + 1]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_matrix_arg_accepts_row_major_values() {
        assert_eq!(
            parse_matrix_arg("3,2,1,3").unwrap(),
            SqMatrix::new([[3, 2], [1, 3]])
        );
    }

    #[test]
    fn load_case_supports_riedel_baker_family() {
        let case = load_case("riedel_baker_k5").unwrap();
        assert_eq!(case.source, SqMatrix::new([[5, 2], [1, 5]]));
        assert_eq!(case.target, SqMatrix::new([[4, 1], [1, 6]]));
    }
}
