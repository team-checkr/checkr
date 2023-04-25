use itertools::Itertools;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    ast::{Commands, Target},
    generation::Generate,
    security::{Flow, SecurityAnalysisOutput, SecurityClass, SecurityLattice},
    sign::Memory,
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct SecurityEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLatticeInput(Vec<Flow<SecurityClass>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisInput {
    pub classification: Memory<SecurityClass>,
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
        let classification = Memory::from_targets_with(
            cx.fv(),
            rng,
            |rng, _| classes.choose(rng).unwrap().clone(),
            |rng, _| classes.choose(rng).unwrap().clone(),
        );
        let lattice = SecurityLatticeInput(vec![
            Flow {
                from: public,
                into: internal.clone(),
            },
            Flow {
                from: internal,
                into: private,
            },
            Flow {
                from: trusted,
                into: dubious,
            },
        ]);

        SecurityAnalysisInput {
            classification,
            lattice,
        }
    }
}

impl ToMarkdown for SecurityAnalysisInput {
    fn to_markdown(&self) -> Markdown {
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
                .map(|e| format!("`{e}`"))
                .sorted()
                .format(", ")
                .to_string(),
        ]);

        format!("{table}").into()
    }
}

impl ToMarkdown for SecurityAnalysisOutput {
    fn to_markdown(&self) -> Markdown {
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
                "**Secure**".to_string()
            } else {
                "**Insecure**".to_string()
            },
        ]);

        format!("{table}").into()
    }
}

impl Environment for SecurityEnv {
    type Input = SecurityAnalysisInput;

    type Output = SecurityAnalysisOutput;

    const ANALYSIS: Analysis = Analysis::Security;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Result<Self::Output, EnvError> {
        let lattice = SecurityLattice::new(&input.lattice.0);
        Ok(SecurityAnalysisOutput::run(
            &input.classification,
            &lattice,
            cmds,
        ))
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> Result<ValidationResult, EnvError>
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        fn stringify(flows: &[Flow<Target>]) -> Vec<Flow<&str>> {
            let mut f = flows.iter().map(|f| f.map(|t| t.name())).collect_vec();
            f.sort();
            f
        }

        let reference = self.run(cmds, input)?;
        let reference_actual = stringify(&reference.actual);
        let reference_allowed = stringify(&reference.allowed);
        let reference_violations = stringify(&reference.violations);
        let output = output.clone();
        let output_actual = stringify(&output.actual);
        let output_allowed = stringify(&output.allowed);
        let output_violations = stringify(&output.violations);

        if reference_actual == output_actual
            && reference_allowed == output_allowed
            && reference_violations == output_violations
        {
            Ok(ValidationResult::CorrectTerminated)
        } else {
            Ok(ValidationResult::Mismatch {
                reason: format!("{input:?}\n{cmds}\n{reference:#?} != {output:#?}"),
            })
        }
    }
}
