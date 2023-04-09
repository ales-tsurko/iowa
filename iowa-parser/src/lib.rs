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

use std::ops::{Deref, DerefMut};

use nom::{
    branch::alt,
    character::complete::{char, multispace0},
    combinator::{all_consuming, complete, map, opt, value},
    multi::{fold_many0, many0, many1, separated_list0},
    sequence::{delimited, terminated},
    IResult,
};

pub use symbol::{Number, Symbol};

/// A chain of messages is a list of messages before a terminator.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct MessageChain<'a>(Vec<Message<'a>>);

impl<'a> MessageChain<'a> {
    /// Create a new message chain.
    pub fn new(messages: Vec<Message<'a>>) -> Self {
        Self(messages)
    }
}

impl<'a> From<Vec<Message<'a>>> for MessageChain<'a> {
    fn from(messages: Vec<Message<'a>>) -> Self {
        Self::new(messages)
    }
}

impl<'a> Deref for MessageChain<'a> {
    type Target = Vec<Message<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MessageChain<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The Message type.
#[derive(Debug, PartialEq, Clone)]
pub struct Message<'a> {
    /// The message.
    pub symbol: Symbol<'a>,
    /// Arguments.
    pub args: Vec<MessageChain<'a>>,
}

impl<'a> Message<'a> {
    /// Create a new message.
    pub fn new(symbol: Symbol<'a>, args: Vec<MessageChain<'a>>) -> Self {
        Self { symbol, args }
    }
}

/// Parser entry-point.
pub fn parse(input: &str) -> IResult<&str, Vec<MessageChain<'_>>> {
    all_consuming(many0(message_chain))(input)
}

fn message_chain(input: &str) -> IResult<&str, MessageChain<'_>> {
    let (input, messages) = many1(message)(input)?;
    let (input, _) = opt(span::terminator)(input)?;
    Ok((input, MessageChain::new(messages)))
}

fn message(input: &str) -> IResult<&str, Message<'_>> {
    let (rest, _) = many0(span::wcpad)(input)?;
    let (rest, symbol) = symbol::symbol(rest)?;
    let (rest, _) = opt(span::scpad)(rest)?;
    let (rest, args) = opt(arguments)(rest)?;
    Ok((rest, Message::new(symbol, args.unwrap_or_default())))
}

fn arguments(input: &str) -> IResult<&str, Vec<MessageChain<'_>>> {
    alt((
        delimited(
            char('('),
            separated_list0(char(','), message_chain),
            char(')'),
        ),
        delimited(
            char('['),
            separated_list0(char(','), message_chain),
            char(']'),
        ),
        delimited(
            char('{'),
            separated_list0(char(','), message_chain),
            char('}'),
        ),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        let input = "foo";
        assert_eq!(
            message(input),
            Ok(("", Message::new(Symbol::Identifier("foo".into()), vec![])))
        );

        let input = "foo()";
        assert_eq!(
            message(input),
            Ok(("", Message::new(Symbol::Identifier("foo".into()), vec![])))
        );

        let input = "foo(1, bar baz)";

        assert_eq!(
            message(input),
            Ok((
                "",
                Message::new(
                    Symbol::Identifier("foo".into()),
                    vec![
                        vec![Message::new(Symbol::Number(1.0.into()), vec![])].into(),
                        vec![
                            Message::new(Symbol::Identifier("bar".into()), vec![]),
                            Message::new(Symbol::Identifier("baz".into()), vec![])
                        ]
                        .into()
                    ]
                )
            ))
        );
    }

    #[test]
    fn test_parse_message_chain() {
        let input = "foo bar baz";
        assert_eq!(
            message_chain(input),
            Ok((
                "",
                MessageChain::new(vec![
                    Message::new(Symbol::Identifier("foo".into()), vec![]),
                    Message::new(Symbol::Identifier("bar".into()), vec![]),
                    Message::new(Symbol::Identifier("baz".into()), vec![])
                ])
            ))
        );

        let input = "foo bar baz;";
        assert_eq!(
            message_chain(input),
            Ok((
                "",
                MessageChain::new(vec![
                    Message::new(Symbol::Identifier("foo".into()), vec![]),
                    Message::new(Symbol::Identifier("bar".into()), vec![]),
                    Message::new(Symbol::Identifier("baz".into()), vec![])
                ])
            ))
        );

        let input = "foo() bar(1) baz;";
        assert_eq!(
            message_chain(input),
            Ok((
                "",
                MessageChain::new(vec![
                    Message::new(Symbol::Identifier("foo".into()), vec![]),
                    Message::new(
                        Symbol::Identifier("bar".into()),
                        vec![vec![Message::new(Symbol::Number(1.0.into()), vec![])].into()]
                    ),
                    Message::new(Symbol::Identifier("baz".into()), vec![]),
                ])
            ))
        );
    }
}
