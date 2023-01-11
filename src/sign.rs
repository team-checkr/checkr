use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    hash::Hash,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{Direction, MonotoneFramework},
    ast::{AExpr, BExpr, Variable},
    pg::{Action, Edge, ProgramGraph},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignAnalysis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sign {
    Positive,
    Zero,
    Negative,
}

impl Default for Sign {
    fn default() -> Self {
        Sign::Positive
    }
}

impl Sign {
    fn representative(self) -> impl Iterator<Item = i64> + Clone {
        match self {
            Sign::Positive => itertools::Either::Left([1, 2]),
            Sign::Negative => itertools::Either::Right([0]),
            Sign::Zero => itertools::Either::Left([-1, -2]),
        }
        .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Memory<T, A = T> {
    pub variables: BTreeMap<Variable, T>,
    pub arrays: BTreeMap<String, A>,
}

pub type SignMemory = Memory<Sign, BTreeSet<Sign>>;

impl<T, A> Memory<T, A> {
    pub fn with_var(mut self, var: &Variable, value: T) -> Self {
        *self.variables.get_mut(var).unwrap() = value;
        self
    }
    pub fn get_var(&self, var: &Variable) -> Option<&T> {
        self.variables.get(var)
    }
    // pub fn get_arr(&self, arr: &Ar) -> Option<&T> {
    //     self.variables.get(arr)
    // }
}

impl MonotoneFramework for SignAnalysis {
    type Domain = HashSet<SignMemory>;

    fn semantic(&self, _pg: &ProgramGraph, e: &Edge, prev: &Self::Domain) -> Self::Domain {
        match e.action() {
            Action::Assignment(var, x) => prev
                .iter()
                .flat_map(|mem| x.semantics_sign(mem).into_iter().map(move |s| (mem, s)))
                .map(|(mem, s)| mem.clone().with_var(var, s))
                .collect(),
            Action::ArrayAssignment(_, _, _) => todo!(),
            Action::Skip => prev.clone(),
            Action::Condition(b) => prev
                .iter()
                .filter(|mem| b.semantics_sign(mem).contains(&true))
                .cloned()
                .collect(),
        }
    }

    fn direction() -> Direction {
        Direction::Forward
    }

    fn initial(&self, pg: &ProgramGraph) -> Self::Domain {
        [Memory {
            variables: pg
                .fv()
                .iter()
                .map(|v| (v.clone(), Sign::default()))
                .collect(),
            arrays: Default::default(),
        }]
        .into_iter()
        .collect()
    }
}

fn cartesian_flat_map<'a, L: 'a, R: 'a, T: Clone, Q>(
    l: L,
    r: R,
    f: impl Fn(T, T) -> Q + 'a,
) -> impl Iterator<Item = Q> + 'a
where
    L: IntoIterator<Item = T> + Clone,
    L::IntoIter: Clone,
    R: IntoIterator<Item = T> + Clone,
    R::IntoIter: Clone,
    Q: Hash + Eq,
{
    l.into_iter()
        .cartesian_product(r.into_iter())
        .map(move |(a, b)| f(a, b))
}

impl BExpr {
    fn semantics_sign(&self, mem: &SignMemory) -> HashSet<bool> {
        match self {
            BExpr::Bool(b) => [*b].into_iter().collect(),
            BExpr::Rel(l, op, r) => cartesian_flat_map(
                l.semantics_sign(mem)
                    .iter()
                    .flat_map(|s| s.representative()),
                r.semantics_sign(mem)
                    .iter()
                    .flat_map(|s| s.representative()),
                |l, r| op.semantic(l, r),
            )
            .collect(),
            BExpr::Logic(l, op, r) => cartesian_flat_map(
                l.semantics_sign(mem).iter().copied(),
                r.semantics_sign(mem).iter().copied(),
                // TODO: Follow definition with weird $S \cap {ff}$
                |l, r| op.semantic(l, r),
            )
            .collect(),
            BExpr::Not(b) => b.semantics_sign(mem).into_iter().map(|i| !i).collect(),
        }
    }
}

fn sign_of(n: i64) -> Sign {
    match n {
        _ if n > 0 => Sign::Positive,
        _ if n < 0 => Sign::Negative,
        _ => Sign::Zero,
    }
}

impl std::ops::Neg for Sign {
    type Output = Sign;

    fn neg(self) -> Self::Output {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
            Sign::Zero => Sign::Zero,
        }
    }
}

impl AExpr {
    fn semantics_sign(&self, mem: &SignMemory) -> HashSet<Sign> {
        match self {
            AExpr::Number(n) => [sign_of(*n)].into_iter().collect(),
            AExpr::Variable(x) => [mem
                .get_var(x)
                .copied()
                .unwrap_or_else(|| panic!("could not get sign of '{x}'"))]
            .into_iter()
            .collect(),
            AExpr::Binary(l, op, r) => cartesian_flat_map(
                l.semantics_sign(mem)
                    .iter()
                    .flat_map(|x| x.representative()),
                r.semantics_sign(mem)
                    .iter()
                    .flat_map(|x| x.representative()),
                |l, r| op.semantic(l, r),
            )
            .map(sign_of)
            .collect(),
            AExpr::Array(_) => todo!(),
            AExpr::Minus(n) => n.semantics_sign(mem).into_iter().map(|x| -x).collect(),
        }
    }
}
