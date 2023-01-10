#![feature(box_patterns, box_syntax)]

use std::time::Duration;

use clap::Parser;
use rand::prelude::*;

use crate::{
    ast::{Command, Commands},
    generation::Context,
    pg::{Determinism, ProgramGraph},
};

pub mod ast;
pub mod fmt;
pub mod generation;
pub mod pg;

#[derive(Debug, Parser)]
enum Cli {
    /// Generate a program
    Generate {
        #[clap(short, long)]
        fuel: Option<u32>,
        #[clap(short, long)]
        seed: Option<u64>,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    match Cli::parse() {
        Cli::Generate { fuel, seed } => {
            for _ in 0.. {
                let seed = match seed {
                    Some(seed) => seed,
                    None => rand::random(),
                };
                let mut rng = SmallRng::seed_from_u64(seed);

                let fuel = match fuel {
                    Some(fuel) => fuel,
                    None => rng.gen_range(10..100),
                };

                dbg!(seed, fuel);

                let mut cx = Context::new(fuel, &mut rng);

                let cmds = Commands(cx.many(5, 10, &mut rng));

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

                std::thread::sleep(Duration::from_secs(2));

                // print!("\x1B[2J\x1B[1;1H");
            }

            Ok(())
        }
    }
}
