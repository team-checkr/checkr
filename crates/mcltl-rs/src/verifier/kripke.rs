use core::fmt;
use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    buchi::{Alphabet, AtomicProperty, AtomicPropertySet, Buchi, BuchiLikeMut as _},
    nodes::{NodeArena, NodeId, SmartNodeSet},
    state::State,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KripkeNode<S, AP: AtomicProperty> {
    pub id: S,
    pub assignment: AP::Set,
}

type KripkeNodeId<S, AP> = NodeId<KripkeNode<S, AP>>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KripkeStructure<S, AP: AtomicProperty> {
    nodes: NodeArena<KripkeNode<S, AP>>,
    inits: SmartNodeSet<KripkeNode<S, AP>>, // s0
    relations: Vec<(KripkeNodeId<S, AP>, KripkeNodeId<S, AP>)>,
}

impl<S: State, AP: AtomicProperty> KripkeStructure<S, AP> {
    pub fn alphabet(&self) -> Alphabet<AP> {
        let mut alphabet = BTreeSet::new();

        for w in self.nodes.iter() {
            for k in w.assignment.iter() {
                alphabet.insert(k.clone());
            }
        }

        alphabet.into_iter().collect()
    }
    /// Computing an [NBA](Buchi) `AM` from a Kripke Structure `M`
    ///
    /// Kripke structure: `M = <hS, S0, R, AP, APi>`
    /// into NBA: `Am = <Q, Σ, δ, I, Fi>`
    ///
    /// * Sates: `Q := S U { init }`
    /// * Alphabets: `Σ := 2^AP`
    /// * Initial State: `I := { init }`
    /// * Accepting States: `F := Q = S U { init }`
    /// * Transitions:
    ///     * `δ : q →a q'` iff `(q, q) ∈ R` and `AP(q') = a`
    ///     * `init ->a q` iff `q ∈ S0` and `AP(q) = a`
    pub fn to_buchi(&self, alphabet: Option<&Alphabet<AP>>) -> Buchi<S, AP> {
        let mut buchi: Buchi<S, AP> = Buchi::new(
            alphabet
                .into_iter()
                .fold(self.alphabet().clone(), |a, b| a.union(b)),
        );

        for &(src, dst) in self.relations.iter() {
            let src_s = &self.nodes[src];
            let dst_s = &self.nodes[dst];
            if let Some(node) = buchi.get_node(&src_s.id) {
                let target = buchi.push(dst_s.id.clone());
                let labels = dst_s.assignment.iter().cloned().collect();
                buchi.add_transition(node, target, labels);
                buchi.add_accepting_state(node);
                buchi.add_accepting_state(target);
            } else {
                let node = buchi.push(src_s.id.clone());
                let target = buchi.push(dst_s.id.clone());
                let labels = dst_s.assignment.iter().cloned().collect();
                buchi.add_transition(node, target, labels);
                buchi.add_accepting_state(node);
                buchi.add_accepting_state(target);
            }
        }

        let init = buchi.push(S::initial());

        for i in self.inits.iter() {
            let world = &self.nodes[i];
            let target_node = buchi.push(world.id.clone());
            let labels = world.assignment.iter().cloned().collect();
            buchi.add_transition(init, target_node, labels);
            buchi.add_accepting_state(target_node);
        }

        buchi.add_init_state(init);
        buchi.add_accepting_state(init);

        buchi
    }
}

impl<S: State, AP: AtomicProperty> KripkeStructure<S, AP> {
    pub fn new(inits: Vec<S>) -> Self {
        let mut worlds = NodeArena::new();
        let mut new_inits = SmartNodeSet::new();
        for i in inits {
            new_inits.insert(worlds.push(KripkeNode {
                id: i,
                assignment: Default::default(),
            }));
        }

        Self {
            inits: new_inits,
            nodes: worlds,
            relations: Vec::new(),
        }
    }

    fn find_world(&self, s: &S) -> Option<KripkeNodeId<S, AP>> {
        self.nodes
            .iter_with_ids()
            .find(|w| &w.1.id == s)
            .map(|w| w.0)
    }

    /// Add a new world
    pub fn add_node(&mut self, w: S, assignment: AP::Set) -> KripkeNodeId<S, AP> {
        if let Some(w) = self.find_world(&w) {
            self.nodes[w].assignment.extend(assignment.iter().cloned());
            w
        } else {
            self.nodes.push(KripkeNode { id: w, assignment })
        }
    }

    /// Add a new relation
    pub fn add_relation(&mut self, w1: KripkeNodeId<S, AP>, w2: KripkeNodeId<S, AP>) {
        self.relations.push((w1, w2));
    }
}

impl<S: State + fmt::Display, AP: AtomicProperty + fmt::Display> fmt::Display
    for KripkeStructure<S, AP>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "init = {{{:?}}}", self.inits.iter().join(", "))?;
        writeln!(f)?;

        for (n_id, n) in self.nodes.iter_with_ids() {
            writeln!(f, "{} = {{{}}}", n.id, n.assignment.iter().join(", "))?;
            for &(m1, m2) in &self.relations {
                if n_id == m1 {
                    writeln!(f, "{} => {} ;;", n.id, self.nodes[m2].id)?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Expr<S, AP: AtomicProperty> {
    Init(Vec<S>),
    World(KripkeNode<S, AP>),
    Relation(S, Vec<S>),
}

#[cfg(test)]
mod tests {

    use crate::buchi::BuchiLike as _;

    use super::*;

    #[test]
    fn it_should_compute_nba_from_kripke_struct() {
        let kripke = crate::kripke! {
            n1 = [ p, q ]
            n2 = [ p ]
            n3 = [ q ]
            ===
            n1 R n2
            n2 R n1
            n2 R n3
            n3 R n1
            ===
            init = [n1, n2]
        };

        let buchi = kripke.to_buchi(None);

        assert_eq!(4, buchi.accepting_states().count());
        assert_eq!(1, buchi.init_states().count());
        assert_eq!(4, buchi.nodes().count());
    }

    #[test]
    fn it_should_compute_nba_from_kripke_struct2() {
        let kripke = crate::kripke! {
            n1 = [ a ]
            n2 = [ b ]
            n3 = [ c ]
            ===
            n1 R n2
            n2 R n3
            n3 R n1
            ===
            init = [n1]
        };

        let buchi = kripke.to_buchi(None);

        assert_eq!(4, buchi.accepting_states().count());
        assert_eq!(1, buchi.init_states().count());
        assert_eq!(4, buchi.nodes().count());
    }
}
