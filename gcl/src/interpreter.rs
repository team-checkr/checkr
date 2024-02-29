use std::collections::BTreeMap;

use crate::{
    ast::{Array, Int, Variable},
    pg::{Edge, Node, ProgramGraph},
    semantics::{SemanticsContext, SemanticsError},
    stringify::Stringify,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct InterpreterMemory {
    pub variables: BTreeMap<Variable, Int>,
    pub arrays: BTreeMap<Array, Vec<Int>>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Interpreter")]
pub struct Step {
    pub action: Stringify<crate::pg::Action>,
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
pub struct Execution {
    initial_memory: InterpreterMemory,
    trace: Vec<(Step, Node)>,
}

impl Execution {
    pub fn new(initial_memory: InterpreterMemory) -> Self {
        Self {
            initial_memory,
            trace: vec![],
        }
    }
    pub fn trace(&self) -> &[(Step, Node)] {
        &self.trace
    }
    pub fn current_node(&self) -> Node {
        self.trace.last().map(|(_, n)| *n).unwrap_or(Node::Start)
    }
    pub fn current_mem(&self) -> &InterpreterMemory {
        self.trace
            .last()
            .map(|(s, _)| &s.memory)
            .unwrap_or(&self.initial_memory)
    }
    pub fn is_finished(&self) -> bool {
        self.current_node() == Node::End
    }
    pub fn is_stuck(&self, pg: &ProgramGraph) -> bool {
        pg.outgoing(self.current_node())
            .iter()
            .all(|Edge(_, action, _)| action.semantics(self.current_mem()).is_err())
    }
    pub fn state(&self, pg: &ProgramGraph) -> TerminationState {
        if self.is_stuck(pg) {
            if self.is_finished() {
                TerminationState::Terminated
            } else {
                TerminationState::Stuck
            }
        } else {
            TerminationState::Running
        }
    }
    pub fn nexts(&self, pg: &ProgramGraph) -> Vec<Execution> {
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
