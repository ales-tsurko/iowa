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
    character::complete::char,
    combinator::{all_consuming, opt},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, preceded, terminated},
    IResult,
};
use rayon::prelude::*;

pub use symbol::*;

/// A chain of messages is a list of messages before a terminator.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct MessageChain<'a>(Vec<Message<'a>>);

impl<'a> MessageChain<'a> {
    /// Create a new message chain.
    pub fn new(messages: Vec<Message<'a>>) -> Self {
        Self(messages)
    }

    fn sort(self) -> Self {
        let mut stack: Vec<Message> = Vec::new();
        let mut output = Vec::new();

        for msg in self.desugar_operators().0.into_iter() {
            match msg.symbol {
                Symbol::Operator(ref msg_op) => {
                    if let Some(top) = stack.last() {
                        if msg_op.precedence() > top.symbol.as_ref_op().precedence() {
                            Self::fold_op_args(&mut stack);
                        }
                    }
                    stack.push(msg);
                }
                _ => output.push(msg),
            }
        }

        Self::fold_op_args(&mut stack);

        output.append(&mut stack);

        output.into()
    }

    fn desugar_operators(self) -> Self {
        let mut stack = Vec::new();
        let mut output = Vec::new();

        for msg in self.0.into_iter() {
            match msg.symbol {
                Symbol::Operator(_) => {
                    if let Some(top) = stack.pop() {
                        output.push(top);
                    }
                    stack.push(msg);
                }
                _ => match stack.last_mut() {
                    Some(top) => top.push_to_first_arg(msg),
                    None => output.push(msg),
                },
            }
        }

        output.append(&mut stack);
        output.into()
    }

    fn fold_op_args(stack: &mut Vec<Message>) {
        if let Some(mut top) = stack.pop() {
            while let Some(mut next) = stack.pop() {
                if top.symbol.as_ref_op().precedence() < next.symbol.as_ref_op().precedence() {
                    next.push_to_first_arg(top);
                    top = next;
                } else {
                    stack.push(next);
                    break;
                }
            }

            stack.push(top);
        }
    }
}

impl<'a, M: Into<Vec<Message<'a>>>> From<M> for MessageChain<'a> {
    fn from(messages: M) -> Self {
        Self::new(messages.into())
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

/// Argument type.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Argument<'a>(Vec<MessageChain<'a>>);

impl<'a> Argument<'a> {
    /// Create a new argument.
    pub fn new(messages: Vec<MessageChain<'a>>) -> Self {
        Self(messages)
    }
}

impl<'a, M: Into<Vec<MessageChain<'a>>>> From<M> for Argument<'a> {
    fn from(messages: M) -> Self {
        Self::new(messages.into())
    }
}

impl<'a> Deref for Argument<'a> {
    type Target = Vec<MessageChain<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Argument<'_> {
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
    pub args: Vec<Argument<'a>>,
}

impl<'a> Message<'a> {
    /// Create a new message.
    pub fn new(symbol: Symbol<'a>, args: Vec<Argument<'a>>) -> Self {
        Self {
            symbol,
            args: args.into(),
        }
    }

    /// Push a message to the first argument.
    pub fn push_to_first_arg(&mut self, msg: Message<'a>) {
        if self.args.is_empty() {
            self.args.push(Argument::from([MessageChain::default()]));
        }

        self.args[0][0].push(msg);
    }
}

impl<'a> From<Symbol<'a>> for Message<'a> {
    fn from(symbol: Symbol<'a>) -> Self {
        Self::new(symbol, vec![])
    }
}

impl<'a, A: Into<Vec<Argument<'a>>>> From<(Symbol<'a>, A)> for Message<'a> {
    fn from((symbol, args): (Symbol<'a>, A)) -> Self {
        Self::new(symbol, args.into())
    }
}

/// Parser entry-point.
pub fn parse(input: &str) -> IResult<&str, Vec<MessageChain<'_>>> {
    let (rest, chains) = all_consuming(many0(delimited(
        many0(span::wcpad),
        message_chain,
        many0(span::wcpad),
    )))(input)?;

    Ok((rest, chains.into_par_iter().map(|c| c.sort()).collect()))
}

fn message_chain(input: &str) -> IResult<&str, MessageChain<'_>> {
    let (input, messages) = many1(message)(input)?;
    let (input, _) = opt(span::terminator)(input)?;
    Ok((input, MessageChain::new(messages)))
}

fn message(input: &str) -> IResult<&str, Message<'_>> {
    let (rest, _) = many0(span::scpad)(input)?;
    let (rest, symbol) = symbol(rest)?;
    let (rest, _) = opt(span::scpad)(rest)?;
    let (rest, args) = opt(arguments)(rest)?;
    Ok((rest, Message::new(symbol, args.unwrap_or_default())))
}

fn arguments(input: &str) -> IResult<&str, Vec<Argument<'_>>> {
    alt((
        delimited(
            char('('),
            terminated(
                separated_list0(char(','), argument),
                opt(preceded(char(','), many0(span::wcpad))),
            ),
            char(')'),
        ),
        delimited(char('['), separated_list0(char(','), argument), char(']')),
        delimited(char('{'), separated_list0(char(','), argument), char('}')),
    ))(input)
}

fn argument(input: &str) -> IResult<&str, Argument<'_>> {
    let (input, _) = many0(span::wcpad)(input)?;
    let (input, messages) = many1(message_chain)(input)?;
    let (input, _) = many0(span::wcpad)(input)?;
    Ok((input, Argument::new(messages)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arguments() {
        let input = r#"(m, n,
            // comment
            foo bar(1)
            baz(0) qux
          )"#;

        let expected: Vec<Argument<'_>> = [
            [[Symbol::Identifier("m".into()).into()].into()].into(),
            [[Symbol::Identifier("n".into()).into()].into()].into(),
            [
                [
                    Symbol::Identifier("foo".into()).into(),
                    (
                        Symbol::Identifier("bar".into()),
                        [[[Symbol::Number(1.0.into()).into()].into()].into()],
                    )
                        .into(),
                ]
                .into(),
                [
                    (
                        Symbol::Identifier("baz".into()),
                        [[[Symbol::Number(0.0.into()).into()].into()].into()],
                    )
                        .into(),
                    Symbol::Identifier("qux".into()).into(),
                ]
                .into(),
            ]
            .into(),
        ]
        .into();

        assert_eq!(arguments(input), Ok(("", expected)));
    }

    #[test]
    fn test_parse_message() {
        let input = "foo";
        assert_eq!(
            message(input),
            Ok(("", Symbol::Identifier("foo".into()).into()))
        );

        let input = "foo()";
        assert_eq!(
            message(input),
            Ok(("", Symbol::Identifier("foo".into()).into()))
        );

        let input = "foo(1, bar baz)";

        assert_eq!(
            message(input),
            Ok((
                "",
                Message::new(
                    Symbol::Identifier("foo".into()),
                    [
                        [[Symbol::Number(1.0.into()).into()].into()].into(),
                        [[
                            Message::new(Symbol::Identifier("bar".into()), vec![]),
                            Message::new(Symbol::Identifier("baz".into()), vec![])
                        ]
                        .into()]
                        .into()
                    ]
                    .into()
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
                [
                    Symbol::Identifier("foo".into()).into(),
                    (
                        Symbol::Identifier("bar".into()),
                        [[[Symbol::Number(1.0.into()).into()].into()].into()],
                    )
                        .into(),
                    Symbol::Identifier("baz".into()).into(),
                ]
                .into()
            ))
        );
    }

    #[test]
    fn test_desugar_operators() {
        let input = "foo bar + baz qux * foo bar";
        let chain = message_chain(input).unwrap().1;
        let expected = message_chain("foo bar +(baz qux) *(foo bar)").unwrap().1;
        assert_eq!(chain.desugar_operators(), expected,);
    }

    #[test]
    fn test_sort_message_chain() {
        let input = "1 >> 2 + 3";
        let expected = message_chain("1 >>(2 +(3))").unwrap().1;
        assert_eq!(message_chain(input).unwrap().1.sort(), expected);

        let input = "1 * 2 + 3 >> 4";
        let expected = message_chain("1 *(2) +(3) >>(4)").unwrap().1;
        assert_eq!(message_chain(input).unwrap().1.sort(), expected);

        let input = "1 + 2 * 3 + 4 >> 5";
        let expected = message_chain("1 +(2 *(3)) +(4) >>(5)").unwrap().1;
        assert_eq!(message_chain(input).unwrap().1.sort(), expected);

        let input = "1 >> 2 + 3 * 4 + 5 >> 6";
        let expected = message_chain("1 >>(2 +(3 *(4)) +(5)) >>(6)").unwrap().1;
        assert_eq!(message_chain(input).unwrap().1.sort(), expected);

        let input = "1 >> 2 bar + 3 * baz qux + 4 >> 5";
        let expected = message_chain("1 >>(2 bar +(3 *(baz qux)) +(4)) >>(5)")
            .unwrap()
            .1;
        assert_eq!(message_chain(input).unwrap().1.sort(), expected);
    }
}
