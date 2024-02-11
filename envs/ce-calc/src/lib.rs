use ce_core::{define_env, gen::GclGenContext, rand, Env, Generate, ValidationResult};
use gcl::{ast::AExpr, stringify::Stringify};
use serde::{Deserialize, Serialize};

define_env!(CalcEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcInput {
    pub expression: Stringify<AExpr>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalcOutput {
    pub result: String,
    pub error: Option<String>,
}

impl Env for CalcEnv {
    type Input = CalcInput;

    type Output = CalcOutput;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let expr = input.expression.try_parse().map_err(|err| {
            ce_core::EnvError::InvalidInputForProgram {
                message: "failed to parse expression".to_string(),
                source: Some(Box::new(err)),
            }
        })?;
        let (result, error) = match expr.semantics(&gcl::semantics::EmptySemanticsContext) {
            Ok(result) => (result.to_string(), None),
            Err(err) => {
                let error = format!("{}", err);
                (String::new(), Some(error))
            }
        };

        Ok(CalcOutput { result, error })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for CalcInput {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        CalcInput {
            expression: Stringify::new(AExpr::gen(
                &mut GclGenContext {
                    names: Vec::new(),
                    ..GclGenContext::new(25, rng)
                },
                rng,
            )),
        }
    }
}
