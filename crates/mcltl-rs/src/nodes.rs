use std::{fmt, hash::Hash, marker::PhantomData};

use itertools::Either;
use smallvec::SmallVec;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct NodeArena<N> {
    nodes: Vec<N>,
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct NodeMap<N, T> {
    map: smallvec::SmallVec<[Option<T>; 16]>,
    _ph: PhantomData<N>,
}

impl<N: fmt::Debug, T: fmt::Debug> fmt::Debug for NodeMap<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct NodeSet<N> {
    inner: NodeMap<N, ()>,
}

impl<N: fmt::Debug> fmt::Debug for NodeSet<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

pub struct NodeId<N> {
    id: u32,
    _ph: PhantomData<N>,
}

impl<N: fmt::Debug> fmt::Debug for NodeId<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "N[{}@{}]",
            std::any::type_name::<N>()
                .split("::")
                .last()
                .unwrap()
                .trim_end_matches('>'),
            self.id
        )
    }
}

impl<N> NodeId<N> {
    fn new(id: u32) -> Self {
        Self {
            id,
            _ph: PhantomData,
        }
    }
}

impl<N> Clone for NodeId<N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<N> Copy for NodeId<N> {}
impl<N> PartialEq for NodeId<N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<N> Eq for NodeId<N> {}
impl<N> Hash for NodeId<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<N> Default for NodeArena<N> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
        }
    }
}

impl<N, T> Default for NodeMap<N, T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            _ph: PhantomData,
        }
    }
}

impl<N> Default for NodeSet<N> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<N> NodeId<N> {
    pub fn transmute<M>(&self) -> NodeId<M> {
        NodeId::new(self.id)
    }
}

impl<T> fmt::Display for NodeId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "N{}", self.id)
    }
}

impl<N> NodeArena<N> {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, node: N) -> NodeId<N> {
        let id = self.nodes.len() as u32;
        self.nodes.push(node);
        NodeId::new(id)
    }

    pub fn iter(&self) -> std::slice::Iter<N> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<N> {
        self.nodes.iter_mut()
    }

    pub fn iter_with_ids(&self) -> impl Iterator<Item = (NodeId<N>, &N)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(id, node)| (NodeId::new(id as _), node))
    }

    pub fn ids(&self) -> impl Iterator<Item = NodeId<N>> + Clone {
        (0..self.nodes.len()).map(|id| NodeId::new(id as _))
    }
}

impl<N> std::ops::Index<NodeId<N>> for NodeArena<N> {
    type Output = N;

    fn index(&self, id: NodeId<N>) -> &Self::Output {
        &self.nodes[id.id as usize]
    }
}

impl<N> std::ops::IndexMut<NodeId<N>> for NodeArena<N> {
    fn index_mut(&mut self, id: NodeId<N>) -> &mut Self::Output {
        &mut self.nodes[id.id as usize]
    }
}

impl<S, T> NodeMap<S, T> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
            _ph: PhantomData,
        }
    }

    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn len(&self) -> usize {
        self.map.iter().filter(|x| x.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn insert(&mut self, id: NodeId<S>, value: T) -> Option<T>
    where
        T: Clone,
    {
        let id = id.id as usize;
        if id >= self.map.len() {
            self.map.resize(id + 1, None);
        }
        self.map[id].replace(value)
    }

    pub fn get(&self, id: NodeId<S>) -> Option<&T> {
        self.map.get(id.id as usize).and_then(|x| x.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId<S>) -> Option<&mut T> {
        self.map.get_mut(id.id as usize).and_then(|x| x.as_mut())
    }

    pub fn contains_key(&self, id: NodeId<S>) -> bool {
        self.get(id).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId<S>, &T)> + Clone {
        self.map
            .iter()
            .enumerate()
            .filter_map(|(id, x)| x.as_ref().map(|x| (NodeId::new(id as _), x)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId<S>, &mut T)> {
        self.map
            .iter_mut()
            .enumerate()
            .filter_map(|(id, x)| x.as_mut().map(|x| (NodeId::new(id as _), x)))
    }

    pub fn ids(&self) -> impl Iterator<Item = NodeId<S>> + '_ {
        self.iter().map(|(id, _)| id)
    }
}

impl<S, T> std::ops::Index<NodeId<S>> for NodeMap<S, T> {
    type Output = T;

    fn index(&self, id: NodeId<S>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<S, T> std::ops::IndexMut<NodeId<S>> for NodeMap<S, T> {
    fn index_mut(&mut self, id: NodeId<S>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

impl<T> NodeSet<T> {
    pub fn new() -> Self {
        Self {
            inner: NodeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn insert(&mut self, id: NodeId<T>) -> bool {
        self.inner.insert(id, ()).is_some()
    }

    pub fn contains(&self, id: NodeId<T>) -> bool {
        self.inner.get(id).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = NodeId<T>> + Clone + '_ {
        self.inner
            .map
            .iter()
            .enumerate()
            .filter_map(|(id, x)| x.as_ref().map(|_| NodeId::new(id as _)))
    }

    pub fn clear(&mut self) {
        self.inner.map.clear();
    }
}

impl<T> std::iter::FromIterator<NodeId<T>> for NodeSet<T> {
    fn from_iter<I: IntoIterator<Item = NodeId<T>>>(iter: I) -> Self {
        let mut set = Self::new();
        for id in iter {
            set.insert(id);
        }
        set
    }
}

const SMART_NODE_MAP_CAP: usize = 16;

#[derive(Eq, PartialEq, Clone, Hash)]
pub enum SmartNodeMap<S, T> {
    Small(SmallVec<[(NodeId<S>, T); SMART_NODE_MAP_CAP]>),
    Big(NodeMap<S, T>),
}

impl<N: fmt::Debug, T: fmt::Debug> fmt::Debug for SmartNodeMap<N, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<S, T> Default for SmartNodeMap<S, T> {
    fn default() -> Self {
        Self::Small(Default::default())
    }
}

impl<S, T> SmartNodeMap<S, T> {
    pub fn len(&self) -> usize {
        match self {
            SmartNodeMap::Small(map) => map.len(),
            SmartNodeMap::Big(map) => map.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, id: NodeId<S>, value: T) -> Option<T>
    where
        T: Clone,
    {
        self.upgrade_if_needed();

        match self {
            SmartNodeMap::Small(map) => {
                if let Some((_, old)) = map.iter_mut().find(|(i, _)| *i == id) {
                    Some(std::mem::replace(old, value))
                } else {
                    map.push((id, value));
                    None
                }
            }
            SmartNodeMap::Big(map) => map.insert(id, value),
        }
    }

    fn upgrade_if_needed(&mut self)
    where
        T: Clone,
    {
        if let SmartNodeMap::Small(map) = self {
            if map.len() >= SMART_NODE_MAP_CAP {
                let mut new_map = NodeMap::new();
                for (id, value) in map.drain(..) {
                    new_map.insert(id, value);
                }
                *self = SmartNodeMap::Big(new_map);
            }
        }
    }

    pub fn get(&self, id: NodeId<S>) -> Option<&T> {
        match self {
            SmartNodeMap::Small(map) => map.iter().find(|(i, _)| *i == id).map(|(_, v)| v),
            SmartNodeMap::Big(map) => map.get(id),
        }
    }

    pub fn contains_key(&self, id: NodeId<S>) -> bool {
        match self {
            SmartNodeMap::Small(map) => map.iter().any(|(i, _)| *i == id),
            SmartNodeMap::Big(map) => map.contains_key(id),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId<S>, &T)> + Clone {
        match self {
            SmartNodeMap::Small(map) => Either::Left(map.iter().map(|(id, value)| (*id, value))),
            SmartNodeMap::Big(map) => Either::Right(map.iter()),
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = NodeId<S>> + Clone + '_ {
        self.iter().map(|(id, _)| id)
    }

    pub fn clear(&mut self) {
        *self = SmartNodeMap::default();
    }
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct SmartNodeSet<N> {
    inner: SmartNodeMap<N, ()>,
}

impl<N: fmt::Debug> fmt::Debug for SmartNodeSet<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<N> Default for SmartNodeSet<N> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<T> SmartNodeSet<T> {
    pub fn new() -> Self {
        Self {
            inner: SmartNodeMap::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn insert(&mut self, id: NodeId<T>) -> bool {
        self.inner.insert(id, ()).is_some()
    }

    pub fn contains(&self, id: NodeId<T>) -> bool {
        self.inner.get(id).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = NodeId<T>> + Clone + '_ {
        self.inner.iter().map(|(id, _)| id)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
