use serde::{Deserialize, Serialize};

use crate::{
    ast::{BExpr, Commands},
    generation::Generate,
};

use super::{Analysis, Environment, Markdown, ToMarkdown, ValidationResult};

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

        table.add_rows(self.verification_conditions.iter().map(|vc| {
            [format!(
                r#"<code class="predicate">`{}`</code>"#,
                crate::parse::parse_predicate(vc).unwrap()
            )]
        }));

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
        crate::ProgramGenerationBuilder::default()
            .no_loop(true)
            .generate_annotated(true)
    }

    fn run(&self, cmds: &Commands, _: &Self::Input) -> Self::Output {
        let verification_conditions = cmds.vc(&BExpr::Bool(true));
        ProgramVerificationEnvOutput {
            verification_conditions: verification_conditions
                .iter()
                .map(|vc| vc.to_string())
                .collect(),
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
