mod lexer;
mod parser;

pub mod ast;

use thiserror::Error;

pub use lexer::{Token, TokenKind, tokenize};
pub use parser::Parser;

/// A byte span within the original source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn through(self, other: Self) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}

/// A source-located parser or lexer failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message} at line {line}, column {column}")]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    #[must_use]
    pub fn new(source: &str, span: Span, message: impl Into<String>) -> Self {
        let (line, column) = line_column(source, span.start);

        Self {
            message: message.into(),
            span,
            line,
            column,
        }
    }
}

/// Parser crate entry point for TS-Native source parsing.
pub fn parse_source(source: &str) -> Result<ast::Program, ParseError> {
    let tokens = tokenize(source)?;
    Parser::new(source, tokens).parse_program()
}

fn line_column(source: &str, index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (offset, ch) in source.char_indices() {
        if offset >= index {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}
