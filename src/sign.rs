use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{Direction, MonotoneFramework},
    ast::{AExpr, Array, BExpr, Variable},
    interpreter::InterpreterError,
    pg::{Action, Edge, ProgramGraph},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignAnalysis {
    pub assignment: SignMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "Case")]
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

impl std::fmt::Display for Sign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sign::Positive => write!(f, "+"),
            Sign::Zero => write!(f, "0"),
            Sign::Negative => write!(f, "-"),
        }
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

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(into = "Vec<Sign>", try_from = "Vec<Sign>")]
    pub struct Signs: u8 {
        const NONE = 0b000;
        const POSITIVE = 0b001;
        const ZERO = 0b010;
        const NEGATIVE = 0b100;
        const ALL = Self::POSITIVE.bits | Self::ZERO.bits | Self::NEGATIVE.bits;
    }
}

impl From<Signs> for Vec<Sign> {
    fn from(value: Signs) -> Self {
        value.iter().collect()
    }
}
impl From<Vec<Sign>> for Signs {
    fn from(value: Vec<Sign>) -> Self {
        value.into_iter().collect()
    }
}

#[test]
fn signs_as_json() {
    use std::collections::BTreeSet;
    assert_eq!(
        serde_json::to_string(&Signs::ALL).unwrap(),
        serde_json::to_string(&Signs::ALL.iter().collect::<BTreeSet<_>>()).unwrap()
    );
}

impl From<Sign> for Signs {
    fn from(value: Sign) -> Self {
        match value {
            Sign::Positive => Signs::POSITIVE,
            Sign::Zero => Signs::ZERO,
            Sign::Negative => Signs::NEGATIVE,
        }
    }
}

impl Signs {
    pub fn iter(self) -> impl Iterator<Item = Sign> {
        [Sign::Positive, Sign::Zero, Sign::Negative]
            .into_iter()
            .filter(move |&s| self.contains(s.into()))
    }
}
impl FromIterator<Sign> for Signs {
    fn from_iter<T: IntoIterator<Item = Sign>>(iter: T) -> Self {
        iter.into_iter().fold(Signs::NONE, |acc, s| acc | s.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Memory<T, A = T> {
    pub variables: BTreeMap<Variable, T>,
    pub arrays: BTreeMap<String, A>,
}

pub type SignMemory = Memory<Sign, Signs>;

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
            Action::ArrayAssignment(arr, idx, expr) => prev
                .iter()
                .flat_map(|mem| {
                    let idx_signs = idx.semantics_sign(mem);
                    if idx_signs.contains(&Sign::Zero) || idx_signs.contains(&Sign::Positive) {
                        let asdf: Signs = mem
                            .arrays
                            .get(arr)
                            .unwrap_or_else(|| panic!("could not get sign of array '{arr}'"))
                            .iter()
                            .collect();

                        let mut new_possible = HashSet::new();

                        let signs: Signs = asdf.iter().collect();

                        for s in std::iter::once(None).chain(asdf.iter().map(Some)) {
                            let mut signs = signs.clone();
                            if let Some(s) = s {
                                signs.remove(s.into());
                            }
                            for new_sign in expr.semantics_sign(mem) {
                                let mut signs = signs.clone();
                                signs.insert(new_sign.into());
                                let mut new_mem = mem.clone();
                                new_mem.arrays.insert(arr.clone(), signs);
                                new_possible.insert(new_mem);
                            }
                        }

                        new_possible
                    } else {
                        Default::default()
                    }
                })
                .collect(),
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

    fn initial(&self, _pg: &ProgramGraph) -> Self::Domain {
        [self.assignment.clone()].into_iter().collect()
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
                |l, r| op.semantic(l, || Ok(r)),
            )
            .map(|res| res.expect("this could not error"))
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
            .filter_map(|res| match res {
                Ok(mem) => Some(mem),
                Err(err) => match err {
                    InterpreterError::DivisionByZero | InterpreterError::NegativeExponent => None,
                    InterpreterError::VariableNotFound { .. }
                    | InterpreterError::ArrayNotFound { .. }
                    | InterpreterError::IndexOutOfBound { .. }
                    | InterpreterError::NoProgression
                    | InterpreterError::ArithmeticOverflow => unreachable!(),
                },
            })
            .map(sign_of)
            .collect(),
            AExpr::Array(Array(arr, idx)) => {
                let idx_signs = idx.semantics_sign(mem);
                if idx_signs.contains(&Sign::Zero) || idx_signs.contains(&Sign::Positive) {
                    mem.arrays
                        .get(arr)
                        .unwrap_or_else(|| panic!("could not get sign of array '{arr}'"))
                        .iter()
                        .collect()
                } else {
                    Default::default()
                }
            }
            AExpr::Minus(n) => n.semantics_sign(mem).into_iter().map(|x| -x).collect(),
        }
    }
}
