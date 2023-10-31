#![allow(non_snake_case)]

pub mod components;
pub mod gen;

use std::marker::PhantomData;

use dioxus::prelude::*;
pub use gen::Generate;
use itertools::Either;
use serde::{Deserialize, Serialize};

pub use dioxus_heroicons;
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
    InvalidInputForProgram { message: String },
}

pub type Result<T, E = EnvError> = std::result::Result<T, E>;

#[derive(Props)]
pub struct RenderProps<'a, E: Env> {
    set_input: Coroutine<E::Input>,
    input: E::Input,
    result: AnalysisResult<E>,
    marker: PhantomData<&'a ()>,
}

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

impl<'a, E: Env> RenderProps<'a, E> {
    pub fn new(
        set_input: Coroutine<E::Input>,
        input: E::Input,
        result: AnalysisResult<E>,
    ) -> RenderProps<'a, E> {
        RenderProps {
            set_input,
            input,
            result,
            marker: Default::default(),
        }
    }
    pub fn set_input(&self, input: E::Input) {
        self.set_input.send(input);
    }
    pub fn input(&self) -> &E::Input {
        &self.input
    }
    pub fn result(&self) -> &AnalysisResult<E> {
        &self.result
    }
    pub fn with_result(
        &self,
        cx: &'a ScopeState,
        f: impl FnOnce(Results<E>) -> Element<'a>,
    ) -> Element<'a> {
        match &self.result {
            AnalysisResult::Nothing => cx.render(
                rsx!(div { class: "grid place-items-center text-xl", span { "Loading..." }}),
            ),
            AnalysisResult::Stale {
                reference,
                real,
                validation,
            }
            | AnalysisResult::Active {
                reference,
                real,
                validation,
            } => f(Results {
                reference,
                real,
                validation,
            }),
        }
    }
}

pub trait Env: Default + std::fmt::Debug + Clone + PartialEq {
    type Input: Generate<Context = ()>
        + Serialize
        + for<'a> Deserialize<'a>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Send
        + Sync;
    type Output: Serialize
        + for<'a> Deserialize<'a>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Send
        + Sync;

    fn run(input: &Self::Input) -> Result<Self::Output>;
    fn validate(input: &Self::Input, output: &Self::Output) -> Result<ValidationResult>;
    fn render<'a>(cx: &'a ScopeState, props: &'a RenderProps<'a, Self>) -> Element<'a>;
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
                    res => panic!("validation failed! {res:?}"),
                }
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
    IncorretPostcondition,
    IncorrectInvariant,
    CannotBeValidated,
}
