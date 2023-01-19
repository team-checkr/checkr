use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{mono_analysis, FiFo},
    ast::{Commands, Variable},
    generation::Generate,
    pg::{Determinism, Node, ProgramGraph},
    sign::{Memory, Sign, SignAnalysis, SignMemory, Signs},
};

use super::{Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct SignEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignAnalysisInput {
    pub determinism: Determinism,
    pub assignment: SignMemory,
}

impl Generate for SignAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        SignAnalysisInput {
            determinism: [Determinism::Deterministic, Determinism::NonDeterministic]
                .choose(rng)
                .copied()
                .unwrap(),
            assignment: Memory::gen(cx, rng),
        }
    }
}

impl ToMarkdown for SignAnalysisInput {
    fn to_markdown(&self) -> String {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(["Input"]);

        table.add_row([
            "Determinism:",
            match self.determinism {
                Determinism::Deterministic => "**✓**",
                Determinism::NonDeterministic => "**✕**",
            },
        ]);

        table.add_row([
            "Memory:".to_string(),
            self.assignment
                .variables
                .iter()
                .map(|(v, x)| format!("`{v} = {x}`"))
                .chain(
                    self.assignment
                        .arrays
                        .iter()
                        .map(|(v, x)| format!("`{v} = {x}`")),
                )
                .format(", ")
                .to_string(),
        ]);

        format!("{table}")
    }
}

impl Generate for Sign {
    type Context = Commands;

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        *[Sign::Positive, Sign::Zero, Sign::Negative]
            .choose(rng)
            .unwrap()
    }
}
impl Generate for Signs {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        [Sign::gen(cx, rng)].into_iter().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignAnalysisOutput {
    pub initial_node: String,
    pub final_node: String,
    pub nodes: HashMap<String, HashSet<SignMemory>>,
}

impl ToMarkdown for SignAnalysisOutput {
    fn to_markdown(&self) -> String {
        let variables: HashSet<_> = self
            .nodes
            .iter()
            .flat_map(|(_, worlds)| worlds.iter().flat_map(|w| w.variables.keys().cloned()))
            .collect();
        let arrays: HashSet<_> = self
            .nodes
            .iter()
            .flat_map(|(_, worlds)| worlds.iter().flat_map(|w| w.arrays.keys().cloned()))
            .collect();
        let variables = variables.into_iter().sorted().collect_vec();
        let arrays = arrays.into_iter().sorted().collect_vec();

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(
                std::iter::once("Node".to_string())
                    .chain(variables.iter().map(|v| v.to_string()))
                    .chain(arrays.iter().cloned()),
            );

        for (n, worlds) in self.nodes.iter().sorted_by_key(|(n, _)| {
            if *n == "qStart" {
                "".to_string()
            } else {
                n.to_string()
            }
        }) {
            let mut first = true;
            for w in worlds {
                let is_first = first;
                first = false;

                table.add_row(
                    std::iter::once(if is_first {
                        n.to_string()
                    } else {
                        "".to_string()
                    })
                    .chain(variables.iter().map(|var| {
                        w.variables
                            .get(var)
                            .cloned()
                            .unwrap_or_default()
                            .to_string()
                    }))
                    .chain(
                        arrays
                            .iter()
                            .map(|arr| w.arrays.get(arr).cloned().unwrap_or_default().to_string()),
                    ),
                );
            }
            if worlds.is_empty() {
                table.add_row([n.to_string()]);
            }
        }
        format!("{table}")
    }
}

impl Environment for SignEnv {
    type Input = SignAnalysisInput;

    type Output = SignAnalysisOutput;

    fn command() -> &'static str {
        "sign"
    }
    fn name(&self) -> String {
        "Detection of Signs Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        SignAnalysisOutput {
            initial_node: Node::Start.to_string(),
            final_node: Node::End.to_string(),
            nodes: mono_analysis::<_, FiFo>(
                SignAnalysis {
                    assignment: input.assignment.clone(),
                },
                &pg,
            )
            .facts
            .into_iter()
            .map(|(k, v)| (format!("{k}"), v))
            .collect(),
        }
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
        let reference = self.run(cmds, input);

        let mut pool = reference.nodes.values().collect_vec();

        for o in output.nodes.values() {
            if let Some(idx) = pool.iter().position(|r| *r == o) {
                pool.remove(idx);
            } else {
                return ValidationResult::Mismatch {
                    reason: "Produced world which did not exist in reference".to_string(),
                };
            }
        }

        if pool.is_empty() {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            }
        }
    }
}
