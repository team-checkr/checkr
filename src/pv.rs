use crate::ast::{AExpr, AOp, BExpr, Command, Commands, Guard, LogicOp, RelOp, Target};

impl Commands {
    pub fn wp(&self, q: &BExpr) -> BExpr {
        self.0.iter().rfold(q.clone(), |q, s| s.wp(&q))
    }
}

impl Command {
    pub fn wp(&self, q: &BExpr) -> BExpr {
        match self {
            Command::Assignment(var @ Target::Variable(_), exp) => BExpr::Logic(
                box q.subst_var(var, exp),
                LogicOp::And,
                box exp.well_defined(),
            ),
            // TODO
            Command::Assignment(Target::Array(_, _), _) => todo!(),
            Command::Skip => q.clone(),
            Command::If(guards) => guards
                .iter()
                .map(|g| g.wp(q))
                .reduce(|l, r| BExpr::Logic(box l, LogicOp::And, box r))
                .unwrap_or_else(|| panic!("if-statement had no guards")),
            // TODO
            Command::Loop(_) => q.clone(),
            // TODO
            Command::Break => q.clone(),
            Command::Continue => q.clone(),
        }
    }
}

impl Guard {
    pub fn wp(&self, q: &BExpr) -> BExpr {
        let a = BExpr::Logic(
            box BExpr::Not(box self.0.clone()),
            LogicOp::Or,
            box self.1.wp(q),
        );
        BExpr::Logic(box a, LogicOp::And, box self.0.well_defined())
    }
}

impl BExpr {
    fn well_defined(&self) -> BExpr {
        match self {
            BExpr::Bool(_) => BExpr::Bool(true),
            BExpr::Rel(l, _, r) => {
                BExpr::Logic(box l.well_defined(), LogicOp::And, box r.well_defined())
            }
            BExpr::Logic(l, _, r) => {
                BExpr::Logic(box l.well_defined(), LogicOp::And, box r.well_defined())
            }
            BExpr::Not(e) => BExpr::Not(box e.well_defined()),
        }
    }
    fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> BExpr {
        match self {
            BExpr::Bool(b) => BExpr::Bool(*b),
            BExpr::Rel(l, op, r) => BExpr::Rel(l.subst_var(t, x), *op, r.subst_var(t, x)),
            BExpr::Logic(l, op, r) => {
                BExpr::Logic(box l.subst_var(t, x), *op, box r.subst_var(t, x))
            }
            BExpr::Not(e) => BExpr::Not(box e.subst_var(t, x)),
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
                    (l, op, r) => BExpr::Logic(box l, op, box r),
                }
            }
            BExpr::Not(x) => {
                let x = x.simplify();
                match x {
                    BExpr::Bool(b) => BExpr::Bool(!b),
                    x => BExpr::Not(box x),
                }
            }
        }
    }
}

impl AExpr {
    fn subst_var<T>(&self, t: &Target<T>, x: &AExpr) -> AExpr {
        match self {
            AExpr::Number(n) => AExpr::Number(*n),
            AExpr::Reference(v) if v.same_name(t) => x.clone(),
            AExpr::Reference(v) => AExpr::Reference(v.clone()),
            AExpr::Binary(l, op, r) => {
                AExpr::Binary(box l.subst_var(t, x), *op, box r.subst_var(t, x))
            }
            AExpr::Minus(e) => AExpr::Minus(box e.subst_var(t, x)),
        }
    }

    fn well_defined(&self) -> BExpr {
        match self {
            AExpr::Number(_) => BExpr::Bool(true),
            AExpr::Reference(_) => BExpr::Bool(true),
            AExpr::Binary(l, op, r) => {
                let p = BExpr::Logic(box l.well_defined(), LogicOp::And, box r.well_defined());
                match op {
                    AOp::Plus | AOp::Minus | AOp::Times => p,
                    AOp::Divide => BExpr::Logic(
                        box BExpr::Rel(*r.clone(), RelOp::Ne, AExpr::Number(0)),
                        LogicOp::And,
                        box p,
                    ),
                    AOp::Pow => BExpr::Logic(
                        box BExpr::Rel(*r.clone(), RelOp::Ge, AExpr::Number(0)),
                        LogicOp::And,
                        box p,
                    ),
                }
            }
            AExpr::Minus(e) => e.well_defined(),
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
            AExpr::Binary(l, op, r) => AExpr::Binary(box l.simplify(), op, box r.simplify()),
            AExpr::Minus(box AExpr::Minus(e)) => e.simplify(),
            AExpr::Minus(e) => AExpr::Minus(box e.simplify()),
        }
    }
}

impl Target<Box<AExpr>> {
    pub fn simplify(&self) -> Self {
        match self {
            Target::Variable(v) => Target::Variable(v.clone()),
            Target::Array(arr, idx) => Target::Array(arr.clone(), box idx.simplify()),
        }
    }
}
