use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variable(pub String);

impl std::fmt::Debug for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Array(pub String, pub Box<AExpr>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands(pub Vec<Command>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Assignment(Variable, AExpr),
    Skip,
    If(Vec<Guard>),
    Loop(Vec<Guard>),
    /// **Extension**
    ArrayAssignment(Array, AExpr),
    /// **Extension**
    Break,
    /// **Extension**
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard(pub BExpr, pub Commands);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AExpr {
    Number(i64),
    Variable(Variable),
    Binary(Box<AExpr>, AOp, Box<AExpr>),
    /// **Extension**z
    Array(Array),
    Minus(Box<AExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AOp {
    Plus,
    Minus,
    Times,
    Divide,
    Pow,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BExpr {
    Bool(bool),
    Rel(AExpr, RelOp, AExpr),
    Logic(Box<BExpr>, LogicOp, Box<BExpr>),
    Not(Box<BExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogicOp {
    And,
    Land,
    Or,
    Lor,
}

impl Commands {
    pub fn fv(&self) -> HashSet<Variable> {
        self.0.iter().flat_map(|c| c.fv()).collect()
    }
}
impl Command {
    pub fn fv(&self) -> HashSet<Variable> {
        match self {
            Command::Assignment(x, a) => [x.clone()].into_iter().chain(a.fv()).collect(),
            Command::Skip => HashSet::default(),
            Command::If(c) => guards_fv(c),
            Command::Loop(c) => guards_fv(c),
            Command::ArrayAssignment(Array(_, idx), a) => {
                idx.fv().union(&a.fv()).cloned().collect()
            }
            Command::Break => HashSet::default(),
            Command::Continue => HashSet::default(),
        }
    }
}
fn guards_fv(guards: &[Guard]) -> HashSet<Variable> {
    guards.iter().flat_map(|g| g.fv()).collect()
}
impl Guard {
    pub fn fv(&self) -> HashSet<Variable> {
        self.0.fv().union(&self.1.fv()).cloned().collect()
    }
}
impl AExpr {
    pub fn fv(&self) -> HashSet<Variable> {
        match self {
            AExpr::Number(_) => Default::default(),
            AExpr::Variable(v) => [v.clone()].into_iter().collect(),
            AExpr::Binary(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            AExpr::Array(Array(_, idx)) => idx.fv(),
            AExpr::Minus(x) => x.fv(),
        }
    }
}
impl BExpr {
    pub fn fv(&self) -> HashSet<Variable> {
        match self {
            BExpr::Bool(_) => Default::default(),
            BExpr::Rel(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Logic(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Not(x) => x.fv(),
        }
    }
}
