use smtlib::prelude::*;

use crate::ast::{AExpr, AOp, BExpr, Function, LogicOp, Quantifier, RelOp};

impl BExpr {
    pub fn smt<'st>(&self, st: &'st smtlib::Storage) -> smtlib::Bool<'st> {
        match self {
            BExpr::Bool(b) => smtlib::Bool::new(st, *b),
            BExpr::Not(b) => !b.smt(st),
            BExpr::Rel(lhs, op, rhs) => {
                let lhs = lhs.smt(st);
                let rhs = rhs.smt(st);
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
                let lhs = lhs.smt(st);
                let rhs = rhs.smt(st);
                match op {
                    LogicOp::And => lhs & rhs,
                    LogicOp::Land => lhs & rhs,
                    LogicOp::Or => lhs | rhs,
                    LogicOp::Lor => lhs | rhs,
                    LogicOp::Implies => lhs.implies(rhs),
                }
            }
            BExpr::Quantified(q, t, e) => {
                let v = smtlib::Int::new_const(st, t.name());
                match q {
                    Quantifier::Exists => smtlib::terms::exists(st, v, e.smt(st)),
                    Quantifier::Forall => smtlib::terms::forall(st, v, e.smt(st)),
                }
            }
        }
    }
}

impl AExpr {
    pub fn smt<'st>(&self, st: &'st smtlib::Storage) -> smtlib::Int<'st> {
        match self {
            AExpr::Number(n) => smtlib::Int::new(st, *n as i64),
            AExpr::Reference(v) => smtlib::Int::new_const(st, v.name()).into(),
            AExpr::Binary(lhs, op, rhs) => {
                let lhs = lhs.smt(st);
                let rhs = rhs.smt(st);
                match op {
                    AOp::Plus => lhs + rhs,
                    AOp::Minus => lhs - rhs,
                    AOp::Times => lhs * rhs,
                    AOp::Divide => lhs / rhs,
                }
            }
            AExpr::Minus(e) => -e.smt(st),
            AExpr::Function(f) => {
                let fun = f.smt(st);
                let args = f.args().map(|a| a.smt(st).into()).collect::<Vec<_>>();
                fun.call(&args).unwrap().as_int().unwrap()
            }
            AExpr::Old(e) => smtlib::Int::new_const(st, e.name()).into(),
        }
    }
}

impl Function {
    pub fn smt<'st>(&self, st: &'st smtlib::Storage) -> smtlib::funs::Fun<'st> {
        let vars = match self {
            Function::Division(_, _) => vec![smtlib::Int::sort(); 2],
            Function::Min(_, _) => vec![smtlib::Int::sort(); 2],
            Function::Max(_, _) => vec![smtlib::Int::sort(); 2],
            Function::Fac(_) => vec![smtlib::Int::sort(); 1],
            Function::Fib(_) => vec![smtlib::Int::sort(); 1],
            Function::Exp(_, _) => vec![smtlib::Int::sort(); 2],
        };
        smtlib::funs::Fun::new(st, self.name(), vars, smtlib::Int::sort())
    }
}
