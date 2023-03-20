use std::fmt::Display;

use itertools::Itertools;

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Function, Guard, LogicOp, Quantifier, RelOp,
    Target, Variable,
};

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Target<Box<AExpr>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => v.fmt(f),
            Self::Array(a, idx) => write!(f, "{a}[{idx}]"),
        }
    }
}
impl std::fmt::Display for Target<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => v.fmt(f),
            Self::Array(a, ()) => a.fmt(f),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Assignment(target, expr) => write!(f, "{target} := {expr}"),
            Command::If(guards) => write!(f, "if {}\nfi", guards.iter().format("\n[] ")),
            Command::Loop(guards) => write!(f, "do {}\nod", guards.iter().format("\n[] ")),
            Command::EnrichedLoop(pred, guards) => {
                write!(f, "do {{{pred}}}\n   {}\nod", guards.iter().format("\n[] "))
            }
            Command::Annotated(p, c, q) => write!(f, "{{{p}}}\n{c}\n{{{q}}}"),
            Command::Break => write!(f, "break"),
            Command::Continue => write!(f, "continue"),
            Command::Skip => write!(f, "skip"),
        }
    }
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().format(" ;\n"))
    }
}

impl Display for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ->\n{}",
            self.0,
            self.1
                .to_string()
                .lines()
                .map(|l| format!("   {l}"))
                .format("\n")
        )
    }
}

impl Display for AExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AExpr::Number(n) => write!(f, "{n}"),
            AExpr::Reference(x) => write!(f, "{x}"),
            AExpr::Binary(l, op, r) => write!(f, "({l} {op} {r})"),
            AExpr::Minus(m) => write!(f, "-{m}"),
            AExpr::Function(fun) => write!(f, "{fun}"),
        }
    }
}
impl Display for AOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AOp::Plus => write!(f, "+"),
            AOp::Minus => write!(f, "-"),
            AOp::Times => write!(f, "*"),
            AOp::Pow => write!(f, "^"),
            AOp::Divide => write!(f, "/"),
        }
    }
}
impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Division(a, b) => write!(f, "division({a}, {b})"),
            Function::Min(a, b) => write!(f, "min({a}, {b})"),
            Function::Max(a, b) => write!(f, "max({a}, {b})"),
            Function::Count(a, b) => write!(f, "count({a}, {b})"),
            Function::LogicalCount(a, b) => write!(f, "count({a}, {b})"),
            Function::Length(x) => write!(f, "length({x})"),
            Function::LogicalLength(x) => write!(f, "length({x})"),
            Function::Fac(x) => write!(f, "fac({x})"),
            Function::Fib(x) => write!(f, "fib({x})"),
        }
    }
}
impl Display for BExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BExpr::Bool(b) => write!(f, "{b}"),
            BExpr::Rel(l, op, r) => write!(f, "({l} {op} {r})"),
            BExpr::Logic(l, op, r) => write!(f, "({l} {op} {r})"),
            BExpr::Not(b) => write!(f, "!{b}"),
            BExpr::Quantified(q, x, b) => write!(f, "({q} {x} :: {b})"),
        }
    }
}
impl Display for Quantifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quantifier::Exists => write!(f, "exists"),
            Quantifier::Forall => write!(f, "forall"),
        }
    }
}
impl Display for RelOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelOp::Eq => write!(f, "="),
            RelOp::Gt => write!(f, ">"),
            RelOp::Ge => write!(f, ">="),
            RelOp::Ne => write!(f, "!="),
            RelOp::Lt => write!(f, "<"),
            RelOp::Le => write!(f, "<="),
        }
    }
}
impl Display for LogicOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicOp::And => write!(f, "&&"),
            LogicOp::Land => write!(f, "&"),
            LogicOp::Or => write!(f, "||"),
            LogicOp::Lor => write!(f, "|"),
            LogicOp::Implies => write!(f, "==>"),
        }
    }
}
