use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
};

use itertools::{chain, Either, Itertools};
use serde::{Deserialize, Serialize};

use crate::{
    analysis::{Direction, MonotoneFramework},
    ast::{AExpr, Array, BExpr, Int, Target, Variable},
    interpreter::InterpreterError,
    pg::{Action, Edge, ProgramGraph},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignAnalysis {
    pub assignment: SignMemory,
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(tag = "Case")]
pub enum Sign {
    #[default]
    Positive,
    Zero,
    Negative,
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
    fn representative(self) -> impl Iterator<Item = Int> + Clone {
        match self {
            Sign::Positive => itertools::Either::Left([1, 2]),
            Sign::Zero => itertools::Either::Right([0]),
            Sign::Negative => itertools::Either::Left([-1, -2]),
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

impl Default for Signs {
    fn default() -> Self {
        Self::NONE
    }
}

impl std::fmt::Display for Signs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.iter().format(", "))
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
    pub fn iter(self) -> impl Iterator<Item = Sign> + Clone {
        [Sign::Positive, Sign::Zero, Sign::Negative]
            .into_iter()
            .filter(move |&s| self.contains(s.into()))
    }
    pub fn map(self, f: impl FnMut(Sign) -> Sign) -> Signs {
        self.iter().map(f).collect()
    }
}
impl FromIterator<Sign> for Signs {
    fn from_iter<T: IntoIterator<Item = Sign>>(iter: T) -> Self {
        iter.into_iter().fold(Signs::NONE, |acc, s| acc | s.into())
    }
}
bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    #[serde(into = "Vec<bool>", try_from = "Vec<bool>")]
    pub struct Bools: u8 {
        const NONE = 0b00;
        const FALSE = 0b01;
        const TRUE = 0b10;
        const ALL = Self::TRUE.bits | Self::FALSE.bits;
    }
}

impl Default for Bools {
    fn default() -> Self {
        Self::NONE
    }
}

impl std::fmt::Display for Bools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.iter().format(", "))
    }
}

impl From<Bools> for Vec<bool> {
    fn from(value: Bools) -> Self {
        value.iter().collect()
    }
}
impl From<Vec<bool>> for Bools {
    fn from(value: Vec<bool>) -> Self {
        value.into_iter().collect()
    }
}

#[test]
fn bools_as_json() {
    use std::collections::BTreeSet;
    assert_eq!(
        serde_json::to_string(&Bools::ALL).unwrap(),
        serde_json::to_string(&Bools::ALL.iter().collect::<BTreeSet<_>>()).unwrap()
    );
}

impl From<bool> for Bools {
    fn from(value: bool) -> Self {
        match value {
            false => Bools::FALSE,
            true => Bools::TRUE,
        }
    }
}

impl Bools {
    pub fn iter(self) -> impl Iterator<Item = bool> + Clone {
        [false, true]
            .into_iter()
            .filter(move |&s| self.contains(s.into()))
    }
    pub fn map(self, f: impl FnMut(bool) -> bool) -> Bools {
        self.iter().map(f).collect()
    }
}
impl FromIterator<bool> for Bools {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        iter.into_iter().fold(Bools::NONE, |acc, s| acc | s.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Memory<T, A = T> {
    pub variables: BTreeMap<Variable, T>,
    pub arrays: BTreeMap<Array, A>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryRef<'a, T, A> {
    Variable(&'a Variable, &'a T),
    Array(&'a Array, &'a A),
}

impl<'a, T, A> MemoryRef<'a, T, A> {
    pub fn target(&self) -> Target {
        match self {
            MemoryRef::Variable(t, _) => Target::Variable((*t).clone()),
            MemoryRef::Array(t, _) => Target::Array((*t).clone(), ()),
        }
    }
}

impl<'a, T> MemoryRef<'a, T, T> {
    pub fn value(&self) -> &T {
        match self {
            MemoryRef::Variable(_, v) | MemoryRef::Array(_, v) => v,
        }
    }
}

impl<'a, T, A> std::fmt::Display for MemoryRef<'a, T, A>
where
    T: std::fmt::Display,
    A: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryRef::Variable(a, b) => write!(f, "{a} = {b}"),
            MemoryRef::Array(a, b) => write!(f, "{a} = {b}"),
        }
    }
}

pub type SignMemory = Memory<Sign, Signs>;

impl<T, A> Memory<T, A> {
    pub fn from_targets(
        targets: impl IntoIterator<Item = Target>,
        mut f_var: impl FnMut(&Variable) -> T,
        mut f_array: impl FnMut(&Array) -> A,
    ) -> Self {
        let mut variables = BTreeMap::new();
        let mut arrays = BTreeMap::new();

        for t in targets {
            match t {
                Target::Variable(var) => {
                    let value = f_var(&var);
                    variables.insert(var, value);
                }
                Target::Array(arr, ()) => {
                    let value = f_array(&arr);
                    arrays.insert(arr, value);
                }
            }
        }

        Self { variables, arrays }
    }
    pub fn from_targets_with<W>(
        targets: impl IntoIterator<Item = Target>,
        mut with: W,
        mut f_var: impl for<'a, 'b> FnMut(&'a mut W, &'b Variable) -> T,
        mut f_array: impl for<'a, 'b> FnMut(&'a mut W, &'b Array) -> A,
    ) -> Self {
        let mut variables = BTreeMap::new();
        let mut arrays = BTreeMap::new();

        for t in targets {
            match t {
                Target::Variable(var) => {
                    let value = f_var(&mut with, &var);
                    variables.insert(var, value);
                }
                Target::Array(arr, ()) => {
                    let value = f_array(&mut with, &arr);
                    arrays.insert(arr, value);
                }
            }
        }

        Self { variables, arrays }
    }

    pub fn iter(&self) -> impl Iterator<Item = MemoryRef<T, A>> + Clone {
        chain!(
            self.variables
                .iter()
                .map(|(var, value)| MemoryRef::Variable(var, value)),
            self.arrays
                .iter()
                .map(|(arr, value)| MemoryRef::Array(arr, value)),
        )
    }

    pub fn with_var(mut self, var: &Variable, value: T) -> Self {
        *self
            .variables
            .get_mut(var)
            .unwrap_or_else(|| panic!("variable `{var}` not declared")) = value;
        self
    }
    pub fn get_var(&self, var: &Variable) -> Option<&T> {
        self.variables.get(var)
    }
    pub fn get_arr(&self, arr: &Array) -> Option<&A> {
        self.arrays.get(arr)
    }
}

impl MonotoneFramework for SignAnalysis {
    type Domain = HashSet<SignMemory>;

    fn semantic(&self, _pg: &ProgramGraph, e: &Edge, prev: &Self::Domain) -> Self::Domain {
        match e.action() {
            Action::Assignment(Target::Variable(var), x) => prev
                .iter()
                .flat_map(|mem| x.semantics_sign(mem).iter().map(move |s| (mem, s)))
                .map(|(mem, s)| mem.clone().with_var(var, s))
                .collect(),
            Action::Assignment(Target::Array(arr, idx), expr) => prev
                .iter()
                .flat_map(|mem| {
                    let idx_signs = idx.semantics_sign(mem);
                    if idx_signs.intersects(Signs::ZERO | Signs::POSITIVE) {
                        let array_signs: Signs = mem
                            .arrays
                            .get(arr)
                            .unwrap_or_else(|| panic!("could not get sign of array '{arr}'"))
                            .iter()
                            .collect();

                        let mut new_possible = HashSet::new();

                        for s in std::iter::once(None).chain(array_signs.iter().map(Some)) {
                            let mut signs = array_signs;
                            if let Some(s) = s {
                                signs.remove(s.into());
                            }
                            for new_sign in expr.semantics_sign(mem).iter() {
                                let new_signs = signs | new_sign.into();
                                let mut new_mem = mem.clone();
                                new_mem.arrays.insert(arr.clone(), new_signs);
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
                .filter(|mem| b.semantics_sign(mem).contains(Bools::TRUE))
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
    f: impl Fn(T, Option<T>) -> Q + 'a,
) -> impl Iterator<Item = Q> + 'a
where
    L: IntoIterator<Item = T> + Clone,
    L::IntoIter: Clone,
    R: IntoIterator<Item = T> + Clone,
    R::IntoIter: Clone,
{
    if r.clone().into_iter().next().is_none() {
        Either::Left(l.into_iter().map(move |a| f(a, None)))
    } else {
        Either::Right(
            l.into_iter()
                .cartesian_product(r.into_iter())
                .map(move |(a, b)| f(a, Some(b))),
        )
    }
}

impl BExpr {
    fn semantics_sign(&self, mem: &SignMemory) -> Bools {
        match self {
            BExpr::Bool(b) => [*b].into_iter().collect(),
            BExpr::Rel(l, op, r) => {
                let l = l.semantics_sign(mem);
                let r = r.semantics_sign(mem);
                cartesian_flat_map(
                    l.iter().flat_map(|s| s.representative()),
                    r.iter().flat_map(|s| s.representative()),
                    |l, r| Some(op.semantic(l, r?)),
                )
                .flatten()
                .collect()
            }
            BExpr::Logic(l, op, r) => {
                let l = l.semantics_sign(mem);
                let r = r.semantics_sign(mem);
                cartesian_flat_map(l.iter(), r.iter(), |l, r| {
                    op.semantic(l, || r.ok_or(InterpreterError::NoProgression))
                })
                .flatten()
                .collect()
            }
            BExpr::Not(b) => b.semantics_sign(mem).map(|i| !i),
            BExpr::Quantified(_, _, _) => {
                todo!("tried to compute signs of a quantified expression")
            }
        }
    }
}

fn sign_of(n: Int) -> Sign {
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
    fn semantics_sign(&self, mem: &SignMemory) -> Signs {
        match self {
            AExpr::Number(n) => [sign_of(*n)].into_iter().collect(),
            AExpr::Reference(Target::Variable(x)) => [mem
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
                |l, r| Some(op.semantic(l, r?)),
            )
            .flatten()
            .filter_map(|res| match res {
                Ok(mem) => Some(mem),
                Err(err) => match err {
                    InterpreterError::DivisionByZero
                    | InterpreterError::NegativeExponent
                    | InterpreterError::EvaluateQuantifier => None,
                    InterpreterError::VariableNotFound { .. }
                    | InterpreterError::ArrayNotFound { .. }
                    | InterpreterError::IndexOutOfBound { .. }
                    | InterpreterError::NoProgression
                    | InterpreterError::OutsideFunctionDomain
                    | InterpreterError::ArithmeticOverflow => unreachable!(),
                },
            })
            .map(sign_of)
            .collect(),
            AExpr::Reference(Target::Array(arr, idx)) => {
                let idx_signs = idx.semantics_sign(mem);
                if idx_signs.intersects(Signs::ZERO | Signs::POSITIVE) {
                    if let Some(arr) = mem.arrays.get(arr) {
                        arr.iter().collect()
                    } else {
                        Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            AExpr::Minus(n) => n.semantics_sign(mem).map(|x| -x),
            AExpr::Function(_) => todo!("sign of a function"),
        }
    }
}
