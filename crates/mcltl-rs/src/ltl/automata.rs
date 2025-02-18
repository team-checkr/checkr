use std::collections::{BTreeSet, HashMap};

use itertools::Itertools;

use crate::{
    buchi::{
        Alphabet, AtomicProperty, AtomicPropertySet, BuchiLike as _, BuchiLikeMut as _,
        GeneralBuchi, Neighbors,
    },
    nodes::NodeSet,
    state::State,
};

use super::expression::NnfLtl;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct AutomataId(u32);

impl std::fmt::Debug for AutomataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_initial() {
            write!(f, "A0i")
        } else {
            write!(f, "A{}", self.0)
        }
    }
}

impl State for AutomataId {
    fn initial() -> Self {
        AutomataId(0)
    }

    fn name(&self) -> String {
        if self.is_initial() {
            "A0i".to_string()
        } else {
            format!("A{}", self.0)
        }
    }
}

impl<AP: AtomicProperty> NnfLtl<AP> {
    /// Construct the General Büchi Automata from the LTL formula.
    ///
    /// Implementation of the method describe in the paper: [Simple On-the-Fly
    /// Automatic Verification of Linear Temporal
    /// Logic](https://link.springer.com/content/pdf/10.1007/978-0-387-34892-6_1.pdf).
    /// The graph constructed by the algorithm can be used to deﬁne an LGBA
    /// accepting the inﬁnite words satisfying the formula.
    pub fn gba(&self, alphabet: Option<&Alphabet<AP>>) -> GeneralBuchi<AutomataId, AP> {
        let nodes = AutomataGraph::create_graph(self);
        extract_buchi(nodes.iter(), alphabet, self)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AutomataGraph<AP> {
    last_id: u32,
    _phantom: std::marker::PhantomData<AP>,
}

impl<AP> Default for AutomataGraph<AP> {
    fn default() -> Self {
        AutomataGraph {
            last_id: 0,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<AP> AutomataGraph<AP> {
    fn new_name(&mut self) -> AutomataId {
        self.last_id += 1;
        AutomataId(self.last_id)
    }

    fn initial(&self) -> AutomataId {
        AutomataId(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node<AP> {
    pub name: AutomataId,
    // "The field Father is not needed, except for the proof of correctness."
    // Simple On-the-Fly Automatic Verification of Linear Temporal Logic p.12
    // https://link.springer.com/content/pdf/10.1007/978-0-387-34892-6_1.pdf
    // pub father: AutomataId,
    pub incoming: BTreeSet<AutomataId>,
    pub next: BTreeSet<NnfLtl<AP>>,
    pub oldf: BTreeSet<NnfLtl<AP>>,
    pub newf: BTreeSet<NnfLtl<AP>>,
}

impl<AP: Clone + Ord> AutomataGraph<AP> {
    /// [Simple On-the-Fly Automatic Verification of Linear Temporal
    /// Logic](https://link.springer.com/content/pdf/10.1007/978-0-387-34892-6_1.pdf)
    /// p.10
    fn create_graph(f: &NnfLtl<AP>) -> BTreeSet<Node<AP>> {
        // return (expand([Name=Father=new_name(), Incoming={init},
        //                 New={f}, Old=∅, Next=∅], ∅))
        let mut g = AutomataGraph::default();
        let init = g.initial();
        let name = g.new_name();
        let n = Node {
            name,
            // father: name,
            incoming: [init].into(),
            newf: [f.clone()].into(),
            oldf: [].into(),
            next: [].into(),
        };
        let node_set = Default::default();
        g.expand(n, node_set)
    }

    /// [Simple On-the-Fly Automatic Verification of Linear Temporal
    /// Logic](https://link.springer.com/content/pdf/10.1007/978-0-387-34892-6_1.pdf)
    /// p.10
    fn expand(
        &mut self,
        mut node: Node<AP>,
        mut node_set: BTreeSet<Node<AP>>,
    ) -> BTreeSet<Node<AP>> {
        // tracing::warn!(?node, ?node_set, "Expanding node");

        fn union<T: Clone + Eq + Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> BTreeSet<T> {
            a.union(b).cloned().collect()
        }
        fn diff<AP: Clone + Ord>(
            a: &BTreeSet<NnfLtl<AP>>,
            b: &BTreeSet<NnfLtl<AP>>,
        ) -> BTreeSet<NnfLtl<AP>> {
            a.difference(b).cloned().collect()
        }

        // if New(node)=∅ then
        match node.newf.pop_first() {
            None => {
                // let span = tracing::debug_span!("ex", node = "∅");
                // let _guard = span.enter();
                // if ∃ ND ∈ node_set with Old(ND) = Old(node) and Next(ND) = Next(node) then
                if let Some(mut nd) = node_set.clone().into_iter().find(|nd| {
                    let p1 = nd.oldf == node.oldf;
                    let p2 = nd.next == node.next;
                    // tracing::debug!(?nd, ?p1, ?p2, "checking");
                    if !p1 {
                        // tracing::debug!(oldf=?nd.oldf, oldf=?node.oldf, "oldf");
                    }
                    if !p2 {
                        // tracing::debug!(next_nd=?nd.next, next_node=?node.next, "next");
                    }
                    p1 && p2
                }) {
                    // Incoming(ND) = Incoming(ND) ∪ Incoming(node)
                    node_set.remove(&nd);
                    nd.incoming = union(&nd.incoming, &node.incoming);
                    node_set.insert(nd);
                    // return(node_set)
                    node_set
                } else {
                    // return(expand([Name=Father=new_name(),
                    //  Incoming={Name(node)}, New=Next(node), ,
                    //  Old=∅, Next=∅], {node} ∪ node_set))
                    let name = self.new_name();
                    let new_node = Node {
                        name,
                        // father: name,
                        incoming: [node.name].into(),
                        newf: node.next.clone(),
                        next: Default::default(),
                        oldf: Default::default(),
                    };
                    node_set.insert(node);
                    // tracing::debug!(this=?new_node, "next we are expanding");
                    self.expand(new_node, node_set)
                }
            }
            // let n = New;
            // New(node) := New(node) \ {n};
            Some(n) => {
                // let span = tracing::debug_span!("ex", node = %n);
                // let _guard = span.enter();
                // case n of
                match &n {
                    // n = Pn, or ¬Pn or n = true or n = false =>
                    NnfLtl::Literal { .. } | NnfLtl::Bool(_) => {
                        // if n = false or Neg(n) ∈ Old(node) then (* Current node contains a contradiction *)
                        let matches = match &n {
                            NnfLtl::Bool(false) => true,
                            NnfLtl::Literal {
                                negated: neg1,
                                name: name1,
                            } => node.oldf.iter().any(|x| {
                                matches!(
                                    x,
                                    NnfLtl::Literal {
                                        negated: neg2,
                                        name: name2
                                    } if name1 == name2 && neg1 != neg2
                                )
                            }),
                            _ => false,
                        };
                        if matches {
                            // reutrn (node_set) (* Discard current node *)
                            node_set
                        } else {
                            // Old(node) := Old(node) ∪ {n}
                            node.oldf.insert(n);
                            // return (expand(node, node_set))
                            self.expand(node, node_set)
                        }
                    }
                    // n = µ U ϕ or n = µ V ϕ or n = µ | ϕ =>
                    NnfLtl::U(_, _) | NnfLtl::V(_, _) | NnfLtl::Or(_, _) => {
                        let (new1, next1, new2) = new1_next1_new2(&n);

                        // Node1 := [Name=new_name(), Father=Name(node), Incoming=Incoming(node),
                        //           New=New(node) ∪ ({New1(n)} \ Old(node)),
                        //           Old=Old(Node) ∪ {n}, Next=Next(node) ∪ {Next1(n)}]
                        let n1 = Node {
                            name: self.new_name(),
                            // father: node.name,
                            incoming: node.incoming.clone(),
                            next: union(&node.next, &next1),
                            oldf: node.oldf.iter().cloned().chain([n.clone()]).collect(),
                            newf: union(&node.newf, &diff(&new1, &node.oldf)),
                        };
                        // Node2 := [Name=new_name(), Father=Name(node), Incoming=Incoming(node),
                        //           New=New(node) ∪ ({New2(n)} \ Old(node)),
                        //           Old=Old(Node) ∪ {n}, Next=Next(node)]
                        let n2 = Node {
                            name: self.new_name(),
                            // father: node.name,
                            incoming: node.incoming.clone(),
                            next: node.next.clone(),
                            oldf: node.oldf.iter().cloned().chain([n.clone()]).collect(),
                            newf: union(&node.newf, &diff(&new2, &node.oldf)),
                        };
                        // return (expand(Node2, expand(Node1, node_set)))
                        let tmp = self.expand(n1, node_set);
                        self.expand(n2, tmp)
                    }
                    // n = µ & ϕ =>
                    NnfLtl::And(p, q) => {
                        // return (expand([Name=Name(node), Father=Father(node,
                        //                 Incoming=Incoming(node), New=New(node) ∪ ({µ, ϕ} \ Old(node)),
                        //                 Old=Old(node) ∪ {µ, ϕ}, Next=Next(node)], node_set))
                        let n = Node {
                            name: node.name,
                            // // father: node.father,
                            incoming: node.incoming.clone(),
                            next: node.next.clone(),
                            oldf: union(&node.oldf, &[(**p).clone(), (**q).clone()].into()),
                            newf: union(
                                &node.newf,
                                &[(**p).clone(), (**q).clone()]
                                    .into_iter()
                                    .filter(|x| !node.oldf.contains(x))
                                    .collect(),
                            ),
                        };
                        self.expand(n, node_set)
                    }
                    // n = X µ
                    NnfLtl::X(p) => {
                        // return (expand([Name=Name(node), Father=Father(node),
                        //                 Incoming=Incoming(node), New=New(node),
                        //                 Old=Old(node) ∪ {n}, Next=Next(node) ∪ {µ}], node_set))
                        let n = Node {
                            name: node.name,
                            // father: node.father,
                            incoming: node.incoming.clone(),
                            next: union(&node.next, &[(**p).clone()].into()),
                            oldf: union(&node.oldf, &[n.clone()].into()),
                            newf: node.newf.clone(),
                        };
                        self.expand(n, node_set)
                    }
                }
            }
        }
    }
}

/// Encoding of the table from [Simple On-the-Fly Automatic Verification of Linear Temporal
/// Logic](https://link.springer.com/content/pdf/10.1007/978-0-387-34892-6_1.pdf)
/// p.9
fn new1_next1_new2<AP: Clone + Ord>(
    n: &NnfLtl<AP>,
) -> (
    BTreeSet<NnfLtl<AP>>,
    BTreeSet<NnfLtl<AP>>,
    BTreeSet<NnfLtl<AP>>,
) {
    fn set<'a, T: Clone + Ord + 'a>(ts: impl IntoIterator<Item = &'a Box<T>>) -> BTreeSet<T> {
        ts.into_iter().map(|t| (**t).clone()).collect()
    }

    let (new1, next1, new2): (
        BTreeSet<NnfLtl<AP>>,
        BTreeSet<NnfLtl<AP>>,
        BTreeSet<NnfLtl<AP>>,
    ) = {
        let n = Box::new(n.clone());
        match &*n {
            NnfLtl::U(p, q) => (set([p]), set([&n]), set([q])),
            NnfLtl::V(p, q) => (set([q]), set([&n]), set([p, q])),
            NnfLtl::Or(p, q) => (set([p]), [].into(), set([q])),
            _ => unreachable!(),
        }
    };
    (new1, next1, new2)
}

impl<AP: Ord> NnfLtl<AP> {
    fn extract_unitl_subf<'a>(
        &'a self,
        mut sub_formulas: BTreeSet<(&'a NnfLtl<AP>, &'a NnfLtl<AP>)>,
    ) -> BTreeSet<(&'a NnfLtl<AP>, &'a NnfLtl<AP>)> {
        match self {
            NnfLtl::Bool(_) => sub_formulas,
            NnfLtl::Literal { .. } => sub_formulas,
            NnfLtl::And(f1, f2) => f2.extract_unitl_subf(f1.extract_unitl_subf(sub_formulas)),
            NnfLtl::Or(f1, f2) => f2.extract_unitl_subf(f1.extract_unitl_subf(sub_formulas)),
            NnfLtl::U(f1, f2) => {
                sub_formulas.insert((f1, f2));
                f2.extract_unitl_subf(f1.extract_unitl_subf(sub_formulas))
            }
            NnfLtl::V(f1, f2) => f1.extract_unitl_subf(f2.extract_unitl_subf(sub_formulas)),
            NnfLtl::X(f) => f.extract_unitl_subf(sub_formulas),
        }
    }
}

/// LGBA construction from create_graph set Q result
fn extract_buchi<'a, AP: AtomicProperty + 'a>(
    result: impl Iterator<Item = &'a Node<AP>> + Clone,
    alphabet: Option<&Alphabet<AP>>,
    f: &NnfLtl<AP>,
) -> GeneralBuchi<AutomataId, AP> {
    let mut b: GeneralBuchi<AutomataId, AP> =
        GeneralBuchi::new(alphabet.into_iter().fold(f.alphabet(), |a, b| a.union(b)));

    let oldfs = result
        .clone()
        .map(|n| (n.name, n.oldf.clone()))
        .collect::<HashMap<_, _>>();

    for n in result.clone() {
        // tracing::debug!(?n, "adding node");

        let id = b.push(n.name);

        for &m in &n.incoming {
            if m.is_initial() {
                b.add_init_state(id);
            } else {
                let required: AP::Set = oldfs[&m]
                    .iter()
                    .flat_map(|p| match p {
                        NnfLtl::Literal {
                            name,
                            negated: false,
                        } => [name.clone()].to_vec(),
                        _ => [].to_vec(),
                    })
                    .collect();
                let disallowed: AP::Set = oldfs[&m]
                    .iter()
                    .flat_map(|p| match p {
                        NnfLtl::Literal {
                            name,
                            negated: true,
                        } => [name.clone()].to_vec(),
                        NnfLtl::Bool(false) => b.alphabet().symbols().cloned().collect(),
                        _ => [].to_vec(),
                    })
                    .collect();
                let m_id = b.push(m);
                let label: Neighbors<AP> =
                    if oldfs[&m].iter().all(|p| matches!(p, NnfLtl::Bool(true))) {
                        Neighbors::any(&*b.alphabet())
                    } else {
                        let alphabet = b.alphabet();
                        let remaining = alphabet
                            .symbols()
                            .filter(|s| !required.contains(s) && !disallowed.contains(s))
                            .collect_vec();
                        Neighbors::Just(
                            (0..=remaining.len())
                                .flat_map(|n| {
                                    remaining.iter().combinations(n).map(|extra| {
                                        let mut label = required.clone();
                                        label.extend(extra.into_iter().cloned().cloned());
                                        label
                                    })
                                })
                                .collect(),
                        )

                        // label.into_iter().collect()
                    };
                // tracing::debug!(%f, ?n, ?m, oldf=?n.oldf, ?label, "adding transition");
                b.add_transition(m_id, id, label);
            }
        }
    }

    let sub_formulas = f.extract_unitl_subf(BTreeSet::new());

    for f in sub_formulas {
        let mut accepting_states = NodeSet::default();

        for n in result.clone() {
            match f {
                (f1, f2) if !n.oldf.contains(&f1.clone().U(f2.clone())) || n.oldf.contains(f2) => {
                    accepting_states.insert(b.get_node(&n.name).unwrap());
                }
                _ => {}
            }
        }

        b.add_accepting_state(&accepting_states.iter().collect());
    }

    b
}

#[cfg(test)]
mod tests;
