#![feature(box_patterns, box_syntax)]

use clap::Parser;
use rand::prelude::*;

use crate::{
    ast::Command,
    generation::{Context, Generate},
};

pub mod ast;
pub mod fmt;
pub mod generation;

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

            let cmds: Vec<Command> = cx.many(2, 10, &mut rng);

            println!("{}", crate::fmt::fmt_commands(&cmds));

            Ok(())
        }
    }
}
