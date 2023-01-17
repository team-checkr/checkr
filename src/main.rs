#![feature(box_patterns, box_syntax)]

use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::info;

use verification_lawyer::{
    env::{Application, Environment, SecurityEnv, SignEnv, StepWiseEnv},
    generate_program,
    interpreter::{Interpreter, InterpreterMemory},
    parse,
    pg::{Determinism, ProgramGraph},
    run_analysis,
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
    /// Test subcommand
    Test {
        #[clap(short, long)]
        fuel: Option<u32>,
        #[clap(short, long)]
        seed: Option<u64>,
        #[clap(short, long)]
        program: String,
        #[command(subcommand)]
        command: Test,
    },
    /// Reference subcommand
    Reference {
        #[command(subcommand)]
        command: Reference,
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
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    let mut app = Application::new();
    app.add_env(SecurityEnv).add_env(StepWiseEnv);

    match Cli::parse() {
        Cli::Generate { fuel, seed } => {
            for _ in 0.. {
                let (cmds, _, _, _) = generate_program(fuel, seed);

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
        Cli::Test {
            fuel,
            seed,
            program,
            command,
        } => match command {
            Test::Interpreter {} => {
                let result = run_analysis(&StepWiseEnv, ".", fuel, seed, &program);
                println!("{result:?}");
                Ok(())
            }
            Test::Security {} => {
                let result = run_analysis(&SecurityEnv, ".", fuel, seed, &program);
                println!("{result:?}");
                Ok(())
            }
            Test::Sign {} => {
                let result = run_analysis(&SignEnv, ".", fuel, seed, &program);
                println!("{result:?}");
                Ok(())
            }
        },
        Cli::Reference { command } => match command {
            Reference::Interpreter { src, input } => {
                let cmds = parse::parse_commands(&src)?;

                let env = StepWiseEnv;
                let output = env.run(&cmds, &serde_json::from_str(&input)?);

                println!("{}", serde_json::to_string(&output)?);

                Ok(())
            }
            Reference::Security { src, input } => {
                let cmds = parse::parse_commands(&src)?;

                let env = SecurityEnv;
                let output = env.run(&cmds, &serde_json::from_str(&input)?);

                println!("{}", serde_json::to_string(&output)?);

                Ok(())
            }
            Reference::Sign { src, input } => {
                let cmds = parse::parse_commands(&src)?;

                let env = SignEnv;
                let output = env.run(&cmds, &serde_json::from_str(&input)?);

                println!("{}", serde_json::to_string(&output)?);

                Ok(())
            }
        },
    }
}
