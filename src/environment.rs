use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ast::{Commands, Variable},
    interpreter::{Interpreter, Memory, ProgramState},
    pg::{Determinism, ProgramGraph},
    security::{SecurityAnalysisResult, SecurityClass, SecurityLattice},
};

pub trait Environment {
    type Input: Serialize + for<'a> Deserialize<'a>;
    type Output: Serialize + for<'a> Deserialize<'a>;

    fn name(&self) -> String;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output;
}

pub struct SecurityAnalysis;

impl Environment for SecurityAnalysis {
    type Input = (HashMap<Variable, SecurityClass>, SecurityLattice);

    type Output = SecurityAnalysisResult;

    fn name(&self) -> String {
        "Security Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        SecurityAnalysisResult::run(&input.0, &input.1, cmds)
    }
}

pub struct StepWise;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWiseParams {
    pub determinism: Determinism,
    pub initialization: HashMap<Variable, i64>,
    pub trace_count: usize,
}

impl Environment for StepWise {
    type Input = SetWiseParams;

    type Output = Vec<ProgramState>;

    fn name(&self) -> String {
        "Step-wise Execution".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);

        Interpreter::evaluate(
            input.trace_count,
            Memory {
                variables: input.initialization.clone(),
                arrays: Default::default(),
            },
            &pg,
        )
    }
}

pub trait AnyEnvironment {
    fn name(&self) -> String;
}

impl<E> AnyEnvironment for E
where
    E: Environment,
{
    fn name(&self) -> String {
        self.name()
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
