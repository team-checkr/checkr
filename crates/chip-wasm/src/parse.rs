use miette::Diagnostic;
use once_cell::sync::Lazy;
use thiserror::Error;

use crate::ast::{AGCLCommands, LTLProgram};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceSpan {
    /// The start of the span.
    offset: usize,
    /// The total length of the span. Think of this as an offset from `start`.
    length: usize,
}

impl SourceSpan {
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[must_use]
    pub fn join(&self, span: SourceSpan) -> SourceSpan {
        let offset = self.offset.min(span.offset);
        let end = self.end().max(span.end());
        let length = end - offset;
        SourceSpan { offset, length }
    }
    #[must_use]
    pub fn end(&self) -> usize {
        self.offset + self.length
    }
    #[must_use]
    pub fn contains(&self, byte_offset: usize) -> bool {
        self.offset() <= byte_offset && byte_offset < self.end()
    }

    #[must_use]
    pub fn union(
        init: SourceSpan,
        span: impl IntoIterator<Item = Option<SourceSpan>>,
    ) -> SourceSpan {
        span.into_iter()
            .fold(init, |a, b| b.map(|b| a.join(b)).unwrap_or(a))
    }

    pub fn cursor_at_start(&self) -> Self {
        SourceSpan {
            offset: self.offset(),
            length: 1,
        }
    }

    pub fn cursor_at_end(&self) -> Self {
        SourceSpan {
            offset: self.end(),
            length: 1,
        }
    }
}

impl From<SourceSpan> for miette::SourceSpan {
    fn from(s: SourceSpan) -> Self {
        Self::new(s.offset.into(), s.length)
    }
}
impl From<(usize, usize)> for SourceSpan {
    fn from((offset, length): (usize, usize)) -> Self {
        SourceSpan { offset, length }
    }
}

pub fn parse_agcl_program(src: &str) -> Result<AGCLCommands, ParseError> {
    static PARSER: Lazy<crate::agcl::AGCLCommandsParser> =
        Lazy::new(crate::agcl::AGCLCommandsParser::new);

    PARSER.parse(src).map_err(|e| ParseError::new(src, e))
}
pub fn parse_ltl_program(src: &str) -> Result<LTLProgram, ParseError> {
    static PARSER: Lazy<crate::agcl::LTLProgramParser> =
        Lazy::new(crate::agcl::LTLProgramParser::new);

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
        err_span: SourceSpan,
    },
    #[error("Unrecognized Token")]
    #[diagnostic(help("Expected tokens here are: {expected}{}", if let Some(hint) = hint { format!("\n{hint}") } else { "".to_string() }))]
    UnrecognizedToken {
        #[source_code]
        src: String,
        #[label = "The token \"{token}\" is unrecognized in this context."]
        err_span: SourceSpan,
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
        err_span: SourceSpan,
        expected: String,
    },
    #[error("Integer is too large")]
    #[diagnostic(help("The integer is too large to be represented"))]
    IntegerTooLarge {
        #[source_code]
        src: String,
        #[label = "The integer is too large to be represented"]
        err_span: SourceSpan,
    },
}

pub(crate) enum CustomError {
    IntegerTooLarge { from: usize, to: usize },
}

impl ParseError {
    pub(crate) fn new(
        src: &str,
        e: lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token, CustomError>,
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
            lalrpop_util::ParseError::User { error } => match error {
                CustomError::IntegerTooLarge { from, to } => ParseError::IntegerTooLarge {
                    src: prep_src(),
                    err_span: (from, to).into(),
                },
            },
        }
    }
}
