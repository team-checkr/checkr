#![allow(non_snake_case)]

mod agcl;
mod ast;
mod ast_ext;
mod ast_smt;
mod fmt;
mod interpreter;
mod parse;
mod triples;

use std::collections::{hash_map::Entry, HashMap, HashSet, VecDeque};

use ast::BExpr;
use itertools::Itertools;
use mcltl::state::State as _;
use miette::Diagnostic;
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::{ast::Target, ast_ext::FreeVariables};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    Ok(())
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct Assertion {
    implication: String,
    smt: String,
    span: MonacoSpan,
    text: Option<String>,
    related: Option<(String, MonacoSpan)>,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct ParseResult {
    pub parse_error: bool,
    pub prelude: String,
    pub assertions: Vec<Assertion>,
    pub markers: Vec<MarkerData>,
    pub is_fully_annotated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
    pub fn to_byte_offset(self, src: &str) -> Option<usize> {
        let mut lines = self.line;
        let mut columns = self.character;
        src.char_indices()
            .find(|&(_, c)| {
                if lines == 0 {
                    if columns == 0 {
                        return true;
                    }
                    columns -= 1
                } else if c == '\n' {
                    lines -= 1;
                }
                false
            })
            .map(|(idx, _)| idx)
    }
    pub fn from_byte_offset(src: &str, byte_offset: usize) -> Self {
        if src.get(0..byte_offset).is_none() {
            tracing::debug!(?src, byte_offset, len=?src.len(), "byte offset out of range");
            // Return the final position
            let l = src.lines().count();
            let c = src.lines().last().unwrap().len();
            return Position::new(l as _, c as _);
        }
        if src[0..byte_offset].is_empty() {
            return Position::new(0, 0);
        }

        if src[0..byte_offset].ends_with('\n') {
            let l = src[0..byte_offset].lines().count();
            Position::new(l as _, 0)
        } else {
            let l = src[0..byte_offset].lines().count() - 1;
            let c = src[0..byte_offset].lines().last().unwrap().len();
            Position::new(l as _, c as _)
        }
    }
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct MonacoSpan {
    pub start_line_number: u32,
    pub start_column: u32,
    pub end_line_number: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct MarkerData {
    // code?: string | {
    //     value: string;
    //     target: Uri;
    // };
    // source?: string;
    related_information: Option<Vec<RelatedInformation>>,
    tags: Option<Vec<MarkerTag>>,
    severity: MarkerSeverity,
    message: String,
    span: MonacoSpan,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[repr(u8)]
pub enum MarkerSeverity {
    Hint = 1,
    Info = 2,
    Warning = 4,
    Error = 8,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[repr(u8)]
pub enum MarkerTag {
    Unnecessary = 1,
    Deprecated = 2,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
struct RelatedInformation {
    /// Is actually a `Uri`
    resource: String,
    message: String,
    span: MonacoSpan,
}

impl MonacoSpan {
    fn from_offset_len(src: &str, offset: usize, length: usize) -> MonacoSpan {
        let start = Position::from_byte_offset(src, offset);
        let end = Position::from_byte_offset(src, offset + length);
        MonacoSpan {
            start_line_number: start.line + 1,
            start_column: start.character + 1,
            end_line_number: end.line + 1,
            end_column: end.character + 1,
        }
    }
}

const PRELUDE: &str = include_str!("chip-theory.smt2");

#[wasm_bindgen]
pub fn parse(src: &str) -> ParseResult {
    let res = parse::parse_agcl_program(src);
    match res {
        Ok(ast) => ParseResult {
            parse_error: false,
            prelude: PRELUDE.to_string(),
            assertions: ast
                .assertions()
                .into_iter()
                .map(|t| Assertion {
                    implication: t.predicate.to_string(),
                    smt: t.smt().join("\n"),
                    text: t.source.text,
                    span: MonacoSpan::from_offset_len(
                        src,
                        t.source.span.offset(),
                        t.source.span.len(),
                    ),
                    related: t.source.related.map(|(s, span)| {
                        (
                            s,
                            MonacoSpan::from_offset_len(src, span.offset(), span.len()),
                        )
                    }),
                })
                .collect(),
            markers: vec![],
            is_fully_annotated: ast.is_fully_annotated(),
        },
        Err(err) => ParseResult {
            parse_error: true,
            prelude: PRELUDE.to_string(),
            assertions: vec![],
            markers: err
                .labels()
                .into_iter()
                .flatten()
                .map(|l| MarkerData {
                    related_information: None,
                    tags: None,
                    severity: MarkerSeverity::Error,
                    message: l.label().unwrap_or_default().to_string(),
                    span: MonacoSpan::from_offset_len(src, l.offset(), l.len()),
                })
                .collect(),
            is_fully_annotated: false,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
    Initial,
    Fresh(usize),
    Real(interpreter::State),
}

impl mcltl::state::State for State {
    fn initial() -> Self {
        State::Initial
    }

    fn new_name() -> Self {
        State::Fresh(<usize as mcltl::state::State>::new_name())
    }

    fn name(&self) -> String {
        match self {
            State::Initial => "INIT".to_string(),
            State::Fresh(n) => format!("n{}", n),
            State::Real(s) => s.raw_id(),
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
struct LtLResult {
    parse_error: bool,
    markers: Vec<MarkerData>,
    kripke_str: String,
    buchi_dot: String,
    gbuchi_property_dot: String,
    buchi_property_dot: String,
    product_ba_dot: String,
}

#[wasm_bindgen]
pub fn parse_ltl(src: &str) -> LtLResult {
    let res = parse::parse_ltl_program(src);

    match res {
        Ok(ast) => {
            let p = interpreter::Program::compile(
                &ast.commands,
                ast.ltl.fv().into_iter().filter_map(|t| match t {
                    Target::Variable(v) => Some(v),
                    _ => None,
                }),
            );
            let state = p.initial_state(|var| ast.initial.get(var).copied().unwrap_or_default());

            // Explore the state space using a breath-first search
            let mut states = Vec::new();
            let mut visited = HashMap::new();
            let mut relations = HashMap::new();
            let mut queue = VecDeque::new();
            states.push(state.clone());
            visited.insert(state.clone(), 0);
            queue.push_back(0);

            while let Some(state_id) = queue.pop_front() {
                for next_state in states[state_id].step(&p).into_iter().flatten() {
                    let id = match visited.entry(next_state.clone()) {
                        Entry::Occupied(id) => *id.get(),
                        Entry::Vacant(v) => {
                            let id = states.len();
                            v.insert(id);
                            states.push(next_state.clone());
                            queue.push_back(id);
                            id
                        }
                    };
                    relations
                        .entry(state_id)
                        .or_insert_with(HashSet::new)
                        .insert(id);
                }
            }

            // Build the LTL property

            let mut relational_properties = Vec::new();
            let ltl_property = ast.ltl.to_mcltl(&mut relational_properties);
            let nnf_ltl_property = ltl_property.rewrite().nnf();

            tracing::debug!("{nnf_ltl_property:?}");

            // Build the Kripke structure

            let inits = vec![State::Real(states.first().unwrap().clone())];
            // let inits = states.iter().map(|s| s.id(&p)).collect_vec();
            let worlds: Vec<_> = states
                .iter()
                .map(|state| mcltl::verifier::kripke::World {
                    id: State::Real(state.clone()),
                    assignement: relational_properties
                        .iter()
                        .enumerate()
                        .map(|(idx, (l, op, r))| {
                            let holds = BExpr::Rel(l.clone(), *op, r.clone())
                                .evaluate(&p, state)
                                .is_ok_and(|x| x);
                            (format!("p{idx}"), holds)
                        })
                        .collect(),
                })
                .collect();
            let relations: Vec<_> = relations
                .into_iter()
                .flat_map(|(src, dsts)| {
                    let worlds = &worlds;
                    dsts.into_iter()
                        .map(move |dst| (worlds[src].clone(), worlds[dst].clone()))
                })
                .collect();

            let debug_str = format!(
                r#"
init = {{{}}}

{}
"#,
                inits.iter().map(|s| s.name()).format(", "),
                worlds
                    .iter()
                    .map(|w| format!(
                        r#"
{} = {{ {} }}
{}
"#,
                        w.id,
                        w.assignement
                            .iter()
                            .map(|(k, v)| if *v {
                                k.to_string()
                            } else {
                                format!("¬{}", k)
                            })
                            .format(", "),
                        relations
                            .iter()
                            .filter(|(src, _)| src.id == w.id)
                            .map(|(src, dst)| format!("{} => {}", src.id, dst.id))
                            .format(", "),
                    ))
                    .format(""),
            );

            tracing::debug!(%debug_str);

            tracing::debug!(?inits, ?worlds, ?relations);

            let mut kripke = mcltl::verifier::kripke::KripkeStructure::new(inits);
            for w in worlds {
                kripke.add_world(w);
            }
            for (src, dst) in relations {
                kripke.add_relation(src, dst);
            }

            // Build the Buchi automaton

            tracing::debug!("building Buchi automaton");
            let buchi: mcltl::buchi::Buchi<_> = kripke.clone().into();

            tracing::debug!("constructing the graph of the LTL property");
            let nodes = mcltl::ltl::automata::create_graph::<State>(nnf_ltl_property.clone());

            tracing::debug!("extracting Buchi automaton from LTL property");
            let gbuchi_property = mcltl::buchi::extract_buchi(nodes, nnf_ltl_property);

            tracing::debug!("converting generalized Buchi automaton into classic Buchi automaton");
            let buchi_property = gbuchi_property.to_buchi();

            tracing::debug!("product automaton");
            let product_ba = mcltl::buchi::product_automata(buchi.clone(), buchi_property.clone());

            let buchi_dot = buchi.dot();
            tracing::info!(buchi=%buchi_dot);
            let gbuchi_property_dot = gbuchi_property.dot();
            tracing::info!(gbuchi_property=%gbuchi_property_dot);
            let buchi_property_dot = buchi_property.dot();
            tracing::info!(buchi_property=%buchi_property_dot);
            let product_ba_dot = product_ba.dot();
            tracing::info!(product_ba=%product_ba_dot);

            tracing::debug!("emptiness check");
            let res = mcltl::verifier::model_checker::emptiness(product_ba);

            match res {
                Ok(()) => {
                    tracing::info!("LTL property holds");
                }
                Err((mut s1, mut s2)) => {
                    s1.reverse();
                    s2.reverse();

                    while let Some(top) = s1.pop() {
                        let id = top.id.0;

                        if let Some(l) = top.labels.first() {
                            tracing::error!(?id, ?l, "counterexample")
                        } else {
                            tracing::error!(?id, "counterexample")
                        }
                    }

                    while let Some(top) = s2.pop() {
                        let label = top.labels.first().unwrap();
                        let id = top.id.0;
                        tracing::error!(?id, ?label, "counterexample")
                    }
                }
            }

            LtLResult {
                parse_error: false,
                markers: vec![],
                kripke_str: debug_str,
                buchi_dot,
                gbuchi_property_dot,
                buchi_property_dot,
                product_ba_dot,
            }
        }
        Err(err) => LtLResult {
            parse_error: true,
            markers: err
                .labels()
                .into_iter()
                .flatten()
                .map(|l| MarkerData {
                    related_information: None,
                    tags: None,
                    severity: MarkerSeverity::Error,
                    message: l.label().unwrap_or_default().to_string(),
                    span: MonacoSpan::from_offset_len(src, l.offset(), l.len()),
                })
                .collect(),
            kripke_str: "".to_string(),
            buchi_dot: "".to_string(),
            gbuchi_property_dot: "".to_string(),
            buchi_property_dot: "".to_string(),
            product_ba_dot: "".to_string(),
        },
    }
}
