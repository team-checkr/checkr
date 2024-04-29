use std::fmt::Display;

use itertools::Itertools;

use crate::ast::{
    AExpr, AOp, Array, BExpr, Command, CommandKind, Commands, Function, Guard, Locator, LogicOp,
    PredicateBlock, PredicateChain, Quantifier, RelOp, Target, Variable,
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

impl<Prev: Display, Inv: Display> Display for Command<Prev, Inv> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pres = &self.pre;
        let posts = &self.post;
        write!(f, "{pres}\n{}\n{posts}", self.kind)
    }
}

impl<Prev: Display, Inv: Display> Display for CommandKind<Prev, Inv> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandKind::Assignment(target, expr) => write!(f, "{target} := {expr}"),
            CommandKind::Skip => write!(f, "skip"),
            CommandKind::If(guards) => write!(f, "if {}\nfi", guards.iter().format("\n[] ")),
            CommandKind::Loop(inv, guards) => {
                write!(f, "do[{inv}] {}\nod", guards.iter().format("\n[] "))
            }
        }
    }
}

impl<Prev: Display, Inv: Display> Display for Commands<Prev, Inv> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().format(" ;\n"))
    }
}

impl<Prev: Display, Inv: Display> Display for Guard<Prev, Inv> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ->\n{}",
            self.guard,
            self.cmds
                .to_string()
                .lines()
                .map(|l| format!("   {l}"))
                .format("\n")
        )
    }
}

impl Display for PredicateChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.predicates.iter().format("\n"))
    }
}

impl Display for PredicateBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.predicate)
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
            AOp::Divide => write!(f, "/"),
        }
    }
}
impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name(), self.args().format(", "))
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
impl Display for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Locator::Init => write!(f, "init"),
            Locator::Stuck => write!(f, "stuck"),
            Locator::Terminated => write!(f, "terminated"),
        }
    }
}
