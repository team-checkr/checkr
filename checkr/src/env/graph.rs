use std::collections::HashMap;

use graphviz_rust::dot_structures::{Attribute, Id};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    ast::Commands,
    generation::Generate,
    pg::{Determinism, ProgramGraph},
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown};

#[derive(Debug)]
pub struct GraphEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEnvInput {
    pub determinism: Determinism,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEnvOutput {
    pub dot: String,
}

impl Generate for GraphEnvInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, _rng: &mut R) -> Self {
        Self {
            // TODO
            determinism: Determinism::Deterministic,
        }
    }
}

impl ToMarkdown for GraphEnvInput {
    fn to_markdown(&self) -> Markdown {
        format!("**Determinism:** {:?}", self.determinism).into()
    }
}
impl ToMarkdown for GraphEnvOutput {
    fn to_markdown(&self) -> Markdown {
        format!("\n\n```dot\n{}\n```\n\n", self.dot).into()
    }
}

impl Environment for GraphEnv {
    type Input = GraphEnvInput;

    type Output = GraphEnvOutput;

    const ANALYSIS: Analysis = Analysis::Graph;

    fn run(
        &self,
        cmds: &crate::ast::Commands,
        input: &Self::Input,
    ) -> Result<Self::Output, EnvError> {
        let pg = ProgramGraph::new(input.determinism, cmds);
        Ok(GraphEnvOutput { dot: pg.dot() })
    }

    fn validate(
        &self,
        cmds: &crate::ast::Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> Result<super::ValidationResult, EnvError> {
        let reference = self.run(cmds, input)?;

        let (a_data, _a_node_mapping, a_graph) = dot_to_petgraph(&reference.dot);
        let (b_data, _b_node_mapping, b_graph) = dot_to_petgraph(&output.dot);

        eprintln!("{a_graph:?}");
        eprintln!("{b_graph:?}");

        let a_graph = &a_graph;
        let b_graph = &b_graph;
        // let mut binding_1 = |_a: &String, _b: &String| a == b;
        let mut binding_1 = |_a: &String, _b: &String| true;
        // let mut binding_2 = |_a: &String, _b: &String| a == b;
        let mut binding_2 = |_a: &String, _b: &String| true;
        let res = petgraph::algo::isomorphism::subgraph_isomorphisms_iter(
            &a_graph,
            &b_graph,
            &mut binding_1,
            &mut binding_2,
        )
        .unwrap();

        let res = res.collect_vec();

        for matching in res {
            for (a_idx, b_idx) in matching.iter().copied().enumerate() {
                let a = &a_data[&a_graph.raw_nodes()[a_idx].weight];
                let b = &b_data[&b_graph.raw_nodes()[b_idx].weight];
                eprintln!("{a:?} == {b:?}");

                // a_graph.find_edge(a, b)

                let a_edges = a_graph
                    .edges_directed(
                        petgraph::graph::NodeIndex::new(a_idx),
                        petgraph::Direction::Outgoing,
                    )
                    .map(|e| e.weight())
                    .collect_vec();
                let b_edges = b_graph
                    .edges_directed(
                        petgraph::graph::NodeIndex::new(b_idx),
                        petgraph::Direction::Outgoing,
                    )
                    .map(|e| e.weight())
                    .collect_vec();
                eprintln!("{a_edges:?} == {b_edges:?}");
            }
        }

        // println!(
        //     "{}",
        //     petgraph::dot::Dot::with_config(a_graph, &[petgraph::dot::Config::EdgeNoLabel])
        // );
        // println!(
        //     "{}",
        //     petgraph::dot::Dot::with_config(b_graph, &[petgraph::dot::Config::EdgeNoLabel])
        // );

        todo!("MADE IT TO THE END!");
    }
}

#[derive(Debug, Default)]
struct Node {
    attributes: Vec<Attribute>,
    outgoing: Vec<String>,
    ingoing: Vec<String>,
}
fn dot_to_petgraph(
    dot: &str,
) -> (
    HashMap<String, Node>,
    HashMap<String, petgraph::graph::NodeIndex>,
    petgraph::Graph<String, String>,
) {
    let mut nodes = HashMap::<String, Node>::new();
    let mut node_mapping = HashMap::<String, petgraph::graph::NodeIndex>::new();
    let mut graph = petgraph::Graph::<String, String>::new();

    let parsed = graphviz_rust::parse(dot).unwrap();

    match parsed {
        graphviz_rust::dot_structures::Graph::Graph { .. } => todo!(),
        graphviz_rust::dot_structures::Graph::DiGraph { stmts, .. } => {
            for stmt in stmts {
                match stmt {
                    graphviz_rust::dot_structures::Stmt::Node(n) => {
                        node_mapping
                            .entry(n.id.0.to_string())
                            .or_insert_with_key(|k| graph.add_node(k.to_string()));

                        nodes
                            .entry(n.id.0.to_string())
                            .or_default()
                            .attributes
                            .extend_from_slice(&n.attributes);
                    }
                    graphviz_rust::dot_structures::Stmt::Subgraph(_) => todo!(),
                    graphviz_rust::dot_structures::Stmt::Attribute(_) => todo!(),
                    graphviz_rust::dot_structures::Stmt::GAttribute(_) => todo!(),
                    graphviz_rust::dot_structures::Stmt::Edge(e) => match e.ty {
                        graphviz_rust::dot_structures::EdgeTy::Pair(a, b) => match (a, b) {
                            (
                                graphviz_rust::dot_structures::Vertex::N(a),
                                graphviz_rust::dot_structures::Vertex::N(b),
                            ) => {
                                let a_id = *node_mapping
                                    .entry(a.0.to_string())
                                    .or_insert_with_key(|k| graph.add_node(k.to_string()));
                                let b_id = *node_mapping
                                    .entry(b.0.to_string())
                                    .or_insert_with_key(|k| graph.add_node(k.to_string()));
                                graph.add_edge(
                                    a_id,
                                    b_id,
                                    e.attributes
                                        .iter()
                                        .find_map(|a| match (&a.0, &a.1) {
                                            (Id::Plain(l), Id::Escaped(v)) if l == "label" => {
                                                Some(v.to_string())
                                            }
                                            (Id::Html(_), Id::Html(_)) => todo!(),
                                            (Id::Html(_), Id::Escaped(_)) => todo!(),
                                            (Id::Html(_), Id::Plain(_)) => todo!(),
                                            (Id::Html(_), Id::Anonymous(_)) => todo!(),
                                            (Id::Escaped(_), Id::Html(_)) => todo!(),
                                            (Id::Escaped(_), Id::Escaped(_)) => todo!(),
                                            (Id::Escaped(_), Id::Plain(_)) => todo!(),
                                            (Id::Escaped(_), Id::Anonymous(_)) => todo!(),
                                            (Id::Plain(_), Id::Html(_)) => todo!(),
                                            (Id::Plain(_), Id::Escaped(_)) => todo!(),
                                            (Id::Plain(_), Id::Plain(_)) => todo!(),
                                            (Id::Plain(_), Id::Anonymous(_)) => todo!(),
                                            (Id::Anonymous(_), Id::Html(_)) => todo!(),
                                            (Id::Anonymous(_), Id::Escaped(_)) => todo!(),
                                            (Id::Anonymous(_), Id::Plain(_)) => todo!(),
                                            (Id::Anonymous(_), Id::Anonymous(_)) => todo!(),
                                        })
                                        .unwrap(),
                                );

                                nodes
                                    .entry(a.0.to_string())
                                    .or_default()
                                    .outgoing
                                    .push(b.0.to_string());
                                nodes
                                    .entry(b.0.to_string())
                                    .or_default()
                                    .ingoing
                                    .push(a.0.to_string());
                            }
                            (a, b) => todo!("{a:?} -> {b:?}"),
                        },
                        graphviz_rust::dot_structures::EdgeTy::Chain(_) => todo!(),
                    },
                }
            }
        }
    }

    let start_node = nodes.iter().find_map(|(n, node)| {
        (node.outgoing.len() == 1 && node.ingoing.is_empty()).then_some((n, node))
    });
    let end_node = nodes.iter().find_map(|(n, node)| {
        (node.outgoing.is_empty() && node.ingoing.len() == 1).then_some((n, node))
    });

    debug!(
        start = format!("{start_node:?}"),
        end = format!("{end_node:?}")
    );

    (nodes, node_mapping, graph)
}
