#![allow(non_snake_case)]

pub mod components;
pub mod gen;

use std::{marker::PhantomData, sync::Arc};

use dioxus::prelude::{Coroutine, Element, Props, ScopeState};
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
    pub set_input: Coroutine<E::Input>,
    pub input: Arc<E::Input>,
    pub reference_output: Arc<E::Output>,
    pub real_output: Arc<E::Output>,
    pub marker: PhantomData<&'a ()>,
}

impl<'a, E: Env> RenderProps<'a, E> {
    pub fn set_input(&self, input: E::Input) {
        self.set_input.send(input);
    }
}

pub trait Env: Default {
    type Input: Generate<Context = ()>
        + Serialize
        + for<'a> Deserialize<'a>
        + std::fmt::Debug
        + Send
        + Sync;
    type Output: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + Send + Sync;

    fn run(input: &Self::Input) -> Result<Self::Output>;
    fn validate(input: &Self::Input, output: &Self::Output) -> Result<ValidationResult>;
    fn render<'a>(cx: &'a ScopeState, props: &'a RenderProps<'a, Self>) -> Element<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u64 },
    Mismatch { reason: String },
    TimeOut,
}
