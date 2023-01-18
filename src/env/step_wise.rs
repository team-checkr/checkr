use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    ast::Commands,
    generation::Generate,
    interpreter::{Interpreter, InterpreterMemory, ProgramState, ProgramTrace},
    pg::{Determinism, Node, ProgramGraph},
    sign::Memory,
};

use super::{Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct StepWiseEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepWiseInput {
    pub determinism: Determinism,
    pub assignment: InterpreterMemory,
    pub trace_count: usize,
}

impl Generate for StepWiseInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        StepWiseInput {
            determinism: Determinism::Deterministic,
            assignment: Memory {
                variables: cx
                    .fv()
                    .into_iter()
                    .sorted()
                    .map(|v| (v, rng.gen_range(-10..=10)))
                    .collect(),
                arrays: Default::default(),
            },
            trace_count: rng.gen_range(10..=15),
        }
    }
}

impl ToMarkdown for StepWiseInput {
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
                        .map(|(v, x)| format!("`{v} = {x:?}`")),
                )
                .format(", ")
                .to_string(),
        ]);

        format!("{table}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepWiseOutput(Vec<ProgramTrace<String>>);

impl ToMarkdown for StepWiseOutput {
    fn to_markdown(&self) -> String {
        let variables = self
            .0
            .iter()
            .flat_map(|t| {
                t.memory
                    .variables
                    .keys()
                    .map(|k| k.to_string())
                    .chain(t.memory.arrays.keys().cloned())
            })
            .sorted()
            .dedup()
            .collect_vec();

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(std::iter::once("Node".to_string()).chain(variables.iter().cloned()));

        for t in &self.0 {
            match t.state {
                ProgramState::Running => {
                    table.add_row(
                        std::iter::once(t.node.to_string()).chain(
                            t.memory
                                .variables
                                .iter()
                                .map(|(var, value)| (value.to_string(), var.to_string()))
                                .chain(t.memory.arrays.iter().map(|(arr, values)| {
                                    (format!("[{}]", values.iter().format(",")), arr.to_string())
                                }))
                                .sorted_by_key(|(_, k)| k.to_string())
                                .map(|(v, _)| v),
                        ),
                    );
                }
                ProgramState::Stuck => {
                    table.add_row([format!("**Stuck**")]);
                }
                ProgramState::Terminated => {
                    table.add_row([format!("**Terminated successfully**")]);
                }
            }
        }
        format!("{table}")
    }
}

impl Environment for StepWiseEnv {
    type Input = StepWiseInput;

    type Output = StepWiseOutput;

    fn command() -> &'static str {
        "interpreter"
    }
    fn name(&self) -> String {
        "Step-wise Execution".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        StepWiseOutput(
            Interpreter::evaluate(input.trace_count, input.assignment.clone(), &pg)
                .into_iter()
                .map(|t| t.map_node(|n| n.to_string()))
                .collect(),
        )
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq,
    {
        let pg = ProgramGraph::new(input.determinism, cmds);
        let mut mem = vec![(Node::Start, input.assignment.clone())];

        for (idx, trace) in output.0.iter().skip(1).enumerate() {
            let mut next_mem = vec![];

            for (current_node, current_mem) in mem {
                for edge in pg.outgoing(current_node) {
                    if let Ok(m) = edge.action().semantics(&current_mem) {
                        // TODO: check state
                        if m == trace.memory {
                            next_mem.push((edge.to(), m));
                        }
                    }
                }
            }
            if next_mem.is_empty() {
                match trace.state {
                    ProgramState::Running => {
                        return ValidationResult::Mismatch {
                            reason: format!("The traces do not match after {idx} iterations"),
                        };
                    }
                    ProgramState::Stuck => break,
                    ProgramState::Terminated => break,
                }
            }
            mem = next_mem;
        }

        if output.0.len() < input.trace_count {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::CorrectNonTerminated {
                iterations: input.trace_count,
            }
        }
    }
}
