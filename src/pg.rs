use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicU64,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::ast::{AExpr, Array, BExpr, Command, Commands, Guard, LogicOp, Variable};

pub struct ProgramGraph {
    edges: Vec<Edge>,
    nodes: HashSet<Node>,
    outgoing: HashMap<Node, Vec<Edge>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "Case")]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Node {
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

static NODE_ID: AtomicU64 = AtomicU64::new(0);
impl Node {
    fn fresh() -> Node {
        Node::Node(NodeId(
            NODE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ))
    }
    fn reset() {
        NODE_ID.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Assignment(Variable, AExpr),
    ArrayAssignment(String, AExpr, AExpr),
    Skip,
    Condition(BExpr),
}
impl Action {
    fn fv(&self) -> HashSet<Variable> {
        match self {
            Action::Assignment(x, a) => [x.clone()].into_iter().chain(a.fv()).collect(),
            // TODO
            Action::ArrayAssignment(_, _, a) => a.fv(),
            Action::Skip => Default::default(),
            Action::Condition(b) => b.fv(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge(pub Node, pub Action, pub Node);

impl Edge {
    pub fn action(&self) -> &Action {
        &self.1
    }

    pub fn from(&self) -> Node {
        self.0
    }
    pub fn to(&self) -> Node {
        self.2
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Assignment(v, x) => write!(f, "{v} := {x}"),
            Action::ArrayAssignment(arr, idx, x) => write!(f, "{arr}[{idx}] := {x}"),
            Action::Skip => write!(f, "skip"),
            Action::Condition(b) => write!(f, "{b}"),
        }
    }
}

impl Commands {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<Edge> {
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

fn guard_edges(det: Determinism, guards: &[Guard], s: Node, t: Node) -> Vec<Edge> {
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

                edges.push(Edge(s, Action::Condition(cond), q));
            }

            edges
        }
        Determinism::NonDeterministic => guards
            .iter()
            .flat_map(|g| {
                let q = Node::fresh();
                let mut edges = g.1.edges(det, q, t);
                edges.push(Edge(s, Action::Condition(g.0.clone()), q));
                edges
            })
            .collect(),
    }
}

impl Command {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<Edge> {
        match self {
            Command::Assignment(v, expr) => {
                vec![Edge(s, Action::Assignment(v.clone(), expr.clone()), t)]
            }
            Command::Skip => vec![Edge(s, Action::Skip, t)],
            Command::If(guards) => guard_edges(det, guards, s, t),
            Command::Loop(guards) => {
                let b = done(guards);
                let mut edges = guard_edges(det, guards, s, s);
                edges.push(Edge(s, Action::Condition(b), t));
                edges
            }
            Command::ArrayAssignment(Array(arr, idx), expr) => {
                vec![Edge(
                    s,
                    Action::ArrayAssignment(arr.clone(), *idx.clone(), expr.clone()),
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
        Node::reset();
        let edges = cmds.edges(det, Node::Start, Node::End);
        let mut outgoing: HashMap<Node, Vec<Edge>> = HashMap::new();
        let mut nodes: HashSet<Node> = Default::default();

        for e in &edges {
            outgoing.entry(e.0).or_default().push(e.clone());
            nodes.insert(e.0);
            nodes.insert(e.2);
        }

        Self {
            outgoing,
            edges,
            nodes,
        }
    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn nodes(&self) -> &HashSet<Node> {
        &self.nodes
    }
    pub fn outgoing(&self, node: Node) -> &[Edge] {
        self.outgoing
            .get(&node)
            .map(|s| s.as_slice())
            .unwrap_or_default()
    }

    pub fn fv(&self) -> HashSet<Variable> {
        self.edges.iter().flat_map(|e| e.action().fv()).collect()
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
