mod number;
mod operator;
mod quote;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    combinator::map,
    IResult,
};

use self::quote::quote;

pub use number::Number;
pub use operator::*;
pub use quote::Quote;

/// The Symbol type.
#[derive(Debug, Clone)]
pub enum Symbol<'a> {
    /// Identifier.
    Identifier(Identifier<'a>),
    /// Number.
    Number(Number),
    /// Operator.
    Operator(Box<dyn Operator>),
    /// Quote.
    Quote(Quote),
}

impl Symbol<'_> {
    pub(crate) fn as_ref_op(&self) -> &Box<dyn Operator> {
        match self {
            Self::Operator(ref op) => op,
            _ => unreachable!(),
        }
    }
}

impl PartialEq for Symbol<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Identifier(a), Self::Identifier(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Operator(a), Self::Operator(b)) => a.symbol() == b.symbol(),
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

impl<T: Operator> From<T> for Symbol<'_> {
    fn from(input: T) -> Self {
        Self::Operator(Box::new(input))
    }
}

impl From<Quote> for Symbol<'_> {
    fn from(input: Quote) -> Self {
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

pub(crate) fn symbol(input: &str) -> IResult<&str, Symbol<'_>> {
    alt((
        map(op_token, Symbol::Operator),
        map(quote, Symbol::Quote),
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

fn op_token(input: &str) -> IResult<&str, Box<dyn Operator>> {
    let table = OperatorTable::global().table.lock().unwrap();
    for op in &*table {
        let res: IResult<&str, &str> = tag(op.symbol())(input);
        if let Ok((input, _)) = res {
            return Ok((input, op.clone()));
        }
    }
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
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
            "==", "&", "^", "|", "&&", "and", "||", "or", "..", "=", ":=", "::=", "%=", "*=", "/=",
            "+=", "-=", "<<=", ">>=", "&=", "^=", "|=", "return",
        ];

        for op in ops {
            assert_eq!(op_token(op).unwrap().1.symbol(), op);
        }
    }
}
