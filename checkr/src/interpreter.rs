use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, AOp, BExpr, LogicOp, RelOp, Target},
    pg::{Action, Node, ProgramGraph},
    sign::Memory,
};

pub struct Interpreter {}

pub type InterpreterMemory = Memory<i64, Vec<i64>>;

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

impl Action {
    pub fn semantics(&self, m: &InterpreterMemory) -> Result<InterpreterMemory, InterpreterError> {
        match self {
            Action::Assignment(Target::Variable(x), a) => {
                if m.variables.contains_key(x) {
                    let mut m2 = m.clone();
                    m2.variables.insert(x.clone(), a.semantics(m)?);
                    Ok(m2)
                } else {
                    todo!("variable '{x}' is not in memory")
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
                    Some(_) => Err(InterpreterError::ArrayNotFound {
                        name: arr.to_string(),
                    }),
                    None => Err(InterpreterError::IndexOutOfBound {
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
            AExpr::Reference(Target::Variable(x)) => {
                if let Some(x) = m.variables.get(x) {
                    *x
                } else {
                    return Err(InterpreterError::VariableNotFound {
                        name: x.to_string(),
                    });
                }
            }
            AExpr::Reference(Target::Array(arr, idx)) => {
                let data = if let Some(data) = m.arrays.get(arr) {
                    data
                } else {
                    return Err(InterpreterError::ArrayNotFound {
                        name: arr.to_string(),
                    });
                };
                let idx = idx.semantics(m)?;
                if let Some(x) = data.get(idx as usize) {
                    *x
                } else {
                    return Err(InterpreterError::IndexOutOfBound {
                        name: arr.to_string(),
                        index: idx,
                    });
                }
            }
            AExpr::Binary(l, op, r) => op.semantic(l.semantics(m)?, r.semantics(m)?)?,
            AExpr::Minus(n) => {
                let temp = n.semantics(m);
                match temp {
                    Ok(val) => return val.checked_neg().ok_or(InterpreterError::ArithmeticOverflow),
                    Err(e) => return Err(e),
                }
            }
            AExpr::Function(f) => return Err(todo!("evaluating functions {f}")),
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
    #[error("tried to evaluate a quantified expression")]
    EvaluateQuantifier,
}

impl AOp {
    pub fn semantic(&self, l: i64, r: i64) -> Result<i64, InterpreterError> {
        Ok(match self {
            AOp::Plus => l
                .checked_add(r)
                .ok_or(InterpreterError::ArithmeticOverflow)?,
            AOp::Minus => l
                .checked_sub(r)
                .ok_or(InterpreterError::ArithmeticOverflow)?,
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
            BExpr::Quantified(_, _, _) => return Err(InterpreterError::EvaluateQuantifier),
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
            LogicOp::Implies => {
                let r = r()?;
                !l || r
            }
        })
    }
}
