use std::path::PathBuf;

use crate::{batch, collect_programs, test_runner::TestRunInput};

use clap::Parser;
use color_eyre::Result;
use xshell::Shell;

#[derive(Debug, Parser)]
#[command(version)]
pub enum Cli {
    /// Parse all provided program TOML files and print out in a canonicalized format.
    DumpPrograms {
        /// The configs file specifying the programs to run in the competition.
        #[clap(long, short)]
        programs: Vec<PathBuf>,
    },
    /// Subcommand for everything batch related.
    Batch {
        #[clap(subcommand)]
        cmd: batch::BatchCli,
    },
    /// The command used within the docker container to generate competition
    /// results of a single group. This is not intended to be used by humans.
    InternalSingleCompetition,
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self {
            Cli::DumpPrograms { programs } => {
                println!(
                    "{}",
                    toml::to_string_pretty(&collect_programs(programs)?.canonicalize()?)?
                );

                Ok(())
            }
            Cli::Batch { cmd } => cmd.run().await,
            Cli::InternalSingleCompetition => {
                let sh = Shell::new()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                TestRunInput::run_from_within_docker(&sh, &input).await?;
                Ok(())
            }
        }
    }
}
