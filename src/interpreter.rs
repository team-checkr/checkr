use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, AOp, Array, BExpr, LogicOp, RelOp},
    pg::{Action, Node, ProgramGraph},
    sign::Memory,
};

pub struct Interpreter {}

pub type InterpreterMemory = Memory<i64, Vec<i64>>;

impl InterpreterMemory {
    pub fn zero(pg: &ProgramGraph) -> InterpreterMemory {
        InterpreterMemory {
            variables: pg.fv().into_iter().map(|k| (k, 0)).collect(),
            arrays: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "Case")]
pub enum ProgramState {
    Running,
    Stuck,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramTrace<N = Node> {
    pub state: ProgramState,
    pub node: N,
    pub memory: InterpreterMemory,
}

impl<A> ProgramTrace<A> {
    pub fn map_node<B>(self, f: impl FnOnce(A) -> B) -> ProgramTrace<B> {
        ProgramTrace {
            state: self.state,
            node: f(self.node),
            memory: self.memory,
        }
    }
}

impl Interpreter {
    pub fn evaluate(
        mut steps: usize,
        memory: InterpreterMemory,
        pg: &ProgramGraph,
    ) -> Vec<ProgramTrace> {
        let mut state = ProgramTrace {
            state: ProgramState::Running,
            node: Node::Start,
            memory,
        };
        let mut trace = vec![state.clone()];

        while let ProgramState::Running = state.state {
            let next = pg.outgoing(state.node).iter().find_map(|e| {
                e.1.semantics(&state.memory)
                    .map(|m| ProgramTrace {
                        state: ProgramState::Running,
                        node: e.2,
                        memory: m,
                    })
                    .ok()
            });
            state = match next {
                Some(s) => s,
                None if state.node == Node::End => ProgramTrace {
                    state: ProgramState::Terminated,
                    node: state.node,
                    memory: state.memory,
                },
                None => ProgramTrace {
                    state: ProgramState::Stuck,
                    node: state.node,
                    memory: state.memory,
                },
            };
            trace.push(state.clone());

            if steps == 0 {
                break;
            }
            steps -= 1;
        }

        trace
    }
}

impl Action {
    pub fn semantics(&self, m: &InterpreterMemory) -> Result<InterpreterMemory, InterpreterError> {
        match self {
            Action::Assignment(x, a) => {
                if m.variables.contains_key(x) {
                    let mut m2 = m.clone();
                    m2.variables.insert(x.clone(), a.semantics(m)?);
                    Ok(m2)
                } else {
                    todo!("variable '{x}' is not in memory")
                }
            }
            Action::ArrayAssignment(arr, idx, a) => {
                let idx = idx.semantics(m)?;
                match m.arrays.get(arr) {
                    Some(data) if 0 <= idx && idx < data.len() as _ => {
                        let mut m2 = m.clone();
                        let data = m2.arrays.get_mut(arr).unwrap();
                        data[idx as usize] = a.semantics(m)?;
                        Ok(m2)
                    }
                    Some(_) => Err(InterpreterError::ArrayNotFound { name: arr.clone() }),
                    None => Err(InterpreterError::IndexOutOfBound {
                        name: arr.clone(),
                        index: idx,
                    }),
                }
            }
            Action::Skip => Ok(m.clone()),
            Action::Condition(b) => {
                if b.semantics(m)? {
                    Ok(m.clone())
                } else {
                    Err(InterpreterError::NoProgression)
                }
            }
        }
    }
}

impl AExpr {
    pub fn semantics(&self, m: &InterpreterMemory) -> Result<i64, InterpreterError> {
        Ok(match self {
            AExpr::Number(n) => *n,
            AExpr::Variable(x) => {
                if let Some(x) = m.variables.get(x) {
                    *x
                } else {
                    return Err(InterpreterError::VariableNotFound {
                        name: x.to_string(),
                    });
                }
            }
            AExpr::Binary(l, op, r) => op.semantic(l.semantics(m)?, r.semantics(m)?)?,
            AExpr::Array(Array(arr, idx)) => {
                let data = if let Some(data) = m.arrays.get(arr) {
                    data
                } else {
                    return Err(InterpreterError::ArrayNotFound { name: arr.clone() });
                };
                let idx = idx.semantics(m)?;
                if let Some(x) = data.get(idx as usize) {
                    *x
                } else {
                    return Err(InterpreterError::IndexOutOfBound {
                        name: arr.clone(),
                        index: idx,
                    });
                }
            }
            AExpr::Minus(n) => -n.semantics(m)?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum InterpreterError {
    #[error("division by zero")]
    DivisionByZero,
    #[error("negative exponent")]
    NegativeExponent,
    #[error("variable '{name}' not found")]
    VariableNotFound { name: String },
    #[error("array '{name}' not found")]
    ArrayNotFound { name: String },
    #[error("index {index} in '{name}' is out-of-bounds")]
    IndexOutOfBound { name: String, index: i64 },
    #[error("no progression")]
    NoProgression,
    #[error("an arithmetic operation overflowed")]
    ArithmeticOverflow,
}

impl AOp {
    pub fn semantic(&self, l: i64, r: i64) -> Result<i64, InterpreterError> {
        Ok(match self {
            AOp::Plus => l + r,
            AOp::Minus => l - r,
            AOp::Times => l
                .checked_mul(r)
                .ok_or(InterpreterError::ArithmeticOverflow)?,
            AOp::Divide => {
                if r != 0 {
                    l / r
                } else {
                    return Err(InterpreterError::DivisionByZero);
                }
            }
            AOp::Pow => {
                if r >= 0 {
                    l.checked_pow(r as _)
                        .ok_or(InterpreterError::ArithmeticOverflow)?
                } else {
                    return Err(InterpreterError::NegativeExponent);
                }
            }
        })
    }
}

impl BExpr {
    pub fn semantics(&self, m: &InterpreterMemory) -> Result<bool, InterpreterError> {
        Ok(match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => op.semantic(l.semantics(m)?, r.semantics(m)?),
            BExpr::Logic(l, op, r) => op.semantic(l.semantics(m)?, || r.semantics(m))?,
            BExpr::Not(b) => !b.semantics(m)?,
        })
    }
}

impl RelOp {
    pub fn semantic(&self, l: i64, r: i64) -> bool {
        match self {
            RelOp::Eq => l == r,
            RelOp::Ne => l != r,
            RelOp::Gt => l > r,
            RelOp::Ge => l >= r,
            RelOp::Lt => l < r,
            RelOp::Le => l <= r,
        }
    }
}

impl LogicOp {
    pub fn semantic(
        &self,
        l: bool,
        r: impl FnOnce() -> Result<bool, InterpreterError>,
    ) -> Result<bool, InterpreterError> {
        Ok(match self {
            LogicOp::And => l && r()?,
            LogicOp::Land => {
                let r = r()?;
                l && r
            }
            LogicOp::Or => l || r()?,
            LogicOp::Lor => {
                let r = r()?;
                l || r
            }
        })
    }
}
