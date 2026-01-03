use crate::span::Span;
use std::cmp;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AfnsError {
    #[error("lexing error: {0}")]
    Lex(#[from] LexError),
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
}

#[derive(Debug, Error)]
pub enum LexError {
    #[error("unexpected character `{ch}` at {span:?}")]
    UnexpectedChar { ch: char, span: Span },
    #[error("unterminated string literal starting at {span:?}")]
    UnterminatedString { span: Span },
    #[error("unterminated block comment starting at {span:?}")]
    UnterminatedBlockComment { span: Span },
    #[error("invalid numeric literal at {span:?}")]
    InvalidNumber { span: Span },
    #[error("invalid character literal at {span:?}")]
    InvalidCharLiteral { span: Span },
    #[error("Non-ASCII identifier characters are not allowed")]
    NonAsciiIdentifierChar { ch: char, span: Span },
}

impl LexError {
    pub fn span(&self) -> Option<Span> {
        match self {
            LexError::UnexpectedChar { span, .. }
            | LexError::UnterminatedString { span }
            | LexError::UnterminatedBlockComment { span }
            | LexError::InvalidNumber { span }
            | LexError::InvalidCharLiteral { span }
            | LexError::NonAsciiIdentifierChar { span, .. } => Some(*span),
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum ParseError {
    #[error("unexpected token {found:?} expected {expected} at {span:?}")]
    UnexpectedToken {
        expected: &'static str,
        found: crate::token::TokenKind,
        span: Span,
    },
    #[allow(dead_code)]
    #[error("unexpected end of file while parsing {context}")]
    UnexpectedEof { context: &'static str },
    #[error("invalid literal for array size at {span:?}")]
    InvalidArraySize { span: Span },
    #[error("unbalanced block starting at {span:?}")]
    UnbalancedBlock { span: Span },
}

impl ParseError {
    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedToken { span, .. }
            | ParseError::InvalidArraySize { span }
            | ParseError::UnbalancedBlock { span } => Some(*span),
            ParseError::UnexpectedEof { .. } => None,
        }
    }
}

pub fn format_error(source: &str, error: &AfnsError) -> String {
    match error {
        AfnsError::Lex(err) => format_with_span(source, err.span(), &err.to_string()),
        AfnsError::Parse(err) => format_with_span(source, err.span(), &err.to_string()),
    }
}

pub fn format_diagnostic(source: &str, span: Option<Span>, message: &str) -> String {
    format_with_span(source, span, message)
}

pub fn print_error(source: &str, error: &AfnsError) {
    eprintln!("{}", format_error(source, error));
}

pub fn line_col_at(source: &str, index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in source.char_indices() {
        if idx >= index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn format_with_span(source: &str, span: Option<Span>, message: &str) -> String {
    if let Some(span) = span {
        let line_str = line_at(source, span.line);
        let pointer_len = cmp::max(1, span.end.saturating_sub(span.start));
        let caret_offset = span.column.saturating_sub(1);
        let caret = format!(
            "{}{}",
            " ".repeat(caret_offset),
            "^".repeat(cmp::min(
                pointer_len,
                line_str.len().saturating_sub(caret_offset).max(1)
            ))
        );
        format!(
            "error: {message}\n --> line {}, column {}\n{:>4} | {}\n     | {}\n",
            span.line, span.column, span.line, line_str, caret
        )
    } else {
        format!("error: {message}")
    }
}

fn line_at(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .to_string()
}
