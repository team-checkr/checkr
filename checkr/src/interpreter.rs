use gcl::{
    ast::{Int, Target},
    semantics::{SemanticsContext, SemanticsError},
};
use serde::{Deserialize, Serialize};

use crate::{
    pg::{Action, Node, ProgramGraph},
    sign::Memory,
};

pub struct Interpreter {}

pub type InterpreterMemory = Memory<Int, Vec<Int>>;

impl InterpreterMemory {
    pub fn zero(pg: &ProgramGraph) -> InterpreterMemory {
        Memory::from_targets(pg.fv(), |_| 0, |_| vec![])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "Case")]
pub enum TerminationState {
    Running,
    Stuck,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Configuration<N = Node> {
    pub node: N,
    pub memory: InterpreterMemory,
}

impl<A> Configuration<A> {
    pub fn map_node<B>(self, f: impl FnOnce(A) -> B) -> Configuration<B> {
        Configuration {
            node: f(self.node),
            memory: self.memory,
        }
    }
}

impl Interpreter {
    pub fn evaluate(
        mut steps: u64,
        memory: InterpreterMemory,
        pg: &ProgramGraph,
    ) -> (Vec<Configuration>, TerminationState) {
        let mut state = Configuration {
            node: Node::Start,
            memory,
        };
        let mut trace = vec![state.clone()];

        let termination = loop {
            if steps < 2 {
                break TerminationState::Running;
            }
            steps -= 1;

            let next = pg.outgoing(state.node).iter().find_map(|e| {
                e.1.semantics(&state.memory)
                    .map(|m| Configuration {
                        node: e.2,
                        memory: m,
                    })
                    .ok()
            });
            state = match next {
                Some(s) => s,
                None if state.node == Node::End => break TerminationState::Terminated,
                None => break TerminationState::Stuck,
            };
            trace.push(state.clone());
        };

        (trace, termination)
    }
}

fn lookup_array<'a>(
    mem: &'a InterpreterMemory,
    array: &gcl::ast::Array,
) -> Result<&'a [Int], SemanticsError> {
    mem.arrays
        .get(array)
        .ok_or_else(|| SemanticsError::ArrayNotFound {
            name: array.to_string(),
        })
        .map(|data| &**data)
}

impl SemanticsContext for InterpreterMemory {
    fn variable(&self, var: &gcl::ast::Variable) -> Result<Int, SemanticsError> {
        self.variables
            .get(var)
            .ok_or_else(|| SemanticsError::VariableNotFound {
                name: var.to_string(),
            })
            .copied()
    }

    fn array_element(&self, array: &gcl::ast::Array, index: Int) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        data.get(index as usize)
            .ok_or_else(|| SemanticsError::IndexOutOfBound {
                name: array.to_string(),
                index,
            })
            .copied()
    }

    fn array_length(&self, array: &gcl::ast::Array) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        Ok(data.len() as _)
    }

    fn array_count(&self, array: &gcl::ast::Array, element: Int) -> Result<Int, SemanticsError> {
        let data = lookup_array(self, array)?;
        Ok(data.iter().filter(|e| **e == element).count() as _)
    }
}

impl Action {
    pub fn semantics(&self, m: &InterpreterMemory) -> Result<InterpreterMemory, SemanticsError> {
        match self {
            Action::Assignment(Target::Variable(x), a) => {
                if m.variables.contains_key(x) {
                    let mut m2 = m.clone();
                    m2.variables.insert(x.clone(), a.semantics(m)?);
                    Ok(m2)
                } else {
                    Err(SemanticsError::VariableNotFound {
                        name: x.to_string(),
                    })
                }
            }
            Action::Assignment(Target::Array(arr, idx), a) => {
                let idx = idx.semantics(m)?;
                match m.get_arr(arr) {
                    Some(data) if 0 <= idx && idx < data.len() as _ => {
                        let mut m2 = m.clone();
                        let data = m2.arrays.get_mut(arr).unwrap();
                        data[idx as usize] = a.semantics(m)?;
                        Ok(m2)
                    }
                    Some(_) => Err(SemanticsError::ArrayNotFound {
                        name: arr.to_string(),
                    }),
                    None => Err(SemanticsError::IndexOutOfBound {
                        name: arr.to_string(),
                        index: idx,
                    }),
                }
            }
            Action::Skip => Ok(m.clone()),
            Action::Condition(b) => {
                if b.semantics(m)? {
                    Ok(m.clone())
                } else {
                    Err(SemanticsError::NoProgression)
                }
            }
        }
    }
}
