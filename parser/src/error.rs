//! Parser error types.

use nom::error::{ContextError, ErrorKind, FromExternalError, ParseError};
use nom_locate::LocatedSpan;

/// Located parser input.
pub type Input<'a> = LocatedSpan<&'a str>;

/// Parser result type.
pub type ParseResult<'a, T> = nom::IResult<Input<'a>, T, ParserError<'a>>;

/// Parser error with source input and structured error kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError<'a> {
    /// Input slice where the error happened.
    pub input: Input<'a>,
    /// Structured parser error kind.
    pub kind: ParserErrorKind,
    /// Parser contexts attached by nom's `context` combinator.
    pub contexts: Vec<&'static str>,
}

impl<'a> ParserError<'a> {
    /// Create a parser error.
    pub fn new(input: Input<'a>, kind: ParserErrorKind) -> Self {
        Self {
            input,
            kind,
            contexts: Vec::new(),
        }
    }

    pub(crate) fn failure(input: Input<'a>, kind: ParserErrorKind) -> nom::Err<Self> {
        nom::Err::Failure(Self::new(input, kind))
    }

    /// Byte offset from the start of the parsed source.
    pub fn offset(&self) -> usize {
        self.input.location_offset()
    }

    /// One-based line number.
    pub fn line(&self) -> u32 {
        self.input.location_line()
    }

    /// One-based byte column.
    pub fn column(&self) -> usize {
        self.input.get_column()
    }
}

impl<'a> ParseError<Input<'a>> for ParserError<'a> {
    fn from_error_kind(input: Input<'a>, kind: ErrorKind) -> Self {
        Self::new(input, ParserErrorKind::Nom(kind))
    }

    fn append(_input: Input<'a>, _kind: ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: Input<'a>, ch: char) -> Self {
        Self::new(input, ParserErrorKind::ExpectedChar(ch))
    }
}

impl<'a> ContextError<Input<'a>> for ParserError<'a> {
    fn add_context(_input: Input<'a>, ctx: &'static str, mut other: Self) -> Self {
        other.contexts.push(ctx);
        other
    }
}

impl<'a, E> FromExternalError<Input<'a>, E> for ParserError<'a> {
    fn from_external_error(input: Input<'a>, kind: ErrorKind, _error: E) -> Self {
        Self::new(input, ParserErrorKind::Nom(kind))
    }
}

/// Structured parser error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserErrorKind {
    /// Error produced by a nom primitive or combinator.
    Nom(ErrorKind),
    /// Expected a specific character.
    ExpectedChar(char),
    /// String escape ended before an escaped character.
    MissingEscape,
    /// String literal contains a raw newline.
    NewlineInString,
    /// String literal ended before the closing quote.
    UnterminatedString,
    /// Hex byte escape is malformed.
    InvalidHexEscape,
    /// Unicode escape is malformed or outside valid scalar values.
    InvalidUnicodeEscape,
}
