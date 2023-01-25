use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

use verification_lawyer::{
    env::{
        graph::GraphEnv, pv::ProgramVerificationEnv, Analysis, Application, Environment,
        InterpreterEnv, SecurityEnv, SignEnv,
    },
    generate_program,
    interpreter::{Interpreter, InterpreterMemory},
    parse,
    pg::{Determinism, ProgramGraph},
};

#[derive(Debug, Parser)]
enum Cli {
    /// Generate a program
    Generate {
        #[clap(short, long)]
        fuel: Option<u32>,
        #[clap(short, long)]
        seed: Option<u64>,
    },
    // /// Test subcommand
    // Test {
    //     #[clap(short, long)]
    //     fuel: Option<u32>,
    //     #[clap(short, long)]
    //     seed: Option<u64>,
    //     #[clap(short, long)]
    //     program: String,
    //     #[command(subcommand)]
    //     command: Test,
    // },
    /// Reference subcommand
    Reference {
        #[arg(value_enum)]
        analysis: Analysis,
        src: String,
        input: String,
    },
}

#[derive(Debug, Subcommand)]
enum Test {
    Interpreter {},
    Security {},
    Sign {},
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
        Cli::Generate { fuel, seed } => {
            for _ in 0.. {
                let cmds = generate_program(fuel, seed).cmds;

                print!("{esc}c", esc = 27 as char);
                // println!("{}", crate::fmt::fmt_commands(&cmds));

                {
                    use syntect::easy::HighlightLines;
                    use syntect::highlighting::{Style, ThemeSet};
                    use syntect::parsing::SyntaxSet;
                    use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

                    // Load these once at the start of your program
                    let ps = SyntaxSet::load_defaults_newlines();
                    let ts = ThemeSet::load_defaults();

                    // panic!("{:?}", ts.themes.keys());

                    let syntax = ps.find_syntax_by_extension("py").unwrap();
                    let mut h = HighlightLines::new(syntax, &ts.themes["base16-eighties.dark"]);
                    let s = cmds.to_string();
                    for line in LinesWithEndings::from(&s) {
                        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
                        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                        print!("{escaped}");
                    }
                    println!();
                }

                let pg = ProgramGraph::new(Determinism::Deterministic, &cmds);
                println!("{}", pg.dot());

                info!(
                    "{:?}",
                    Interpreter::evaluate(1000, InterpreterMemory::zero(&pg), &pg)
                );

                std::thread::sleep(Duration::from_secs(2));

                // print!("\x1B[2J\x1B[1;1H");
            }

            Ok(())
        }
        // Cli::Test {
        //     fuel,
        //     seed,
        //     program,
        //     command,
        // } => match command {
        //     Test::Interpreter {} => {
        //         let result = run_analysis(&InterpreterEnv, fuel, seed, &program);
        //         println!("{result:?}");
        //         Ok(())
        //     }
        //     Test::Security {} => {
        //         let result = run_analysis(&SecurityEnv, fuel, seed, &program);
        //         println!("{result:?}");
        //         Ok(())
        //     }
        //     Test::Sign {} => {
        //         let result = run_analysis(&SignEnv, fuel, seed, &program);
        //         println!("{result:?}");
        //         Ok(())
        //     }
        // },
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
