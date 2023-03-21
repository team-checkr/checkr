//! # checkr
//!
//! The checkr crate is the core analysis code for the checkr project. It
//! contains code for parsing the Guarded Command Language variant, running
//! analysis on them, infrastructure for generating random programs and inputs,
//! and a way to communicate with external implementations of the same analysis.
//!
//! ## Structuring analysis in environments
//!
//! Each analysis must implement the [`Environment`] trait. This defines the
//! input and output format for each analysis or environment. The input and
//! output are required to implement the serde [`Serialize`](serde::Serialize)
//! and [`Deserialize`](serde::Deserialize) traits, such that they can
//! communicate with the external world.
//!
//! ## Interacting with external implementations
//!
//! The primary goal of this project is to aid implementers of the same analysis
//! with checking their work continuously during development. To do so the
//! [`Driver`] struct together with the aforementioned [`Environment`] trait
//! provides an interface to interact with external code-bases in a generic way.
//!
//! ## Generating sample programs
//!
//! The [`generation`] module defines a trait for generating structures given a
//! source of randomness. It also implements this trait for all of the GCL
//! constructs, which allows programs to be generated in a programmatic way.
//! Similarly, the inputs of [`Environment`] implementations must too implement
//! [`Generate`].

use std::time::Duration;

use driver::Driver;
use env::{Environment, ValidationResult};
use generation::Generate;
pub use miette;
use rand::prelude::*;
use tracing::debug;

use crate::ast::Commands;

pub mod analysis;
pub mod ast;
pub mod driver;
pub mod egg;
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

#[derive(Debug, Default)]
pub struct ProgramGenerationBuilder {
    fuel: Option<u32>,
    seed: Option<u64>,
    no_loop: bool,
    no_division: bool,
    generate_annotated: bool,
}

impl Commands {
    pub fn builder() -> ProgramGenerationBuilder {
        ProgramGenerationBuilder::default()
    }
}

impl ProgramGenerationBuilder {
    pub fn fuel(self, fuel: Option<u32>) -> Self {
        ProgramGenerationBuilder { fuel, ..self }
    }
    pub fn seed(self, seed: Option<u64>) -> Self {
        ProgramGenerationBuilder { seed, ..self }
    }
    pub fn no_loop(self, no_loop: bool) -> Self {
        ProgramGenerationBuilder { no_loop, ..self }
    }
    pub fn no_division(self, no_division: bool) -> Self {
        ProgramGenerationBuilder {
            no_division,
            ..self
        }
    }

    pub fn generate_annotated(self, generate_annotated: bool) -> ProgramGenerationBuilder {
        ProgramGenerationBuilder {
            generate_annotated,
            ..self
        }
    }
    fn internal_build(self, cmds: Option<Commands>) -> GeneratedProgram {
        let seed = match self.seed {
            Some(seed) => seed,
            None => rand::random(),
        };
        let mut rng = SmallRng::seed_from_u64(seed);

        let fuel = self.fuel.unwrap_or(10);

        let mut cx = generation::Context::new(fuel, &mut rng);
        cx.set_no_loop(self.no_loop)
            .set_no_division(self.no_division);

        let cmds = cmds.unwrap_or_else(|| Commands(cx.many(5, 10, &mut rng)));
        let cmds = if self.generate_annotated {
            Commands(vec![generation::annotate_cmds(cmds, &mut rng)])
        } else {
            cmds
        };

        GeneratedProgram {
            cmds,
            fuel,
            seed,
            rng,
        }
    }
    pub fn from_cmds(self, cmds: Commands) -> GeneratedProgram {
        self.internal_build(Some(cmds))
    }
    pub fn build(self) -> GeneratedProgram {
        self.internal_build(None)
    }
}

#[derive(Debug)]
pub struct GeneratedProgram {
    pub cmds: Commands,
    pub fuel: u32,
    pub seed: u64,
    pub rng: SmallRng,
}

impl GeneratedProgram {
    pub async fn run_analysis<E: Environment>(
        self,
        env: &E,
        driver: &Driver,
    ) -> AnalysisSummary<E> {
        debug!(name = E::ANALYSIS.to_string(), "running analysis");

        let GeneratedProgram {
            cmds,
            fuel,
            seed,
            mut rng,
        } = self;

        let input = <E as Environment>::Input::gen(&mut cmds.clone(), &mut rng);
        let exec_result = driver.exec::<E>(&cmds, &input).await;
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
                driver::ExecError::RunExec { cmd, source } => AnalysisSummary {
                    fuel,
                    seed,
                    cmds,
                    input,
                    output: None,
                    time: Duration::ZERO,
                    stdout: String::new(),
                    stderr: String::new(),
                    result: Err(source.into()),
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
                    stderr: String::from_utf8(run_output.stderr)
                        .expect("stderr should be valid utf8"),
                    result: Err(inner.into()),
                },
            },
        }
    }
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
    pub result: color_eyre::Result<ValidationResult>,
}
