use std::sync::atomic::AtomicU64;

use gcl::ast::{
    AExpr, BExpr, Command, Commands, Guard, LogicOp, Quantifier, RelOp, Target, Variable,
};

pub trait StrongestPostcondition {
    fn sp(&self, p: &BExpr) -> BExpr;
    fn vc(&self, r: &BExpr) -> Vec<BExpr>;
}

impl StrongestPostcondition for Commands {
    fn sp(&self, p: &BExpr) -> BExpr {
        self.0.iter().fold(p.clone(), |acc, c| c.sp(&acc))
    }
    fn vc(&self, r: &BExpr) -> Vec<BExpr> {
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

pub struct StrongestPostconditionCounter;
pub const STRONGEST_POSTCONDITION_COUNTER: StrongestPostconditionCounter =
    StrongestPostconditionCounter;

static FRESH_ID: AtomicU64 = AtomicU64::new(0);
impl StrongestPostconditionCounter {
    pub fn reset_sp_counter() {
        FRESH_ID.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl StrongestPostcondition for Command {
    fn sp(&self, p: &BExpr) -> BExpr {
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
    fn vc(&self, r: &BExpr) -> Vec<BExpr> {
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

impl StrongestPostcondition for Guard {
    fn sp(&self, p: &BExpr) -> BExpr {
        self.1
            .sp(&BExpr::logic(self.0.clone(), LogicOp::Land, p.clone()))
    }
    fn vc(&self, r: &BExpr) -> Vec<BExpr> {
        self.1
            .vc(&BExpr::logic(self.0.clone(), LogicOp::Land, r.clone()))
    }
}
