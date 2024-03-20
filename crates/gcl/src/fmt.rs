use std::fmt::{Debug, Display};

use itertools::Itertools;

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, Commands, Flow, Guard, LogicOp, RelOp, SecurityClass,
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
            Self::Variable(v) => Display::fmt(v, f),
            Self::Array(a, idx) => write!(f, "{a}[{idx}]"),
        }
    }
}
impl std::fmt::Display for Target<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Variable(v) => Display::fmt(v, f),
            Self::Array(a, ()) => Display::fmt(a, f),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Assignment(target, expr) => write!(f, "{target} := {expr}"),
            Command::Skip => write!(f, "skip"),
            Command::If(guards) => write!(f, "if {}\nfi", guards.iter().format("\n[] ")),
            Command::Loop(guards) => write!(f, "do {}\nod", guards.iter().format("\n[] ")),
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
impl Display for BExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BExpr::Bool(b) => write!(f, "{b}"),
            BExpr::Rel(l, op, r) => write!(f, "({l} {op} {r})"),
            BExpr::Logic(l, op, r) => write!(f, "({l} {op} {r})"),
            BExpr::Not(b) => write!(f, "!{b}"),
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
        }
    }
}

impl<T> Debug for Flow<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Flow({:?} -> {:?})", self.from, self.into)
    }
}
impl<T> Display for Flow<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.from, self.into)
    }
}

impl Debug for SecurityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecurityClass({})", self.0)
    }
}
impl Display for SecurityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
