use std::{collections::HashSet, str::FromStr};

use itertools::Either;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target<Idx = ()> {
    Variable(Variable),
    Array(Array, Idx),
}

impl Target<()> {
    pub fn promote_to_array(self) -> Target<()> {
        match self {
            Target::Variable(Variable(var)) => Target::Array(Array(var), ()),
            Target::Array(arr, ()) => Target::Array(arr, ()),
        }
    }
}
impl<Idx> Target<Idx> {
    pub fn name(&self) -> &str {
        match self {
            Target::Variable(x) => &x.0,
            Target::Array(a, _) => &a.0,
        }
    }
    pub fn map_idx<T>(self, f: impl FnOnce(Idx) -> T) -> Target<T> {
        match self {
            Target::Variable(var) => Target::Variable(var),
            Target::Array(arr, idx) => Target::Array(arr, f(idx)),
        }
    }
    pub fn unit(self) -> Target {
        self.map_idx(|_| ())
    }
    pub fn same_name<T>(&self, other: &Target<T>) -> bool {
        match (self, other) {
            (Target::Variable(a), Target::Variable(b)) => a == b,
            (Target::Array(a, _), Target::Array(b, _)) => a == b,
            _ => false,
        }
    }
    pub fn is_logical(&self) -> bool {
        match self {
            Target::Variable(v) => v.is_logical(),
            Target::Array(a, _) => a.is_logical(),
        }
    }
}

impl<Idx> std::fmt::Debug for Target<Idx>
where
    Idx: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => v.fmt(f),
            Self::Array(a, idx) => write!(f, "Array({a}, {idx:?})"),
        }
    }
}

impl serde::Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Target::Variable(v) => v.serialize(serializer),
            Target::Array(a, ()) => a.serialize(serializer),
        }
    }
}
impl<'de> serde::Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Target::Variable(Variable::deserialize(deserializer)?))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Variable(pub String);
impl Variable {
    pub fn is_logical(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl std::fmt::Debug for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl FromStr for Variable {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Variable(s.to_string()))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Array(pub String);
impl Array {
    pub fn is_logical(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl std::fmt::Debug for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl FromStr for Array {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Array(s.to_string()))
    }
}

impl<Idx> From<Variable> for Target<Idx> {
    fn from(value: Variable) -> Self {
        Target::Variable(value)
    }
}
impl From<Array> for Target<()> {
    fn from(value: Array) -> Self {
        Target::Array(value, ())
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

impl Commands {
    pub fn fv(&self) -> HashSet<Target> {
        self.0.iter().flat_map(|c| c.fv()).collect()
    }
}
impl Command {
    pub fn fv(&self) -> HashSet<Target> {
        match self {
            Command::Assignment(x, a) => x.fv().union(&a.fv()).cloned().collect(),
            Command::Skip => HashSet::default(),
            Command::If(c) => guards_fv(c),
            Command::Loop(c) => guards_fv(c),
            // TODO: Maybe the pred should also be looked at?
            Command::EnrichedLoop(_, c) => guards_fv(c),
            // TODO: Maybe the pred should also be looked at?
            Command::Annotated(_, c, _) => c.fv(),
            Command::Break => HashSet::default(),
            Command::Continue => HashSet::default(),
        }
    }
}
fn guards_fv(guards: &[Guard]) -> HashSet<Target> {
    guards.iter().flat_map(|g| g.fv()).collect()
}
impl Guard {
    pub fn fv(&self) -> HashSet<Target> {
        self.0.fv().union(&self.1.fv()).cloned().collect()
    }
}
impl Target<Box<AExpr>> {
    pub fn fv(&self) -> HashSet<Target> {
        match self {
            Target::Variable(v) => [Target::Variable(v.clone())].into_iter().collect(),
            Target::Array(Array(a), idx) => {
                let mut fv = idx.fv();
                fv.insert(Target::Array(Array(a.clone()), ()));
                fv
            }
        }
    }
}
impl AExpr {
    pub fn binary(lhs: Self, op: AOp, rhs: Self) -> Self {
        Self::Binary(Box::new(lhs), op, Box::new(rhs))
    }
    pub fn fv(&self) -> HashSet<Target> {
        match self {
            AExpr::Number(_) => Default::default(),
            AExpr::Reference(v) => v.fv(),
            AExpr::Binary(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            AExpr::Minus(x) => x.fv(),
            AExpr::Function(f) => f.fv(),
        }
    }
}
impl Function {
    pub fn exprs(&self) -> impl Iterator<Item = &AExpr> {
        match self {
            Function::Division(a, b) | Function::Min(a, b) | Function::Max(a, b) => {
                Either::Left([a.as_ref(), b.as_ref()].into_iter())
            }
            Function::Count(_, x)
            | Function::LogicalCount(_, x)
            | Function::Fac(x)
            | Function::Fib(x) => Either::Right(Either::Left([x.as_ref()].into_iter())),
            Function::Length(_) | Function::LogicalLength(_) => {
                Either::Right(Either::Right(std::iter::empty()))
            }
        }
    }
    pub fn fv(&self) -> HashSet<Target> {
        match self {
            Function::Count(a, x) | Function::LogicalCount(a, x) => [Target::Array(a.clone(), ())]
                .into_iter()
                .chain(x.fv())
                .collect(),
            Function::Length(a) | Function::LogicalLength(a) => {
                [Target::Array(a.clone(), ())].into_iter().collect()
            }
            _ => self.exprs().flat_map(|x| x.fv()).collect(),
        }
    }
}
impl BExpr {
    pub fn logic(lhs: Self, op: LogicOp, rhs: Self) -> Self {
        Self::Logic(Box::new(lhs), op, Box::new(rhs))
    }
    pub fn rel(lhs: AExpr, op: RelOp, rhs: AExpr) -> Self {
        Self::Rel(lhs, op, rhs)
    }
    pub fn fv(&self) -> HashSet<Target> {
        match self {
            BExpr::Bool(_) => Default::default(),
            BExpr::Rel(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Logic(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Not(x) => x.fv(),
            BExpr::Quantified(_, x, b) => {
                let mut fv = b.fv();
                fv.remove(x);
                fv
            }
        }
    }
}
