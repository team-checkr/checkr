#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use ce_core::{
    define_env,
    rand::{self, seq::SliceRandom},
    Env, Generate, ValidationResult,
};
use gcl::{
    ast::{Array, Commands, Int, TargetDef, Variable},
    pg::{Determinism, Edge, Node, ProgramGraph},
    semantics::{SemanticsContext, SemanticsError},
    stringify::Stringify,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

define_env!(InterpreterEnv);

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct InterpreterMemory {
    pub variables: BTreeMap<Variable, i64>,
    pub arrays: BTreeMap<Array, Vec<i64>>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct Input {
    pub commands: Stringify<Commands>,
    pub determinism: Determinism,
    pub assignment: InterpreterMemory,
    pub trace_length: Int,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct Step {
    pub action: Stringify<gcl::pg::Action>,
    pub node: String,
    pub memory: InterpreterMemory,
}

#[derive(tapi::Tapi, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub enum TerminationState {
    Running,
    Stuck,
    Terminated,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct Output {
    pub initial_node: String,
    pub final_node: String,
    pub dot: String,
    pub trace: Vec<Step>,
    pub termination: TerminationState,
}

fn lookup_array<'a>(
    mem: &'a InterpreterMemory,
    array: &Array,
) -> Result<&'a [Int], SemanticsError> {
    mem.arrays
        .get(array)
        .ok_or_else(|| SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
        .map(|data| &**data)
}

impl SemanticsContext for InterpreterMemory {
    fn variable(&self, var: &Variable) -> Result<Int, SemanticsError> {
        self.variables
            .get(var)
            .ok_or_else(|| SemanticsError::VariableNotFound {
                name: var.to_string(),
            })
            .copied()
    }

    fn set_variable(&self, var: &Variable, value: Int) -> Result<Self, SemanticsError> {
        if self.variables.contains_key(var) {
            let mut m2 = self.clone();
            m2.variables.insert(var.clone(), value);
            Ok(m2)
        } else {
            Err(SemanticsError::VariableNotFound {
                name: var.to_string(),
            })
        }
    }

    fn array_element(&self, array: &Array, index: Int) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        data.get(index as usize)
            .ok_or_else(|| SemanticsError::IndexOutOfBound {
                name: array.to_string(),
                index,
            })
            .copied()
    }

    fn set_array_element(
        &self,
        array: &Array,
        index: Int,
        value: Int,
    ) -> Result<Self, SemanticsError> {
        match self.arrays.get(array) {
            Some(data) if 0 <= index && index < data.len() as _ => {
                let mut m2 = self.clone();
                let data = m2.arrays.get_mut(array).unwrap();
                data[index as usize] = value;
                Ok(m2)
            }
            Some(_) => Err(SemanticsError::ArrayNotFound {
                name: array.to_string(),
            }),
            None => Err(SemanticsError::IndexOutOfBound {
                name: array.to_string(),
                index,
            }),
        }
    }

    fn array_length(&self, array: &Array) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        Ok(data.len() as _)
    }

    fn array_count(&self, array: &Array, element: Int) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        Ok(data.iter().filter(|e| **e == element).count() as _)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Execution {
    initial_memory: InterpreterMemory,
    trace: Vec<(Step, Node)>,
}

impl Execution {
    fn new(input: &Input) -> Self {
        Self {
            initial_memory: input.assignment.clone(),
            trace: vec![],
        }
    }
    fn current_node(&self) -> Node {
        self.trace.last().map(|(_, n)| *n).unwrap_or(Node::Start)
    }
    fn current_mem(&self) -> &InterpreterMemory {
        self.trace
            .last()
            .map(|(s, _)| &s.memory)
            .unwrap_or(&self.initial_memory)
    }
    fn is_finished(&self) -> bool {
        self.current_node() == Node::End
    }
    fn is_stuck(&self, pg: &ProgramGraph) -> bool {
        pg.outgoing(self.current_node())
            .iter()
            .all(|Edge(_, action, _)| action.semantics(self.current_mem()).is_err())
    }
    fn nexts(&self, pg: &ProgramGraph) -> Vec<Execution> {
        if self.is_stuck(pg) {
            return vec![];
        }

        let mem = self.current_mem();
        pg.outgoing(self.current_node())
            .iter()
            .filter_map(|Edge(_, action, next_node)| {
                action.semantics(mem).ok().map(|next_mem| {
                    let mut next = self.clone();
                    next.trace.push((
                        Step {
                            node: next_node.to_string(),
                            action: Stringify::new(action.clone()),
                            memory: next_mem,
                        },
                        *next_node,
                    ));
                    next
                })
            })
            .collect_vec()
    }
}

impl Env for InterpreterEnv {
    type Input = Input;

    type Output = Output;

    type Meta = BTreeSet<TargetDef>;

    fn meta(input: &Self::Input) -> Self::Meta {
        if let Ok(commands) = input.commands.try_parse() {
            commands.fv().into_iter().map(|t| t.def()).collect()
        } else {
            Default::default()
        }
    }

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let pg = gcl::pg::ProgramGraph::new(
            input.determinism,
            &input.commands.try_parse().map_err(|err| {
                ce_core::EnvError::InvalidInputForProgram {
                    message: "failed to parse commands".to_string(),
                    source: Some(Box::new(err)),
                }
            })?,
        );

        let mut termination = TerminationState::Running;
        let mut exe = Execution::new(input);

        for _ in 0..input.trace_length {
            if let Some(next) = exe.nexts(&pg).first().cloned() {
                if next.is_stuck(&pg) {
                    termination = if next.is_finished() {
                        TerminationState::Terminated
                    } else {
                        TerminationState::Stuck
                    };
                    exe = next;
                    break;
                }
                exe = next;
                continue;
            }

            termination = if exe.is_finished() {
                TerminationState::Terminated
            } else {
                TerminationState::Stuck
            };
            break;
        }

        Ok(Output {
            initial_node: Node::Start.to_string(),
            final_node: Node::End.to_string(),
            dot: pg.dot(),
            trace: exe.trace.into_iter().map(|(s, _)| s).collect(),
            termination,
        })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let pg = gcl::pg::ProgramGraph::new(
            input.determinism,
            &input.commands.try_parse().map_err(|err| {
                ce_core::EnvError::InvalidInputForProgram {
                    message: "failed to parse commands".to_string(),
                    source: Some(Box::new(err)),
                }
            })?,
        );
        let mut possible_executions = vec![Execution::new(input)];

        for step in &output.trace {
            possible_executions = possible_executions
                .iter()
                .flat_map(|exe| exe.nexts(&pg))
                .filter(|exe| exe.current_mem() == &step.memory)
                .collect();

            if possible_executions.is_empty() {
                return Ok(ValidationResult::Mismatch {
                    reason: "No possible execution found".to_string(),
                });
            }
        }

        if output.termination == TerminationState::Running && !possible_executions.is_empty() {
            return Ok(ValidationResult::CorrectNonTerminated {
                iterations: output.trace.len() as u64,
            });
        }

        if output.termination == TerminationState::Terminated {
            if possible_executions.iter().any(|s| s.is_finished()) {
                return Ok(ValidationResult::CorrectTerminated);
            }
            return Ok(ValidationResult::Mismatch {
                reason: "No execution reached the end".to_string(),
            });
        }

        if output.trace.len() < input.trace_length as usize
            || output.termination == TerminationState::Stuck
        {
            if output.termination == TerminationState::Running {
                return Ok(ValidationResult::Mismatch {
                    reason: "Not enough traces were produced".to_string(),
                });
            }

            if !possible_executions.iter().any(|exe| exe.is_stuck(&pg)) {
                return Ok(ValidationResult::Mismatch {
                    reason: "No stuck execution found".to_string(),
                });
            }

            return Ok(ValidationResult::CorrectTerminated);
        }

        // TODO: check termination status is correct

        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for Input {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, mut rng: &mut R) -> Self {
        let commands = gcl::ast::Commands::gen(&mut Default::default(), rng);
        let initial_memory = gcl::memory::Memory::from_targets_with(
            commands.fv(),
            &mut rng,
            |rng, _| rng.gen_range(-10..=10),
            |rng, _| {
                let len = rng.gen_range(5..=10);
                (0..len).map(|_| rng.gen_range(-10..=10)).collect()
            },
        );
        let assignment = InterpreterMemory {
            variables: initial_memory.variables,
            arrays: initial_memory.arrays,
        };

        let determinism = *[Determinism::Deterministic, Determinism::NonDeterministic]
            .choose(rng)
            .unwrap();

        Input {
            commands: Stringify::new(commands),
            determinism,
            assignment,
            trace_length: rng.gen_range(10..=15),
        }
    }
}
