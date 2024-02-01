use ce_core::{define_env, rand, Env, Generate, ValidationResult};
use gcl::ast::Commands;
use serde::{Deserialize, Serialize};

define_env!(ParseEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseInput {
    commands: Commands,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseOutput {
    pretty: String,
}

impl Env for ParseEnv {
    type Input = ParseInput;

    type Output = ParseOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(ParseOutput {
            pretty: input.commands.to_string(),
        })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for ParseInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Self {
            commands: Commands::gen(&mut Default::default(), rng),
        }
    }
}
