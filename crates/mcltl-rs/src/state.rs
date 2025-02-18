use std::{fmt, hash::Hash};

pub trait State: Clone + PartialEq + Eq + Hash + fmt::Debug {
    fn initial() -> Self;
    fn is_initial(&self) -> bool {
        self == &Self::initial()
    }
    fn name(&self) -> String;
}

impl State for &str {
    fn initial() -> Self {
        "INIT"
    }

    fn name(&self) -> String {
        self.to_string()
    }
}

impl State for String {
    fn initial() -> Self {
        const INIT_NODE_ID: &str = "INIT";
        INIT_NODE_ID.to_string()
    }

    fn name(&self) -> String {
        self.clone()
    }
}

impl State for usize {
    fn initial() -> Self {
        0
    }

    fn name(&self) -> String {
        self.to_string()
    }
}

impl<A, B> State for (A, B)
where
    A: State,
    B: State,
{
    fn initial() -> Self {
        (A::initial(), B::initial())
    }

    fn name(&self) -> String {
        format!("{}#{}", self.0.name(), self.1.name())
    }
}
