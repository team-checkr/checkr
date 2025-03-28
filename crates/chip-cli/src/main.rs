mod chip_check;
mod moka_check;

use camino::Utf8PathBuf;
use clap::Parser as _;
use color_eyre::Result;
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
    ChipCheck {
        /// The .toml files containing the groups
        groups: Utf8PathBuf,
        /// The directory containing the reference tests (.gcl files)
        reference: Utf8PathBuf,
        /// The subdirectory of the repository containing the tasks (e.g.
        /// "task4")
        tasks_dir: String,
    },
    /// Test all groups printing the results to stdout
    MokaCheck {
        /// The .toml files containing the groups
        groups: Utf8PathBuf,
        /// The directory containing the reference tests (.gcl files)
        reference: Utf8PathBuf,
        /// The subdirectory of the repository containing the tasks (e.g.
        /// "task4")
        tasks_dir: String,
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
    path: String,
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.cmd {
        Cmd::ChipCheck {
            groups,
            reference,
            tasks_dir,
        } => chip_check::chip_check(reference, groups, tasks_dir).await?,
        Cmd::MokaCheck {
            groups,
            reference,
            tasks_dir,
        } => moka_check::moka_check(reference, groups, tasks_dir).await?,
    }

    Ok(())
}
