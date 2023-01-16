use std::collections::{BTreeSet, HashMap, HashSet};

use itertools::Itertools;
use rand::{rngs::SmallRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    analysis::{mono_analysis, FiFo},
    ast::{Commands, Variable},
    generation::Generate,
    interpreter::{Interpreter, InterpreterMemory, ProgramTrace},
    pg::{Determinism, Node, ProgramGraph},
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
    ) -> ValidationResult;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated,
    Mismatch { reason: String },
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
            ValidationResult::Mismatch {
                reason: format!("{input:?}\n{cmds}\n{reference:#?} != {output:#?}"),
            }
        }
    }
}

pub struct StepWise;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepWiseInput {
    pub determinism: Determinism,
    pub assignment: InterpreterMemory,
    pub trace_count: usize,
}

impl Generate for StepWiseInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        StepWiseInput {
            determinism: Determinism::Deterministic,
            assignment: Memory {
                variables: cx
                    .fv()
                    .into_iter()
                    .map(|v| (v, rng.gen_range(-10..=10)))
                    .collect(),
                arrays: Default::default(),
            },
            trace_count: rng.gen_range(10..=15),
        }
    }
}

impl Environment for StepWise {
    type Input = StepWiseInput;

    type Output = Vec<ProgramTrace<String>>;

    fn name(&self) -> String {
        "Step-wise Execution".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        Interpreter::evaluate(input.trace_count, input.assignment.clone(), &pg)
            .into_iter()
            .map(|t| t.map_node(|n| n.to_string()))
            .collect()
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
        let pg = ProgramGraph::new(input.determinism, cmds);
        let mut mem = vec![(Node::Start, input.assignment.clone())];

        for (idx, trace) in output.iter().skip(1).enumerate() {
            let mut next_mem = vec![];

            for (current_node, current_mem) in mem {
                for edge in pg.outgoing(current_node) {
                    if let Some(m) = edge.action().semantics(&current_mem) {
                        // TODO: check state
                        if m == trace.memory {
                            next_mem.push((edge.to(), m));
                        } else {
                            eprintln!("{cmds}");
                            debug!("Initial: {:?}", input.assignment);
                            debug!("Ref:     {m:?}");
                            debug!("Their:   {:?}", trace.memory);
                        }
                    }
                }
            }
            if next_mem.is_empty() {
                return ValidationResult::Mismatch {
                    reason: format!("The traces do not match after {idx} iterations"),
                };
            }
            mem = next_mem;
        }

        if output.len() < input.trace_count {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::CorrectNonTerminated
        }
    }
}

pub trait AnyEnvironment {
    fn name(&self) -> String;

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> (String, String);
}

impl<E> AnyEnvironment for E
where
    E: Environment,
    E::Input: std::fmt::Debug,
    E::Output: std::fmt::Debug,
{
    fn name(&self) -> String {
        self.name()
    }

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> (String, String) {
        let input = E::Input::gen(&mut cmds.clone(), rng);
        let output = self.run(cmds, &input);

        (format!("{input:#?}"), format!("{output:#?}"))
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
        E::Input: std::fmt::Debug,
        E::Output: std::fmt::Debug,
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
                return ValidationResult::Mismatch {
                    reason: "Produced world which did not exist in reference".to_string(),
                };
            }
        }

        if pool.is_empty() {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            }
        }
    }
}
