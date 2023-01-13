use std::collections::{BTreeSet, HashMap, HashSet};

use itertools::Itertools;
use rand::{rngs::SmallRng, seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{mono_analysis, AnalysisResults, FiFo},
    ast::{Commands, Variable},
    generation::Generate,
    interpreter::{Interpreter, InterpreterMemory, ProgramState},
    pg::{Determinism, ProgramGraph},
    security::{Flow, SecurityAnalysisResult, SecurityClass, SecurityLattice},
    sign::{Memory, Sign, SignAnalysis, SignMemory},
};

pub trait Environment {
    type Input: Generate<Context = Commands> + Serialize + for<'a> Deserialize<'a>;
    type Output: Serialize + for<'a> Deserialize<'a>;

    fn name(&self) -> String;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output;

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        let reference = self.run(cmds, input);

        if &reference == output {
            ValidationResult::CorrectTerminated
        } else {
            println!("{reference:#?} != {output:#?}");
            ValidationResult::Mismatch
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated,
    Mismatch,
    TimeOut,
}

pub struct SecurityAnalysis;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLatticeInput(Vec<Flow<SecurityClass>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisInput {
    pub classification: HashMap<Variable, SecurityClass>,
    pub lattice: SecurityLatticeInput,
}

impl Generate for SecurityAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let classification = cx
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
                    .choose(rng)
                    .unwrap()
                    .clone(),
                )
            })
            .collect();
        let lattice = SecurityLatticeInput(vec![
            Flow {
                from: SecurityClass("A".to_string()),
                into: SecurityClass("B".to_string()),
            },
            Flow {
                from: SecurityClass("C".to_string()),
                into: SecurityClass("D".to_string()),
            },
        ]);

        SecurityAnalysisInput {
            classification,
            lattice,
        }
    }
}

impl Environment for SecurityAnalysis {
    type Input = SecurityAnalysisInput;

    type Output = SecurityAnalysisResult;

    fn name(&self) -> String {
        "Security Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let lattice = SecurityLattice::new(&input.lattice.0);
        SecurityAnalysisResult::run(&input.classification, &lattice, cmds)
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        let mut reference = self.run(cmds, input);
        reference.actual.sort();
        reference.allowed.sort();
        reference.violations.sort();
        let mut output = output.clone();
        output.actual.sort();
        output.allowed.sort();
        output.violations.sort();

        if reference == output {
            ValidationResult::CorrectTerminated
        } else {
            println!("{input:?}");
            println!("{cmds}");
            println!("{reference:#?} != {output:#?}");
            ValidationResult::Mismatch
        }
    }
}

pub struct StepWise;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepWiseInput {
    pub determinism: Determinism,
    pub initialization: HashMap<Variable, i64>,
    pub trace_count: usize,
}

impl Generate for StepWiseInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        StepWiseInput {
            determinism: Determinism::Deterministic,
            initialization: cx
                .fv()
                .into_iter()
                .map(|v| (v, rng.gen_range(-10..=10)))
                .collect(),
            trace_count: rng.gen_range(10..=15),
        }
    }
}

impl Environment for StepWise {
    type Input = StepWiseInput;

    type Output = Vec<ProgramState>;

    fn name(&self) -> String {
        "Step-wise Execution".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);

        Interpreter::evaluate(
            input.trace_count,
            InterpreterMemory {
                variables: input.initialization.clone(),
                arrays: Default::default(),
            },
            &pg,
        )
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq,
    {
        let reference = self.run(cmds, input);

        if &reference != output {
            return ValidationResult::Mismatch;
        }

        if let Some(last) = output.last() {
            match last {
                ProgramState::Running(_, _) => ValidationResult::CorrectNonTerminated,
                ProgramState::Terminated(_) | ProgramState::Stuck(_, _) => {
                    ValidationResult::CorrectTerminated
                }
            }
        } else {
            ValidationResult::Mismatch
        }
    }
}

pub trait AnyEnvironment {
    fn name(&self) -> String;

    fn gen_input(
        &self,
        cmds: &Commands,
        rng: &mut SmallRng,
    ) -> (serde_json::Value, serde_json::Value);
}

impl<E> AnyEnvironment for E
where
    E: Environment,
{
    fn name(&self) -> String {
        self.name()
    }

    fn gen_input(
        &self,
        cmds: &Commands,
        rng: &mut SmallRng,
    ) -> (serde_json::Value, serde_json::Value) {
        let input = E::Input::gen(&mut cmds.clone(), rng);
        let output = self.run(cmds, &input);

        (
            serde_json::to_value(input)
                .unwrap_or_else(|e| panic!("serializing input for '{}'\n{e}", self.name())),
            serde_json::to_value(output)
                .unwrap_or_else(|e| panic!("serializing output for '{}'\n{e}", self.name())),
        )
    }
}

pub struct Application {
    pub envs: Vec<Box<dyn AnyEnvironment>>,
}

impl Application {
    pub fn new() -> Self {
        Application { envs: vec![] }
    }
    pub fn add_env<E>(&mut self, env: E) -> &mut Self
    where
        E: Environment + 'static,
    {
        self.envs.push(box env);
        self
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SignEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignAnalysisInput {
    pub determinism: Determinism,
    pub assignment: SignMemory,
}

impl Generate for SignAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        SignAnalysisInput {
            determinism: [Determinism::Deterministic, Determinism::NonDeterministic]
                .choose(rng)
                .copied()
                .unwrap(),
            assignment: Memory::gen(cx, rng),
        }
    }
}

impl Generate for Sign {
    type Context = Commands;

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        *[Sign::Positive, Sign::Zero, Sign::Negative]
            .choose(rng)
            .unwrap()
    }
}
impl Generate for BTreeSet<Sign> {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        [Sign::gen(cx, rng)].into_iter().collect()
    }
}

impl<T, A> Generate for Memory<T, A>
where
    T: Generate<Context = Commands>,
    A: Generate<Context = Commands>,
{
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        Memory {
            variables: cx.fv().into_iter().map(|v| (v, T::gen(cx, rng))).collect(),
            arrays: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignAnalysisOutput(HashMap<String, HashSet<SignMemory>>);

impl Environment for SignEnv {
    type Input = SignAnalysisInput;

    type Output = SignAnalysisOutput;

    fn name(&self) -> String {
        "Detection of Signs Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        SignAnalysisOutput(
            mono_analysis::<_, FiFo>(
                SignAnalysis {
                    assignment: input.assignment.clone(),
                },
                &pg,
            )
            .facts
            .into_iter()
            .map(|(k, v)| (format!("{k}"), v))
            .collect(),
        )
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        let reference = self.run(cmds, input);

        let mut pool = reference.0.values().collect_vec();

        for o in output.0.values() {
            if let Some(idx) = pool.iter().position(|r| *r == o) {
                pool.remove(idx);
            } else {
                eprintln!("Produced world which did not exist in reference");
                return ValidationResult::Mismatch;
            }
        }

        if pool.is_empty() {
            ValidationResult::CorrectTerminated
        } else {
            eprintln!("Reference had world which was not present");
            ValidationResult::Mismatch
        }
    }
}
