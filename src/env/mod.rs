use std::str::FromStr;

use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};

use crate::{ast::Commands, generation::Generate, sign::Memory};
pub use graph::GraphEnv;
pub use interpreter::InterpreterEnv;
pub use pv::ProgramVerificationEnv;
pub use security::SecurityEnv;
pub use sign::SignEnv;

pub mod graph;
pub mod interpreter;
pub mod pv;
pub mod security;
pub mod sign;

#[typeshare::typeshare]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
pub enum Analysis {
    Graph,
    Interpreter,
    ProgramVerification,
    Sign,
    Security,
}

pub enum AnalysisInput {
    Graph(<GraphEnv as Environment>::Input),
    Sign(<SignEnv as Environment>::Input),
    Interpreter(<InterpreterEnv as Environment>::Input),
    Security(<SecurityEnv as Environment>::Input),
    ProgramVerification(<ProgramVerificationEnv as Environment>::Input),
}

impl std::fmt::Display for Analysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Analysis::Graph => write!(f, "Graph"),
            Analysis::Sign => write!(f, "Sign"),
            Analysis::Interpreter => write!(f, "Interpreter"),
            Analysis::Security => write!(f, "Security"),
            Analysis::ProgramVerification => write!(f, "Program verification"),
        }
    }
}
impl Analysis {
    pub fn command(&self) -> &'static str {
        match self {
            Analysis::Graph => "graph",
            Analysis::Sign => "sign",
            Analysis::Interpreter => "interpreter",
            Analysis::Security => "security",
            Analysis::ProgramVerification => "program-verification",
        }
    }
}
impl FromStr for Analysis {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "graph" => Ok(Analysis::Graph),
            "sign" => Ok(Analysis::Sign),
            "interpreter" => Ok(Analysis::Interpreter),
            "security" => Ok(Analysis::Security),
            "program-verification" => Ok(Analysis::ProgramVerification),
            _ => Err(()),
        }
    }
}

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}

pub trait Environment {
    type Input: Generate<Context = Commands> + Serialize + for<'a> Deserialize<'a>;
    type Output: Serialize + for<'a> Deserialize<'a>;

    const ANALYSIS: Analysis;

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
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    pub input_json: serde_json::Value,
    pub input_markdown: String,
    pub output_markdown: String,
}

pub trait AnyEnvironment {
    fn analysis(&self) -> Analysis;

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> serde_json::Value;

    fn gen_sample(&self, cmds: &Commands, rng: &mut SmallRng) -> Sample;
}

impl<E> AnyEnvironment for E
where
    E: Environment,
    E::Input: std::fmt::Debug + ToMarkdown,
    E::Output: std::fmt::Debug + ToMarkdown,
{
    fn analysis(&self) -> Analysis {
        E::ANALYSIS
    }

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> serde_json::Value {
        serde_json::to_value(&E::Input::gen(&mut cmds.clone(), rng))
            .expect("failed to serialize input")
    }

    fn gen_sample(&self, cmds: &Commands, rng: &mut SmallRng) -> Sample {
        let input = E::Input::gen(&mut cmds.clone(), rng);
        let output = self.run(cmds, &input);

        Sample {
            input_json: serde_json::to_value(&input).unwrap(),
            input_markdown: input.to_markdown(),
            output_markdown: output.to_markdown(),
        }
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
        Memory::from_targets_with(
            cx.fv(),
            (cx, rng),
            |(cx, rng), _| T::gen(cx, rng),
            |(cx, rng), _| A::gen(cx, rng),
        )
    }
}
