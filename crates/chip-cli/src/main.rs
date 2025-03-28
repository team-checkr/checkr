mod chip_check;
mod moka_check;

use std::time::Duration;

use camino::Utf8PathBuf;
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
    ChipGroup {
        /// The .toml files containing the groups
        groups: Utf8PathBuf,
        /// The directory containing the reference tests (.gcl files)
        reference: Utf8PathBuf,
        /// The subdirectory of the repository containing the tasks (e.g.
        /// "task4")
        tasks_dir: String,
    },
    /// Test all groups printing the results to stdout
    MokaGroup {
        /// The .toml files containing the groups
        groups: Utf8PathBuf,
        /// The directory containing the reference tests (.gcl files)
        reference: Utf8PathBuf,
        /// The subdirectory of the repository containing the tasks (e.g.
        /// "task4")
        tasks_dir: String,
    },
    /// Check a program
    ///
    /// If neither --chip nor --moka is set, the kind will be determined from
    /// the contents of the file.
    Check {
        /// The .gcl file to check
        path: Utf8PathBuf,
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
        Cmd::ChipGroup {
            groups,
            reference,
            tasks_dir,
        } => chip_check::chip_check(reference, groups, tasks_dir).await?,
        Cmd::MokaGroup {
            groups,
            reference,
            tasks_dir,
        } => moka_check::moka_check(reference, groups, tasks_dir).await?,
        Cmd::Check {
            path,
            timeout,
            format,
            chip,
            moka,
        } => {
            #[derive(Debug)]
            enum Kind {
                Chip,
                Moka,
            }
            let src =
                std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
            let kind = match (chip, moka) {
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
            };
            match kind {
                Kind::Chip => {
                    let p = chip::parse::parse_agcl_program(&src)
                        .with_context(|| format!("failed to parse {path}"))?;
                    for r in chip_check::chip_chip(Duration::from_secs(*timeout), &p).await? {
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
                        match format {
                            OutputFormat::Human => {
                                let report = miette::Report::new(diag).with_source_code(
                                    miette::NamedSource::new(path, src.to_string()),
                                );
                                println!("{report:?}");
                            }
                            OutputFormat::Json => {
                                serde_json::to_writer(std::io::stdout(), &diag)?;
                            }
                        }
                    }
                }
                Kind::Moka => todo!("implement moka check"),
            }
        }
    }

    Ok(())
}

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}
