use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};

use crate::{ast::Commands, generation::Generate, sign::Memory};
pub use security::SecurityEnv;
pub use sign::SignEnv;
pub use step_wise::StepWiseEnv;

pub mod graph;
pub mod pv;
pub mod security;
pub mod sign;
pub mod step_wise;

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}

pub trait Environment {
    type Input: Generate<Context = Commands> + Serialize + for<'a> Deserialize<'a>;
    type Output: Serialize + for<'a> Deserialize<'a>;

    fn command() -> &'static str;
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
    CorrectNonTerminated { iterations: usize },
    Mismatch { reason: String },
    TimeOut,
}
pub trait AnyEnvironment {
    fn command(&self) -> &'static str;
    fn name(&self) -> String;

    fn gen_sample(&self, cmds: &Commands, rng: &mut SmallRng) -> (String, String);
}

impl<E> AnyEnvironment for E
where
    E: Environment,
    E::Input: std::fmt::Debug + ToMarkdown,
    E::Output: std::fmt::Debug + ToMarkdown,
{
    fn command(&self) -> &'static str {
        E::command()
    }
    fn name(&self) -> String {
        self.name()
    }

    fn gen_sample(&self, cmds: &Commands, rng: &mut SmallRng) -> (String, String) {
        let input = E::Input::gen(&mut cmds.clone(), rng);
        let output = self.run(cmds, &input);

        (input.to_markdown(), output.to_markdown())
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
        E::Input: std::fmt::Debug + ToMarkdown,
        E::Output: std::fmt::Debug + ToMarkdown,
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
