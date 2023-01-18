#![feature(box_patterns, box_syntax)]

use std::{path::Path, time::Duration};

use driver::Driver;
use env::{Environment, ValidationResult};
use generation::Generate;
use rand::prelude::*;
use tracing::debug;

use crate::ast::Commands;

pub mod analysis;
pub mod ast;
pub mod driver;
pub mod env;
pub mod fmt;
mod gcl;
pub mod generation;
pub mod interpreter;
pub mod parse;
pub mod pg;
pub mod pv;
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
) -> AnalysisSummary<E> {
    debug!(name = env.name(), "running analysis");

    let (cmds, fuel, seed, mut rng) = generate_program(fuel, seed);

    let input = <E as Environment>::Input::gen(&mut cmds.clone(), &mut rng);
    let exec_result =
        Driver::new(current_dir.as_ref().to_owned(), program.to_string()).exec::<E>(&cmds, &input);
    match exec_result {
        Ok(exec_result) => {
            let validation_result = env.validate(&cmds, &input, &exec_result.parsed);
            AnalysisSummary {
                fuel,
                seed,
                cmds,
                time: exec_result.took,
                input,
                output: Some(exec_result.parsed),
                stdout: String::from_utf8(exec_result.output.stdout)
                    .expect("failed to parse stdout"),
                stderr: String::from_utf8(exec_result.output.stderr)
                    .expect("failed to parse stderr"),
                result: Ok(validation_result),
            }
        }
        Err(err) => match err {
            driver::ExecError::Serialize(err) => AnalysisSummary {
                fuel,
                seed,
                cmds,
                input,
                output: None,
                time: Duration::ZERO,
                stdout: String::new(),
                stderr: String::new(),
                result: Err(err.into()),
            },
            driver::ExecError::RunExec(err) => AnalysisSummary {
                fuel,
                seed,
                cmds,
                input,
                output: None,
                time: Duration::ZERO,
                stdout: String::new(),
                stderr: String::new(),
                result: Err(err.into()),
            },
            driver::ExecError::CommandFailed(output, time) => AnalysisSummary {
                fuel,
                seed,
                cmds,
                input,
                output: None,
                time,
                stdout: String::from_utf8(output.stdout.clone())
                    .expect("stdout should be valid utf8"),
                stderr: String::from_utf8(output.stderr.clone())
                    .expect("stderr should be valid utf8"),
                result: Err(driver::ExecError::CommandFailed(output, time).into()),
            },
            driver::ExecError::Parse {
                inner,
                run_output,
                time,
            } => AnalysisSummary {
                fuel,
                seed,
                cmds,
                input,
                output: None,
                time,
                stdout: String::from_utf8(run_output.stdout.clone())
                    .expect("stdout should be valid utf8"),
                stderr: String::from_utf8(run_output.stderr.clone())
                    .expect("stderr should be valid utf8"),
                result: Err(inner.into()),
            },
        },
    }
}
