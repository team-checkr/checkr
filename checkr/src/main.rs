use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[clap(long, default_value = "false")]
    spin: bool,
    #[clap(long, default_value = "false")]
    spam: bool,
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Reference subcommand
    Reference {
        #[arg(value_enum)]
        analysis: ce_shell::Analysis,
        input: String,
    },
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    let cli = Cli::parse();

    if cli.spin {
        std::thread::sleep(Duration::from_secs(1_000_000));
    }

    if cli.spam {
        loop {
            println!("spam");
        }
    }

    match &cli.cmd {
        Cmd::Reference { analysis, input } => {
            let input = analysis.input_from_str(input)?;
            let output = input.reference_output()?;
            println!("{output}");

            Ok(())
        }
    }
}
