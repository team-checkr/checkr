use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ast::{AExpr, AOp, Array, BExpr, LogicOp, RelOp, Variable},
    pg::{Action, Node, ProgramGraph},
};

pub struct Interpreter {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterMemory {
    pub variables: HashMap<Variable, i64>,
    pub arrays: HashMap<(String, i64), i64>,
}

impl InterpreterMemory {
    pub fn zero(pg: &ProgramGraph) -> InterpreterMemory {
        InterpreterMemory {
            variables: pg.fv().into_iter().map(|k| (k, 0)).collect(),
            arrays: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramState {
    Running(Node, InterpreterMemory),
    Terminated(InterpreterMemory),
    Stuck(Node, InterpreterMemory),
}

impl Interpreter {
    pub fn evaluate(
        mut steps: usize,
        memory: InterpreterMemory,
        pg: &ProgramGraph,
    ) -> Vec<ProgramState> {
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
    pub fn semantics(&self, m: &InterpreterMemory) -> Option<InterpreterMemory> {
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
    pub fn semantics(&self, m: &InterpreterMemory) -> i64 {
        match self {
            AExpr::Number(n) => *n,
            AExpr::Variable(x) => {
                if let Some(x) = m.variables.get(x) {
                    *x
                } else {
                    todo!("not in memory")
                }
            }
            AExpr::Binary(l, op, r) => op.semantic(l.semantics(m), r.semantics(m)),
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

impl AOp {
    pub fn semantic(&self, l: i64, r: i64) -> i64 {
        match self {
            AOp::Plus => l + r,
            AOp::Minus => l - r,
            AOp::Times => l * r,
            AOp::Divide => {
                if r != 0 {
                    l / r
                } else {
                    // TODO: Return an error instead of crashing
                    todo!("cannot divide by 0")
                }
            }
            AOp::Pow => {
                if r >= 0 {
                    l.pow(r as _)
                } else {
                    // TODO: Return an error instead of crashing
                    todo!("cannot take negative power")
                }
            }
        }
    }
}

impl BExpr {
    pub fn semantics(&self, m: &InterpreterMemory) -> bool {
        match self {
            BExpr::Bool(b) => *b,
            BExpr::Rel(l, op, r) => op.semantic(l.semantics(m), r.semantics(m)),
            BExpr::Logic(l, op, r) => op.semantic(l.semantics(m), r.semantics(m)),
            BExpr::Not(b) => !b.semantics(m),
        }
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
    pub fn semantic(&self, l: bool, r: bool) -> bool {
        match self {
            LogicOp::And => l && r,
            LogicOp::Land => l && r,
            LogicOp::Or => l || r,
            LogicOp::Lor => l || r,
        }
    }
}
