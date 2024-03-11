mod agcl;
mod ast;
mod ast_ext;
mod ast_smt;
mod fmt;
mod parse;
mod triples;

use itertools::Itertools;
use miette::Diagnostic;
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

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
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct ParseResult {
    pub parse_error: bool,
    pub assertions: Vec<Assertion>,
    pub markers: Vec<MarkerData>,
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
    pub startLineNumber: u32,
    pub startColumn: u32,
    pub endLineNumber: u32,
    pub endColumn: u32,
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
            startLineNumber: start.line + 1,
            startColumn: start.character + 1,
            endLineNumber: end.line + 1,
            endColumn: end.character + 1,
        }
    }
}

#[wasm_bindgen]
pub fn parse(src: &str) -> ParseResult {
    let res = parse::parse_program(src);
    match res {
        Ok(ast) => ParseResult {
            parse_error: false,
            assertions: ast
                .assertions()
                .into_iter()
                .map(|t| Assertion {
                    implication: t.predicate.to_string(),
                    smt: t.smt().join("\n"),
                    span: MonacoSpan::from_offset_len(src, t.span.offset(), t.span.len()),
                })
                .collect(),
            markers: vec![],
        },
        Err(err) => ParseResult {
            parse_error: true,
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
        },
    }
}
