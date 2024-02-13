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
    pub error: String,
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
            Ok(result) => (result.to_string(), String::new()),
            Err(err) => {
                let error = format!("{}", err);
                (String::new(), error)
            }
        };

        Ok(CalcOutput { result, error })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let reference = Self::run(input)?;

        match (
            !reference.result.is_empty(),
            !output.result.is_empty(),
            !reference.error.is_empty(),
            !output.error.is_empty(),
        ) {
            // Both results are present
            (true, true, _, _) => Ok(ValidationResult::CorrectTerminated),
            // Both errors are present
            (_, _, true, true) => Ok(ValidationResult::CorrectTerminated),
            (_, _, _, _) => {
                let info = format!(
                    "Output: result={:?}, error={:?}; Reference: result={:?}, error={:?}",
                    output.result, output.error, reference.result, reference.error,
                );
                Ok(ValidationResult::Mismatch {
                    reason: format!("Did not produce same as reference. {info}"),
                })
            }
        }
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
