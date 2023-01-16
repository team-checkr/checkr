#![feature(box_patterns, box_syntax)]

use std::path::Path;

use anyhow::Context;
use environment::{Environment, ValidationResult};
use generation::Generate;
use rand::prelude::*;

use crate::ast::Commands;

pub mod analysis;
pub mod ast;
pub mod environment;
pub mod fmt;
mod gcl;
pub mod generation;
pub mod interpreter;
pub mod parse;
pub mod pg;
pub mod security;
pub mod sign;

pub fn generate_program(fuel: Option<u32>, seed: Option<u64>) -> (Commands, SmallRng) {
    let seed = match seed {
        Some(seed) => seed,
        None => rand::random(),
    };
    let mut rng = SmallRng::seed_from_u64(seed);

    let fuel = fuel.unwrap_or(10);

    let mut cx = generation::Context::new(fuel, &mut rng);

    (Commands(cx.many(5, 10, &mut rng)), rng)
}

pub fn run_analysis<E: Environment>(
    env: E,
    current_dir: impl AsRef<Path>,
    fuel: Option<u32>,
    seed: Option<u64>,
    program: &str,
    command: &str,
) -> anyhow::Result<ValidationResult> {
    let (src, mut rng) = generate_program(fuel, seed);

    let mut args = program.split(' ');

    let mut cmd = std::process::Command::new(args.next().unwrap());
    cmd.args(args);
    cmd.arg(command);
    cmd.arg(src.to_string());

    let current_dir = current_dir.as_ref();
    cmd.current_dir(current_dir);

    let input = <E as Environment>::Input::gen(&mut src.clone(), &mut rng);
    cmd.arg(serde_json::to_string(&input)?);

    let output = cmd
        .output()
        .with_context(|| format!("spawning {program:?}"))?;
    eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());

    let output = serde_json::from_slice(&output.stdout).with_context(|| "parsing output")?;

    Ok(env.validate(&src, &input, &output))
}
