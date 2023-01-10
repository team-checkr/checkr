use std::sync::atomic::AtomicU64;

use itertools::Itertools;

use crate::ast::{AExpr, Array, BExpr, Command, Commands, Guard, LogicOp};

pub struct ProgramGraph {
    edges: Vec<(Node, Edge, Node)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct NodeId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Node {
    Start,
    Node(NodeId),
    End,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Start => write!(f, "qStart"),
            Node::Node(n) => write!(f, "q{}", n.0),
            Node::End => write!(f, "qFinal"),
        }
    }
}

impl Node {
    fn fresh() -> Node {
        static NODE_ID: AtomicU64 = AtomicU64::new(0);
        Node::Node(NodeId(
            NODE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Edge {
    Assignment(String, AExpr),
    ArrayAssignment(String, AExpr, AExpr),
    Skip,
    Condition(BExpr),
}

impl std::fmt::Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edge::Assignment(v, x) => write!(f, "{v} := {x}"),
            Edge::ArrayAssignment(arr, idx, x) => write!(f, "{arr}[{idx}] := {x}"),
            Edge::Skip => write!(f, "skip"),
            Edge::Condition(b) => write!(f, "{b}"),
        }
    }
}

impl Commands {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<(Node, Edge, Node)> {
        let mut edges = vec![];

        let mut prev = s;
        for (idx, cmd) in self.0.iter().enumerate() {
            let is_last = idx + 1 == self.0.len();
            let next = if is_last { t } else { Node::fresh() };
            edges.extend(cmd.edges(det, prev, next));
            prev = next;
        }

        edges
    }
}

fn guard_edges(det: Determinism, guards: &[Guard], s: Node, t: Node) -> Vec<(Node, Edge, Node)> {
    match det {
        Determinism::Deterministic => {
            let mut prev: Option<BExpr> = None;

            let mut edges = vec![];

            for g in guards {
                let q = Node::fresh();
                edges.extend(g.1.edges(det, q, t));

                let cond = if let Some(p) = prev {
                    prev = Some(BExpr::Logic(
                        box p.clone(),
                        LogicOp::Or,
                        box g.0.to_owned().clone(),
                    ));
                    BExpr::Logic(box BExpr::Not(box p), LogicOp::And, box g.0.clone())
                } else {
                    prev = Some(g.0.clone());
                    g.0.clone()
                };

                edges.push((s, Edge::Condition(cond), q));
            }

            edges
        }
        Determinism::NonDeterministic => guards
            .iter()
            .flat_map(|g| {
                let q = Node::fresh();
                let mut edges = g.1.edges(det, q, t);
                edges.push((s, Edge::Condition(g.0.clone()), q));
                edges
            })
            .collect(),
    }
}

impl Command {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<(Node, Edge, Node)> {
        match self {
            Command::Assignment(v, expr) => {
                vec![(s, Edge::Assignment(v.0.clone(), expr.clone()), t)]
            }
            Command::Skip => vec![(s, Edge::Skip, t)],
            Command::If(guards) => guard_edges(det, guards, s, t),
            Command::Loop(guards) => {
                let b = done(guards);
                let mut edges = guard_edges(det, guards, s, s);
                edges.push((s, Edge::Condition(b), t));
                edges
            }
            Command::ArrayAssignment(Array(arr, idx), expr) => {
                vec![(
                    s,
                    Edge::ArrayAssignment(arr.clone(), *idx.clone(), expr.clone()),
                    t,
                )]
            }
            Command::Break => todo!(),
            Command::Continue => todo!(),
        }
    }
}

fn done(guards: &[Guard]) -> BExpr {
    guards
        .iter()
        .map(|g| BExpr::Not(box g.0.clone()))
        .reduce(|a, b| BExpr::Logic(box a, LogicOp::And, box b))
        .unwrap_or(BExpr::Bool(true))
}

impl ProgramGraph {
    pub fn new(det: Determinism, cmds: &Commands) -> Self {
        Self {
            edges: cmds.edges(det, Node::Start, Node::End),
        }
    }

    pub fn dot(&self) -> String {
        format!(
            "digraph G {{\n{}\n}}",
            self.edges
                .iter()
                .map(|e| format!(
                    "  {}; {} -> {}[label={:?}];",
                    e.0,
                    e.0,
                    e.2,
                    e.1.to_string()
                ))
                .format("  \n")
        )
    }
}
