use std::collections::HashMap;

use itertools::Itertools;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    ast::{Commands, Variable},
    generation::Generate,
    security::{Flow, SecurityAnalysisOutput, SecurityClass, SecurityLattice},
};

use super::{Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct SecurityEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLatticeInput(Vec<Flow<SecurityClass>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisInput {
    pub classification: HashMap<Variable, SecurityClass>,
    pub lattice: SecurityLatticeInput,
}

impl Generate for SecurityAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let private = SecurityClass("Private".to_string());
        let internal = SecurityClass("Internal".to_string());
        let public = SecurityClass("Public".to_string());
        let dubious = SecurityClass("Dubious".to_string());
        let trusted = SecurityClass("Trusted".to_string());
        let classes = [&private, &internal, &public, &dubious, &trusted].map(Clone::clone);
        let variable_classification = cx
            .fv()
            .into_iter()
            .map(|v| (v, classes.choose(rng).unwrap().clone()))
            .collect_vec();
        let array_classification = cx
            .fa()
            .into_iter()
            .map(|arr| {
                (
                    Variable(arr.to_string()),
                    classes.choose(rng).unwrap().clone(),
                )
            })
            .collect_vec();
        let classification = variable_classification
            .into_iter()
            .chain(array_classification)
            .collect();
        let lattice = SecurityLatticeInput(vec![
            Flow {
                from: public.clone(),
                into: internal.clone(),
            },
            Flow {
                from: internal.clone(),
                into: private.clone(),
            },
            Flow {
                from: trusted.clone(),
                into: dubious.clone(),
            },
        ]);

        SecurityAnalysisInput {
            classification,
            lattice,
        }
    }
}

impl ToMarkdown for SecurityAnalysisInput {
    fn to_markdown(&self) -> String {
        let mut table = comfy_table::Table::new();
        table.load_preset(comfy_table::presets::ASCII_MARKDOWN);

        table.set_header(["Input"]);
        table.add_row([
            "Lattice:".to_string(),
            self.lattice
                .0
                .iter()
                .map(|f| format!("`{} < {}`", f.from, f.into))
                .format(", ")
                .to_string(),
        ]);

        table.add_row([
            "Classification:".to_string(),
            self.classification
                .iter()
                .map(|(a, c)| format!("`{a} = {c}`"))
                .sorted()
                .format(", ")
                .to_string(),
        ]);

        format!("{table}")
    }
}

impl ToMarkdown for SecurityAnalysisOutput {
    fn to_markdown(&self) -> String {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["", "Flows"]);

        // ->͢→↦⇒⇛⇨➙➞➝➜➱➽➼⟴⟶➾
        table.add_row([
            "Actual".to_string(),
            self.actual
                .iter()
                .map(|f| format!("`{} → {}`", f.from, f.into))
                .format(", ")
                .to_string(),
        ]);
        table.add_row([
            "Allowed".to_string(),
            self.allowed
                .iter()
                .map(|f| format!("`{} → {}`", f.from, f.into))
                .format(", ")
                .to_string(),
        ]);
        table.add_row([
            "Violations".to_string(),
            self.violations
                .iter()
                .map(|f| format!("`{} → {}`", f.from, f.into))
                .format(", ")
                .to_string(),
        ]);

        table.add_row([
            "Result".to_string(),
            if self.violations.is_empty() {
                format!("**Secure**")
            } else {
                format!("**Insecure**")
            },
        ]);

        format!("{table}")
    }
}

impl Environment for SecurityEnv {
    type Input = SecurityAnalysisInput;

    type Output = SecurityAnalysisOutput;

    fn command() -> &'static str {
        "security"
    }
    fn name(&self) -> String {
        "Security Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let lattice = SecurityLattice::new(&input.lattice.0);
        SecurityAnalysisOutput::run(&input.classification, &lattice, cmds)
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        let mut reference = self.run(cmds, input);
        reference.actual.sort();
        reference.allowed.sort();
        reference.violations.sort();
        let mut output = output.clone();
        output.actual.sort();
        output.allowed.sort();
        output.violations.sort();

        if reference == output {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::Mismatch {
                reason: format!("{input:?}\n{cmds}\n{reference:#?} != {output:#?}"),
            }
        }
    }
}
