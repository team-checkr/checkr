use std::{fmt::Debug, str::FromStr};

use indexmap::IndexSet;
use itertools::Either;

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, CommandKind, Commands, Function, Guard, LogicOp, RelOp,
    Target, TargetDef, TargetKind, Variable,
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
    pub fn def(&self) -> TargetDef {
        match self {
            Target::Variable(v) => TargetDef {
                name: Target::Variable(v.clone()),
                kind: TargetKind::Variable,
            },
            Target::Array(a, _) => TargetDef {
                name: Target::Array(a.clone(), ()),
                kind: TargetKind::Array,
            },
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
    pub fn fv(&self) -> IndexSet<Target> {
        self.0.iter().flat_map(|c| c.fv()).collect()
    }
}
impl Command {
    pub fn fv(&self) -> IndexSet<Target> {
        let a = self.pre_predicates.iter().flat_map(|p| p.predicate.fv());
        let b = self.post_predicates.iter().flat_map(|p| p.predicate.fv());
        a.chain(self.kind.fv()).chain(b).collect()
    }
}
impl CommandKind {
    pub fn fv(&self) -> IndexSet<Target> {
        match self {
            CommandKind::Assignment(x, a) => x.fv().union(&a.fv()).cloned().collect(),
            CommandKind::Skip => IndexSet::default(),
            CommandKind::If(c) => guards_fv(c),
            CommandKind::Loop(inv, c) => inv.predicate.fv().union(&guards_fv(c)).cloned().collect(),
        }
    }
}
fn guards_fv(guards: &[Guard]) -> IndexSet<Target> {
    guards.iter().flat_map(|g| g.fv()).collect()
}
impl Guard {
    pub fn fv(&self) -> IndexSet<Target> {
        self.guard.fv().union(&self.cmds.fv()).cloned().collect()
    }
}
impl Target<Box<AExpr>> {
    pub fn fv(&self) -> IndexSet<Target> {
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
    pub fn fv(&self) -> IndexSet<Target> {
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
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Function::Division(_, _) => "div",
            Function::Min(_, _) => "min",
            Function::Max(_, _) => "max",
            Function::Fac(_) => "fac",
            Function::Fib(_) => "fib",
            Function::Exp(_, _) => "exp",
        }
    }

    pub(crate) fn args(&self) -> impl Iterator<Item = &AExpr> {
        match self {
            Function::Division(a, b)
            | Function::Min(a, b)
            | Function::Max(a, b)
            | Function::Exp(a, b) => Either::Left([a.as_ref(), b.as_ref()].into_iter()),
            Function::Fac(x) | Function::Fib(x) => Either::Right([x.as_ref()].into_iter()),
        }
    }

    pub fn fv(&self) -> IndexSet<Target> {
        self.args().flat_map(|x| x.fv()).collect()
    }
}
impl BExpr {
    pub fn logic(lhs: Self, op: LogicOp, rhs: Self) -> Self {
        Self::Logic(Box::new(lhs), op, Box::new(rhs))
    }
    pub fn implies(self, rhs: Self) -> Self {
        Self::logic(self, LogicOp::Implies, rhs)
    }
    pub fn and(self, rhs: Self) -> Self {
        Self::logic(self, LogicOp::And, rhs)
    }
    pub fn or(self, rhs: Self) -> Self {
        Self::logic(self, LogicOp::Or, rhs)
    }
    pub fn fv(&self) -> IndexSet<Target> {
        match self {
            BExpr::Bool(_) => Default::default(),
            BExpr::Rel(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Logic(l, _, r) => l.fv().union(&r.fv()).cloned().collect(),
            BExpr::Not(x) => x.fv(),
            BExpr::Quantified(_, x, b) => {
                let mut fv = b.fv();
                fv.shift_remove(x);
                fv
            }
        }
    }
}
impl std::ops::Not for BExpr {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
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
            Function::Fac(n) => Function::Fac(Box::new(n.subst_var(t, x))),
            Function::Fib(n) => Function::Fib(Box::new(n.subst_var(t, x))),
            Function::Exp(a, b) => {
                Function::Exp(Box::new(a.subst_var(t, x)), Box::new(b.subst_var(t, x)))
            }
        }
    }
}
