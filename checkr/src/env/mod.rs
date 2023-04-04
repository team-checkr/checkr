use std::{ops::Deref, str::FromStr};

use itertools::Either;
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
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        ProgramGenerationBuilder::new(Self::ANALYSIS)
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Result<Self::Output, EnvError>;

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> Result<ValidationResult, EnvError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    analysis: Analysis,
    json: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    analysis: Analysis,
    json: serde_json::Value,
}

impl Input {
    pub fn from_concrete<E: Environment + ?Sized>(input: &E::Input) -> Self {
        Self {
            analysis: E::ANALYSIS,
            json: serde_json::to_value(input).expect("input is always valid json"),
        }
    }
    pub fn parsed<E: Environment + ?Sized>(self) -> Result<E::Input, EnvError> {
        // TODO: Assert that E::ANALYSIS == self.analysis
        serde_json::from_value(self.json.clone()).map_err(|source| EnvError::ParseInput {
            source,
            json: Either::Left(self.json),
        })
    }
    pub fn to_markdown(&self) -> Result<Markdown, EnvError> {
        self.analysis.input_markdown(self.clone())
    }
}
impl Output {
    pub fn from_concrete<E: Environment + ?Sized>(output: &E::Output) -> Self {
        Self {
            analysis: E::ANALYSIS,
            json: serde_json::to_value(output).expect("output is always valid json"),
        }
    }
    pub fn parsed<E: Environment + ?Sized>(self) -> Result<E::Output, EnvError> {
        // TODO: Assert that E::ANALYSIS == self.analysis
        serde_json::from_value(self.json.clone()).map_err(|source| EnvError::ParseOutput {
            source,
            json: Either::Left(self.json),
        })
    }
    pub fn to_markdown(&self) -> Result<Markdown, EnvError> {
        self.analysis.output_markdown(self.clone())
    }
}
impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.json.fmt(f)
    }
}
impl std::fmt::Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.json.fmt(f)
    }
}

pub trait AnyEnvironment {
    fn analysis(&self) -> Analysis;

    fn setup_generation(&self) -> ProgramGenerationBuilder;

    fn run(&self, cmds: &Commands, input: Input) -> Result<Output, EnvError>;

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> Input;

    fn validate(
        &self,
        cmds: &Commands,
        input: Input,
        output: Output,
    ) -> Result<ValidationResult, EnvError>;

    fn input_markdown(&self, input: Input) -> Result<Markdown, EnvError>;
    fn output_markdown(&self, output: Output) -> Result<Markdown, EnvError>;

    fn input_from_str(&self, src: &str) -> Result<Input, EnvError>;
    fn input_from_slice(&self, src: &[u8]) -> Result<Input, EnvError>;
    fn output_from_str(&self, src: &str) -> Result<Output, EnvError>;
    fn output_from_slice(&self, src: &[u8]) -> Result<Output, EnvError>;
}

impl<E: Environment + ?Sized> AnyEnvironment for E {
    fn analysis(&self) -> Analysis {
        E::ANALYSIS
    }

    fn setup_generation(&self) -> ProgramGenerationBuilder {
        self.setup_generation()
    }

    fn run(&self, cmds: &Commands, input: Input) -> Result<Output, EnvError> {
        Ok(Output {
            analysis: self.analysis(),
            json: serde_json::to_value(&self.run(cmds, &input.parsed::<E>()?)?)
                .expect("all output should be serializable"),
        })
    }

    fn gen_input(&self, cmds: &Commands, rng: &mut SmallRng) -> Input {
        Input {
            analysis: self.analysis(),
            json: serde_json::to_value(&E::Input::gen(&mut cmds.clone(), rng))
                .expect("failed to serialize input"),
        }
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: Input,
        output: Output,
    ) -> Result<ValidationResult, EnvError> {
        self.validate(cmds, &input.parsed::<E>()?, &output.parsed::<E>()?)
    }

    fn input_markdown(&self, input: Input) -> Result<Markdown, EnvError> {
        let input = input.parsed::<E>()?;
        Ok(input.to_markdown())
    }

    fn output_markdown(&self, output: Output) -> Result<Markdown, EnvError> {
        let output = output.parsed::<E>()?;
        Ok(output.to_markdown())
    }

    fn input_from_str(&self, src: &str) -> Result<Input, EnvError> {
        Ok(Input {
            analysis: self.analysis(),
            json: serde_json::from_str(src).map_err(|source| EnvError::ParseInput {
                source,
                json: Either::Right(src.to_string()),
            })?,
        })
    }

    fn input_from_slice(&self, src: &[u8]) -> Result<Input, EnvError> {
        Ok(Input {
            analysis: self.analysis(),
            json: serde_json::from_slice(src).map_err(|source| EnvError::ParseInput {
                source,
                json: Either::Right(
                    std::str::from_utf8(src)
                        .expect("input should be valid utf8")
                        .to_string(),
                ),
            })?,
        })
    }

    fn output_from_str(&self, src: &str) -> Result<Output, EnvError> {
        Ok(Output {
            analysis: self.analysis(),
            json: serde_json::from_str(src).map_err(|source| EnvError::ParseOutput {
                source,
                json: Either::Right(src.to_string()),
            })?,
        })
    }

    fn output_from_slice(&self, src: &[u8]) -> Result<Output, EnvError> {
        Ok(Output {
            analysis: self.analysis(),
            json: serde_json::from_slice(src).map_err(|source| EnvError::ParseOutput {
                source,
                json: Either::Right(
                    std::str::from_utf8(src)
                        .expect("input should be valid utf8")
                        .to_string(),
                ),
            })?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnvError {
    #[error("failed to parse json input: {source}")]
    ParseInput {
        source: serde_json::Error,
        json: Either<serde_json::Value, String>,
    },
    #[error("failed to parse json output: {source}")]
    ParseOutput {
        source: serde_json::Error,
        json: Either<serde_json::Value, String>,
    },
    #[error("input is not valid for the current program: {message}")]
    InvalidInputForProgram { input: Input, message: String },
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
