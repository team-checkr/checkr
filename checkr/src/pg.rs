use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::atomic::AtomicU64,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::ast::{AExpr, BExpr, Command, Commands, Guard, LogicOp, Target};

#[derive(Debug, Clone)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Node {
    Start,
    Node(NodeId),
    End,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Start => write!(f, "qStart"),
            Node::Node(n) => write!(f, "q{}", n.0),
            Node::End => write!(f, "qFinal"),
        }
    }
}
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Start => write!(f, "q▷"),
            Node::Node(n) => write!(
                f,
                "q{}",
                n.0,
                // n.0.to_string()
                //     .chars()
                //     .map(|c| match c {
                //         '0' => '₀',
                //         '1' => '₁',
                //         '2' => '₂',
                //         '3' => '₃',
                //         '4' => '₄',
                //         '5' => '₅',
                //         '6' => '₆',
                //         '7' => '₇',
                //         '8' => '₈',
                //         '9' => '₉',
                //         c => c,
                //     })
                //     .format("")
            ),
            Node::End => write!(f, "q◀"),
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
    Assignment(Target<Box<AExpr>>, AExpr),
    Skip,
    Condition(BExpr),
}
impl Action {
    fn fv(&self) -> HashSet<Target> {
        match self {
            Action::Assignment(x, a) => x.fv().union(&a.fv()).cloned().collect(),
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

/// Computes the edges and the condition which is true iff all guards are false
fn guard_edges(det: Determinism, guards: &[Guard], s: Node, t: Node) -> (Vec<Edge>, BExpr) {
    match det {
        Determinism::Deterministic => {
            // See the "if" and "do" Commands on Page 25 of Formal Methods
            let mut prev = BExpr::Bool(false);

            let mut edges = vec![];

            for Guard(b, c) in guards {
                let q = Node::fresh();

                edges.push(Edge(
                    s,
                    Action::Condition(BExpr::logic(
                        b.clone(),
                        LogicOp::Land,
                        BExpr::Not(Box::new(prev.clone())),
                    )),
                    q,
                ));
                edges.extend(c.edges(det, q, t));
                prev = BExpr::logic(b.to_owned().clone(), LogicOp::Lor, prev);
            }

            // Wraps in "not" so that the "d" part can be used directly by "do"
            (edges, BExpr::Not(Box::new(prev)))
        }
        Determinism::NonDeterministic => {
            let e = guards
                .iter()
                .flat_map(|Guard(b, c)| {
                    let q = Node::fresh();
                    let mut edges = c.edges(det, q, t);
                    edges.push(Edge(s, Action::Condition(b.clone()), q));
                    edges
                })
                .collect();
            (e, done(guards))
        }
    }
}

impl Command {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<Edge> {
        match self {
            Command::Assignment(v, expr) => {
                vec![Edge(s, Action::Assignment(v.clone(), expr.clone()), t)]
            }
            Command::Skip => vec![Edge(s, Action::Skip, t)],
            Command::If(guards) => guard_edges(det, guards, s, t).0,
            Command::Loop(guards) | Command::EnrichedLoop(_, guards) => {
                let (mut edges, b) = guard_edges(det, guards, s, s);
                edges.push(Edge(s, Action::Condition(b), t));
                edges
            }
            Command::Annotated(_, c, _) => c.edges(det, s, t),
            Command::Break => todo!(),
            Command::Continue => todo!(),
        }
    }
}

fn done(guards: &[Guard]) -> BExpr {
    guards
        .iter()
        .map(|Guard(b, _c)| BExpr::Not(Box::new(b.clone())))
        .reduce(|a, b| BExpr::logic(a, LogicOp::Land, b))
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
        .rename_with_reverse_post_order()
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

    pub fn fv(&self) -> HashSet<Target> {
        self.edges.iter().flat_map(|e| e.action().fv()).collect()
    }

    pub fn dot(&self) -> String {
        format!(
            "digraph G {{\n{}\n}}",
            self.edges
                .iter()
                .map(|e| format!(
                    "  {:?}[label=\"{}\"]; {:?} -> {:?}[label={:?}]; {:?}[label=\"{}\"];",
                    e.0,
                    e.0,
                    e.0,
                    e.2,
                    e.1.to_string(),
                    e.2,
                    e.2,
                ))
                .format("  \n")
        )
    }

    pub fn as_petgraph(
        &self,
    ) -> (
        petgraph::Graph<Node, Action>,
        BTreeMap<Node, petgraph::graph::NodeIndex>,
        BTreeMap<petgraph::graph::NodeIndex, Node>,
    ) {
        let mut g = petgraph::Graph::new();

        let node_mapping: BTreeMap<Node, petgraph::graph::NodeIndex> = self
            .nodes
            .iter()
            .copied()
            .map(|n| (n, g.add_node(n)))
            .collect();
        let node_mapping_rev: BTreeMap<petgraph::graph::NodeIndex, Node> =
            node_mapping.iter().map(|(a, b)| (*b, *a)).collect();

        for Edge(from, action, to) in &self.edges {
            g.add_edge(node_mapping[from], node_mapping[to], action.clone());
        }

        (g, node_mapping, node_mapping_rev)
    }

    pub fn rename_with_reverse_post_order(&self) -> Self {
        let (g, node_mapping, node_mapping_rev) = self.as_petgraph();

        let initial_node = if let Some(n) = node_mapping.get(&Node::Start) {
            *n
        } else {
            warn!("graph did not have a start node");
            return self.clone();
        };
        let mut dfs = petgraph::visit::DfsPostOrder::new(&g, initial_node);

        let mut new_order = VecDeque::new();

        while let Some(n) = dfs.next(&g) {
            new_order.push_front(node_mapping_rev[&n]);
        }

        let mut node_mapping_new: BTreeMap<Node, Node> = Default::default();

        enum NamingStage {
            Start,
            Middle { idx: u64 },
        }

        let mut stage = NamingStage::Start;
        for n in new_order.iter() {
            stage = match stage {
                NamingStage::Start => {
                    node_mapping_new.insert(*n, Node::Start);
                    NamingStage::Middle { idx: 1 }
                }
                NamingStage::Middle { idx } => match n {
                    Node::Start => todo!(),
                    Node::Node(_) => {
                        node_mapping_new.insert(*n, Node::Node(NodeId(idx)));
                        NamingStage::Middle { idx: idx + 1 }
                    }
                    Node::End => {
                        node_mapping_new.insert(*n, Node::End);
                        NamingStage::Middle { idx }
                    }
                },
            }
        }

        Self {
            edges: self
                .edges
                .iter()
                .map(|Edge(a, action, b)| {
                    Edge(node_mapping_new[a], action.clone(), node_mapping_new[b])
                })
                .collect(),
            nodes: node_mapping_new.values().copied().collect(),
            outgoing: self
                .outgoing
                .iter()
                .map(|(n, outgoing)| {
                    (
                        node_mapping_new[n],
                        outgoing
                            .iter()
                            .map(|Edge(a, action, b)| {
                                Edge(node_mapping_new[a], action.clone(), node_mapping_new[b])
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}
