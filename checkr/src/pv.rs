use std::sync::atomic::AtomicU64;

use crate::ast::{
    AExpr, BExpr, Command, Commands, Function, Guard, LogicOp, Quantifier, RelOp, Target, Variable,
};

impl Commands {
    pub fn sp(&self, p: &BExpr) -> BExpr {
        self.0.iter().fold(p.clone(), |acc, c| c.sp(&acc))
    }
    pub fn vc(&self, r: &BExpr) -> Vec<BExpr> {
        self.0
            .iter()
            .scan(r.clone(), |acc, c| {
                let vc = c.vc(acc);

                *acc = c.sp(acc);

                Some(vc)
            })
            .flatten()
            .collect()
    }
}

static FRESH_ID: AtomicU64 = AtomicU64::new(0);
impl Command {
    pub fn reset_sp_counter() {
        FRESH_ID.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn sp(&self, p: &BExpr) -> BExpr {
        match self {
            Command::Assignment(x, e) => {
                fn fresh() -> Target<Box<AExpr>> {
                    Target::Variable(Variable(format!(
                        "_fresh_{}",
                        FRESH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    )))
                }

                let y = fresh();
                let y_expr = AExpr::Reference(y.clone());

                BExpr::Quantified(
                    Quantifier::Exists,
                    y.unit(),
                    Box::new(BExpr::logic(
                        p.subst_var(x, &y_expr),
                        LogicOp::Land,
                        BExpr::rel(
                            AExpr::Reference(x.clone()),
                            RelOp::Eq,
                            e.subst_var(x, &y_expr),
                        ),
                    )),
                )
            }
            Command::Skip => p.clone(),
            Command::If(guards) => guards_sp(guards, p),
            Command::Loop(guards) => guards
                .iter()
                .map(|gc| BExpr::Not(gc.0.clone().into()))
                .reduce(|a, b| BExpr::logic(a, LogicOp::Land, b))
                .unwrap(),
            Command::EnrichedLoop(i, guards) => {
                let done = guards
                    .iter()
                    .map(|gc| BExpr::Not(gc.0.clone().into()))
                    .reduce(|a, b| BExpr::logic(a, LogicOp::Land, b))
                    .unwrap();
                BExpr::logic(i.clone(), LogicOp::Land, done)
            }
            // TODO: Does this even make sense? It should never be called anyway
            Command::Annotated(_, _, q) => q.clone(),
            Command::Break => todo!(),
            Command::Continue => todo!(),
        }
    }
    pub fn vc(&self, r: &BExpr) -> Vec<BExpr> {
        match self {
            Command::Assignment(_, _) => vec![],
            Command::Skip => vec![],
            Command::If(guards) => guards_vc(guards, r),
            // TODO: Could we make something more useful/obvious here?
            Command::Loop(_) => vec![],
            Command::EnrichedLoop(i, guards) => {
                let mut conditions = vec![
                    BExpr::logic(r.clone(), LogicOp::Implies, i.clone()),
                    BExpr::logic(guards_sp(guards, i), LogicOp::Implies, i.clone()),
                ];

                conditions.extend_from_slice(&guards_vc(guards, i));

                conditions
            }
            Command::Annotated(p, c, q) => {
                let mut conditions = vec![BExpr::logic(c.sp(p), LogicOp::Implies, q.clone())];

                conditions.extend_from_slice(&c.vc(p));

                conditions
            }
            Command::Break => todo!(),
            Command::Continue => todo!(),
        }
    }
}
fn guards_sp(guards: &[Guard], p: &BExpr) -> BExpr {
    guards
        .iter()
        .map(|gc| gc.sp(p))
        .reduce(|a, b| BExpr::logic(a, LogicOp::Lor, b))
        .unwrap()
}
fn guards_vc(guards: &[Guard], r: &BExpr) -> Vec<BExpr> {
    guards.iter().flat_map(|gc| gc.vc(r)).collect()
}

impl Guard {
    pub fn sp(&self, p: &BExpr) -> BExpr {
        self.1
            .sp(&BExpr::logic(self.0.clone(), LogicOp::Land, p.clone()))
    }
    pub fn vc(&self, r: &BExpr) -> Vec<BExpr> {
        self.1
            .vc(&BExpr::logic(self.0.clone(), LogicOp::Land, r.clone()))
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
            .semantics(&Default::default())
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
    fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> AExpr {
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
            .semantics(&Default::default())
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
    fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> Function {
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
