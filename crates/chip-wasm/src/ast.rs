use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::parse::SourceSpan;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Target<Idx = ()> {
    Variable(Variable),
    Array(Array, Idx),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TargetKind {
    Variable,
    Array,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetDef {
    pub name: Target,
    pub kind: TargetKind,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variable(pub String);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Array(pub String);

pub type AGCLCommands = Commands<PredicateChain, PredicateBlock>;
pub type AGCLCommand = Command<PredicateChain, PredicateBlock>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Commands<Pred, Inv>(pub Vec<Command<Pred, Inv>>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Command<Pred, Inv> {
    pub kind: CommandKind<Pred, Inv>,
    pub span: SourceSpan,
    pub pre: Pred,
    pub post: Pred,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandKind<Pred, Inv> {
    Assignment(Target<Box<AExpr>>, AExpr),
    Skip,
    If(Vec<Guard<Pred, Inv>>),
    Loop(Inv, Vec<Guard<Pred, Inv>>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guard<Pred, Inv> {
    pub guard_span: SourceSpan,
    pub guard: BExpr,
    pub cmds: Commands<Pred, Inv>,
}

pub type Int = i32;

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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Function {
    Division(Box<AExpr>, Box<AExpr>),
    Min(Box<AExpr>, Box<AExpr>),
    Max(Box<AExpr>, Box<AExpr>),
    Fac(Box<AExpr>),
    Fib(Box<AExpr>),
    Exp(Box<AExpr>, Box<AExpr>),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PredicateChain {
    pub predicates: Vec<PredicateBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PredicateBlock {
    pub predicate: Predicate,
    pub span: SourceSpan,
}

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LTLFormula {
    Bool(bool),
    Rel(AExpr, RelOp, AExpr),
    Not(Box<LTLFormula>),
    And(Box<LTLFormula>, Box<LTLFormula>),
    Or(Box<LTLFormula>, Box<LTLFormula>),
    Implies(Box<LTLFormula>, Box<LTLFormula>),
    Until(Box<LTLFormula>, Box<LTLFormula>),
    Next(Box<LTLFormula>),
    Globally(Box<LTLFormula>),
    Finally(Box<LTLFormula>),
}

pub struct LTLProgram {
    pub initial: IndexMap<Variable, i32>,
    pub commands: Vec<Commands<(), ()>>,
    pub ltl: LTLFormula,
}
