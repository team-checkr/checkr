pub mod gen;

pub use gen::Generate;
use itertools::Either;
use serde::{Deserialize, Serialize};

pub use rand;

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
    InvalidInputForProgram {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
}

impl EnvError {
    pub fn from_parse_input(
        json: &serde_json::Value,
    ) -> impl FnOnce(serde_json::Error) -> EnvError + '_ {
        move |source| EnvError::ParseInput {
            source,
            json: Either::Left(json.clone()),
        }
    }
    pub fn from_parse_output(
        json: &serde_json::Value,
    ) -> impl FnOnce(serde_json::Error) -> EnvError + '_ {
        move |source| EnvError::ParseOutput {
            source,
            json: Either::Left(json.clone()),
        }
    }
}

pub type Result<T, E = EnvError> = std::result::Result<T, E>;

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisResult<E: Env> {
    Nothing,
    Stale {
        reference: E::Output,
        real: E::Output,
        validation: ValidationResult,
    },
    Active {
        reference: E::Output,
        real: E::Output,
        validation: ValidationResult,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Results<'a, E: Env> {
    reference: &'a E::Output,
    real: &'a E::Output,
    validation: &'a ValidationResult,
}

impl<'a, E: Env> Results<'a, E> {
    pub fn reference(&self) -> &'a E::Output {
        self.reference
    }
    pub fn real(&self) -> &'a E::Output {
        self.real
    }
    pub fn validation(&self) -> &'a ValidationResult {
        self.validation
    }
}

pub trait Env: Default + std::fmt::Debug + Clone + PartialEq {
    type Input: Generate<Context = ()>
        + Serialize
        + for<'a> Deserialize<'a>
        + tapi::Tapi
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Send
        + Sync;
    type Output: Serialize
        + for<'a> Deserialize<'a>
        + tapi::Tapi
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Send
        + Sync;
    type Meta: Default
        + Serialize
        + for<'a> Deserialize<'a>
        + tapi::Tapi
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Send
        + Sync;

    fn meta(_input: &Self::Input) -> Self::Meta {
        Default::default()
    }

    fn run(input: &Self::Input) -> Result<Self::Output>;
    fn validate(input: &Self::Input, output: &Self::Output) -> Result<ValidationResult>;
}

#[macro_export]
macro_rules! define_env {
    ($name:ident) => {
        #[derive(Debug, Default, Clone, PartialEq)]
        pub struct $name;

        #[test]
        fn env_roundtrip() {
            let mut rng =
                <$crate::rand::rngs::SmallRng as $crate::rand::SeedableRng>::seed_from_u64(0xCEC34);
            for _ in 0..1000 {
                let input =
                    <<$name as $crate::Env>::Input as $crate::Generate>::gen(&mut (), &mut rng);
                let output = <$name as $crate::Env>::run(&input).unwrap();
                let validation_result =
                    <$name as $crate::Env>::validate(&input, &output).expect("failed to validate");
                match validation_result {
                    $crate::ValidationResult::CorrectTerminated
                    | $crate::ValidationResult::CorrectNonTerminated { .. } => {
                        // Ok!
                    }
                    res => {
                        eprintln!("{}", serde_json::to_string_pretty(&input).unwrap());
                        panic!("validation failed! {res:?}")
                    }
                }
            }
        }
    };
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
}
