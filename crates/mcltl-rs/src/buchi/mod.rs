use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    fmt,
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use ahash::{AHashMap, AHashSet};
use itertools::Itertools;

use crate::{
    ltl::expression::{Literal, NnfLtl},
    nodes::{NodeArena, NodeId, NodeMap, NodeSet, SmartNodeMap, SmartNodeSet},
    state::State,
};

pub trait AtomicPropertySet<AP: AtomicProperty>:
    AtomicProperty + std::fmt::Debug + Default + Clone + Ord + Hash + FromIterator<AP> + Extend<AP>
{
    fn set(&mut self, ap: AP);
    fn contains(&self, ap: &AP) -> bool;
    fn is_empty(&self) -> bool;
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a AP>
    where
        AP: 'a;
    fn union(&self, other: &Self) -> Self;
    fn intersection(&self, other: &Self) -> Self;
    fn is_disjoint(&self, other: &Self) -> bool;
}

impl<AP: AtomicProperty> AtomicPropertySet<AP> for BTreeSet<AP> {
    fn set(&mut self, ap: AP) {
        self.insert(ap);
    }
    fn contains(&self, ap: &AP) -> bool {
        self.contains(ap)
    }
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a AP>
    where
        AP: 'a,
    {
        self.iter()
    }
    fn union(&self, other: &Self) -> Self {
        self.union(other).cloned().collect()
    }
    fn intersection(&self, other: &Self) -> Self {
        self.intersection(other).cloned().collect()
    }
    fn is_disjoint(&self, other: &Self) -> bool {
        self.is_disjoint(other)
    }
}
impl<AP: AtomicProperty> AtomicPropertySet<AP> for Vec<AP> {
    fn set(&mut self, ap: AP) {
        if !self.contains(&ap) {
            self.push(ap);
        }
    }
    fn contains(&self, ap: &AP) -> bool {
        self.iter().any(|x| x == ap)
    }
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a AP>
    where
        AP: 'a,
    {
        self.as_slice().iter()
    }
    fn union(&self, other: &Self) -> Self {
        let mut new = self.clone();
        new.extend(other.iter().filter(|x| !self.contains(x)).cloned());
        new
    }
    fn intersection(&self, b: &Self) -> Self {
        if self.len() < b.len() {
            self.iter().filter(|x| b.contains(x)).cloned().collect()
        } else {
            b.iter().filter(|x| self.contains(x)).cloned().collect()
        }
    }
    fn is_disjoint(&self, b: &Self) -> bool {
        if self.len() < b.len() {
            self.iter().all(|x| !b.contains(x))
        } else {
            b.iter().all(|x| !self.contains(x))
        }
    }
}
impl<AP: AtomicProperty> AtomicProperty for BTreeSet<AP> {
    type Set = BTreeSet<Self>;
}

impl<AP: AtomicProperty> AtomicProperty for Vec<AP> {
    type Set = Vec<Self>;
}

pub trait AtomicProperty: Clone + Ord + Eq + Hash + fmt::Debug {
    type Set: AtomicPropertySet<Self>;
}

impl<L: AtomicProperty> AtomicProperty for NnfLtl<L> {
    type Set = BTreeSet<Self>;
}

impl AtomicProperty for Literal {
    type Set = BTreeSet<Self>;
}

impl AtomicProperty for String {
    type Set = BTreeSet<Self>;
}
impl AtomicProperty for &str {
    type Set = BTreeSet<Self>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Alphabet<AP> {
    symbols: Arc<BTreeSet<AP>>,
}
impl<AP> Alphabet<AP> {
    pub fn symbols(&self) -> impl Iterator<Item = &AP> {
        self.symbols.iter()
    }
    pub fn union(&self, other: &Self) -> Self
    where
        AP: Clone + Ord,
    {
        Self {
            symbols: Arc::new(
                self.symbols
                    .iter()
                    .chain(other.symbols.iter())
                    .cloned()
                    .collect(),
            ),
        }
    }
}

impl<AP: Ord> FromIterator<AP> for Alphabet<AP> {
    fn from_iter<T: IntoIterator<Item = AP>>(iter: T) -> Self {
        Self {
            symbols: Arc::new(iter.into_iter().collect()),
        }
    }
}

impl<AP: Ord, const N: usize> From<[AP; N]> for Alphabet<AP> {
    fn from(arr: [AP; N]) -> Self {
        Alphabet {
            symbols: Arc::new(arr.into()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum Neighbors<AP: AtomicProperty> {
    Any,
    Just(BTreeSet<AP::Set>),
}
impl<AP: AtomicProperty> Neighbors<AP> {
    pub fn none() -> Self {
        Neighbors::Just(BTreeSet::new())
    }

    fn intersection(&self, other: &Self) -> Self {
        match (self, other) {
            (Neighbors::Just(a), Neighbors::Just(b)) => {
                Neighbors::Just(a.intersection(b).cloned().collect())
            }
            (Neighbors::Any, Neighbors::Any) => Neighbors::Any,
            (Neighbors::Any, just @ Neighbors::Just(_))
            | (just @ Neighbors::Just(_), Neighbors::Any) => just.clone(),
        }
    }

    fn is_disjoint(&self, other: &Self) -> bool {
        match (self, other) {
            (Neighbors::Just(a), Neighbors::Just(b)) => a.is_disjoint(b),
            _ => false,
        }
    }

    fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Neighbors::Just(a), Neighbors::Just(b)) => {
                Neighbors::Just(a.union(b).cloned().collect())
            }
            (Neighbors::Any, Neighbors::Any) => Neighbors::Any,
            (Neighbors::Any, Neighbors::Just(_)) | (Neighbors::Just(_), Neighbors::Any) => {
                Neighbors::Any
            }
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Neighbors::Just(set) => set.is_empty(),
            Neighbors::Any => false,
        }
    }

    pub fn any(_alphabet: &Alphabet<AP>) -> Neighbors<AP> {
        Neighbors::Any
    }
}
impl<AP: AtomicProperty> FromIterator<AP> for Neighbors<AP> {
    fn from_iter<T: IntoIterator<Item = AP>>(iter: T) -> Self {
        let mut set = AP::Set::default();
        for i in iter {
            set.set(i);
        }
        Neighbors::Just([set].into())
    }
}
impl<AP: AtomicProperty, const N: usize> From<[AP; N]> for Neighbors<AP> {
    fn from(arr: [AP; N]) -> Self {
        let mut set = AP::Set::default();
        for i in arr {
            set.set(i);
        }
        Neighbors::Just([set].into())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct BuchiNode<S, AP: AtomicProperty> {
    id: S,
    adj: SmartNodeMap<BuchiNode<S, AP>, Neighbors<AP>>,
}

impl<S, AP: AtomicProperty> BuchiNode<S, AP> {
    pub fn new(id: S) -> Self {
        Self {
            id,
            adj: Default::default(),
        }
    }
}

impl<S, AP: AtomicProperty> fmt::Display for BuchiNode<S, AP>
where
    S: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buff = String::new();
        buff.push_str(&format!("{}id = {}\n", &buff, self.id));

        let adjs = self
            .adj
            .iter()
            .fold("".to_string(), |acc, a| acc + &format!("{},", a.0));
        buff.push_str(&format!("{}{}.adj = [{}]\n", &buff, self.id, adjs));

        write!(f, "{}", buff)
    }
}

pub trait BuchiLike<S, AP: AtomicProperty> {
    type NodeId: Copy;
    type AcceptingState<'a>
    where
        Self: 'a;

    fn nodes(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_;
    fn init_states(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_;
    fn accepting_states<'a>(
        &'a self,
    ) -> impl Iterator<Item = Self::AcceptingState<'a>> + Clone + 'a
    where
        Self: 'a;
    fn is_accepting_state(&self, node_id: Self::NodeId) -> bool;
    fn adj_labels<'a>(
        &'a self,
        id: Self::NodeId,
    ) -> impl Iterator<Item = (Self::NodeId, Cow<'a, Neighbors<AP>>)> + Clone + 'a
    where
        AP: 'a;
    fn adj_ids(&self, id: Self::NodeId) -> impl Iterator<Item = Self::NodeId> + Clone + '_;

    fn alphabet(&self) -> Cow<'_, Alphabet<AP>>;

    fn fmt_node(&self, id: Self::NodeId) -> String;
    fn fmt_accepting_state<'a>(&'a self, accepting_states: Self::AcceptingState<'a>) -> String;

    fn display(&self) -> DisplayBuchi<'_, S, AP, Self>
    where
        Self: Sized,
    {
        DisplayBuchi(self, PhantomData)
    }
}

pub trait BuchiLikeMut<S, AP: AtomicProperty>: BuchiLike<S, AP> {
    fn push(&mut self, state: S) -> Self::NodeId;
    fn add_accepting_state<'a>(&'a mut self, state: Self::AcceptingState<'a>)
    where
        S: 'a,
        AP: 'a;
    fn add_init_state(&mut self, node_id: Self::NodeId);
    fn add_transition(&mut self, from: Self::NodeId, to: Self::NodeId, labels: Neighbors<AP>);
}

pub struct DisplayBuchi<'a, S, AP: AtomicProperty, B: BuchiLike<S, AP>>(
    &'a B,
    PhantomData<(S, AP)>,
);

impl<S: State, AP: AtomicProperty + fmt::Display, B: BuchiLike<S, AP>> fmt::Display
    for DisplayBuchi<'_, S, AP, B>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "States:")?;
        for state in self.0.nodes().sorted_by_key(|s| self.0.fmt_node(*s)) {
            writeln!(f, " {} []", self.0.fmt_node(state))?;
            for (adj, adj_labels) in self.0.adj_labels(state) {
                writeln!(
                    f,
                    "   =[{}]=> {}",
                    match &*adj_labels {
                        Neighbors::Any => "*".to_string(),
                        Neighbors::Just(labels) =>
                            labels.iter().map(|ap| ap.iter().format(",")).join(" | "),
                    },
                    self.0.fmt_node(adj)
                )?;
            }
        }
        writeln!(
            f,
            "Initial: {}",
            self.0.init_states().map(|s| self.0.fmt_node(s)).format(" ")
        )?;
        writeln!(
            f,
            "Accept:  [{}]",
            self.0
                .accepting_states()
                .map(|s| self.0.fmt_accepting_state(s))
                .format(", ")
        )?;
        Ok(())
    }
}

///  generalized Büchi automaton (GBA) automaton.
/// The difference with the Büchi automaton is its accepting condition, i.e., a set of sets of states.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeneralBuchi<S, AP: AtomicProperty> {
    alphabet: Alphabet<AP>,
    nodes: NodeArena<BuchiNode<S, AP>>,
    accepting_states: Vec<NodeSet<BuchiNode<S, AP>>>,
    init_states: NodeSet<BuchiNode<S, AP>>,
}

impl<S: State, AP: AtomicProperty> BuchiLike<S, AP> for GeneralBuchi<S, AP> {
    type NodeId = BuchiNodeId<S, AP>;
    type AcceptingState<'a>
        = &'a NodeSet<BuchiNode<S, AP>>
    where
        S: 'a,
        AP: 'a;

    fn nodes(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.nodes.ids()
    }

    fn init_states(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.init_states.iter()
    }

    fn accepting_states<'a>(&'a self) -> impl Iterator<Item = Self::AcceptingState<'a>> + Clone + 'a
    where
        Self: 'a,
    {
        self.accepting_states.iter()
    }

    fn is_accepting_state(&self, node_id: Self::NodeId) -> bool {
        self.accepting_states
            .iter()
            .all(|s: &NodeSet<BuchiNode<S, AP>>| s.contains(node_id))
    }

    fn adj_labels<'a>(
        &'a self,
        id: Self::NodeId,
    ) -> impl Iterator<Item = (Self::NodeId, Cow<'a, Neighbors<AP>>)> + Clone + 'a
    where
        AP: 'a,
    {
        self.nodes[id]
            .adj
            .iter()
            .map(|(id, n)| (id, Cow::Borrowed(n)))
    }

    fn adj_ids(&self, id: Self::NodeId) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.nodes[id].adj.ids()
    }

    fn alphabet(&self) -> Cow<'_, Alphabet<AP>> {
        Cow::Borrowed(&self.alphabet)
    }

    fn fmt_node(&self, id: Self::NodeId) -> String {
        format!("{:?}", self.id(id))
    }

    fn fmt_accepting_state<'a>(&'a self, accepting_state: Self::AcceptingState<'a>) -> String {
        format!(
            "{{{:?}}}",
            accepting_state.iter().map(|s| self.id(s)).format(" ")
        )
    }
}

impl<S: State, AP: AtomicProperty> BuchiLikeMut<S, AP> for GeneralBuchi<S, AP> {
    fn push(&mut self, state: S) -> Self::NodeId {
        self.get_node(&state)
            .unwrap_or_else(|| self.nodes.push(BuchiNode::new(state)))
    }

    fn add_accepting_state<'a>(&'a mut self, ids: Self::AcceptingState<'a>)
    where
        S: 'a,
        AP: 'a,
    {
        self.accepting_states.push(ids.clone());
    }

    fn add_init_state(&mut self, node_id: BuchiNodeId<S, AP>) {
        self.init_states.insert(node_id);
    }

    fn add_transition(
        &mut self,
        from: BuchiNodeId<S, AP>,
        to: BuchiNodeId<S, AP>,
        labels: Neighbors<AP>,
    ) {
        self.nodes[from].adj.insert(to, labels);
    }
}

impl<S: State, AP: AtomicProperty> GeneralBuchi<S, AP> {
    pub fn new(alphabet: Alphabet<AP>) -> Self {
        Self {
            alphabet,
            nodes: Default::default(),
            accepting_states: Default::default(),
            init_states: Default::default(),
        }
    }

    pub fn get_node(&self, name: &S) -> Option<BuchiNodeId<S, AP>> {
        self.nodes
            .iter_with_ids()
            .find_map(|(id, adj)| if &adj.id == name { Some(id) } else { None })
    }

    pub fn id(&self, node_id: BuchiNodeId<S, AP>) -> &S {
        &self.nodes[node_id].id
    }
}

impl<S: State, AP: AtomicProperty> std::ops::Index<BuchiNodeId<S, AP>> for GeneralBuchi<S, AP> {
    type Output = BuchiNode<S, AP>;

    fn index(&self, index: BuchiNodeId<S, AP>) -> &Self::Output {
        &self.nodes[index]
    }
}

pub type BuchiNodeId<S, AP> = NodeId<BuchiNode<S, AP>>;

/// Büchi automaton is a type of ω-automaton, which extends
/// a finite automaton to infinite inputs.
#[derive(Debug, Clone)]
pub struct Buchi<S, AP: AtomicProperty> {
    alphabet: Alphabet<AP>,
    mapping: HashMap<S, BuchiNodeId<S, AP>>,
    nodes: NodeArena<BuchiNode<S, AP>>,
    accepting_states: SmartNodeSet<BuchiNode<S, AP>>,
    init_states: SmartNodeSet<BuchiNode<S, AP>>,
}

impl<S: State, AP: AtomicProperty> BuchiLike<S, AP> for Buchi<S, AP> {
    type NodeId = BuchiNodeId<S, AP>;
    type AcceptingState<'a>
        = BuchiNodeId<S, AP>
    where
        S: 'a,
        AP: 'a;

    fn nodes(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.nodes.ids()
    }

    fn init_states(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.init_states.iter()
    }

    fn accepting_states<'a>(&'a self) -> impl Iterator<Item = Self::AcceptingState<'a>> + Clone + 'a
    where
        Self: 'a,
    {
        self.accepting_states.iter()
    }

    fn is_accepting_state(&self, node_id: Self::NodeId) -> bool {
        self.accepting_states.contains(node_id)
    }

    fn adj_labels<'a>(
        &'a self,
        id: Self::NodeId,
    ) -> impl Iterator<Item = (Self::NodeId, Cow<'a, Neighbors<AP>>)> + Clone + 'a
    where
        AP: 'a,
    {
        self.nodes[id]
            .adj
            .iter()
            .map(|(id, n)| (id, Cow::Borrowed(n)))
    }

    fn adj_ids(&self, id: Self::NodeId) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.nodes[id].adj.ids()
    }

    fn alphabet(&self) -> Cow<'_, Alphabet<AP>> {
        Cow::Borrowed(&self.alphabet)
    }

    fn fmt_node(&self, id: Self::NodeId) -> String {
        format!("{:?}", self.id(id))
    }

    fn fmt_accepting_state<'a>(&'a self, accepting_state: Self::AcceptingState<'a>) -> String {
        self.fmt_node(accepting_state)
    }
}

impl<S: State, AP: AtomicProperty> BuchiLikeMut<S, AP> for Buchi<S, AP> {
    fn push(&mut self, state: S) -> Self::NodeId {
        self.get_node(&state).unwrap_or_else(|| {
            let id = self.nodes.push(BuchiNode::new(state.clone()));
            self.mapping.insert(state, id);
            id
        })
    }

    fn add_accepting_state<'a>(&'a mut self, node_id: Self::AcceptingState<'a>)
    where
        S: 'a,
        AP: 'a,
    {
        self.accepting_states.insert(node_id);
    }

    fn add_init_state(&mut self, node_id: BuchiNodeId<S, AP>) {
        self.init_states.insert(node_id);
    }

    fn add_transition(
        &mut self,
        from: BuchiNodeId<S, AP>,
        to: BuchiNodeId<S, AP>,
        labels: Neighbors<AP>,
    ) {
        self.nodes[from].adj.insert(to, labels);
    }
}

impl<S: State, AP: AtomicProperty> Buchi<S, AP> {
    pub fn new(alphabet: Alphabet<AP>) -> Self {
        Self {
            alphabet,
            nodes: Default::default(),
            mapping: Default::default(),
            accepting_states: Default::default(),
            init_states: Default::default(),
        }
    }

    pub fn get_node(&self, name: &S) -> Option<BuchiNodeId<S, AP>> {
        self.mapping.get(name).copied()
    }

    pub fn id(&self, node_id: BuchiNodeId<S, AP>) -> &S {
        &self.nodes[node_id].id
    }

    pub fn add_necessary_self_loops(&mut self) {
        for state in self.nodes().collect_vec() {
            if self.adj_ids(state).next().is_none() {
                let neighbors = self
                    .nodes()
                    .flat_map(|id| self.adj_labels(id))
                    .filter_map(|(id, adj)| if id == state { Some(adj) } else { None })
                    .fold(Neighbors::none(), |a, b| a.union(&*b));

                // self.add_transition(state, state, Neighbors::any(self.alphabet()));
                self.add_transition(state, state, neighbors);
            }
        }
    }

    pub fn pruned(&self) -> Buchi<S, AP> {
        let mut pruned: Buchi<S, AP> = Buchi::new(self.alphabet().into_owned());
        let mut stack = self.init_states().collect_vec();
        let mut visited = NodeSet::default();
        let mut mapping: HashMap<BuchiNodeId<S, AP>, BuchiNodeId<S, AP>> = HashMap::default();

        while let Some(state) = stack.pop() {
            visited.insert(state);

            let new_state = *mapping
                .entry(state)
                .or_insert_with(|| pruned.push(self.id(state).clone()));

            for (adj, labels) in self.adj_labels(state) {
                let new_adj = *mapping
                    .entry(adj)
                    .or_insert_with(|| pruned.push(self.id(adj).clone()));
                pruned.add_transition(new_state, new_adj, labels.into_owned());
                if !visited.insert(adj) {
                    stack.push(adj);
                }
            }
        }

        for state in self.init_states() {
            pruned.add_init_state(mapping[&state]);
        }

        for state in self.accepting_states() {
            if let Some(id) = mapping.get(&state) {
                pruned.add_accepting_state(*id);
            }
        }

        pruned
    }

    /// Product of the program and the property
    /// Let `A1 = (S1 ,Σ1 , ∆1 ,I1 ,F1)`
    /// and  `A2 = (S2 ,Σ2 , ∆2 ,I2 ,F2 )` be two automata.
    ///
    /// We define `A1 × A2` , as the quituple:
    /// `(S,Σ,∆,I,F) := (S1 × S2, Σ1 × Σ2, ∆1 × ∆2, I1 × I2, F1 × F2)`,
    ///
    /// where where ∆ is a function from `S × Σ` to `P(S1) × P(S2) ⊆ P(S)`,
    ///
    /// given by `∆((q1, q2), a, (q1', q2')) ∈ ∆`
    /// iff `(q1, a, q1') ∈ ∆1`
    /// and `(q2, a, q2') ∈ ∆2`
    pub fn product<'a, 'b, T: State>(
        &'a self,
        other: &'b Buchi<T, AP>,
    ) -> ProductBuchi<'a, 'b, S, T, AP> {
        ProductBuchi::new(self, other)
    }
}

pub struct ProductBuchi<'a, 'b, S, T, AP: AtomicProperty> {
    a: &'a Buchi<S, AP>,
    b: &'b Buchi<T, AP>,
    adj_ids_cache: Mutex<
        AHashMap<
            ProductBuchiNodeId<S, T, AP>,
            smallvec::SmallVec<[ProductBuchiNodeId<S, T, AP>; 16]>,
        >,
    >,
}

pub type ProductBuchiNodeId<S, T, AP> = (BuchiNodeId<S, AP>, BuchiNodeId<T, AP>);

pub struct ProductBuchiNodeSet<S, T, AP: AtomicProperty>(
    NodeMap<BuchiNode<S, AP>, SmartNodeSet<BuchiNode<T, AP>>>,
);

impl<S, T: State, AP: AtomicProperty> Default for ProductBuchiNodeSet<S, T, AP> {
    fn default() -> Self {
        Self(NodeMap::new())
    }
}

impl<S, T: State, AP: AtomicProperty> ProductBuchiNodeSet<S, T, AP> {
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn insert(&mut self, node: ProductBuchiNodeId<S, T, AP>) {
        if self.0.contains_key(node.0) {
            self.0[node.0].insert(node.1);
        } else {
            let mut set = SmartNodeSet::new();
            set.insert(node.1);
            self.0.insert(node.0, set);
        }
    }
    pub fn contains(&self, node: ProductBuchiNodeId<S, T, AP>) -> bool {
        self.0.get(node.0).map_or(false, |set| set.contains(node.1))
    }
}

impl<S: State, T: State, AP: AtomicProperty> BuchiLike<(S, T), AP>
    for ProductBuchi<'_, '_, S, T, AP>
{
    type NodeId = ProductBuchiNodeId<S, T, AP>;
    type AcceptingState<'a>
        = ProductBuchiNodeId<S, T, AP>
    where
        Self: 'a;

    fn nodes(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        let mut nodes = AHashSet::default();

        let mut queue = self.init_states().collect_vec();

        while let Some(node) = queue.pop() {
            nodes.insert(node);
            for adj in self.adj_ids(node) {
                if !nodes.contains(&adj) {
                    queue.push(adj);
                }
            }
        }

        // HACK: This is stupid, but AHashSet::into_iter, does not impl Clone
        nodes.into_iter().collect_vec().into_iter()
    }

    fn init_states(&self) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        Itertools::cartesian_product(self.a.init_states(), self.b.init_states())
    }

    fn accepting_states<'a>(&'a self) -> impl Iterator<Item = Self::AcceptingState<'a>> + Clone + 'a
    where
        Self: 'a,
    {
        Itertools::cartesian_product(self.a.accepting_states(), self.b.accepting_states())
    }

    fn is_accepting_state(&self, node_id: Self::NodeId) -> bool {
        self.a.is_accepting_state(node_id.0) && self.b.is_accepting_state(node_id.1)
    }

    fn adj_labels<'a>(
        &'a self,
        (a, b): Self::NodeId,
    ) -> impl Iterator<Item = (Self::NodeId, Cow<'a, Neighbors<AP>>)> + Clone + 'a
    where
        AP: 'a,
    {
        Itertools::cartesian_product(self.a.adj_labels(a), self.b.adj_labels(b)).filter_map(
            move |((a, a_labels), (b, b_labels))| {
                let dst = (a, b);
                let dst_labels = a_labels.intersection(&*b_labels);
                if dst_labels.is_empty() {
                    None
                } else {
                    Some((dst, Cow::Owned(dst_labels)))
                }
            },
        )
    }

    fn adj_ids(&self, (a, b): Self::NodeId) -> impl Iterator<Item = Self::NodeId> + Clone + '_ {
        self.adj_ids_cache
            .lock()
            .unwrap()
            .entry((a, b))
            .or_insert_with(|| {
                Itertools::cartesian_product(self.a.adj_labels(a), self.b.adj_labels(b))
                    .filter_map(move |((a, a_labels), (b, b_labels))| {
                        if a_labels.is_disjoint(&*b_labels) {
                            None
                        } else {
                            Some((a, b))
                        }
                    })
                    .collect()
            })
            .clone()
            .into_iter()
    }

    fn alphabet(&self) -> Cow<'_, Alphabet<AP>> {
        // TODO: should we make sure that we include both?
        Cow::Owned(self.a.alphabet.union(&self.b.alphabet))
    }

    fn fmt_node(&self, id: Self::NodeId) -> String {
        format!("({:?}, {:?})", self.a.id(id.0), self.b.id(id.1))
    }

    fn fmt_accepting_state<'a>(&'a self, accepting_state: Self::AcceptingState<'a>) -> String {
        self.fmt_node(accepting_state)
    }
}

impl<'a, 'b, S, T, AP> ProductBuchi<'a, 'b, S, T, AP>
where
    S: State,
    T: State,
    AP: AtomicProperty,
{
    pub fn new(a: &'a Buchi<S, AP>, b: &'b Buchi<T, AP>) -> Self {
        Self {
            a,
            b,
            adj_ids_cache: Default::default(),
        }
    }
}

impl<S: State, AP: AtomicProperty> std::ops::Index<BuchiNodeId<S, AP>> for Buchi<S, AP> {
    type Output = BuchiNode<S, AP>;

    fn index(&self, index: BuchiNodeId<S, AP>) -> &Self::Output {
        &self.nodes[index]
    }
}

/// Multiple sets of states in acceptance condition can be translated into one set of states
/// by an automata construction, which is known as "counting construction".
/// Let's say `A = (Q, Σ, ∆, q0, {F1,...,Fn})` is a GBA, where `F1,...,Fn` are sets of accepting states
/// then the equivalent Büchi automaton is `A' = (Q', Σ, ∆',q'0,F')`, where
/// * `Q' = Q × {1,...,n}`
/// * `q'0 = ( q0,1 )`
/// * `∆' = { ( (q,i), a, (q',j) ) | (q,a,q') ∈ ∆ and if q ∈ Fi then j=((i+1) mod n) else j=i }`
/// * `F'=F1× {1}`
impl<S: State, AP: AtomicProperty> GeneralBuchi<S, AP> {
    pub fn to_buchi(&self) -> Buchi<(S, usize), AP> {
        let mut ba: Buchi<(S, usize), AP> = Buchi::new(self.alphabet().into_owned());

        if self.accepting_states.is_empty() {
            // tracing::debug!(%self, "no accepting states found, adding all states as accepting states");
            let mut gb = self.clone();
            let accepting_states = gb.nodes().collect();
            gb.add_accepting_state(&accepting_states);
            return gb.to_buchi();
        }
        // let F = {F0,...,Fk-1}

        // Q' = Q × 0..k
        for (k, _) in self.accepting_states().enumerate() {
            for n in self.nodes() {
                ba.push((self.id(n).clone(), k));
            }
        }

        // Q'0 = Q0 × {0} = { (q0,0) | q0 ∈ Q0 }
        for n in self.init_states() {
            let init = ba.push((self.id(n).clone(), 0));
            ba.add_init_state(init);
        }

        // F' = F1 × {0} = { (qF,0) | qF ∈ F1 }
        for f in self.accepting_states().next().unwrap().iter() {
            let accepting = ba.push((self.id(f).clone(), 0));
            ba.add_accepting_state(accepting);
        }

        // ∆'((q, i), A) = if q ∈ Fi then { (q', i+1) | q' ∈ ∆(q, A) } else { (q', i) | q' ∈ ∆(q, A) }
        for (i, f) in self.accepting_states().enumerate() {
            for n in self.nodes() {
                for (adj, adj_labels) in self.adj_labels(n) {
                    let j = if f.iter().any(|m| self.id(n) == self.id(m)) {
                        (i + 1) % self.accepting_states.len()
                    } else {
                        i
                    };
                    let new = ba.push((self.id(adj).clone(), j));
                    ba.add_transition(
                        ba.get_node(&(self.id(n).clone(), i)).unwrap(),
                        new,
                        adj_labels.into_owned(),
                    );
                }
            }
        }

        ba
    }
}

#[cfg(test)]
mod tests;
