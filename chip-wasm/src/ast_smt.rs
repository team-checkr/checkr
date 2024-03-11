use smtlib::Sort;

use crate::ast::{AExpr, AOp, BExpr, LogicOp, RelOp};

impl BExpr {
    pub fn smt(&self) -> smtlib::Bool {
        match self {
            BExpr::Bool(b) => smtlib::Bool::from(*b),
            BExpr::Not(b) => !b.smt(),
            BExpr::Rel(lhs, op, rhs) => {
                let lhs = lhs.smt();
                let rhs = rhs.smt();
                match op {
                    RelOp::Eq => lhs._eq(rhs),
                    RelOp::Ne => lhs._neq(rhs),
                    RelOp::Lt => lhs.lt(rhs),
                    RelOp::Le => lhs.le(rhs),
                    RelOp::Gt => lhs.gt(rhs),
                    RelOp::Ge => lhs.ge(rhs),
                }
            }
            BExpr::Logic(lhs, op, rhs) => {
                let lhs = lhs.smt();
                let rhs = rhs.smt();
                match op {
                    LogicOp::And => lhs & rhs,
                    LogicOp::Land => lhs & rhs,
                    LogicOp::Or => lhs | rhs,
                    LogicOp::Lor => lhs | rhs,
                    LogicOp::Implies => lhs.implies(rhs),
                }
            }
            BExpr::Quantified(_, _, _) => todo!(),
        }
    }
}

impl AExpr {
    pub fn smt(&self) -> smtlib::Int {
        match self {
            AExpr::Number(n) => smtlib::Int::from(*n as i64),
            AExpr::Reference(v) => smtlib::Int::from_name(v.name()).into(),
            AExpr::Binary(lhs, op, rhs) => {
                let lhs = lhs.smt();
                let rhs = rhs.smt();
                match op {
                    AOp::Plus => lhs + rhs,
                    AOp::Minus => lhs - rhs,
                    AOp::Times => lhs * rhs,
                    AOp::Divide => lhs / rhs,
                }
            }
            AExpr::Minus(e) => -e.smt(),
            AExpr::Function(f) => todo!("{f:?}"),
        }
    }
}
