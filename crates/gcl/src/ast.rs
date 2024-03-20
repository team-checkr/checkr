use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target<Idx = ()> {
    Variable(Variable),
    Array(Array, Idx),
}
#[derive(
    tapi::Tapi, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[tapi(path = "GCL")]
pub enum TargetKind {
    Variable,
    Array,
}

#[derive(tapi::Tapi, Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[tapi(path = "GCL")]
pub struct TargetDef {
    pub name: Target,
    pub kind: TargetKind,
}

#[derive(tapi::Tapi, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[tapi(path = "GCL")]
#[serde(transparent)]
pub struct Variable(pub String);

#[derive(tapi::Tapi, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[tapi(path = "GCL")]
#[serde(transparent)]
pub struct Array(pub String);

impl FromStr for Commands {
    type Err = crate::parse::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse::parse_commands(s)
    }
}
impl FromStr for BExpr {
    type Err = crate::parse::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse::parse_bexpr(s)
    }
}
impl FromStr for AExpr {
    type Err = crate::parse::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse::parse_aexpr(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands(pub Vec<Command>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Assignment(Target<Box<AExpr>>, AExpr),
    Skip,
    If(Vec<Guard>),
    Loop(Vec<Guard>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard(pub BExpr, pub Commands);

pub type Int = i32;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AExpr {
    Number(Int),
    Reference(Target<Box<AExpr>>),
    Binary(Box<AExpr>, AOp, Box<AExpr>),
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

// Security

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Flow<T> {
    pub from: T,
    pub into: T,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecurityClass(pub String);
