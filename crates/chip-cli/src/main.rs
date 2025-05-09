mod chip_check;
mod moka_check;

use std::time::Duration;

use camino::Utf8PathBuf;
use chip::{
    ast::{Command, Commands, PredicateBlock, PredicateChain},
    ast_ext::SyntacticallyEquiv,
};
use chip_check::AssertionResultKind;
use clap::Parser as _;
use color_eyre::{Result, eyre::Context as _};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time()
                .with_writer(std::io::stderr),
        )
        .with(tracing_subscriber::filter::FilterFn::new(|m| {
            !m.target().contains("hyper")
        }))
        .init();

    run().await
}

#[derive(Debug, clap::Parser)]
#[command(version)]
struct Cli {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Subcommand)]
enum Cmd {
    /// Test all groups printing the results to stdout
    ///
    /// Either --chip or --moka must be set
    Group {
        /// The .toml files containing the groups
        groups: Utf8PathBuf,
        /// The directory containing the reference tests (.gcl files)
        reference: Utf8PathBuf,
        /// The subdirectory of the repository containing the tasks (e.g.
        /// "task4")
        tasks_dir: String,
        /// Test Chip programs
        #[clap(long)]
        chip: bool,
        /// Test Moka programs
        #[clap(long)]
        moka: bool,
    },

    /// Check a program
    ///
    /// If neither --chip nor --moka is set, the kind will be determined from
    /// the contents of the file.
    Check {
        /// The .gcl file to check
        path: Utf8PathBuf,
        /// The reference file to check against
        ///
        /// Checks that the program is syntaxically equivalent to the reference
        /// and that the reference precondition implies the precondition of the
        /// given program, and that the reference postcondition is implied by
        /// the postcondition of the given program.
        #[clap(long)]
        reference: Option<Utf8PathBuf>,
        /// Check that the program is fully annotated
        #[clap(long)]
        fully: bool,
        #[clap(long, short, default_value = "human")]
        format: OutputFormat,
        /// Timeout per assertion in seconds
        #[clap(long, short, default_value = "3")]
        timeout: u64,
        /// If the program is a chip program
        #[clap(long)]
        chip: bool,
        /// If the program is a moka program
        #[clap(long)]
        moka: bool,
    },

    /// Format a program
    Fmt {
        /// The .gcl file to format
        path: Utf8PathBuf,
        /// If the program is a chip program
        #[clap(long)]
        chip: bool,
        /// If the program is a moka program
        #[clap(long)]
        moka: bool,
    },
}

#[derive(Debug, serde::Deserialize)]
pub struct Groups {
    groups: Vec<Group>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Group {
    name: String,
    git: String,
    #[allow(unused)]
    path: String,
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.cmd {
        Cmd::Group {
            groups,
            reference,
            tasks_dir,
            chip,
            moka,
        } => match (chip, moka) {
            (true, true) => {
                tracing::error!("Both --chip and --moka are set, only one can be set");
                std::process::exit(1);
            }
            (true, false) => {
                chip_check::chip_check(reference, groups, tasks_dir).await?;
            }
            (false, true) => {
                moka_check::moka_check(reference, groups, tasks_dir).await?;
            }
            (false, false) => {
                tracing::error!("Neither --chip nor --moka are set, one must be set");
                std::process::exit(1);
            }
        },
        Cmd::Check {
            path,
            reference,
            fully,
            timeout,
            format,
            chip,
            moka,
        } => {
            let src =
                std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
            let report_diag = |diag: miette::MietteDiagnostic| match format {
                OutputFormat::Human => {
                    let report = miette::Report::new(diag)
                        .with_source_code(miette::NamedSource::new(path, src.to_string()));
                    println!("{report:?}");
                    Ok(())
                }
                OutputFormat::Json => serde_json::to_writer(std::io::stdout(), &diag),
            };
            match determine_chip_or_moka(chip, moka, &src) {
                Kind::Chip => {
                    let p = chip::parse::parse_agcl_program(&src)
                        .with_context(|| format!("failed to parse {path}"))?;
                    let mut did_error = false;

                    if *fully {
                        did_error |= p.is_fully_annotated();
                        report_diag(miette::diagnostic!(
                            labels = [],
                            severity = miette::Severity::Error,
                            "The program is not fully annotated",
                        ))?;
                    }

                    did_error |= check_program(*timeout, report_diag, &p).await?;
                    if let Some(reference_path) = reference {
                        let reference = std::fs::read_to_string(reference_path)
                            .with_context(|| format!("failed to read {reference_path}"))?;
                        let reference = chip::parse::parse_agcl_program(&reference)
                            .with_context(|| format!("failed to parse {reference_path}"))?;

                        if !reference.is_syntactically_equiv(&p) {
                            did_error = true;
                            report_diag(miette::diagnostic!(
                                labels = [],
                                severity = miette::Severity::Error,
                                "The program is not syntactically equivalent to the reference",
                            ))?;
                        }
                        // NOTE: create a new program where the reference
                        // precondition is followed by the precondition of the
                        // program and then skip.
                        let pre_hack: Commands<chip::ast::PredicateChain, PredicateBlock> =
                            Commands(
                                [Command {
                                    kind: chip::ast::CommandKind::Skip,
                                    span: (0, 0).into(),
                                    pre: PredicateChain {
                                        predicates: reference
                                            .precondition()
                                            .iter()
                                            .flat_map(|pre| &pre.predicates)
                                            .chain(
                                                p.precondition()
                                                    .iter()
                                                    .flat_map(|pre| &pre.predicates),
                                            )
                                            .cloned()
                                            .collect(),
                                    },
                                    post: PredicateChain {
                                        predicates: [].to_vec(),
                                    },
                                }]
                                .to_vec(),
                            );
                        // NOTE: do the same for the postcondition, but flip the order of reference
                        // and p
                        let post_hack: Commands<chip::ast::PredicateChain, PredicateBlock> =
                            Commands(
                                [Command {
                                    kind: chip::ast::CommandKind::Skip,
                                    span: (0, 0).into(),
                                    pre: PredicateChain {
                                        predicates: [].to_vec(),
                                    },
                                    post: PredicateChain {
                                        predicates: reference
                                            .postcondition()
                                            .iter()
                                            .flat_map(|pre| &pre.predicates)
                                            .chain(
                                                p.postcondition()
                                                    .iter()
                                                    .flat_map(|pre| &pre.predicates),
                                            )
                                            .cloned()
                                            .collect(),
                                    },
                                }]
                                .to_vec(),
                            );
                        did_error |= check_program(*timeout, report_diag, &pre_hack).await?;
                        did_error |= check_program(*timeout, report_diag, &post_hack).await?;
                    }
                    if did_error {
                        std::process::exit(1);
                    }
                }
                Kind::Moka => todo!("implement moka check"),
            }
        }
        Cmd::Fmt { path, chip, moka } => {
            let src =
                std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
            match determine_chip_or_moka(chip, moka, &src) {
                Kind::Chip => {
                    let p = chip::parse::parse_agcl_program(&src)
                        .with_context(|| format!("failed to parse {path}"))?;
                    println!("{p}");
                }
                Kind::Moka => {
                    let p = chip::parse::parse_ltl_program(&src)
                        .with_context(|| format!("failed to parse {path}"))?;
                    println!("{p}");
                }
            }
        }
    }

    Ok(())
}

fn determine_chip_or_moka(chip: &bool, moka: &bool, src: &str) -> Kind {
    match (chip, moka) {
        (true, true) => {
            tracing::error!("Both --chip and --moka are set, only one can be set");
            std::process::exit(1);
        }
        (true, false) => Kind::Chip,
        (false, true) => Kind::Moka,
        (false, false) => match () {
            () if src.trim().starts_with(">") => {
                tracing::debug!(kind=?Kind::Moka, "determined from starting '>'");
                Kind::Moka
            }
            () if src.trim().starts_with("{") => {
                tracing::debug!(kind=?Kind::Chip, "determined from starting '{{'");
                Kind::Chip
            }
            () if src.trim().ends_with("}") => {
                tracing::debug!(kind=?Kind::Chip, "determined from ending '}}'");
                Kind::Chip
            }
            _ => {
                tracing::debug!(kind=?Kind::Chip, "defaulting to chip");
                Kind::Chip
            }
        },
    }
}

async fn check_program(
    timeout: u64,
    report_diag: impl Fn(miette::MietteDiagnostic) -> Result<(), serde_json::Error>,
    p: &Commands<PredicateChain, PredicateBlock>,
) -> Result<bool> {
    let mut did_error = false;
    for r in chip_check::chip_chip(Duration::from_secs(timeout), p).await? {
        let span = r.assertion.source.span;
        let text = r
            .assertion
            .source
            .text
            .as_deref()
            .unwrap_or("Verification failed");
        let label = miette::LabeledSpan::at((span.offset(), span.len()), text);

        let diag = match r.result {
            AssertionResultKind::Unsat => continue,
            AssertionResultKind::Timeout => miette::diagnostic!(
                labels = [label],
                severity = miette::Severity::Warning,
                "Timed out while checking",
            ),
            _ => miette::diagnostic!(
                labels = [label],
                severity = miette::Severity::Error,
                "{text}",
            ),
        };
        did_error = true;
        report_diag(diag)?;
    }
    Ok(did_error)
}

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Debug)]
enum Kind {
    Chip,
    Moka,
}
