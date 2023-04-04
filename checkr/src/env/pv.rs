use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    ast::{BExpr, Commands, Predicate},
    egg::EquivChecker,
    generation::Generate,
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct ProgramVerificationEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvInput {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvOutput {
    pub verification_conditions: Vec<SerializedPredicate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerializedPredicate {
    predicate: String,
}

impl From<Predicate> for SerializedPredicate {
    fn from(value: Predicate) -> Self {
        SerializedPredicate {
            predicate: value.to_string(),
        }
    }
}
impl From<&'_ Predicate> for SerializedPredicate {
    fn from(value: &'_ Predicate) -> Self {
        SerializedPredicate {
            predicate: value.to_string(),
        }
    }
}
impl SerializedPredicate {
    pub fn parse(&self) -> Result<Predicate, crate::parse::ParseError> {
        crate::parse::parse_predicate(&self.predicate)
    }
}

#[allow(dead_code)]
fn camillaify(s: &str) -> String {
    s.replace(" | ", " ∨ ")
        .replace("<=", "≤")
        .replace(">=", "≤")
        .replace(" & ", " ∧ ")
        .replace("!=", "≠")
        .replace("!!", "")
        .replace("!!!", "¬")
        .replace('!', "¬")
}

impl ToMarkdown for ProgramVerificationEnvInput {
    fn to_markdown(&self) -> Markdown {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Input"]);

        format!("{table}").into()
    }
}
impl ToMarkdown for ProgramVerificationEnvOutput {
    fn to_markdown(&self) -> Markdown {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Verification conditions"]);

        // r#"<code class="predicate">`{}`</code>"#,
        table.add_rows(
            self.verification_conditions
                .iter()
                .map(|vc| [format!("`{}`", vc.parse().unwrap()).replace('|', "\\|")]),
        );

        format!("{table}").into()
    }
}

impl Generate for ProgramVerificationEnvInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {}
    }
}

impl Environment for ProgramVerificationEnv {
    type Input = ProgramVerificationEnvInput;

    type Output = ProgramVerificationEnvOutput;

    const ANALYSIS: Analysis = Analysis::ProgramVerification;

    fn setup_generation(&self) -> crate::ProgramGenerationBuilder {
        crate::ast::Command::reset_sp_counter();

        crate::ProgramGenerationBuilder::new(Self::ANALYSIS)
            .no_loop(true)
            .no_division(true)
            .generate_annotated(true)
    }

    fn run(&self, cmds: &Commands, _: &Self::Input) -> Result<Self::Output, EnvError> {
        let verification_conditions = cmds.vc(&BExpr::Bool(true));
        Ok(ProgramVerificationEnvOutput {
            verification_conditions: verification_conditions
                .iter()
                .map(|vc| vc.renumber_quantifiers().into())
                .collect(),
        })
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> Result<super::ValidationResult, EnvError> {
        let reference = self.run(cmds, input)?;
        let ref_vc: Result<Vec<_>, _> = reference
            .verification_conditions
            .iter()
            .map(|vc| vc.parse().map(|pred| pred.renumber_quantifiers()))
            .collect();
        let rel_vc: Result<Vec<_>, _> = output
            .verification_conditions
            .iter()
            .map(|vc| vc.parse().map(|pred| pred.renumber_quantifiers()))
            .collect();

        let ref_vc = match ref_vc {
            Ok(ref_vc) => ref_vc,
            Err(err) => {
                return Ok(ValidationResult::Mismatch {
                    reason: format!("failed to parse verification conditions: {err}"),
                })
            }
        };
        let rel_vc = match rel_vc {
            Ok(rel_vc) => rel_vc,
            Err(err) => {
                return Ok(ValidationResult::Mismatch {
                    reason: format!("failed to parse verification conditions: {err}"),
                })
            }
        };

        if ref_vc.len() != rel_vc.len() {
            return Ok(ValidationResult::Mismatch {
                reason: format!(
                    "produced '{}' verification conditions, expected '{}'",
                    rel_vc.len(),
                    ref_vc.len()
                ),
            });
        }

        let mut checker = EquivChecker::default();

        let mut ref_exprs = ref_vc.iter().map(|vc| checker.register(vc)).collect_vec();
        let mut rel_exprs = rel_vc.iter().map(|vc| checker.register(vc)).collect_vec();

        checker.run();

        ref_exprs.retain(|ref_e| {
            if let Some(rel_idx) = rel_exprs
                .iter()
                .position(|rel_e| checker.are_equivalent(ref_e, rel_e))
            {
                rel_exprs.remove(rel_idx);
                false
            } else {
                true
            }
        });

        if ref_exprs.is_empty() {
            Ok(ValidationResult::CorrectTerminated)
        } else {
            Ok(ValidationResult::Mismatch {
                reason: format!(
                    "{}. Left in the reference were [{}] and left in the given were [{}]",
                    "some verification conditions were not found",
                    ref_exprs.iter().format(", "),
                    rel_exprs.iter().format(", "),
                ),
            })
        }

        // let a = crate::parse::parse_bexpr(&reference.pre_condition).unwrap();
        // let b = crate::parse::parse_bexpr(&output.pre_condition)
        //     .expect("could not parse pre-condition");

        // let mut rng = SmallRng::seed_from_u64(0xBADA55);

        // for _ in 0..100 {
        //     let sample = InterpreterInput::gen(&mut cmds.clone(), &mut rng);

        //     if a.semantics(&sample.assignment) != b.semantics(&sample.assignment) {
        //         return ValidationResult::Mismatch {
        //             reason: format!(
        //                 "Did not produce the same logical value for {:?}",
        //                 sample.assignment
        //             ),
        //         };
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn normalization_simple() -> miette::Result<()> {
        let a = "exists _f0 :: exists _f1 :: _f0 = _f1";
        let b = "exists _f1 :: exists _f0 :: _f1 = _f0";
        let a = crate::parse::parse_predicate(a)?.renumber_quantifiers();
        let b = crate::parse::parse_predicate(b)?.renumber_quantifiers();
        assert_eq!(a, b);
        Ok(())
    }
    #[test]
    fn normalization_large() -> miette::Result<()> {
        let a = "((exists _f2 :: (((d <= d) & (exists _f1 :: ((exists _f0 :: (((((a > 0) && (_f1 = 0)) && (_f2 = 0)) && (_f0 < 0)) & (d = _f1))) & (b = d)))) & (c = -77))) ==> ((((a = 0) && (b = 0)) && (c > 0)) && (d = 0)))";
        let b = "((exists _f0 :: (((d <= d) & (exists _f1 :: ((exists _f2 :: (((((a > 0) && (_f1 = 0)) && (_f0 = 0)) && (_f2 < 0)) & (d = _f1))) & (b = d)))) & (c = -77))) ==> ((((a = 0) && (b = 0)) && (c > 0)) && (d = 0)))";
        let a = crate::parse::parse_predicate(a)?.renumber_quantifiers();
        let b = crate::parse::parse_predicate(b)?.renumber_quantifiers();
        assert_eq!(a, b);

        Ok(())
    }
}
