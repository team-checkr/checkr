use std::collections::{BTreeSet, HashMap};

use rand::{rngs::SmallRng, seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{mono_analysis, AnalysisResults, FiFo},
    ast::{Commands, Variable},
    generation::Generate,
    interpreter::{Interpreter, InterpreterMemory, ProgramState},
    pg::{Determinism, ProgramGraph},
    security::{SecurityAnalysisResult, SecurityClass, SecurityLattice},
    sign::{Memory, Sign, SignAnalysis, SignMemory},
};

pub trait Environment {
    type Input: Generate<Context = Commands> + Serialize + for<'a> Deserialize<'a>;
    type Output: Serialize + for<'a> Deserialize<'a>;

    fn name(&self) -> String;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output;
}

pub struct SecurityAnalysis;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisInput {
    pub classification: HashMap<Variable, SecurityClass>,
    pub lattice: SecurityLattice,
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
        let lattice = SecurityLattice::parse("A < B, C < D").unwrap();

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
        SecurityAnalysisResult::run(&input.classification, &input.lattice, cmds)
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
    pub initial_signs: SignMemory,
}

impl Generate for SignAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        SignAnalysisInput {
            determinism: [Determinism::Deterministic, Determinism::NonDeterministic]
                .choose(rng)
                .copied()
                .unwrap(),
            initial_signs: Memory::gen(cx, rng),
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

impl Environment for SignEnv {
    type Input = SignAnalysisInput;

    type Output = AnalysisResults<SignAnalysis>;

    fn name(&self) -> String {
        "Detection of Signs Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        mono_analysis::<_, FiFo>(SignAnalysis, &pg)
    }
}
