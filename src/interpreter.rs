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
                e.1.semantics(&state.memory).map(|m| ProgramTrace {
                    state: ProgramState::Running,
                    node: e.2,
                    memory: m,
                })
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
                match m.arrays.get(arr) {
                    Some(data) if 0 <= idx && idx < data.len() as _ => {
                        let mut m2 = m.clone();
                        let data = m2.arrays.get_mut(arr).unwrap();
                        data[idx as usize] = a.semantics(m);
                        Some(m2)
                    }
                    _ => todo!("array '{arr}[{idx}]' is not in memory"),
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
                if let Some(x) = m
                    .arrays
                    .get(arr)
                    .and_then(|data| data.get(idx.semantics(m) as usize))
                {
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
