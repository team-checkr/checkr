use std::collections::HashSet;

use indexmap::IndexMap;
use itertools::{chain, Itertools};

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
    analysis::{mono_analysis, FiFo, NodeOrder},
    ast::{Commands, Target},
    generation::Generate,
    pg::{Determinism, Node, ProgramGraph},
    sign::{Memory, Sign, SignAnalysis, SignMemory, Signs},
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown, ValidationResult};

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
    fn to_markdown(&self) -> Markdown {
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
                .iter()
                .map(|e| format!("`{e}`"))
                .format(", ")
                .to_string(),
        ]);

        format!("{table}").into()
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
    pub nodes: IndexMap<String, HashSet<SignMemory>>,
}

impl ToMarkdown for SignAnalysisOutput {
    fn to_markdown(&self) -> Markdown {
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
            .set_header(chain!(
                ["Node".to_string()],
                variables.iter().map(|v| v.to_string()),
                arrays.iter().map(|v| v.to_string())
            ));

        for (n, worlds) in self
            .nodes
            .iter()
            .sorted_by_key(|(n, _)| NodeOrder::parse(n))
        {
            let mut first = true;
            for w in worlds {
                let is_first = first;
                first = false;

                table.add_row(chain!(
                    [if is_first {
                        n.to_string()
                    } else {
                        "".to_string()
                    }],
                    variables.iter().map(|var| {
                        w.variables
                            .get(var)
                            .cloned()
                            .unwrap_or_default()
                            .to_string()
                    }),
                    arrays.iter().map(|arr| w
                        .arrays
                        .get(arr)
                        .cloned()
                        .unwrap_or_default()
                        .to_string()),
                ));
            }
            if worlds.is_empty() {
                table.add_row([n.to_string()]);
            }
        }
        format!("{table}").into()
    }
}

impl Environment for SignEnv {
    type Input = SignAnalysisInput;

    type Output = SignAnalysisOutput;

    const ANALYSIS: Analysis = Analysis::Sign;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Result<Self::Output, EnvError> {
        let pg = ProgramGraph::new(input.determinism, cmds);

        for t in pg.fv() {
            match t {
                Target::Variable(var) => {
                    if input.assignment.get_var(&var).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            input: super::Input::from_concrete::<Self>(input),
                            message: format!("variable `{var}` was not in the given input"),
                        });
                    }
                }
                Target::Array(arr, _) => {
                    if input.assignment.get_arr(&arr).is_none() {
                        return Err(EnvError::InvalidInputForProgram {
                            input: super::Input::from_concrete::<Self>(input),
                            message: format!("array `{arr}` was not in the given input"),
                        });
                    }
                }
            }
        }

        Ok(SignAnalysisOutput {
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
        })
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
        let reference = self.run(cmds, input)?;

        let mut pool = reference.nodes.values().collect_vec();

        for (n, o) in &output.nodes {
            if let Some(idx) = pool.iter().position(|r| *r == o) {
                pool.remove(idx);
            } else {
                error!(not_in_reference = format!("{o:?}"), "damn...");
                return Ok(ValidationResult::Mismatch {
                    reason: format!(
                        "Produced world which did not exist in reference: {n:?} ~> {o:?}"
                    ),
                });
            }
        }

        if pool.is_empty() {
            Ok(ValidationResult::CorrectTerminated)
        } else {
            error!(missing = format!("{pool:?}"), "oh no...");
            Ok(ValidationResult::Mismatch {
                reason: "Reference had world which was not present".to_string(),
            })
        }
    }
}
