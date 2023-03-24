use clap::Parser;

use checkr::{env::Analysis, parse};

#[derive(Debug, Parser)]
#[command(version)]
enum Cli {
    /// Reference subcommand
    Reference {
        #[arg(value_enum)]
        analysis: Analysis,
        src: String,
        input: String,
    },
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    match Cli::parse() {
        Cli::Reference {
            analysis,
            src,
            input,
        } => {
            let cmds = parse::parse_commands(&src)?;
            let output = analysis.run(&cmds, analysis.input_from_str(&input)?)?;

            println!("{output}");

            Ok(())
        }
    }
}
