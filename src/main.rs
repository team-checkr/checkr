#![feature(box_patterns, box_syntax)]

use std::{collections::HashMap, path::PathBuf, time::Duration};

use anyhow::Context;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use rand::prelude::*;
use tracing::info;

use crate::{
    ast::{Commands, Variable},
    interpreter::Interpreter,
    pg::{Determinism, ProgramGraph},
    security::{SecurityAnalysis, SecurityClass, SecurityLattice},
};

pub mod analysis;
pub mod ast;
pub mod fmt;
pub mod generation;
pub mod interpreter;
pub mod parse;
pub mod pg;
pub mod security;

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
}

#[derive(Debug, Subcommand)]
enum Reference {
    Interpreter {
        #[clap(short, long)]
        src: String,
    },
    Security {
        #[clap(short, long)]
        src: String,
        #[clap(short, long)]
        classification: String,
        #[clap(short, long)]
        lattice: String,
    },
}

fn generate_program(fuel: Option<u32>, seed: Option<u64>) -> (Commands, SmallRng) {
    let seed = match seed {
        Some(seed) => seed,
        None => rand::random(),
    };
    let mut rng = SmallRng::seed_from_u64(seed);

    let fuel = match fuel {
        Some(fuel) => fuel,
        None => rng.gen_range(10..100),
    };

    let mut cx = generation::Context::new(fuel, &mut rng);

    (Commands(cx.many(5, 10, &mut rng)), rng)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    match Cli::parse() {
        Cli::Generate { fuel, seed } => {
            for _ in 0.. {
                let (cmds, _) = generate_program(fuel, seed);

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

                info!("{:?}", Interpreter::evaluate(&pg));

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
                let mut args = program.split(' ');

                let mut cmd = std::process::Command::new(args.next().unwrap());
                cmd.args(args);
                cmd.arg("interpreter");

                let (src, _) = generate_program(fuel, seed);
                cmd.args(["--src", &src.to_string()]);

                let output = cmd
                    .output()
                    .with_context(|| format!("spawning {program:?}"))?;

                todo!("{:?}", output);
            }
            Test::Security {} => {
                let mut args = program.split(' ');

                let mut cmd = std::process::Command::new(args.next().unwrap());
                cmd.args(args);
                cmd.arg("security");

                let (src, mut rng) = generate_program(fuel, seed);
                println!("{src}");
                let classification: HashMap<Variable, SecurityClass> = src
                    .fv()
                    .into_iter()
                    .map(|v| {
                        (
                            v,
                            [
                                SecurityClass("A".to_string()),
                                SecurityClass("B".to_string()),
                                SecurityClass("C".to_string()),
                                SecurityClass("D".to_string()),
                            ]
                            .choose(&mut rng)
                            .unwrap()
                            .clone(),
                        )
                    })
                    .collect();
                let lattice: SecurityLattice = SecurityLattice::parse("A < B, C < D")?;

                cmd.args(["--src", &src.to_string()]);
                cmd.args(["--lattice", &serde_json::to_string(&lattice)?]);
                cmd.args(["--classification", &serde_json::to_string(&classification)?]);

                let output = cmd
                    .output()
                    .with_context(|| format!("spawning {program:?}"))?;

                let result: SecurityAnalysis = serde_json::from_slice(&output.stdout)?;

                info!("Actual:     {}", result.actual.iter().sorted().format(", "));
                info!(
                    "Allowed:    {}",
                    result.allowed.iter().sorted().format(", ")
                );
                info!(
                    "Violations: {}",
                    result.violations.iter().sorted().format(", ")
                );

                Ok(())
            }
        },
        Cli::Reference { command } => match command {
            Reference::Interpreter { src } => {
                let cmds = parse::parse_commands(&src)?;

                let pg = ProgramGraph::new(Determinism::Deterministic, &cmds);

                println!("{:?}", Interpreter::evaluate(&pg));

                Ok(())
            }
            Reference::Security {
                src,
                classification,
                lattice,
            } => {
                let cmds = parse::parse_commands(&src)?;

                let classification = serde_json::from_str(&classification)?;
                let lattice = serde_json::from_str(&lattice)?;

                let result = SecurityAnalysis::run(&classification, &lattice, &cmds);

                println!("{}", serde_json::to_string(&result)?);

                Ok(())
            }
        },
    }
}
