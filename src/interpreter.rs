use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, AOp, Array, BExpr, LogicOp, RelOp, Variable},
    pg::{Action, Node, ProgramGraph},
};

pub struct Interpreter {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub variables: HashMap<Variable, i64>,
    pub arrays: HashMap<(String, i64), i64>,
}

impl Memory {
    pub fn zero(pg: &ProgramGraph) -> Memory {
        Memory {
            variables: pg.fv().into_iter().map(|k| (k, 0)).collect(),
            arrays: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramState {
    Running(Node, Memory),
    Terminated(Memory),
    Stuck(Node, Memory),
}

impl Interpreter {
    pub fn evaluate(mut steps: usize, memory: Memory, pg: &ProgramGraph) -> Vec<ProgramState> {
        let mut state = ProgramState::Running(Node::Start, memory);
        let mut trace = vec![state.clone()];

        while let ProgramState::Running(n, m) = state {
            let next = pg
                .outgoing(n)
                .iter()
                .find_map(|e| e.1.semantics(&m).map(|m| ProgramState::Running(e.2, m)));
            state = match next {
                Some(s) => s,
                None if n == Node::End => ProgramState::Terminated(m),
                None => ProgramState::Stuck(n, m),
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
    pub fn semantics(&self, m: &Memory) -> Option<Memory> {
        match self {
            Action::Assignment(x, a) => {
                if m.variables.contains_key(x) {
                    let mut m2 = m.clone();
                    m2.variables.insert(x.clone(), a.semantics(m));
                    Some(m2)
                } else {
                    todo!("variable '{x}' is not in memory")
                }
            }
            Action::ArrayAssignment(arr, idx, a) => {
                let idx = idx.semantics(m);
                if m.arrays.contains_key(&(arr.clone(), idx)) {
                    let mut m2 = m.clone();
                    m2.arrays.insert((arr.clone(), idx), a.semantics(m));
                    Some(m2)
                } else {
                    todo!("array '{arr}[{idx}]' is not in memory")
                }
            }
            Action::Skip => Some(m.clone()),
            Action::Condition(b) => {
                if b.semantics(m) {
                    Some(m.clone())
                } else {
                    None
                }
            }
        }
    }
}

impl AExpr {
    pub fn semantics(&self, m: &Memory) -> i64 {
        match self {
            AExpr::Number(n) => *n,
            AExpr::Variable(x) => {
                if let Some(x) = m.variables.get(x) {
                    *x
                } else {
                    todo!("not in memory")
                }
            }
            AExpr::Binary(l, op, r) => match op {
                AOp::Plus => l.semantics(m) + r.semantics(m),
                AOp::Minus => l.semantics(m) - r.semantics(m),
                AOp::Times => l.semantics(m) * r.semantics(m),
                AOp::Divide => {
                    if r.semantics(m) != 0 {
                        l.semantics(m) / r.semantics(m)
                    } else {
                        todo!("cannot divide by 0")
                    }
                }
                AOp::Pow => {
                    if r.semantics(m) >= 0 {
                        l.semantics(m).pow(r.semantics(m) as _)
                    } else {
                        todo!("cannot take negative power")
                    }
                }
            },
            AExpr::Array(Array(arr, idx)) => {
                if let Some(x) = m.arrays.get(&(arr.clone(), idx.semantics(m))) {
                    *x
                } else {
                    todo!("not in memory")
                }
            }
            AExpr::Minus(n) => -n.semantics(m),
        }
    }
}

impl BExpr {
    pub fn semantics(&self, m: &Memory) -> bool {
        match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => match op {
                RelOp::Eq => l.semantics(m) == r.semantics(m),
                RelOp::Ne => l.semantics(m) != r.semantics(m),
                RelOp::Gt => l.semantics(m) > r.semantics(m),
                RelOp::Ge => l.semantics(m) >= r.semantics(m),
                RelOp::Lt => l.semantics(m) < r.semantics(m),
                RelOp::Le => l.semantics(m) <= r.semantics(m),
            },
            BExpr::Logic(l, op, r) => match op {
                LogicOp::And => l.semantics(m) && r.semantics(m),
                LogicOp::Land => l.semantics(m) && r.semantics(m),
                LogicOp::Or => l.semantics(m) || r.semantics(m),
                LogicOp::Lor => l.semantics(m) || r.semantics(m),
            },
            BExpr::Not(b) => !b.semantics(m),
        }
    }
}
