use clap::{Parser, Subcommand};

use checkr::{
    env::{
        graph::GraphEnv, pv::ProgramVerificationEnv, Analysis, Application, Environment,
        InterpreterEnv, SecurityEnv, SignEnv,
    },
    parse,
};

#[derive(Debug, Parser)]
enum Cli {
    /// Reference subcommand
    Reference {
        #[arg(value_enum)]
        analysis: Analysis,
        src: String,
        input: String,
    },
}

#[derive(Debug, Subcommand)]
enum Reference {
    Interpreter { src: String, input: String },
    Security { src: String, input: String },
    Sign { src: String, input: String },
    Pv { src: String, input: String },
    Graph { src: String, input: String },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    let mut app = Application::new();
    app.add_env(SecurityEnv).add_env(InterpreterEnv);

    match Cli::parse() {
        Cli::Reference {
            analysis,
            src,
            input,
        } => {
            let cmds = parse::parse_commands(&src)?;
            let output = match analysis {
                Analysis::Graph => {
                    serde_json::to_string(&GraphEnv.run(&cmds, &serde_json::from_str(&input)?))?
                }
                Analysis::Sign => {
                    serde_json::to_string(&SignEnv.run(&cmds, &serde_json::from_str(&input)?))?
                }
                Analysis::Interpreter => serde_json::to_string(
                    &InterpreterEnv.run(&cmds, &serde_json::from_str(&input)?),
                )?,
                Analysis::Security => {
                    serde_json::to_string(&SecurityEnv.run(&cmds, &serde_json::from_str(&input)?))?
                }
                Analysis::ProgramVerification => serde_json::to_string(
                    &ProgramVerificationEnv.run(&cmds, &serde_json::from_str(&input)?),
                )?,
            };

            println!("{output}");

            Ok(())
        }
    }
}
