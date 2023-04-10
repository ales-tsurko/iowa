mod number;
mod quote;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    combinator::map,
    IResult,
};

use self::quote::quote;

pub use number::Number;
pub use quote::Quote;

/// The Symbol type.
#[derive(Debug, PartialEq, Clone)]
pub enum Symbol<'a> {
    /// Identifier.
    Identifier(Identifier<'a>),
    /// Number.
    Number(Number),
    /// Operator.
    Operator(Operator),
    /// Quote.
    Quote(Quote<'a>),
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

impl From<Operator> for Symbol<'_> {
    fn from(input: Operator) -> Self {
        Self::Operator(input)
    }
}

impl<'a> From<Quote<'a>> for Symbol<'a> {
    fn from(input: Quote<'a>) -> Self {
        Self::Quote(input)
    }
}

/// Identifier token.
#[derive(Debug, PartialEq, Clone)]
pub struct Identifier<'a>(&'a str);

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(input: &'a str) -> Self {
        Self(input)
    }
}

/// Operator token.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Operator {
    Colon,
    Dot,
    Quote,
    Tilde,
    Bang,
    At,
    Dollar,
    Percent,
    Caret,
    And,
    Star,
    Minus,
    Plus,
    Slash,
    Equal,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Bar,
    Backslash,
    LessThan,
    GreaterThan,
    QuestionMark,
}

pub(crate) fn symbol(input: &str) -> IResult<&str, Symbol<'_>> {
    alt((
        map(quote, Symbol::Quote),
        map(op_token, Symbol::Operator),
        map(number::number, Symbol::Number),
        map(identifier, Symbol::Identifier),
    ))(input)
}

fn identifier(input: &str) -> IResult<&str, Identifier<'_>> {
    map(
        take_while1(|c: char| c.is_alphanumeric() || c == '_'),
        Identifier,
    )(input)
}

fn op_token(input: &str) -> IResult<&str, Operator> {
    alt((
        map(tag(":"), |_| Operator::Colon),
        map(tag("."), |_| Operator::Dot),
        map(tag("'"), |_| Operator::Quote),
        map(tag("~"), |_| Operator::Tilde),
        map(tag("!"), |_| Operator::Bang),
        map(tag("@"), |_| Operator::At),
        map(tag("$"), |_| Operator::Dollar),
        map(tag("%"), |_| Operator::Percent),
        map(tag("^"), |_| Operator::Caret),
        map(tag("&"), |_| Operator::And),
        map(tag("*"), |_| Operator::Star),
        map(tag("-"), |_| Operator::Minus),
        alt((
            map(tag("+"), |_| Operator::Plus),
            map(tag("/"), |_| Operator::Slash),
            map(tag("="), |_| Operator::Equal),
            map(tag("{"), |_| Operator::LBrace),
            map(tag("}"), |_| Operator::RBrace),
            map(tag("["), |_| Operator::LBracket),
            map(tag("]"), |_| Operator::RBracket),
            map(tag("|"), |_| Operator::Bar),
            map(tag("\\"), |_| Operator::Backslash),
            map(tag("<"), |_| Operator::LessThan),
            map(tag(">"), |_| Operator::GreaterThan),
            map(tag("?"), |_| Operator::QuestionMark),
        )),
    ))(input)
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
}
