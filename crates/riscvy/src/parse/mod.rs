use lalrpop_util::lalrpop_mod;
use miette::Diagnostic;
use once_cell::sync::Lazy;
use thiserror::Error;

use crate::RiscVFile;

lalrpop_mod!(riscv, "/parse/riscv.rs");

pub fn parse_file(src: &str) -> Result<RiscVFile, ParseError> {
    static PARSER: Lazy<riscv::FileParser> = Lazy::new(riscv::FileParser::new);

    PARSER.parse(src).map_err(|e| ParseError::new(src, e))
}

#[derive(Debug, Error, Diagnostic, Clone)]
pub enum ParseError {
    #[error("Invalid Token")]
    #[diagnostic()]
    InvalidToken {
        #[source_code]
        src: String,
        #[label("This token is not valid in this context")]
        err_span: miette::SourceSpan,
    },
    #[error("Unrecognized Token")]
    #[diagnostic(help("Expected tokens here are: {expected}{}", if let Some(hint) = hint { format!("\n{hint}") } else { "".to_string() }))]
    UnrecognizedToken {
        #[source_code]
        src: String,
        #[label = "The token \"{token}\" is unrecognized in this context."]
        err_span: miette::SourceSpan,
        token: String,
        expected: String,
        hint: Option<String>,
    },
    #[error("Unrecognized EOF")]
    #[diagnostic(help("Expected tokens in this context are:\n{expected}"))]
    UnrecognizedEof {
        #[source_code]
        src: String,
        #[label = "The document ends too early. Are you missing a token?"]
        err_span: miette::SourceSpan,
        expected: String,
    },
}

impl ParseError {
    pub(crate) fn new(
        src: &str,
        e: lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token, &str>,
    ) -> Self {
        let prep_src = || format!("{src}\n");

        match e {
            lalrpop_util::ParseError::InvalidToken { location } => ParseError::InvalidToken {
                src: prep_src(),
                err_span: (location, 0).into(),
            },
            lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
                ParseError::UnrecognizedEof {
                    src: prep_src(),
                    err_span: (location, 0).into(),
                    expected: expected.join(", "),
                }
            }
            lalrpop_util::ParseError::UnrecognizedToken { token, expected } => {
                ParseError::UnrecognizedToken {
                    src: prep_src(),
                    err_span: (token.0, token.2 - token.0).into(),
                    token: token.1.to_string(),
                    expected: expected.join(", "),
                    hint: None,
                }
            }
            lalrpop_util::ParseError::ExtraToken { .. } => todo!(),
            lalrpop_util::ParseError::User { error: _ } => todo!(),
        }
    }
}
