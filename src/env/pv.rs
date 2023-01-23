use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, BExpr, Commands, LogicOp, RelOp},
    generation::Generate,
    sign::Sign,
};

use super::{
    sign::SignAnalysisInput, step_wise::StepWiseInput, Environment, SignEnv, ToMarkdown,
    ValidationResult,
};

#[derive(Debug)]
pub struct ProgramVerificationEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvInput {
    pub post_condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvOutput {
    pub pre_condition: String,
}

fn camillaify(s: &str) -> String {
    s.replace(" | ", " ∨ ")
        .replace("<=", "≤")
        .replace(">=", "≤")
        .replace(" & ", " ∧ ")
        .replace("!=", "≠")
        .replace("!!", "")
        .replace("!!!", "¬")
        .replace("!", "¬")
}

impl ToMarkdown for ProgramVerificationEnvInput {
    fn to_markdown(&self) -> String {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Input"]);

        table.add_row([
            "Postcondition".to_string(),
            camillaify(&format!("`Q = {}`", self.post_condition)),
        ]);

        format!("{table}")
    }
}
impl ToMarkdown for ProgramVerificationEnvOutput {
    fn to_markdown(&self) -> String {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Weakest Precondition", ""]);

        table.add_row([camillaify(&format!(
            "`WP = {}`",
            crate::parse::parse_bexpr(&self.pre_condition)
                .unwrap()
                .simplify()
                .to_string(),
        ))]);
        // table.add_row([camillaify(&format!(
        //     "`WP = {} {{{}}}`",
        //     crate::parse::parse_bexpr(&self.pre_condition)
        //         .unwrap()
        //         .simplify()
        //         .to_string(),
        //     self.pre_condition
        // ))]);

        format!("{table}")
    }
}

impl Generate for ProgramVerificationEnvInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let input = SignAnalysisInput::gen(cx, rng);
        let sign_result = SignEnv.run(cx, &input);

        let final_assignment = &sign_result.nodes[&sign_result.final_node];

        let final_signs = final_assignment
            .iter()
            .filter_map(|world| {
                world
                    .variables
                    .iter()
                    .map(|(v, s)| match s {
                        Sign::Positive => BExpr::Rel(
                            AExpr::Reference(v.clone().into()),
                            RelOp::Gt,
                            AExpr::Number(0),
                        ),
                        Sign::Zero => BExpr::Rel(
                            AExpr::Reference(v.clone().into()),
                            RelOp::Eq,
                            AExpr::Number(0),
                        ),
                        Sign::Negative => BExpr::Rel(
                            AExpr::Reference(v.clone().into()),
                            RelOp::Lt,
                            AExpr::Number(0),
                        ),
                    })
                    .reduce(|a, b| BExpr::Logic(box a, LogicOp::And, box b))
            })
            .reduce(|a, b| BExpr::Logic(box a, LogicOp::Or, box b))
            .unwrap_or(BExpr::Bool(true));

        Self {
            // post_condition: BExpr::Bool(true).to_string(),
            // post_condition: BExpr::gen(&mut crate::generation::Context::new(10, rng), rng)
            //     .to_string(),
            post_condition: final_signs.to_string(),
        }
    }
}

impl Environment for ProgramVerificationEnv {
    type Input = ProgramVerificationEnvInput;

    type Output = ProgramVerificationEnvOutput;

    fn command() -> &'static str {
        "pv"
    }

    fn name(&self) -> String {
        "Program Verification".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let q = crate::parse::parse_bexpr(&input.post_condition).unwrap();
        ProgramVerificationEnvOutput {
            pre_condition: cmds.wp(&q).to_string(),
        }
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> super::ValidationResult {
        let reference = self.run(cmds, input);
        let a = crate::parse::parse_bexpr(&reference.pre_condition).unwrap();
        let b = crate::parse::parse_bexpr(&output.pre_condition)
            .expect("could not parse pre-condition");

        let mut rng = SmallRng::seed_from_u64(0xBADA55);

        for _ in 0..100 {
            let sample = StepWiseInput::gen(&mut cmds.clone(), &mut rng);

            if a.semantics(&sample.assignment) != b.semantics(&sample.assignment) {
                return ValidationResult::Mismatch {
                    reason: format!(
                        "Did not produce the same logical value for {:?}",
                        sample.assignment
                    ),
                };
            }
        }

        ValidationResult::CorrectTerminated
    }
}
