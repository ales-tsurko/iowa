//! Io programming language parser.

#![warn(
    clippy::all,
    deprecated_in_future,
    missing_docs,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unreachable_pub
)]

mod span;
mod symbol;

use nom::{
    branch::alt,
    character::complete::char,
    combinator::{all_consuming, eof, map, opt, value},
    multi::many0,
    sequence::{delimited, preceded, terminated},
    IResult,
};

pub use symbol::Symbol;

/// Expression.
#[derive(Debug, PartialEq, Clone)]
pub enum Expression<'a> {
    /// Message.
    Message(Message<'a>),
    /// Terminator.
    Terminator,
    /// Space.
    Space,
}

/// The Message type.
#[derive(Debug, PartialEq, Clone)]
pub struct Message<'a> {
    /// The message.
    pub symbol: Symbol<'a>,
    /// Arguments.
    pub args: Vec<Expression<'a>>,
}

impl<'a> Message<'a> {
    /// Create a new message.
    pub fn new(symbol: Symbol<'a>, args: Vec<Expression<'a>>) -> Self {
        Self { symbol, args }
    }
}

/// Parser entry-point.
pub fn parse(input: &str) -> IResult<&str, Vec<Expression<'_>>> {
    all_consuming(terminated(many0(expression), eof))(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(span::sctpad, |res| match res {
            Some(_) => Expression::Terminator,
            None => Expression::Space,
        }),
        map(message, Expression::Message),
    ))(input)
}

fn message(input: &str) -> IResult<&str, Message<'_>> {
    let (rest, _) = opt(span::wcpad)(input)?;
    let (rest, symbol) = symbol::symbol(rest)?;
    let (rest, args) = preceded(opt(span::scpad), opt(arguments))(rest)?;
    Ok((rest, Message::new(symbol, args.unwrap_or_default())))
}

fn arguments(input: &str) -> IResult<&str, Vec<Expression<'_>>> {
    let inner = many0(alt((argument, preceded(char(','), argument))));
    delimited(open, inner, close)(input)
}

fn argument(input: &str) -> IResult<&str, Expression> {
    delimited(opt(span::wcpad), expression, opt(span::wcpad))(input)
}

fn open(input: &str) -> IResult<&str, ()> {
    value((), alt((char('('), char('['), char('{'))))(input)
}

fn close(input: &str) -> IResult<&str, ()> {
    value((), alt((char(')'), char(']'), char('}'))))(input)
}
