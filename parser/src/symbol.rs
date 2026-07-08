mod number;
mod operator;
mod quote;

use nom::{Parser, branch::alt, bytes::complete::take_while1, combinator::map};
pub use number::Number;
pub use operator::Operator;
pub use quote::Quote;

use self::quote::quote;
use crate::{Input, ParseResult};

/// The Symbol type.
#[derive(Debug, Clone)]
pub enum Symbol<'a> {
    /// Identifier.
    Identifier(Identifier<'a>),
    /// Number.
    Number(Number),
    /// Operator.
    Operator(Operator<'a>),
    /// Quote.
    Quote(Quote),
}

impl PartialEq for Symbol<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Identifier(a), Self::Identifier(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Operator(a), Self::Operator(b)) => a == b,
            (Self::Quote(a), Self::Quote(b)) => a == b,
            _ => false,
        }
    }
}

impl<'a> From<Identifier<'a>> for Symbol<'a> {
    fn from(input: Identifier<'a>) -> Self {
        Self::Identifier(input)
    }
}

impl From<Number> for Symbol<'_> {
    fn from(input: Number) -> Self {
        Self::Number(input)
    }
}

impl<'a> From<Operator<'a>> for Symbol<'a> {
    fn from(input: Operator<'a>) -> Self {
        Self::Operator(input)
    }
}

impl From<Quote> for Symbol<'_> {
    fn from(input: Quote) -> Self {
        Self::Quote(input)
    }
}

/// Identifier token.
#[derive(Debug, PartialEq, Clone)]
pub struct Identifier<'a>(pub &'a str);

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(input: &'a str) -> Self {
        Self(input)
    }
}

impl<'a> Identifier<'a> {
    /// Get the string value of the identifier
    pub fn name(&self) -> &'a str {
        self.0
    }
}

pub(crate) fn symbol(input: Input<'_>) -> ParseResult<'_, Symbol<'_>> {
    alt((
        map(quote, Symbol::Quote),
        map(number::number, Symbol::Number),
        map(operator::operator, Symbol::Operator),
        map(identifier, Symbol::Identifier),
    ))
    .parse(input)
}

fn identifier(input: Input<'_>) -> ParseResult<'_, Identifier<'_>> {
    let id_parser = take_while1(|c: char| c.is_alphanumeric() || c == '_');
    map(id_parser, |id: Input<'_>| Identifier(id.fragment())).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed<'a, T>(
        result: ParseResult<'a, T>,
    ) -> Result<(&'a str, T), nom::Err<crate::ParserError<'a>>> {
        result.map(|(rest, value)| (*rest.fragment(), value))
    }

    #[test]
    fn test_parse_identifier() {
        assert_eq!(
            parsed(identifier(Input::new("foo"))),
            Ok(("", Identifier("foo")))
        );
        assert_eq!(
            parsed(identifier(Input::new("foo_bar"))),
            Ok(("", Identifier("foo_bar")))
        );
        assert_eq!(
            parsed(identifier(Input::new("foo_bar_123_"))),
            Ok(("", Identifier("foo_bar_123_")))
        );
        assert_eq!(
            parsed(identifier(Input::new("тест"))),
            Ok(("", Identifier("тест")))
        );
        assert_eq!(
            parsed(identifier(Input::new("_тест"))),
            Ok(("", Identifier("_тест")))
        );
        assert_eq!(
            parsed(identifier(Input::new("_"))),
            Ok(("", Identifier("_")))
        );
    }

    #[test]
    fn test_parse_operator() {
        let ops = [
            "?", "@", "@@", "**", "%", "*", "/", "+", "-", "<<", ">>", "<", "<=", ">", ">=", "!=",
            "==", "&", "^", "|", "&&", "||", "..", "=", ":=", "::=", "%=", "*=", "/=", "+=", "-=",
            "<<=", ">>=", "&=", "^=", "|=", "%%", "<=>", "!!!", "?!",
        ];

        for op in ops {
            assert_eq!(
                operator::operator(Input::new(op))
                    .map(|(_rest, operator)| operator)
                    .expect("operator should parse")
                    .name(),
                op
            );
        }
    }
}
