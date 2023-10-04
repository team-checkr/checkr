use std::{collections::HashSet, fmt::Debug, str::FromStr};

use itertools::Either;

use crate::{
    ast::{
        AExpr, AOp, Array, BExpr, Command, Commands, Flow, Function, Guard, LogicOp, RelOp, Target,
        Variable,
    },
    semantics::EmptySemanticsContext,
};

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

impl<Idx> Debug for Target<Idx>
where
    Idx: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => Debug::fmt(&v, f),
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
impl Variable {
    pub fn is_logical(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl Debug for Variable {
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

impl Array {
    pub fn is_logical(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl Debug for Array {
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

impl BExpr {
    pub fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> BExpr {
        match self {
            BExpr::Bool(b) => BExpr::Bool(*b),
            BExpr::Rel(l, op, r) => BExpr::Rel(l.subst_var(t, x), *op, r.subst_var(t, x)),
            BExpr::Logic(l, op, r) => BExpr::logic(l.subst_var(t, x), *op, r.subst_var(t, x)),
            BExpr::Not(e) => BExpr::Not(Box::new(e.subst_var(t, x))),
            BExpr::Quantified(q, v, e) => {
                if v.same_name(t) {
                    self.clone()
                } else {
                    BExpr::Quantified(*q, v.clone(), Box::new(e.subst_var(t, x)))
                }
            }
        }
    }

    pub fn simplify(&self) -> BExpr {
        match self
            .semantics(&EmptySemanticsContext)
            .map(BExpr::Bool)
            .unwrap_or_else(|_| self.clone())
        {
            BExpr::Bool(b) => BExpr::Bool(b),
            BExpr::Rel(l, op, r) => BExpr::Rel(l.simplify(), op, r.simplify()),
            BExpr::Logic(l, op, r) => {
                let l = l.simplify();
                let r = r.simplify();

                match (l, op, r) {
                    (BExpr::Bool(true), LogicOp::And, x) | (x, LogicOp::And, BExpr::Bool(true)) => {
                        x
                    }
                    (BExpr::Bool(false), LogicOp::And, _)
                    | (_, LogicOp::And, BExpr::Bool(false)) => BExpr::Bool(false),
                    (BExpr::Bool(false), LogicOp::Or, x) | (x, LogicOp::Or, BExpr::Bool(false)) => {
                        x
                    }
                    (BExpr::Bool(true), LogicOp::Or, _) | (_, LogicOp::Or, BExpr::Bool(true)) => {
                        BExpr::Bool(true)
                    }
                    (l, op, r) => BExpr::logic(l, op, r),
                }
            }
            BExpr::Not(x) => {
                let x = x.simplify();
                match x {
                    BExpr::Bool(b) => BExpr::Bool(!b),
                    x => BExpr::Not(Box::new(x)),
                }
            }
            BExpr::Quantified(_, _, _) => todo!(),
        }
    }
}

impl AExpr {
    pub fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> AExpr {
        match self {
            AExpr::Number(n) => AExpr::Number(*n),
            AExpr::Reference(v) if v.same_name(t) => x.clone(),
            AExpr::Reference(v) => AExpr::Reference(v.clone()),
            AExpr::Binary(l, op, r) => AExpr::binary(l.subst_var(t, x), *op, r.subst_var(t, x)),
            AExpr::Minus(e) => AExpr::Minus(Box::new(e.subst_var(t, x))),
            AExpr::Function(f) => AExpr::Function(f.subst_var(t, x)),
        }
    }

    pub fn simplify(&self) -> AExpr {
        match self
            .semantics(&EmptySemanticsContext)
            .map(AExpr::Number)
            .unwrap_or_else(|_| self.clone())
        {
            AExpr::Number(n) => AExpr::Number(n),
            AExpr::Reference(v) => AExpr::Reference(v.simplify()),
            AExpr::Binary(l, op, r) => AExpr::binary(l.simplify(), op, r.simplify()),
            AExpr::Minus(e) => match &*e {
                AExpr::Minus(inner) => inner.simplify(),
                _ => AExpr::Minus(Box::new(e.simplify())),
            },
            AExpr::Function(_) => self.clone(),
        }
    }
}

impl Function {
    pub fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> Function {
        match self {
            Function::Division(a, b) => {
                Function::Division(Box::new(a.subst_var(t, x)), Box::new(b.subst_var(t, x)))
            }
            Function::Min(a, b) => {
                Function::Min(Box::new(a.subst_var(t, x)), Box::new(b.subst_var(t, x)))
            }
            Function::Max(a, b) => {
                Function::Max(Box::new(a.subst_var(t, x)), Box::new(b.subst_var(t, x)))
            }
            Function::Count(arr, idx) => {
                Function::Count(arr.clone(), Box::new(idx.subst_var(t, x)))
            }
            Function::LogicalCount(arr, idx) => {
                Function::LogicalCount(arr.clone(), Box::new(idx.subst_var(t, x)))
            }
            Function::Length(arr) => Function::Length(arr.clone()),
            Function::LogicalLength(arr) => Function::LogicalLength(arr.clone()),
            Function::Fac(n) => Function::Fac(Box::new(n.subst_var(t, x))),
            Function::Fib(n) => Function::Fib(Box::new(n.subst_var(t, x))),
        }
    }
}

impl Target<Box<AExpr>> {
    pub fn simplify(&self) -> Self {
        match self {
            Target::Variable(v) => Target::Variable(v.clone()),
            Target::Array(arr, idx) => Target::Array(arr.clone(), Box::new(idx.simplify())),
        }
    }
}

// Security

impl<T> Flow<T> {
    pub fn map<'a, S>(&'a self, f: impl Fn(&'a T) -> S) -> Flow<S> {
        Flow {
            from: f(&self.from),
            into: f(&self.into),
        }
    }
}
