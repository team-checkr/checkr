use core::fmt;
use std::collections::BTreeSet;

use itertools::Itertools;
use plex::{lexer, parser};

use crate::{
    buchi::{Alphabet, AtomicProperty, AtomicPropertySet, Buchi, BuchiLikeMut as _},
    ltl::expression::Literal,
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

impl KripkeStructure<String, Literal> {
    pub fn parse(program: &str) -> Result<Self, &'static str> {
        let lexer = KripkeLexer::new(program);
        let parse_result = parser::parse(lexer);

        match parse_result {
            Ok(exprs) => {
                match KripkeStructure::from_exprs(exprs) {
                    Ok(k) => Ok(k),
                    Err(_) => Err("can't parse kripke structure"), //FIXME: should use the error return by parser
                }
            }
            Err(e) => Err(e.1),
        }
    }
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

    fn from_exprs(exprs: Vec<Expr<S, AP>>) -> Result<Self, String> {
        let mut kripke = KripkeStructure::new(vec![]);

        // extract worlds
        for e in exprs.iter() {
            match e {
                Expr::Init(inits) => {
                    for i in inits.iter() {
                        let n = kripke.add_node(i.clone(), Default::default());
                        kripke.inits.insert(n);
                    }
                }
                Expr::World(w) => {
                    kripke.add_node(w.id.clone(), w.assignment.clone());
                }
                Expr::Relation(_, _) => {}
            }
        }

        for e in exprs.iter() {
            match e {
                Expr::Relation(src, dst) => {
                    for dst in dst.iter() {
                        let src_world = kripke.find_world(src);
                        let dst_world = kripke.find_world(dst);

                        match (src_world, dst_world) {
                            (Some(src), Some(dst)) => kripke.add_relation(src, dst),
                            (Some(_), None) => {
                                return Err(format!(
                                    "cannot find world `{}` in this scope",
                                    dst.name()
                                ));
                            }
                            (None, Some(_)) => {
                                return Err(format!(
                                    "cannot find world `{}` in this scope",
                                    src.name()
                                ));
                            }
                            (None, None) => {
                                return Err(format!(
                                    "cannot find world `{}` and `{}` in this scope",
                                    src.name(),
                                    dst.name()
                                ));
                            }
                        }
                    }
                }
                Expr::World(_) => {}
                Expr::Init(inits) => {
                    for i in inits.iter() {
                        if !kripke.nodes.iter().any(|w| &w.id == i) {
                            return Err(format!(
                                "cannot find init world `{}` in this scope",
                                i.name()
                            ));
                        }
                    }
                }
            }
        }

        Ok(kripke)
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
pub enum Token {
    Ident(String),
    Whitespace,
    LBrace,
    RBrace,
    Equ,
    Not,
    Relation,
    Comma,
    Worlds,
    Init,
}

struct KripkeLexer<'a> {
    original: &'a str,
    remaining: &'a str,
}

impl<'a> KripkeLexer<'a> {
    pub fn new(s: &'a str) -> KripkeLexer<'a> {
        Self {
            original: s,
            remaining: s,
        }
    }
}

lexer! {
    fn next_token(text: 'a) -> Token;
    r#"[ \t\r\n]+"# => Token::Whitespace,
    r#"{"# => Token::LBrace,
    r#"}"# => Token::RBrace,

    r#"init|INIT"# => Token::Init,
    r#"\~|not"# => Token::Not,
    r#"[a-z0-9_][a-z0-9_]*"# => Token::Ident(text.into()),
    r#"R|=>"# => Token::Relation,
    r#","# => Token::Comma,
    r#"="# => Token::Equ,

    r#"."# => panic!("unexpected character: {}", text),
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Iterator for KripkeLexer<'_> {
    type Item = (Token, Span);
    fn next(&mut self) -> Option<(Token, Span)> {
        loop {
            let (tok, span) = if let Some((tok, new_remaining)) = next_token(self.remaining) {
                let lo = self.original.len() - self.remaining.len();
                let hi = self.original.len() - new_remaining.len();
                self.remaining = new_remaining;
                (tok, Span { lo, hi })
            } else {
                return None;
            };
            match tok {
                Token::Whitespace => {
                    continue;
                }
                tok => {
                    return Some((tok, span));
                }
            }
        }
    }
}

impl KripkeLexer<'_> {
    pub fn tokenize(&mut self) -> Vec<(Token, Span)> {
        let mut result = Vec::new();

        while !self.remaining.is_empty() {
            if let Some((token, span)) = self.next() {
                result.push((token, span))
            }
        }

        result
    }
}

#[derive(Debug, Clone)]
pub enum Expr<S, AP: AtomicProperty> {
    Init(Vec<S>),
    World(KripkeNode<S, AP>),
    Relation(S, Vec<S>),
}

mod parser {
    #![allow(
        clippy::enum_variant_names,
        clippy::let_unit_value,
        clippy::ptr_arg,
        clippy::redundant_closure_call,
        clippy::redundant_field_names,
        clippy::type_complexity,
        clippy::unused_unit
    )]

    use super::Token::*;
    use super::*;

    parser! {
        fn parse_(Token, Span);
        (a, b) {
            Span {
                lo: a.lo,
                hi: b.hi,
            }
        }

        statements: Vec<Expr<String, Literal>> {
            => vec![],
            statements[mut st] term[e] => {
                st.push(e);
                st
            }
        }

        term: Expr<String, Literal> {
            Ident(i) Equ LBrace props[p] RBrace =>  {
                Expr::World(KripkeNode{ id: i, assignment: p.into_iter().filter_map(|(a, b)| b.then_some(a)).collect()})
            },
            Ident(src) Relation LBrace idents[ws] RBrace => {
                Expr::Relation(src, ws)
            },
            Ident(src) Relation Ident(dst) => {
                Expr::Relation(src, vec![dst])
            }
            Init Equ Ident(i) => Expr::Init(vec![i]),
            Init Equ LBrace idents[i] RBrace => Expr::Init(i),
        }

        idents: Vec<String> {
            => Vec::new(),
            idents[mut ws] Ident(ident) optionalComa => {
                ws.push(ident);
                ws
            },
        }

        props: Vec<(Literal, bool)> {
            => Vec::new(),
            props[mut p] Ident(ident) optionalComa => {
                p.push((ident.into(), true));
                p
            },
            props[mut p] Not Ident(ident) optionalComa => {
                p.push((ident.into(), false));
                p
            }
        }

        optionalComa: () {
            => (),
            Comma => (),
        }
    }

    pub fn parse<I: Iterator<Item = (Token, Span)>>(
        i: I,
    ) -> Result<Vec<Expr<String, Literal>>, (Option<(Token, Span)>, &'static str)> {
        parse_(i)
    }
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

    #[test]
    fn it_should_parse_kripke_structure() {
        let input = r#"
            init = {n1, n2}

            n1 = { p, not q }
            n1 => n2

            n2 = { p, ~q }
            n2 => { n2, n3 }

            n3 = { p, q }
            n3 R n1
        "#;
        let lexer = KripkeLexer::new(input);
        let parse_result = parser::parse(lexer);

        assert!(parse_result.is_ok());

        let res = KripkeStructure::from_exprs(parse_result.unwrap());
        assert!(res.is_ok());

        let kripke = res.unwrap();
        assert_eq!(2, kripke.inits.len());
        assert_eq!(3, kripke.nodes.len());
        assert_eq!(4, kripke.relations.len());
    }

    #[test]
    fn it_should_parse_kripke_structure_and_fail_to_init_struct_when_some_worlds_are_not_declared()
    {
        let input = r#"
            init = {n1, n2}

            n1 = { p, not q }
            n1 => n4

            n2 = {p, q}
        "#;
        let lexer = KripkeLexer::new(input);
        let parse_result = parser::parse(lexer);

        assert!(parse_result.is_ok());

        let res = KripkeStructure::from_exprs(parse_result.unwrap());
        assert!(res.is_err());
        assert_eq!(
            "cannot find world `n4` in this scope",
            res.unwrap_err().as_str()
        );
    }

    // TODO: readd this once we decide wheter or not we want this constraint
    // #[test]
    // fn it_should_parse_kripke_structure_and_fail_when_some_inits_worlds_are_not_declared() {
    //     let input = r#"
    //         init = {n1, n4}
    //         n1 = { p, not q }
    //     "#;
    //     let lexer = KripkeLexer::new(input);
    //     let parse_result = parser::parse(lexer);

    //     assert!(parse_result.is_ok());

    //     let res = KripkeStructure::from_exprs(parse_result.unwrap());
    //     assert!(res.is_err());
    //     assert_eq!(
    //         "cannot find init world `n4` in this scope",
    //         res.unwrap_err().as_str()
    //     );
    // }
}
