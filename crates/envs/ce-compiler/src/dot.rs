use std::collections::BTreeMap;

use graphviz_rust::dot_structures::{Attribute, EdgeTy, Graph, Id, Stmt, Vertex};
use petgraph::graph::NodeIndex;

#[derive(Debug)]
pub struct ParsedGraph {
    #[allow(unused)]
    pub nodes: BTreeMap<String, Node>,
    #[allow(unused)]
    pub node_mapping: BTreeMap<String, NodeIndex>,
    pub graph: petgraph::Graph<String, gcl::pg::Action>,
}

#[derive(Debug, Default)]
pub struct Node {
    pub attributes: Vec<Attribute>,
    pub outgoing: Vec<String>,
    pub ingoing: Vec<String>,
}

pub fn dot_to_petgraph(dot: &str) -> Result<ParsedGraph, String> {
    let mut nodes = BTreeMap::<String, Node>::new();
    let mut node_mapping = BTreeMap::<String, NodeIndex>::new();
    let mut graph = petgraph::Graph::<String, gcl::pg::Action>::new();

    let parsed = graphviz_rust::parse(dot)?;

    match parsed {
        Graph::Graph { .. } => todo!(),
        Graph::DiGraph { stmts, .. } => {
            for stmt in stmts {
                match stmt {
                    Stmt::Node(n) => {
                        node_mapping
                            .entry(n.id.0.to_string())
                            .or_insert_with_key(|k| graph.add_node(k.to_string()));

                        nodes
                            .entry(n.id.0.to_string())
                            .or_default()
                            .attributes
                            .extend_from_slice(&n.attributes);
                    }
                    Stmt::Subgraph(_) => {}
                    Stmt::Attribute(_) => {}
                    Stmt::GAttribute(_) => {}
                    Stmt::Edge(e) => match e.ty {
                        EdgeTy::Pair(a, b) => {
                            if let (Vertex::N(a), Vertex::N(b)) = (a, b) {
                                let a_id = *node_mapping
                                    .entry(a.0.to_string())
                                    .or_insert_with_key(|k| graph.add_node(k.to_string()));
                                let b_id = *node_mapping
                                    .entry(b.0.to_string())
                                    .or_insert_with_key(|k| graph.add_node(k.to_string()));
                                let label = e
                                    .attributes
                                    .iter()
                                    .find_map(|a| match (&a.0, &a.1) {
                                        (Id::Plain(l), Id::Escaped(v)) if l == "label" => {
                                            Some(v.to_string())
                                        }
                                        _ => None,
                                    })
                                    .ok_or("edge label not found")?;
                                let label = label.trim_matches('"');
                                let action = gcl::parse::parse_action(label)
                                    .map_err(|e| format!("failed to parse action: {label}. {e}"))?;
                                graph.add_edge(a_id, b_id, action);

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
                        }
                        EdgeTy::Chain(_) => {}
                    },
                }
            }
        }
    }

    Ok(ParsedGraph {
        nodes,
        node_mapping,
        graph,
    })
}
