use itertools::{chain, Itertools};
use serde::{Deserialize, Serialize};

use crate::{
    ast::Commands,
    generation::Generate,
    interpreter::{Interpreter, InterpreterMemory, ProgramState, ProgramTrace},
    pg::{Determinism, Node, ProgramGraph},
    sign::{Memory, MemoryRef},
};

use super::{Analysis, Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct InterpreterEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterInput {
    pub determinism: Determinism,
    pub assignment: InterpreterMemory,
    pub trace_size: u64,
}

impl Generate for InterpreterInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, mut rng: &mut R) -> Self {
        let assignment = Memory::from_targets_with(
            cx.fv(),
            &mut rng,
            |rng, _| rng.gen_range(-10..=10),
            |rng, _| {
                let len = rng.gen_range(5..=10);
                (0..len).map(|_| rng.gen_range(-10..=10)).collect()
            },
        );
        InterpreterInput {
            determinism: Determinism::Deterministic,
            assignment,
            trace_size: rng.gen_range(10..=15),
        }
    }
}

impl ToMarkdown for InterpreterInput {
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
                .iter()
                .map(|e| match e {
                    MemoryRef::Variable(v, x) => format!("`{v} = {x}`"),
                    MemoryRef::Array(v, x) => format!("`{v} = {x:?}`"),
                })
                .format(", ")
                .to_string(),
        ]);

        format!("{table}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterOutput(Vec<ProgramTrace<String>>);

impl ToMarkdown for InterpreterOutput {
    fn to_markdown(&self) -> String {
        let variables = self
            .0
            .iter()
            .flat_map(|t| t.memory.variables.keys().map(|k| k.to_string()))
            .sorted()
            .dedup()
            .collect_vec();
        let arrays = self
            .0
            .iter()
            .flat_map(|t| t.memory.arrays.keys().map(|k| k.to_string()))
            .sorted()
            .dedup()
            .collect_vec();

        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::ASCII_MARKDOWN)
            .set_header(chain!(
                ["Node".to_string()],
                variables.iter().cloned(),
                arrays.iter().cloned()
            ));

        for t in &self.0 {
            match t.state {
                ProgramState::Running => {
                    table.add_row(chain!(
                        [t.node.to_string()],
                        chain!(
                            t.memory
                                .variables
                                .iter()
                                .map(|(var, value)| (value.to_string(), var.to_string()))
                                .sorted_by_key(|(_, k)| k.to_string()),
                            t.memory
                                .arrays
                                .iter()
                                .map(|(arr, values)| {
                                    (format!("[{}]", values.iter().format(",")), arr.to_string())
                                })
                                .sorted_by_key(|(_, k)| k.to_string()),
                        )
                        .map(|(v, _)| v),
                    ));
                }
                ProgramState::Stuck => {
                    table.add_row(["**Stuck**".to_string()]);
                }
                ProgramState::Terminated => {
                    table.add_row(["**Terminated successfully**".to_string()]);
                }
            }
        }
        format!("{table}")
    }
}

impl Environment for InterpreterEnv {
    type Input = InterpreterInput;

    type Output = InterpreterOutput;

    const ANALYSIS: Analysis = Analysis::Interpreter;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let pg = ProgramGraph::new(input.determinism, cmds);
        InterpreterOutput(
            Interpreter::evaluate(input.trace_size, input.assignment.clone(), &pg)
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

        if output.0.len() < input.trace_size as usize {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::CorrectNonTerminated {
                iterations: input.trace_size,
            }
        }
    }
}
