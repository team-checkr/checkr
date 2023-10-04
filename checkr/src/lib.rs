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
//! [`Generate`](generation::Generate).

use std::{borrow::Cow, time::Duration};

use driver::Driver;
use env::{Analysis, Environment, Input, ValidationResult};
pub use miette;
use rand::prelude::*;
use tracing::debug;

use gcl::ast::Commands;

pub mod config;
pub mod driver;
pub mod egg;
pub mod env;
pub mod generation;
pub mod interpreter;
pub mod pv;
pub mod security;

#[derive(Debug)]
pub struct ProgramGenerationBuilder {
    analysis: Analysis,
    fuel: Option<u32>,
    seed: Option<u64>,
    no_loop: bool,
    no_division: bool,
    generate_annotated: bool,
}

impl ProgramGenerationBuilder {
    pub fn new(analysis: Analysis) -> ProgramGenerationBuilder {
        ProgramGenerationBuilder {
            analysis,
            fuel: Default::default(),
            seed: Default::default(),
            no_loop: Default::default(),
            no_division: Default::default(),
            generate_annotated: Default::default(),
        }
    }

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

    pub fn generate_annotated(self, generate_annotated: bool) -> Self {
        ProgramGenerationBuilder {
            generate_annotated,
            ..self
        }
    }
    fn internal_build(self, cmds: Option<Commands>, input: Option<Input>) -> GeneratedProgram {
        let seed = match self.seed {
            Some(seed) => seed,
            None => rand::random(),
        };
        let mut rng = SmallRng::seed_from_u64(seed);

        let fuel = self.fuel.unwrap_or(10);

        let mut cx = generation::Context::new(fuel, &mut rng);
        cx.set_no_loop(self.no_loop)
            .set_no_division(self.no_division);

        let cmds = match cmds {
            Some(cmds) => cmds,
            None => {
                let cmds = Commands(cx.many(5, 10, &mut rng));
                if self.generate_annotated {
                    Commands(vec![generation::annotate_cmds(cmds, &mut rng)])
                } else {
                    cmds
                }
            }
        };
        let input = input.unwrap_or_else(|| self.analysis.gen_input(&cmds, &mut rng));

        GeneratedProgram {
            cmds,
            input,
            fuel,
            seed,
        }
    }
    pub fn from_cmds(self, cmds: Commands) -> GeneratedProgram {
        self.internal_build(Some(cmds), None)
    }
    pub fn from_cmds_and_input(self, cmds: Commands, input: Input) -> GeneratedProgram {
        self.internal_build(Some(cmds), Some(input))
    }
    pub fn build(self) -> GeneratedProgram {
        self.internal_build(None, None)
    }
}

#[derive(Debug)]
pub struct GeneratedProgram {
    pub cmds: Commands,
    pub input: Input,
    pub fuel: u32,
    pub seed: u64,
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
            input,
            fuel,
            seed,
        } = self;

        let input = input.parsed::<E>().unwrap();

        let timeout_duration = Duration::from_secs(10);
        let exec_result =
            tokio::time::timeout(timeout_duration, driver.exec::<E>(&cmds, &input)).await;
        match exec_result {
            Err(_) => AnalysisSummary {
                fuel,
                seed,
                cmds,
                input,
                output: None,
                time: timeout_duration,
                stdout: String::new(),
                stderr: String::new(),
                result: Ok(ValidationResult::TimeOut),
            },
            Ok(Ok(exec_result)) => {
                let validation_result = env.validate(&cmds, &input, &exec_result.parsed);
                AnalysisSummary {
                    fuel,
                    seed,
                    cmds,
                    time: exec_result.took,
                    input,
                    output: Some(exec_result.parsed),
                    stdout: truncated_from_utf8(exec_result.output.stdout),
                    stderr: truncated_from_utf8(exec_result.output.stderr),
                    result: validation_result.map_err(|err| err.into()),
                }
            }
            Ok(Err(err)) => match err {
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
                driver::ExecError::RunExec { cmd: _, source } => AnalysisSummary {
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
                    stdout: truncated_from_utf8(&output.stdout),
                    stderr: truncated_from_utf8(&output.stderr),
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
                    stdout: truncated_from_utf8(run_output.stdout),
                    stderr: truncated_from_utf8(run_output.stderr),
                    result: Err(inner.into()),
                },
            },
        }
    }
}

fn truncated_from_utf8<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> String {
    const MAX_SIZE: usize = 10_000;
    let bytes = match bytes.into() {
        Cow::Borrowed(bytes) => bytes.get(0..MAX_SIZE).unwrap_or(bytes).to_vec(),
        Cow::Owned(mut bytes) => {
            bytes.truncate(MAX_SIZE);
            bytes
        }
    };
    String::from_utf8(bytes).expect("should be valid utf8")
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
