use std::collections::BTreeMap;

use gcl::{
    ast::{AExpr, Array, BExpr, Int, Target, Variable},
    pg::{
        Action, Edge, ProgramGraph,
        analysis::{Direction, MonotoneFramework},
    },
    semantics::SemanticsError,
};
use indexmap::IndexSet;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};

// use crate::analysis::{Direction, MonotoneFramework};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignAnalysis {
    pub assignment: SignMemory,
}

#[derive(
    tapi::Tapi,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[tapi(path = "SignAnalysis")]
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
    // TODO: derive tapi::Tapi
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
    #[serde(into = "Vec<Sign>", try_from = "Vec<Sign>")]
    pub struct Signs: u8 {
        const POSITIVE = 0b001;
        const ZERO = 0b010;
        const NEGATIVE = 0b100;
        const ALL = Self::POSITIVE.bits() | Self::ZERO.bits() | Self::NEGATIVE.bits();
    }
}

impl tapi::Tapi for Signs {
    fn name() -> &'static str {
        "Signs"
    }

    fn kind() -> tapi::kind::TypeKind {
        tapi::kind::TypeKind::List(Sign::boxed())
    }

    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}

impl Default for Signs {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Display for Signs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.signs().format(", "))
    }
}

impl From<Signs> for Vec<Sign> {
    fn from(value: Signs) -> Self {
        value.signs().collect()
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
        serde_json::to_string(&Signs::ALL.signs().collect::<BTreeSet<_>>()).unwrap()
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
    pub fn signs(self) -> impl Iterator<Item = Sign> + Clone {
        [Sign::Positive, Sign::Zero, Sign::Negative]
            .into_iter()
            .filter(move |&s| self.contains(s.into()))
    }
    pub fn map(self, f: impl FnMut(Sign) -> Sign) -> Signs {
        self.signs().map(f).collect()
    }
}
impl FromIterator<Sign> for Signs {
    fn from_iter<T: IntoIterator<Item = Sign>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Signs::empty(), |acc, s| acc | s.into())
    }
}
bitflags::bitflags! {
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
    #[serde(into = "Vec<bool>", try_from = "Vec<bool>")]
    pub struct Bools: u8 {
        const TRUE = 0b10;
        const FALSE = 0b01;
        const ALL = Self::TRUE.bits() | Self::FALSE.bits();
    }
}

impl Default for Bools {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Display for Bools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.iter().format(", "))
    }
}

impl From<Bools> for Vec<bool> {
    fn from(value: Bools) -> Self {
        value.bools().collect()
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
        serde_json::to_string(&Bools::ALL.bools().collect::<BTreeSet<_>>()).unwrap()
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
    pub fn bools(self) -> impl Iterator<Item = bool> + Clone {
        [false, true]
            .into_iter()
            .filter(move |&s| self.contains(s.into()))
    }
    pub fn map(self, f: impl FnMut(bool) -> bool) -> Bools {
        self.bools().map(f).collect()
    }
}
impl FromIterator<bool> for Bools {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Bools::empty(), |acc, s| acc | s.into())
    }
}

#[derive(tapi::Tapi, Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[tapi(path = "SignAnalysis")]
pub struct SignMemory {
    pub variables: BTreeMap<Variable, Sign>,
    pub arrays: BTreeMap<Array, Signs>,
}
impl SignMemory {
    pub fn with_var(mut self, var: &Variable, value: Sign) -> Self {
        *self
            .variables
            .get_mut(var)
            .unwrap_or_else(|| panic!("variable `{var}` not declared")) = value;
        self
    }
    pub fn get_var(&self, var: &Variable) -> Option<Sign> {
        self.variables.get(var).copied()
    }
    pub fn get_arr(&self, arr: &Array) -> Option<Signs> {
        self.arrays.get(arr).copied()
    }
}
impl From<gcl::memory::Memory<Sign, Signs>> for SignMemory {
    fn from(mem: gcl::memory::Memory<Sign, Signs>) -> Self {
        Self {
            variables: mem.variables,
            arrays: mem.arrays,
        }
    }
}

impl MonotoneFramework for SignAnalysis {
    type Domain = IndexSet<SignMemory>;

    fn semantic(&self, _pg: &ProgramGraph, e: &Edge, prev: &Self::Domain) -> Self::Domain {
        match e.action() {
            Action::Assignment(Target::Variable(var), x) => prev
                .iter()
                .flat_map(|mem| x.semantics_sign(mem).signs().map(move |s| (mem, s)))
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

                        let mut new_possible = IndexSet::new();

                        for s in std::iter::once(None).chain(array_signs.iter().map(Some)) {
                            let mut signs = array_signs;
                            if let Some(s) = s {
                                signs.remove(s);
                            }
                            for new_sign in expr.semantics_sign(mem).iter() {
                                let new_signs = signs | new_sign;
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

fn cartesian_flat_map<'a, L, R, T: Clone, Q>(
    l: L,
    r: R,
    f: impl Fn(T, Option<T>) -> Q + 'a,
) -> impl Iterator<Item = Q> + 'a
where
    L: 'a + IntoIterator<Item = T> + Clone,
    L::IntoIter: Clone,
    R: 'a + IntoIterator<Item = T> + Clone,
    R::IntoIter: Clone,
{
    if r.clone().into_iter().next().is_none() {
        Either::Left(l.into_iter().map(move |a| f(a, None)))
    } else {
        Either::Right(
            l.into_iter()
                .cartesian_product(r)
                .map(move |(a, b)| f(a, Some(b))),
        )
    }
}

trait SemanticSign {
    type Items;

    fn semantics_sign(&self, mem: &SignMemory) -> Self::Items;
}

impl SemanticSign for BExpr {
    type Items = Bools;

    fn semantics_sign(&self, mem: &SignMemory) -> Bools {
        match self {
            BExpr::Bool(b) => [*b].into_iter().collect(),
            BExpr::Rel(l, op, r) => {
                let l = l.semantics_sign(mem);
                let r = r.semantics_sign(mem);
                cartesian_flat_map(
                    l.signs().flat_map(|s| s.representative()),
                    r.signs().flat_map(|s| s.representative()),
                    |l, r| Some(op.semantic(l, r?)),
                )
                .flatten()
                .collect()
            }
            BExpr::Logic(l, op, r) => {
                let l = l.semantics_sign(mem);
                let r = r.semantics_sign(mem);
                cartesian_flat_map(l.bools(), r.bools(), |l, r| {
                    op.semantic(l, || r.ok_or(SemanticsError::NoProgression))
                })
                .flatten()
                .collect()
            }
            BExpr::Not(b) => b.semantics_sign(mem).map(|i| !i),
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

impl SemanticSign for AExpr {
    type Items = Signs;

    fn semantics_sign(&self, mem: &SignMemory) -> Signs {
        match self {
            AExpr::Number(n) => [sign_of(*n)].into_iter().collect(),
            AExpr::Reference(Target::Variable(x)) => [mem
                .get_var(x)
                .unwrap_or_else(|| panic!("could not get sign of '{x}'"))]
            .into_iter()
            .collect(),
            AExpr::Binary(l, op, r) => cartesian_flat_map(
                l.semantics_sign(mem)
                    .signs()
                    .flat_map(|x| x.representative()),
                r.semantics_sign(mem)
                    .signs()
                    .flat_map(|x| x.representative()),
                |l, r| Some(op.semantic(l, r?)),
            )
            .flatten()
            .filter_map(|res| match res {
                Ok(mem) => Some(mem),
                Err(err) => match err {
                    SemanticsError::DivisionByZero
                    | SemanticsError::NegativeExponent
                    | SemanticsError::EvaluateQuantifier => None,
                    SemanticsError::VariableNotFound { .. }
                    | SemanticsError::ArrayNotFound { .. }
                    | SemanticsError::IndexOutOfBound { .. }
                    | SemanticsError::NoProgression
                    | SemanticsError::OutsideFunctionDomain
                    | SemanticsError::ArithmeticOverflow => unreachable!(),
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
        }
    }
}
