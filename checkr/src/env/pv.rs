use rand::{rngs::SmallRng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, BExpr, Command, Commands, LogicOp, RelOp},
    generation::Generate,
    sign::Sign,
};

use super::{
    interpreter::InterpreterInput, sign::SignAnalysisInput, Analysis, Environment, Markdown,
    SignEnv, ToMarkdown, ValidationResult,
};

#[derive(Debug)]
pub struct ProgramVerificationEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvInput {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramVerificationEnvOutput {
    pub verification_conditions: Vec<String>,
}

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

        table.add_rows(
            self.verification_conditions
                .iter()
                .map(|vc| [crate::parse::parse_predicate(vc).unwrap()]),
        );
        // table.add_row([camillaify(&format!(
        //     "``",
        //     crate::parse::parse_bexpr(&self.pre_condition)
        //         .unwrap()
        //         .simplify(),
        // ))]);
        // table.add_row([camillaify(&format!(
        //     "`WP = {} {{{}}}`",
        //     crate::parse::parse_bexpr(&self.pre_condition)
        //         .unwrap()
        //         .simplify()
        //         .to_string(),
        //     self.pre_condition
        // ))]);

        format!("{table}").into()
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
                    .reduce(|a, b| BExpr::logic(a, LogicOp::And, b))
            })
            .reduce(|a, b| BExpr::logic(a, LogicOp::Or, b))
            .unwrap_or(BExpr::Bool(true));

        Self {
            // post_condition: BExpr::Bool(true).to_string(),
            // post_condition: BExpr::gen(&mut crate::generation::Context::new(10, rng), rng)
            //     .to_string(),
            // post_condition: final_signs.to_string(),
        }
    }
}

impl Environment for ProgramVerificationEnv {
    type Input = ProgramVerificationEnvInput;

    type Output = ProgramVerificationEnvOutput;

    const ANALYSIS: Analysis = Analysis::ProgramVerification;

    fn setup_generation(&self) -> crate::ProgramGenerationBuilder {
        crate::ProgramGenerationBuilder::default().no_loop(true)
    }

    fn run(&self, cmds: &Commands, _: &Self::Input) -> Self::Output {
        let (p, q) = match &cmds.0[0] {
            Command::Annotated(p, _, q) => (p, q),
            _ => todo!(),
        };
        // let q = crate::parse::parse_bexpr(&input.post_condition).unwrap();
        ProgramVerificationEnvOutput {
            verification_conditions: vec![p.clone().to_string(), q.clone().to_string()],
        }
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> super::ValidationResult {
        // let reference = self.run(cmds, input);
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

        ValidationResult::CorrectTerminated
    }
}
