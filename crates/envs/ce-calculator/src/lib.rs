use ce_core::{Env, Generate, ValidationResult, define_env, gn::GclGenContext, rand};
use gcl::ast::AExpr;
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;

define_env!(CalcEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Calculator")]
pub struct Input {
    pub expression: Stringify<AExpr>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Calculator")]
pub struct Output {
    pub result: String,
    pub error: String,
}

impl Env for CalcEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let expr =
            input
                .expression
                .try_parse()
                .map_err(ce_core::EnvError::invalid_input_for_program(
                    "failed to parse expression",
                ))?;
        let (result, error) = match expr.semantics(&gcl::semantics::EmptySemanticsContext) {
            Ok(result) => (result.to_string(), String::new()),
            Err(err) => {
                let error = format!("{}", err);
                (String::new(), error)
            }
        };

        Ok(Output { result, error })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let reference = Self::run(input)?;

        Ok(
            match (
                &reference.result,
                &output.result,
                !reference.error.is_empty(),
                !output.error.is_empty(),
            ) {
                // Both errors are present
                (_, _, true, true) => ValidationResult::Correct,
                // Both results are present
                (r, o, _, _) if r == o => ValidationResult::Correct,
                (_, _, _, _) => {
                    let info = format!(
                        "Output: result={:?}, error={:?}; Reference: result={:?}, error={:?}",
                        output.result, output.error, reference.result, reference.error,
                    );
                    ValidationResult::Mismatch {
                        reason: format!("Did not produce same as reference. {info}"),
                    }
                }
            },
        )
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        let mut gen_expr = || {
            AExpr::gn(
                &mut GclGenContext {
                    names: Vec::new(),
                    ..GclGenContext::new(25, rng)
                },
                rng,
            )
        };

        let mut expr = gen_expr();
        for _ in 0..10 {
            if expr
                .semantics(&gcl::semantics::EmptySemanticsContext)
                .is_ok()
            {
                break;
            }
            expr = gen_expr();
        }

        Input {
            expression: Stringify::new(expr),
        }
    }
}
