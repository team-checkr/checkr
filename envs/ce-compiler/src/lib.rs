use ce_core::{define_env, Env, Generate, ValidationResult};
use gcl::{
    ast::Commands,
    pg::{Determinism, ProgramGraph},
    stringify::Stringify,
};
use serde::{Deserialize, Serialize};

define_env!(CompilerEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Compiler")]
pub struct Input {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Compiler")]
pub struct Output {
    pub dot: String,
}

impl Env for CompilerEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let dot = ProgramGraph::new(
            input.determinism,
            &input.commands.try_parse().map_err(|err| {
                ce_core::EnvError::InvalidInputForProgram {
                    message: "failed to parse commands".to_string(),
                    source: Some(Box::new(err)),
                }
            })?,
        )
        .dot();
        Ok(Output { dot })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for Input {
    type Context = ();

    fn gen<R: ce_core::rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Input {
            commands: Stringify::new(Commands::gen(&mut Default::default(), rng)),
            determinism: Determinism::NonDeterministic,
        }
    }
}
