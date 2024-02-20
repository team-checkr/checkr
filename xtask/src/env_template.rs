use ce_core::{define_env, rand, Env, Generate, ValidationResult};
use serde::{Deserialize, Serialize};

define_env!(TemplateEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInput {}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateOutput {}

impl Env for TemplateEnv {
    type Input = TemplateInput;

    type Output = TemplateOutput;

    type Meta = ();

    fn run(_input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(TemplateOutput::default())
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for TemplateInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self::default()
    }
}
