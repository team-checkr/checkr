use ce_core::{define_env, Env, Generate, ValidationResult};
use gcl::{
    ast::Commands,
    pg::{Determinism, ProgramGraph},
    stringify::Stringify,
};
use serde::{Deserialize, Serialize};

define_env!(GraphEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphInput {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphOutput {
    pub dot: String,
}

impl Env for GraphEnv {
    type Input = GraphInput;

    type Output = GraphOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let dot = ProgramGraph::new(input.determinism, input.commands.inner()).dot();
        Ok(GraphOutput { dot })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for GraphInput {
    type Context = ();

    fn gen<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        GraphInput {
            commands: Stringify::new(Commands::gen(&mut Default::default(), rng)),
            determinism: Determinism::NonDeterministic,
        }
    }
}
