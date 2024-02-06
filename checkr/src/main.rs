use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
enum Cli {
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

    // for i in 0..3 {
    //     eprintln!("Hello from stderr! {i} xD>--<");
    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // }

    match Cli::parse() {
        Cli::Reference { analysis, input } => {
            let input = analysis.input_from_str(&input)?;
            let output = input.reference_output()?;
            println!("{output}");

            Ok(())
        }
    }
}
