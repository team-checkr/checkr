use gcl::{
    ast::{Array, Int, Variable},
    memory::Memory,
    pg::{Node, ProgramGraph},
    semantics::{SemanticsContext, SemanticsError},
};
use serde::{Deserialize, Serialize};

pub struct Interpreter {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterMemory(Memory<Int, Vec<Int>>);

impl From<Memory<Int, Vec<Int>>> for InterpreterMemory {
    fn from(mem: Memory<Int, Vec<Int>>) -> Self {
        InterpreterMemory(mem)
    }
}

impl std::ops::Deref for InterpreterMemory {
    type Target = Memory<Int, Vec<Int>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for InterpreterMemory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
        match self.get_arr(array) {
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
