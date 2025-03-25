use ce_core::{Env, Generate, ValidationResult, define_env, rand};
use serde::{Deserialize, Serialize};

define_env!(TemplateEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {}

impl Env for TemplateEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(_input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(Output::default())
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self::default()
    }
}
