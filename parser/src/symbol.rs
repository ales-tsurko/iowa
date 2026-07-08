mod number;
mod operator;
mod quote;

use nom::{IResult, Parser, branch::alt, bytes::complete::take_while1, combinator::map};

use self::quote::quote;

pub use number::Number;
pub use operator::Operator;
pub use quote::Quote;

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

pub(crate) fn symbol(input: &str) -> IResult<&str, Symbol<'_>> {
    alt((
        map(quote, Symbol::Quote),
        map(number::number, Symbol::Number),
        map(operator::operator, Symbol::Operator),
        map(identifier, Symbol::Identifier),
    ))
    .parse(input)
}

fn identifier(input: &str) -> IResult<&str, Identifier<'_>> {
    let id_parser = take_while1(|c: char| c.is_alphanumeric() || c == '_');
    map(id_parser, Identifier).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_identifier() {
        assert_eq!(identifier("foo"), Ok(("", Identifier("foo"))));
        assert_eq!(identifier("foo_bar"), Ok(("", Identifier("foo_bar"))));
        assert_eq!(
            identifier("foo_bar_123_"),
            Ok(("", Identifier("foo_bar_123_")))
        );
        assert_eq!(identifier("тест"), Ok(("", Identifier("тест"))));
        assert_eq!(identifier("_тест"), Ok(("", Identifier("_тест"))));
        assert_eq!(identifier("_"), Ok(("", Identifier("_"))));
    }

    #[test]
    fn test_parse_operator() {
        let ops = [
            "?", "@", "@@", "**", "%", "*", "/", "+", "-", "<<", ">>", "<", "<=", ">", ">=", "!=",
            "==", "&", "^", "|", "&&", "||", "..", "=", ":=", "::=", "%=", "*=", "/=", "+=", "-=",
            "<<=", ">>=", "&=", "^=", "|=", "%%", "<=>", "!!!", "?!",
        ];

        for op in ops {
            assert_eq!(operator::operator(op).unwrap().1.name(), op);
        }
    }
}
