pub mod agcl;
pub mod ast;
pub mod ast_ext;
pub mod ast_smt;
pub mod fmt;
pub mod interpreter;
pub mod parse;
pub mod triples;

pub const SMT_PRELUDE: &str = include_str!("chip-theory.smt2");
