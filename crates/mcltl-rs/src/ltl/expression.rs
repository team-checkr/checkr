use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;

use smol_str::SmolStr;

use crate::buchi::Alphabet;

use super::parser::{lexer::Lexer, parser};

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Literal(pub SmolStr);

impl From<String> for Literal {
    fn from(s: String) -> Self {
        Literal(s.into())
    }
}
impl From<SmolStr> for Literal {
    fn from(s: SmolStr) -> Self {
        Literal(s)
    }
}
impl<'a> From<&'a String> for Literal {
    fn from(s: &'a String) -> Self {
        Literal(s.into())
    }
}
impl<'a> From<&'a SmolStr> for Literal {
    fn from(s: &'a SmolStr) -> Self {
        Literal(s.clone())
    }
}
impl<'a> From<&'a str> for Literal {
    fn from(s: &'a str) -> Self {
        Literal(s.into())
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum LTLExpressionError {
    True,
    False,
    // In case an invalid variable in references from the expression.
    InvalidVariable,
    // In case of an invalid operation.
    InvalidOperation,
}

/// The inductive set of LTL formulas over AP
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum LTLExpression {
    True,
    False,
    Literal(Literal),
    Not(Box<LTLExpression>),
    And(Box<LTLExpression>, Box<LTLExpression>),
    Or(Box<LTLExpression>, Box<LTLExpression>),
    G(Box<LTLExpression>),
    F(Box<LTLExpression>),
    X(Box<LTLExpression>),
    U(Box<LTLExpression>, Box<LTLExpression>),
    R(Box<LTLExpression>, Box<LTLExpression>),
    V(Box<LTLExpression>, Box<LTLExpression>),
}

impl LTLExpression {
    pub fn parse(formula: &str) -> Result<Self, &'static str> {
        let lexer = Lexer::new(formula);
        parser::parse(lexer).map(|span| span.expr).map_err(|e| e.1)
    }
}

impl TryFrom<&str> for LTLExpression {
    type Error = &'static str;

    fn try_from(formula: &str) -> Result<Self, Self::Error> {
        Self::parse(formula)
    }
}

impl fmt::Display for LTLExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LTLExpression::True => write!(f, "T"),
            LTLExpression::False => write!(f, "⊥"),
            LTLExpression::Literal(l) => write!(f, "{}", l),
            LTLExpression::Not(e) => write!(f, "¬{}", e),
            LTLExpression::And(p, q) => write!(f, "{} ∧ {}", p, q),
            LTLExpression::Or(p, q) => write!(f, "{} ∨ {}", p, q),
            LTLExpression::G(e) => write!(f, "G ({})", e),
            LTLExpression::F(e) => write!(f, "F ({})", e),
            LTLExpression::X(e) => write!(f, "X ({})", e),
            LTLExpression::U(p, q) => write!(f, "({} U {})", p, q),
            LTLExpression::R(p, q) => write!(f, "({} R {})", p, q),
            LTLExpression::V(p, q) => write!(f, "({} V {})", p, q),
        }
    }
}

impl std::ops::BitOr for LTLExpression {
    type Output = LTLExpression;

    fn bitor(self, rhs: LTLExpression) -> LTLExpression {
        LTLExpression::Or(Box::new(self), Box::new(rhs))
    }
}
impl std::ops::BitAnd for LTLExpression {
    type Output = LTLExpression;

    fn bitand(self, rhs: LTLExpression) -> LTLExpression {
        LTLExpression::And(Box::new(self), Box::new(rhs))
    }
}
impl std::ops::Not for LTLExpression {
    type Output = LTLExpression;

    fn not(self) -> LTLExpression {
        LTLExpression::Not(Box::new(self))
    }
}
impl std::ops::Not for &LTLExpression {
    type Output = LTLExpression;

    fn not(self) -> LTLExpression {
        LTLExpression::Not(Box::new(self.clone()))
    }
}

impl LTLExpression {
    pub fn lit(s: impl fmt::Display) -> LTLExpression {
        LTLExpression::Literal(s.to_string().into())
    }

    #[allow(non_snake_case)]
    pub fn U(self, other: LTLExpression) -> LTLExpression {
        LTLExpression::U(Box::new(self), Box::new(other))
    }

    #[allow(non_snake_case)]
    pub fn R(self, other: LTLExpression) -> LTLExpression {
        LTLExpression::R(Box::new(self), Box::new(other))
    }

    #[allow(non_snake_case)]
    pub fn V(self, other: LTLExpression) -> LTLExpression {
        LTLExpression::V(Box::new(self), Box::new(other))
    }

    /// A version of `std::ops::Not` that takes `self` by reference.
    pub fn neg(&self) -> Self {
        Self::Not(Box::new(self.clone()))
    }

    pub fn nnf(&self) -> NnfLtl<Literal> {
        use LTLExpression::*;

        match self {
            True => NnfLtl::Bool(true),
            False => NnfLtl::Bool(false),
            Literal(l) => NnfLtl::lit(l.clone()),
            Not(e) => match e.as_ref() {
                True => NnfLtl::Bool(false),
                False => NnfLtl::Bool(true),
                Literal(l) => NnfLtl::neg_lit(l.clone()),
                And(p, q) => p.neg().nnf() | q.neg().nnf(),
                Or(p, q) => p.neg().nnf() & q.neg().nnf(),
                G(p) => NnfLtl::F(p.neg().nnf()),
                F(p) => NnfLtl::G(p.neg().nnf()),
                X(p) => NnfLtl::X(Box::new(p.neg().nnf())),
                U(p, q) => p.neg().nnf().V(q.neg().nnf()),
                R(p, q) => p.neg().nnf().U(q.neg().nnf()),
                V(p, q) => p.neg().nnf().U(q.neg().nnf()),
                Not(p) => p.nnf(),
            },
            And(p, q) => p.nnf() & q.nnf(),
            Or(p, q) => p.nnf() | q.nnf(),
            G(e) => NnfLtl::G(e.nnf()),
            F(e) => NnfLtl::F(e.nnf()),
            X(e) => NnfLtl::X(Box::new(e.nnf())),
            U(p, q) => p.nnf().U(q.nnf()),
            R(p, q) => p.nnf().R(q.nnf()),
            V(p, q) => p.nnf().V(q.nnf()),
        }
    }
}

impl From<LTLExpression> for NnfLtl<Literal> {
    fn from(ltl: LTLExpression) -> Self {
        ltl.nnf()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum NnfLtl<L> {
    Literal { negated: bool, name: L },
    Bool(bool),
    U(Box<NnfLtl<L>>, Box<NnfLtl<L>>),
    V(Box<NnfLtl<L>>, Box<NnfLtl<L>>),
    Or(Box<NnfLtl<L>>, Box<NnfLtl<L>>),
    And(Box<NnfLtl<L>>, Box<NnfLtl<L>>),
    X(Box<NnfLtl<L>>),
}

impl<L> NnfLtl<L> {
    pub(crate) fn lit(name: impl Into<L>) -> Self {
        NnfLtl::Literal {
            negated: false,
            name: name.into(),
        }
    }

    pub(crate) fn neg_lit(name: impl Into<L>) -> Self {
        NnfLtl::Literal {
            negated: true,
            name: name.into(),
        }
    }

    #[allow(non_snake_case)]
    pub fn U(self, other: NnfLtl<L>) -> NnfLtl<L> {
        NnfLtl::U(Box::new(self), Box::new(other))
    }

    /// `ψ R φ ≡ ¬(¬ψ U ¬φ) ≡ ψ V φ`
    // TODO: Check that this conversion is correct
    #[allow(non_snake_case)]
    pub fn R(self, other: NnfLtl<L>) -> NnfLtl<L> {
        NnfLtl::V(Box::new(self), Box::new(other))
    }

    #[allow(non_snake_case)]
    pub fn V(self, other: NnfLtl<L>) -> NnfLtl<L> {
        NnfLtl::V(Box::new(self), Box::new(other))
    }

    #[allow(non_snake_case)]
    pub fn G(self) -> NnfLtl<L> {
        NnfLtl::Bool(false).R(self)
    }

    #[allow(non_snake_case)]
    pub fn F(self) -> NnfLtl<L> {
        NnfLtl::Bool(true).U(self)
    }

    pub fn alphabet(&self) -> Alphabet<L>
    where
        L: Clone + Ord,
    {
        let mut alphabet = BTreeSet::new();
        self.alphabet_(&mut alphabet);
        alphabet.into_iter().collect()
    }

    fn alphabet_(&self, alphabet: &mut BTreeSet<L>)
    where
        L: Clone + Ord,
    {
        match self {
            NnfLtl::Literal { name, .. } => {
                alphabet.insert(name.clone());
            }
            NnfLtl::U(p, q) | NnfLtl::V(p, q) | NnfLtl::Or(p, q) | NnfLtl::And(p, q) => {
                p.alphabet_(alphabet);
                q.alphabet_(alphabet);
            }
            NnfLtl::X(p) => p.alphabet_(alphabet),
            NnfLtl::Bool(_) => {}
        }
    }
}

impl<L> std::ops::BitOr for NnfLtl<L> {
    type Output = NnfLtl<L>;

    fn bitor(self, rhs: NnfLtl<L>) -> NnfLtl<L> {
        NnfLtl::Or(Box::new(self), Box::new(rhs))
    }
}
impl<L> std::ops::BitAnd for NnfLtl<L> {
    type Output = NnfLtl<L>;

    fn bitand(self, rhs: NnfLtl<L>) -> NnfLtl<L> {
        NnfLtl::And(Box::new(self), Box::new(rhs))
    }
}

impl<L: fmt::Display> fmt::Display for NnfLtl<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NnfLtl::Literal { negated, name } => {
                if *negated {
                    write!(f, "¬{}", name)
                } else {
                    write!(f, "{}", name)
                }
            }
            NnfLtl::Bool(true) => write!(f, "T"),
            NnfLtl::Bool(false) => write!(f, "⊥"),
            NnfLtl::U(p, q) => write!(f, "({} U {})", p, q),
            NnfLtl::V(p, q) => write!(f, "({} V {})", p, q),
            NnfLtl::Or(p, q) => write!(f, "{} ∨ {}", p, q),
            NnfLtl::And(p, q) => write!(f, "{} ∧ {}", p, q),
            NnfLtl::X(p) => write!(f, "X ({})", p),
        }
    }
}
