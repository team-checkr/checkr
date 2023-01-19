use std::fmt::Display;

use itertools::Itertools;

use crate::ast::{AExpr, AOp, Array, BExpr, Command, Commands, Guard, LogicOp, RelOp, Variable};

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<Idx> Display for Array<Idx>
where
    Idx: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.0, self.1)
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Assignment(target, expr) => write!(f, "{target} := {expr}"),
            Command::ArrayAssignment(arr, expr) => write!(f, "{arr} := {expr}"),
            Command::If(guards) => write!(f, "if {}\nfi", guards.iter().format("\n[] ")),
            Command::Loop(guards) => write!(f, "do {}\nod", guards.iter().format("\n[] ")),
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
            AExpr::Variable(x) => write!(f, "{x}"),
            AExpr::Binary(l, op, r) => write!(f, "({l} {op} {r})"),
            AExpr::Array(a) => write!(f, "{a}"),
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
            LogicOp::And => write!(f, "&"),
            LogicOp::Land => write!(f, "&&"),
            LogicOp::Or => write!(f, "|"),
            LogicOp::Lor => write!(f, "||"),
        }
    }
}
