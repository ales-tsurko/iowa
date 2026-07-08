use nom::{Parser, bytes::complete::take_while1, combinator::map};

use crate::{Input, ParseResult};

/// Operator token.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Operator<'a>(pub &'a str);

impl<'a> From<&'a str> for Operator<'a> {
    fn from(input: &'a str) -> Self {
        Self(input)
    }
}

impl<'a> Operator<'a> {
    /// Get the operator text.
    pub fn name(&self) -> &'a str {
        self.0
    }
}

pub(crate) fn operator(input: Input<'_>) -> ParseResult<'_, Operator<'_>> {
    map(take_while1(is_operator_char), |op: Input<'_>| {
        Operator(op.fragment())
    })
    .parse(input)
}

fn is_operator_char(ch: char) -> bool {
    matches!(
        ch,
        ':' | '.'
            | '\''
            | '~'
            | '!'
            | '@'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '-'
            | '+'
            | '/'
            | '='
            | '|'
            | '\\'
            | '<'
            | '>'
            | '?'
    )
}
