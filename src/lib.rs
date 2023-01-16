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

pub fn generate_program(fuel: Option<u32>, seed: Option<u64>) -> (Commands, u32, u64, SmallRng) {
    let seed = match seed {
        Some(seed) => seed,
        None => rand::random(),
    };
    let mut rng = SmallRng::seed_from_u64(seed);

    let fuel = fuel.unwrap_or(10);

    let mut cx = generation::Context::new(fuel, &mut rng);

    (Commands(cx.many(5, 10, &mut rng)), fuel, seed, rng)
}

#[derive(Debug)]
pub struct AnalysisSummary<E: Environment> {
    pub fuel: u32,
    pub seed: u64,
    pub cmds: Commands,
    pub input: E::Input,
    pub output: Option<E::Output>,
    pub time: std::time::Duration,
    pub stdout: String,
    pub stderr: String,
    pub result: anyhow::Result<ValidationResult>,
}

pub fn run_analysis<E: Environment>(
    env: &E,
    current_dir: impl AsRef<Path>,
    fuel: Option<u32>,
    seed: Option<u64>,
    program: &str,
    command: &str,
) -> anyhow::Result<AnalysisSummary<E>> {
    let (src, fuel, seed, mut rng) = generate_program(fuel, seed);

    let mut args = program.split(' ');

    let mut cmd = std::process::Command::new(args.next().unwrap());
    cmd.args(args);
    cmd.arg(command);
    cmd.arg(src.to_string());

    let current_dir = current_dir.as_ref();
    cmd.current_dir(current_dir);

    let input = <E as Environment>::Input::gen(&mut src.clone(), &mut rng);
    cmd.arg(serde_json::to_string(&input)?);

    let before = std::time::Instant::now();
    let cmd_output = cmd
        .output()
        .with_context(|| format!("spawning {program:?}"))?;
    let took = before.elapsed();
    let stdout = std::str::from_utf8(&cmd_output.stdout).unwrap().to_string();
    let stderr = std::str::from_utf8(&cmd_output.stderr).unwrap().to_string();

    let (output, result) =
        match serde_json::from_slice(&cmd_output.stdout).with_context(|| "parsing output") {
            Ok(output) => {
                let result = env.validate(&src, &input, &output);
                (Some(output), Ok(result))
            }
            Err(err) => (None, Err(err)),
        };

    Ok(AnalysisSummary {
        fuel,
        seed,
        cmds: src,
        time: took,
        input,
        output,
        stdout,
        stderr,
        result,
    })
}
