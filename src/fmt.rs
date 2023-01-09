use std::fmt::Display;

use itertools::Itertools;

use crate::ast::{AExpr, AOp, Array, BExpr, Command, Guard, RelOp, Variable};

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.0, self.1)
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Assignment(target, expr) => write!(f, "{target} := {expr}"),
            Command::ArrayAssignment(arr, expr) => write!(f, "{arr} := {expr}"),
            Command::If(guards) => write!(
                f,
                "if {}\nfi",
                guards
                    .iter()
                    .map(|g| g
                        .to_string()
                        .lines()
                        .enumerate()
                        .map(|(idx, l)| if idx == 0 {
                            l.to_string()
                        } else {
                            format!("  {l}")
                        })
                        .join("\n"))
                    .format("\n[] ")
            ),
            Command::Loop(guards) => write!(
                f,
                "do {}\nod",
                guards
                    .iter()
                    .map(|g| g
                        .to_string()
                        .lines()
                        .enumerate()
                        .map(|(idx, l)| if idx == 0 {
                            l.to_string()
                        } else {
                            format!("  {l}")
                        })
                        .join("\n"))
                    .format("\n[] ")
            ),
            Command::Break => write!(f, "break"),
            Command::Continue => write!(f, "continue"),
            Command::Skip => write!(f, "skip"),
        }
    }
}

pub fn fmt_commands(cmds: &[Command]) -> String {
    cmds.iter()
        .map(|l| l.to_string().lines().map(|l| format!("   {l}")).join("\n"))
        .format(" ;\n")
        .to_string()
}

impl Display for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ->\n{}", self.0, fmt_commands(&self.1))
    }
}

impl Display for AExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AExpr::Number(n) => write!(f, "{n}"),
            AExpr::Variable(x) => write!(f, "{x}"),
            AExpr::Binary(l, op, r) => write!(f, "{l} {op} {r}"),
            AExpr::Array(a) => write!(f, "{a}"),
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
        }
    }
}
impl Display for BExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BExpr::Bool(b) => write!(f, "{b}"),
            BExpr::Rel(l, op, r) => write!(f, "{l} {op} {r}"),
            BExpr::And(l, r) => write!(f, "{l} ∧ {r}"),
            BExpr::Land(l, r) => write!(f, "{l} && {r}"),
            BExpr::Not(b) => write!(f, "¬({b})"),
        }
    }
}
impl Display for RelOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelOp::Eq => write!(f, "="),
            RelOp::Gt => write!(f, ">"),
            RelOp::Ge => write!(f, ">="),
        }
    }
}
