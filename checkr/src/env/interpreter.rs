use itertools::{chain, Itertools};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    ast::Commands,
    generation::Generate,
    interpreter::{Configuration, Interpreter, InterpreterMemory, TerminationState},
    pg::{Determinism, Node, ProgramGraph},
    sign::{Memory, MemoryRef},
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct InterpreterEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterInput {
    pub determinism: Determinism,
    pub assignment: InterpreterMemory,
    pub trace_length: u64,
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
            determinism: *[Determinism::Deterministic, Determinism::NonDeterministic]
                .choose(rng)
                .unwrap(),
            assignment,
            trace_length: rng.gen_range(10..=15),
        }
    }
}

impl ToMarkdown for InterpreterInput {
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
                .map(|e| match e {
                    MemoryRef::Variable(v, x) => format!("`{v} = {x}`"),
                    MemoryRef::Array(v, x) => format!("`{v} = {x:?}`"),
                })
                .format(", ")
                .to_string(),
        ]);

        table.add_row(["Trace length:".to_string(), self.trace_length.to_string()]);

        format!("{table}").into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterOutput {
    execution_sequence: Vec<Configuration<String>>,
    #[serde(rename = "final")]
    final_state: TerminationState,
}

impl ToMarkdown for InterpreterOutput {
    fn to_markdown(&self) -> Markdown {
        let variables = self
            .execution_sequence
            .iter()
            .flat_map(|t| t.memory.variables.keys().map(|k| k.to_string()))
            .sorted()
            .dedup()
            .collect_vec();
        let arrays = self
            .execution_sequence
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

        for t in &self.execution_sequence {
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
        let final_message = match self.final_state {
            TerminationState::Running => {
                format!("**Stopped after {} steps**", self.execution_sequence.len())
            }
            TerminationState::Stuck => "**Stuck**".to_string(),
            TerminationState::Terminated => "**Terminated successfully**".to_string(),
        };
        table.add_row([final_message]);

        format!("{table}").into()
    }
}

impl Environment for InterpreterEnv {
    type Input = InterpreterInput;

    type Output = InterpreterOutput;

    const ANALYSIS: Analysis = Analysis::Interpreter;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Result<Self::Output, EnvError> {
        let pg = ProgramGraph::new(input.determinism, cmds);
        let (execution_sequence, final_state) =
            Interpreter::evaluate(input.trace_length, input.assignment.clone(), &pg);
        let execution_sequence = execution_sequence
            .into_iter()
            .map(|t| t.map_node(|n| n.to_string()))
            .collect();

        Ok(InterpreterOutput {
            execution_sequence,
            final_state,
        })
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> Result<ValidationResult, EnvError>
    where
        Self::Output: PartialEq,
    {
        if let TerminationState::Running = output.final_state {
            if output.execution_sequence.len() < input.trace_length as usize {
                return Ok(ValidationResult::Mismatch {
                    reason: format!(
                        "Not enough traces were produced. Expected '{}' found '{}'",
                        input.trace_length,
                        output.execution_sequence.len()
                    ),
                });
            }
        }

        let pg = ProgramGraph::new(input.determinism, cmds);
        let mut mem = vec![(Node::Start, input.assignment.clone())];

        if let Some(first_cfg) = output.execution_sequence.first() {
            if first_cfg.memory != input.assignment {
                return Ok(ValidationResult::Mismatch {
                    reason: "The initial configuration did not match the starting memory"
                        .to_string(),
                });
            }
        } else if input.trace_length > 0 {
            return Ok(ValidationResult::Mismatch {
                reason: "Did not produce any execution sequences".to_string(),
            });
        }

        for (idx, trace) in output.execution_sequence.iter().skip(1).enumerate() {
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
                let is_last = idx + 1 == output.execution_sequence.len();

                if is_last {
                    // NOTE: They reached the last state at the same time we did
                    break;
                } else {
                    // NOTE: We could not continue, while they had more execution steps left
                    return Ok(ValidationResult::Mismatch {
                        reason: format!("The traces do not match after {idx} iterations"),
                    });
                }
            }
            mem = next_mem;
        }

        if output.execution_sequence.len() < input.trace_length as usize {
            Ok(ValidationResult::CorrectTerminated)
        } else {
            Ok(ValidationResult::CorrectNonTerminated {
                iterations: output.execution_sequence.len() as _,
            })
        }
    }
}
