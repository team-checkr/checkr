use std::{ops::Deref, str::FromStr};

use rand::rngs::SmallRng;
use serde::{Deserialize, Serialize};

use crate::{ast::Commands, generation::Generate, sign::Memory, ProgramGenerationBuilder};
pub use graph::GraphEnv;
pub use interpreter::InterpreterEnv;
pub use parse::ParseEnv;
pub use pv::ProgramVerificationEnv;
pub use security::SecurityEnv;
pub use sign::SignEnv;

pub mod graph;
pub mod interpreter;
pub mod parse;
pub mod pv;
pub mod security;
pub mod sign;

macro_rules! define_analysis {
    ( $( $name:ident($env:path, $display:literal, $cmd:literal) ),* $(,)? ) => {
        impl std::fmt::Display for Analysis {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Analysis::$name => write!(f, $display), )*
                }
            }
        }

        impl Analysis {
            pub fn command(&self) -> &'static str {
                match self {
                    $( Analysis::$name => $cmd, )*
                }
            }
        }

        impl FromStr for Analysis {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $cmd => Ok(Analysis::$name), )*
                    _ => Err(()),
                }
            }
        }

        impl std::ops::Deref for Analysis {
            type Target = dyn AnyEnvironment;

            fn deref(&self) -> &Self::Target {
                match self {
                    $( Analysis::$name => &$env, )*
                }
            }
        }

        #[typeshare::typeshare]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, tsify::Tsify)]
        #[tsify(into_wasm_abi, from_wasm_abi)]
        pub enum AnalysisInput {
            $( $name(<$env as Environment>::Input), )*
        }

        impl AnalysisInput {
            pub fn analysis(&self) -> Analysis {
                match self {
                    $( AnalysisInput::$name(_) => Analysis::$name, )*
                }
            }
        }

        #[typeshare::typeshare]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, tsify::Tsify)]
        #[tsify(into_wasm_abi, from_wasm_abi)]
        pub enum AnalysisOutput {
            $( $name(<$env as Environment>::Output), )*
        }

        impl AnalysisOutput {
            pub fn analysis(&self) -> Analysis {
                match self {
                    $( AnalysisOutput::$name(_) => Analysis::$name, )*
                }
            }
        }
    };
}
#[typeshare::typeshare]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
    tsify::Tsify,
)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum Analysis {
    Graph,
    Parse,
    Interpreter,
    ProgramVerification,
    Sign,
    Security,
}

define_analysis!(
    Graph(GraphEnv, "Graph", "graph"),
    Parse(ParseEnv, "Parse", "parse"),
    Interpreter(InterpreterEnv, "Interpreter", "interpreter"),
    ProgramVerification(
        ProgramVerificationEnv,
        "Program verification",
        "program-verification"
    ),
    Sign(SignEnv, "Sign", "sign"),
    Security(SecurityEnv, "Security", "security"),
);

#[typeshare::typeshare]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, tsify::Tsify,
)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Markdown(String);

impl From<String> for Markdown {
    fn from(value: String) -> Self {
        Markdown(value)
    }
}
impl From<Markdown> for String {
    fn from(value: Markdown) -> Self {
        value.0
    }
}
impl std::ops::Deref for Markdown {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

pub trait ToMarkdown {
    fn to_markdown(&self) -> Markdown;
}

pub trait Environment {
    type Input: Generate<Context = Commands> + Serialize + for<'a> Deserialize<'a> + ToMarkdown;
    type Output: Serialize + for<'a> Deserialize<'a> + ToMarkdown;

    const ANALYSIS: Analysis;

    fn setup_generation(&self) -> ProgramGenerationBuilder {
        Default::default()
    }

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

pub trait AnyEnvironment {
    fn analysis(&self) -> Analysis;

    fn setup_generation(&self) -> ProgramGenerationBuilder;

    fn run(&self, cmds: &Commands, input: &str) -> Result<String, serde_json::Error>;

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> String;

    fn validate(
        &self,
        cmds: &Commands,
        input: &str,
        output: &str,
    ) -> Result<ValidationResult, serde_json::Error>;

    fn input_markdown(&self, input: &str) -> Result<Markdown, serde_json::Error>;
    fn output_markdown(&self, output: &str) -> Result<Markdown, serde_json::Error>;
}

impl<E: Environment> AnyEnvironment for E {
    fn analysis(&self) -> Analysis {
        E::ANALYSIS
    }

    fn setup_generation(&self) -> ProgramGenerationBuilder {
        self.setup_generation()
    }

    fn run(&self, cmds: &Commands, input: &str) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.run(cmds, &serde_json::from_str(input)?))
    }

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> String {
        serde_json::to_string(&E::Input::gen(&mut cmds.clone(), rng))
            .expect("failed to serialize input")
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &str,
        output: &str,
    ) -> Result<ValidationResult, serde_json::Error> {
        Ok(self.validate(
            cmds,
            &serde_json::from_str(input)?,
            &serde_json::from_str(output)?,
        ))
    }

    fn input_markdown(&self, input: &str) -> Result<Markdown, serde_json::Error> {
        let input: E::Input = serde_json::from_str(input)?;
        Ok(input.to_markdown())
    }

    fn output_markdown(&self, output: &str) -> Result<Markdown, serde_json::Error> {
        let output: E::Output = serde_json::from_str(output)?;
        Ok(output.to_markdown())
    }
}

impl Analysis {
    pub fn as_env(&self) -> &dyn AnyEnvironment {
        self.deref()
    }

    pub fn map_env<T>(&self, mut f: impl FnMut(&dyn AnyEnvironment) -> T) -> T {
        f(self.as_env())
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
