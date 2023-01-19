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
pub struct Array<Idx>(pub String, pub Idx);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands(pub Vec<Command>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Assignment(Variable, AExpr),
    Skip,
    If(Vec<Guard>),
    Loop(Vec<Guard>),
    /// **Extension**
    ArrayAssignment(Array<Box<AExpr>>, AExpr),
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
    Array(Array<Box<AExpr>>),
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
    pub fn fa(&self) -> HashSet<String> {
        self.0.iter().flat_map(|c| c.fa()).collect()
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
    pub fn fa(&self) -> HashSet<String> {
        match self {
            Command::Assignment(_, a) => a.fa(),
            Command::Skip => HashSet::default(),
            Command::If(c) => guards_fa(c),
            Command::Loop(c) => guards_fa(c),
            Command::ArrayAssignment(Array(name, idx), a) => std::iter::once(name.to_string())
                .chain(idx.fa().union(&a.fa()).cloned())
                .collect(),
            Command::Break => HashSet::default(),
            Command::Continue => HashSet::default(),
        }
    }
}
fn guards_fv(guards: &[Guard]) -> HashSet<Variable> {
    guards.iter().flat_map(|g| g.fv()).collect()
}
fn guards_fa(guards: &[Guard]) -> HashSet<String> {
    guards.iter().flat_map(|g| g.fa()).collect()
}
impl Guard {
    pub fn fv(&self) -> HashSet<Variable> {
        self.0.fv().union(&self.1.fv()).cloned().collect()
    }
    pub fn fa(&self) -> HashSet<String> {
        self.0.fa().union(&self.1.fa()).cloned().collect()
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
    pub fn fa(&self) -> HashSet<String> {
        match self {
            AExpr::Number(_) => Default::default(),
            AExpr::Variable(_) => Default::default(),
            AExpr::Binary(l, _, r) => l.fa().union(&r.fa()).cloned().collect(),
            AExpr::Array(Array(name, idx)) => {
                std::iter::once(name.to_string()).chain(idx.fa()).collect()
            }
            AExpr::Minus(x) => x.fa(),
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
    pub fn fa(&self) -> HashSet<String> {
        match self {
            BExpr::Bool(_) => Default::default(),
            BExpr::Rel(l, _, r) => l.fa().union(&r.fa()).cloned().collect(),
            BExpr::Logic(l, _, r) => l.fa().union(&r.fa()).cloned().collect(),
            BExpr::Not(x) => x.fa(),
        }
    }
}
