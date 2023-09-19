use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target<Idx = ()> {
    Variable(Variable),
    Array(Array, Idx),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variable(pub String);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Array(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands(pub Vec<Command>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Assignment(Target<Box<AExpr>>, AExpr),
    Skip,
    If(Vec<Guard>),
    Loop(Vec<Guard>),
    /// **Extension**
    EnrichedLoop(Predicate, Vec<Guard>),
    /// **Extension**
    Annotated(Predicate, Commands, Predicate),
    /// **Extension**
    Break,
    /// **Extension**
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard(pub BExpr, pub Commands);

pub type Int = i64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AExpr {
    Number(Int),
    Reference(Target<Box<AExpr>>),
    Binary(Box<AExpr>, AOp, Box<AExpr>),
    Minus(Box<AExpr>),
    Function(Function),
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
pub enum Function {
    Division(Box<AExpr>, Box<AExpr>),
    Min(Box<AExpr>, Box<AExpr>),
    Max(Box<AExpr>, Box<AExpr>),
    Count(Array, Box<AExpr>),
    LogicalCount(Array, Box<AExpr>),
    Length(Array),
    LogicalLength(Array),
    Fac(Box<AExpr>),
    Fib(Box<AExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BExpr {
    Bool(bool),
    Rel(AExpr, RelOp, AExpr),
    Logic(Box<BExpr>, LogicOp, Box<BExpr>),
    Not(Box<BExpr>),
    Quantified(Quantifier, Target<()>, Box<BExpr>),
}

pub type Predicate = BExpr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quantifier {
    Exists,
    Forall,
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
    /// **Enriched**
    Implies,
}

// Security

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Flow<T> {
    pub from: T,
    pub into: T,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SecurityClass(pub String);
