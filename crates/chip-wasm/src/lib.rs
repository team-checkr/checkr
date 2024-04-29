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

use ast::{BExpr, Locator};
use itertools::Itertools;
use mcltl::{
    buchi::{Alphabet, AtomicProperty, ProductBuchi},
    ltl::expression::Literal,
    state::State as _,
};
use miette::Diagnostic;
use parse::SourceSpan;
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
    fn from_source_span(src: &str, span: SourceSpan) -> MonacoSpan {
        Self::from_offset_len(src, span.offset(), span.len())
    }
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
    Real(interpreter::State),
}

impl mcltl::state::State for State {
    fn initial() -> Self {
        State::Initial
    }

    fn name(&self) -> String {
        match self {
            State::Initial => "INIT".to_string(),
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
pub struct LtLResult {
    parse_error: bool,
    markers: Vec<(MarkerData, Vec<String>)>,
    ts_dot: String,
    ts_map: HashMap<String, Vec<MonacoSpan>>,
    kripke_str: String,
    buchi_dot: String,
    negated_nnf_ltl_property_str: String,
    gbuchi_property_dot: String,
    buchi_property_dot: String,
    product_ba_dot: String,
}

#[derive(Debug, Default)]
struct Timing {}

struct TimingGuard {
    name: String,
}

const PRINT_TIMINGS: bool = false;

impl Timing {
    fn start(&self, name: impl std::fmt::Display) -> TimingGuard {
        if PRINT_TIMINGS {
            web_sys::console::time_with_label(&name.to_string());
        }
        TimingGuard {
            name: name.to_string(),
        }
    }
    fn time<T>(&self, name: impl std::fmt::Display, f: impl FnOnce() -> T) -> T {
        let guard = self.start(name);
        let res = f();
        drop(guard);
        res
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        if PRINT_TIMINGS {
            web_sys::console::time_end_with_label(&self.name);
        }
    }
}

#[wasm_bindgen]
pub fn parse_ltl(src: &str) -> LtLResult {
    let timing = Timing::default();

    let res = timing.time("parse", || parse::parse_ltl_program(src));

    match res {
        Ok(ast) => {
            let p = timing.time("compile", || {
                interpreter::Program::compile(
                    &ast.commands,
                    ast.properties
                        .iter()
                        .flat_map(|(_, property)| property.fv())
                        .filter_map(|t| match t {
                            Target::Variable(v) => Some(v),
                            _ => None,
                        })
                        .chain(ast.initial.keys().cloned()),
                )
            });
            let state = p.initial_state(|var| ast.initial.get(var).copied().unwrap_or_default());

            let mut fuel: u32 = 5000;

            // Explore the state space using a breath-first search
            let mut states = Vec::new();
            let mut visited = HashMap::new();
            let mut relations: HashMap<usize, HashSet<_>> = HashMap::new();
            let mut queue = VecDeque::new();
            states.push(state.clone());
            visited.insert(state.clone(), 0);
            queue.push_back(0);

            while let Some(state_id) = queue.pop_front() {
                for next_state in states[state_id].step(&p) {
                    if let Some(new_fuel) = fuel.checked_sub(1) {
                        fuel = new_fuel;
                    } else {
                        return LtLResult {
                            parse_error: false,
                            markers: vec![(
                                MarkerData {
                                    related_information: None,
                                    tags: None,
                                    severity: MarkerSeverity::Error,
                                    message: "Exploration of the state space".to_string(),
                                    span: MonacoSpan::from_offset_len(src, 0, 0),
                                },
                                Vec::new(),
                            )],
                            ts_dot: "".to_string(),
                            ts_map: Default::default(),
                            kripke_str: "".to_string(),
                            buchi_dot: "".to_string(),
                            negated_nnf_ltl_property_str: "".to_string(),
                            gbuchi_property_dot: "".to_string(),
                            buchi_property_dot: "".to_string(),
                            product_ba_dot: "".to_string(),
                        };
                    }

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
                    relations.entry(state_id).or_default().insert(id);
                    relations.entry(id).or_default();
                }
            }

            let ts_dot = format!(
                "digraph G {{\n{}\n{}\n{}\n}}",
                states
                    .iter()
                    .enumerate()
                    .map(|(idx, s)| format!("{}[label={:?}];", idx, s.format(&p).to_string()))
                    .format("\n"),
                r#"init -> 0 ; init[label="",opacity=0]"#,
                relations
                    .iter()
                    .flat_map(|(from, tos)| tos.iter().map(move |to| format!("{from} -> {to};")))
                    .format("\n"),
            );
            let ts_map = states
                .iter()
                .enumerate()
                .map(|(idx, s)| {
                    (
                        idx.to_string(),
                        s.spans(&p)
                            .map(|span| MonacoSpan::from_source_span(src, span.cursor_at_start()))
                            .collect_vec(),
                    )
                })
                .collect();

            let mut kripke_str = "".to_string();
            let mut buchi_dot = "".to_string();
            let mut negated_nnf_ltl_property_str = "".to_string();
            let mut gbuchi_property_dot = "".to_string();
            let mut buchi_property_dot = "".to_string();
            let mut product_ba_dot = "".to_string();

            let mut markers: Vec<(_, _)> = Vec::new();

            for (property_span, property) in &ast.properties {
                // Build NNF LTL properties

                let mut relational_properties = Vec::new();

                let ltl_property = !property.to_mcltl(&mut relational_properties);
                let nnf_ltl_property = ltl_property.nnf();

                let mut alphabet: Alphabet<Literal> = [
                    Locator::Init.to_lit(),
                    Locator::Stuck.to_lit(),
                    Locator::Terminated.to_lit(),
                ]
                .into_iter()
                .collect();

                // tracing::debug!("{nnf_ltl_property:?}");

                // Build the Kripke structure

                let kripke = timing.time("kripke", || {
                    let mut kripke: mcltl::verifier::kripke::KripkeStructure<State, Literal> =
                        mcltl::verifier::kripke::KripkeStructure::new(
                            [State::Real(states.first().unwrap().clone())].to_vec(),
                        );
                    let worlds: Vec<_> = states
                        .iter()
                        .map(|state| {
                            let mut assignment = relational_properties
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, (l, op, r))| {
                                    let holds = BExpr::Rel(l.clone(), *op, r.clone())
                                        .evaluate(&p, state)
                                        .is_ok_and(|x| x);
                                    holds.then(|| Literal::from(format!("p{idx}")))
                                })
                                .collect::<<Literal as AtomicProperty>::Set>();

                            if state.is_terminated(&p) {
                                assignment.insert(Locator::Terminated.to_lit());
                            } else if state.is_stuck(&p) {
                                assignment.insert(Locator::Stuck.to_lit());
                            } else if states.first().unwrap() == state {
                                assignment.insert(Locator::Init.to_lit());
                            }

                            kripke.add_node(State::Real(state.clone()), assignment)
                        })
                        .collect();

                    for (src, dsts) in relations.iter() {
                        let worlds = &worlds;
                        for dst in dsts.iter() {
                            kripke.add_relation(worlds[*src], worlds[*dst]);
                        }
                    }
                    kripke
                });

                // Build the Buchi automaton

                // tracing::debug!("building Buchi automaton");
                let buchi: mcltl::buchi::Buchi<State, Literal> = timing.time("buchi", || {
                    let mut buchi = kripke.to_buchi(Some(&alphabet));
                    timing.time("add_necessary_self_loops", || {
                        buchi.add_necessary_self_loops()
                    });
                    buchi
                });

                // tracing::debug!("extracting Buchi automaton from LTL property");
                let gbuchi_property =
                    timing.time("gbuchi_property", || nnf_ltl_property.gba(Some(&alphabet)));

                // tracing::debug!("converting generalized Buchi automaton into classic Buchi automaton");
                let buchi_property = timing.time("buchi_property", || gbuchi_property.to_buchi());

                // tracing::debug!("product automaton");
                let product_ba = timing.time("product", || {
                    // buchi.product(&buchi_property).pruned()
                    ProductBuchi::new(&buchi, &buchi_property)
                });

                if kripke_str.is_empty() {
                    kripke_str = kripke.to_string();

                    // NOTE: This is currently disabled since it's not rendered
                    // and takes a substantial time to create
                    timing.time("dot", || {
                        buchi_dot = buchi.dot();
                        negated_nnf_ltl_property_str = nnf_ltl_property.to_string();
                        gbuchi_property_dot = gbuchi_property.dot();
                        buchi_property_dot = buchi_property.dot();
                        product_ba_dot = product_ba.dot();
                    });
                }

                // tracing::debug!("emptiness check");
                let res = timing.time("find_accepting_cycle", || product_ba.find_accepting_cycle());

                if let Some(cycle) = res {
                    let mut trace = Vec::new();

                    for (top, _) in cycle.iter() {
                        let id = buchi.id(top);

                        if let State::Real(s) = id {
                            trace.push(s.clone());
                        }
                    }

                    let mut table = comfy_table::Table::new();
                    table.set_header(
                        std::iter::once("Step".to_string())
                            .chain(p.variables().map(|v| v.to_string())),
                    );
                    for (idx, s) in trace.iter().enumerate() {
                        table.add_row(
                            std::iter::once((idx + 1).to_string())
                                .chain(s.variables(&p).map(|(_, v)| v.to_string())),
                        );
                    }

                    enum Alignment {
                        Left,
                        Center,
                        Right,
                    }
                    enum CellType {
                        Plain,
                        Code,
                    }
                    struct HtmlTable {
                        header: Vec<(String, Alignment, CellType)>,
                        rows: Vec<Vec<(String, Alignment, CellType)>>,
                    }
                    impl CellType {
                        fn wrap(&self, s: impl std::fmt::Display) -> String {
                            match self {
                                CellType::Plain => s.to_string(),
                                CellType::Code => format!("<code>{}</code>", s),
                            }
                        }
                    }
                    impl std::fmt::Display for Alignment {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            match self {
                                Alignment::Left => "left",
                                Alignment::Center => "center",
                                Alignment::Right => "right",
                            }
                            .fmt(f)
                        }
                    }
                    impl std::fmt::Display for HtmlTable {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            write!(f, "<table><thead><tr>")?;
                            for (v, a, t) in &self.header {
                                write!(f, "<th align=\"{}\">{}</th>", a, t.wrap(v))?;
                            }
                            write!(f, "</tr></thead><tbody>")?;
                            for row in &self.rows {
                                write!(f, "<tr>")?;
                                for (v, a, t) in row {
                                    write!(f, "<td align=\"{}\">{}</td>", a, t.wrap(v))?;
                                }
                                write!(f, "</tr>")?;
                            }
                            write!(f, "</tbody></table>")
                        }
                    }

                    let html_table = HtmlTable {
                        header: std::iter::once((
                            "Step".to_string(),
                            Alignment::Right,
                            CellType::Plain,
                        ))
                        .chain(
                            p.variables()
                                .map(|v| (v.to_string(), Alignment::Right, CellType::Code)),
                        )
                        .collect(),
                        rows: trace
                            .iter()
                            .enumerate()
                            .map(|(idx, s)| {
                                std::iter::once((
                                    (idx + 1).to_string(),
                                    Alignment::Right,
                                    CellType::Plain,
                                ))
                                .chain(s.variables(&p).map(|(_, value)| {
                                    (value.to_string(), Alignment::Right, CellType::Code)
                                }))
                                .collect()
                            })
                            .collect(),
                    };

                    markers.push((
                        MarkerData {
                            related_information: None,
                            tags: None,
                            severity: MarkerSeverity::Error,
                            message: format!("LTL property does not hold\n\n{}", html_table),
                            span: MonacoSpan::from_offset_len(
                                src,
                                property_span.offset(),
                                property_span.len(),
                            ),
                        },
                        cycle
                            .iter()
                            .filter_map(|(state, _)| {
                                Some(
                                    states
                                        .iter()
                                        .position(|s| State::Real(s.clone()) == *buchi.id(state))?
                                        .to_string(),
                                )
                            })
                            .collect(),
                    ))
                };
            }

            LtLResult {
                parse_error: false,
                markers,
                ts_dot,
                ts_map,
                kripke_str,
                buchi_dot,
                negated_nnf_ltl_property_str,
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
                .map(|l| {
                    (
                        MarkerData {
                            related_information: None,
                            tags: None,
                            severity: MarkerSeverity::Error,
                            message: l.label().unwrap_or_default().to_string(),
                            span: MonacoSpan::from_offset_len(src, l.offset(), l.len()),
                        },
                        Vec::new(),
                    )
                })
                .collect(),
            ts_dot: "".to_string(),
            ts_map: Default::default(),
            kripke_str: "".to_string(),
            buchi_dot: "".to_string(),
            negated_nnf_ltl_property_str: "".to_string(),
            gbuchi_property_dot: "".to_string(),
            buchi_property_dot: "".to_string(),
            product_ba_dot: "".to_string(),
        },
    }
}
