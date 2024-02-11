use ce_core::{define_env, rand, Env, Generate, ValidationResult};
use serde::{Deserialize, Serialize};

define_env!(CalcEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcInput {}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcOutput {}

impl Env for CalcEnv {
    type Input = CalcInput;

    type Output = CalcOutput;

    fn run(_input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(CalcOutput::default())
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for CalcInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self::default()
    }
}
