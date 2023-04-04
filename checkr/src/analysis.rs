use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::pg::{Edge, Node, ProgramGraph};

pub enum Direction {
    Forward,
    Backward,
}

pub trait MonotoneFramework {
    type Domain: Lattice + Serialize + for<'a> Deserialize<'a>;
    fn semantic(&self, _pg: &ProgramGraph, e: &Edge, prev: &Self::Domain) -> Self::Domain;
    fn direction() -> Direction;
    fn initial(&self, pg: &ProgramGraph) -> Self::Domain;
    fn debug(&self, _item: &Self::Domain) {}
}

pub trait Lattice: Sized + Clone {
    fn bottom() -> Self;
    fn lub_extend(&mut self, other: &Self) {
        *self = self.lub(other);
    }
    fn lub(&self, other: &Self) -> Self;
    fn contains(&self, other: &Self) -> bool;
}

pub trait Worklist {
    fn empty() -> Self;
    fn insert(&mut self, n: Node);
    fn extract(&mut self, pg: &ProgramGraph) -> Option<Node>;
}

pub struct FiFo(VecDeque<Node>);
impl Worklist for FiFo {
    fn empty() -> Self {
        FiFo(Default::default())
    }

    fn insert(&mut self, n: Node) {
        self.0.push_back(n)
    }

    fn extract(&mut self, _pg: &ProgramGraph) -> Option<Node> {
        self.0.pop_front()
    }
}

pub struct LiFo(Vec<Node>);
impl Worklist for LiFo {
    fn empty() -> Self {
        LiFo(Default::default())
    }

    fn insert(&mut self, n: Node) {
        self.0.push(n);
    }

    fn extract(&mut self, _pg: &ProgramGraph) -> Option<Node> {
        self.0.pop()
    }
}

// pub struct RoundRobin(VecDeque<Node>, HashSet<Node>);
// impl Worklist for RoundRobin {
//     fn empty() -> Self {
//         RoundRobin(Default::default(), Default::default())
//     }

//     fn insert(&mut self, n: Node) {
//         let RoundRobin(v, p) = self;
//         if !v.contains(&n) {
//             p.insert(n);
//         }
//     }

//     fn extract(&mut self, pg: &ProgramGraph) -> Option<Node> {
//         let RoundRobin(v, p) = self;
//         match (v.is_empty(), p.is_empty()) {
//             (true, true) => return None,
//             (true, false) => {
//                 *v = pg.reverse_post_order().filter(|n| p.contains(n)).collect();
//                 p.clear();
//                 v.pop_back()
//             }
//             _ => v.pop_back(),
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResults<A: MonotoneFramework> {
    pub facts: HashMap<Node, A::Domain>,
    pub semantic_calls: usize,
}

pub fn mono_analysis<A: MonotoneFramework, W: Worklist>(
    a: A,
    pg: &ProgramGraph,
) -> AnalysisResults<A> {
    let mut worklist = W::empty();

    let bot = A::Domain::bottom();

    let mut facts: HashMap<Node, A::Domain> = HashMap::default();
    for &n in pg.nodes() {
        facts.insert(n, bot.clone());
        worklist.insert(n);
    }

    let initial = a.initial(pg);
    let initial_node = match A::direction() {
        Direction::Forward => Node::Start,
        Direction::Backward => Node::End,
    };
    facts.insert(initial_node, initial);

    let mut calls = 0;

    while let Some(n) = worklist.extract(pg) {
        for e in pg.edges() {
            let (from, to) = match A::direction() {
                Direction::Forward => (e.from(), e.to()),
                Direction::Backward => (e.to(), e.from()),
            };
            if n != from {
                continue;
            }

            let constraint = a.semantic(pg, e, &facts[&from]);
            calls += 1;

            let target = facts.get_mut(&to).unwrap();

            if !target.contains(&constraint) {
                target.lub_extend(&constraint);
                worklist.insert(to);
            }
        }
    }

    AnalysisResults {
        facts,
        semantic_calls: calls,
    }
}

impl<T> Lattice for HashSet<T>
where
    T: std::hash::Hash + PartialEq + Eq + Clone,
{
    fn bottom() -> Self {
        HashSet::default()
    }

    fn lub_extend(&mut self, other: &Self) {
        self.extend(other.iter().cloned());
    }

    fn lub(&self, other: &Self) -> Self {
        self.union(other).cloned().collect()
    }

    fn contains(&self, other: &Self) -> bool {
        other.is_subset(self)
    }
}

impl<K, V> Lattice for HashMap<K, V>
where
    K: std::hash::Hash + PartialEq + Eq + Clone,
    V: Lattice + Clone,
{
    fn bottom() -> Self {
        HashMap::default()
    }

    fn lub_extend(&mut self, other: &Self) {
        for (k, b) in other {
            if let Some(a) = self.get_mut(k) {
                a.lub_extend(b);
            } else {
                self.insert(k.clone(), b.clone());
            }
        }
    }

    fn lub(&self, other: &Self) -> Self {
        let mut result = HashMap::default();

        for (k, a) in self {
            if let Some(b) = other.get(k) {
                result.insert(k.clone(), a.lub(b));
            } else {
                result.insert(k.clone(), a.clone());
            }
        }
        for (k, b) in other {
            if !self.contains_key(k) {
                result.insert(k.clone(), b.clone());
            }
        }

        result
    }

    fn contains(&self, other: &Self) -> bool {
        other.iter().all(|(k, a)| {
            if let Some(b) = self.get(k) {
                b.contains(a)
            } else {
                false
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeOrder<'a> {
    First,
    Middle(i32),
    Random(&'a str),
    Last,
}

impl<'a> NodeOrder<'a> {
    pub fn parse(n: &'a str) -> Self {
        match n {
            _ if n.contains('▷') => NodeOrder::First,
            "qS" => NodeOrder::First,

            _ if n.contains('◀') => NodeOrder::Last,
            "qF" => NodeOrder::Last,

            _ if n.contains(|c: char| c.is_numeric()) => NodeOrder::Middle(
                n.chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .unwrap_or_default(),
            ),
            _ => NodeOrder::Random(n),
        }
    }
}
