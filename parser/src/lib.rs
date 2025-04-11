//! Io programming language parser.

mod span;
mod symbol;

use std::ops::{Deref, DerefMut};

use nom::{
    IResult, Offset, Parser,
    branch::alt,
    character::complete::char,
    combinator::{all_consuming, opt},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, preceded, terminated},
};
use rayon::prelude::*;

pub use symbol::*;

/// Location information for AST nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    /// Start offset in the source
    pub start: usize,
    /// End offset in the source
    pub end: usize,
}

impl Span {
    /// Create a new span with start and end positions
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a span from input string and remaining substring
    pub fn from_input(input: &str, remaining: &str) -> Self {
        let start = 0;
        // Note: nom's offset works with byte positions, which is correct for UTF-8
        let end = input.offset(remaining);
        Self { start, end }
    }

    /// Create a span with explicit start and length from the input
    pub fn with_length(input: &str, start_offset: usize, length: usize) -> Self {
        Self {
            start: start_offset,
            end: start_offset + length.min(input[start_offset..].len()),
        }
    }

    /// Get line and column information from the span and source text
    pub fn location_info(&self, source: &str) -> (usize, usize) {
        // Handle UTF-8 correctly by working with byte offsets and converting to char counts
        let bytes_before = &source[..self.start];

        // Count newlines to determine line number (1-based)
        let line = bytes_before.chars().filter(|&c| c == '\n').count() + 1;

        // Calculate column by finding the last newline
        let last_newline = bytes_before.rfind('\n');
        let column = match last_newline {
            Some(pos) => {
                // Count Unicode characters from the last newline to the span start
                bytes_before[pos + 1..].chars().count() + 1
            }
            None => {
                // No newline found, column is the number of Unicode characters
                bytes_before.chars().count() + 1
            }
        };

        (line, column)
    }

    /// Convert span to a human-readable string with line:column format
    pub fn to_location_string(&self, source: &str) -> String {
        let (line, column) = self.location_info(source);
        format!("{}:{}", line, column)
    }
}

/// A chain of messages is a list of messages before a terminator.
#[derive(Debug, Default, PartialEq)]
pub struct MessageChain<'a> {
    /// The messages in this chain
    pub messages: Vec<Message<'a>>,
    /// Source location information
    pub span: Option<Span>,
}

impl<'a> MessageChain<'a> {
    /// Create a new message chain.
    pub fn new(messages: Vec<Message<'a>>) -> Self {
        Self {
            messages,
            span: None,
        }
    }

    /// Create a new message chain with span information
    pub fn with_span(messages: Vec<Message<'a>>, span: Span) -> Self {
        Self {
            messages,
            span: Some(span),
        }
    }

    fn sort(self) -> Self {
        let mut stack: Vec<Message> = Vec::new();
        let mut output = Vec::new();
        let span = self.span;

        for mut msg in self.desugar_operators().messages.into_iter() {
            // sort arguments
            msg.args = msg
                .args
                .into_par_iter()
                .map(|arg| {
                    arg.chains
                        .into_par_iter()
                        .map(|chain: MessageChain| chain.sort())
                        .collect::<Vec<MessageChain>>()
                        .into()
                })
                .collect();
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

        Self {
            messages: output,
            span,
        }
    }

    fn desugar_operators(self) -> Self {
        let mut stack = Vec::new();
        let mut output = Vec::new();
        let span = self.span;

        for msg in self.messages.into_iter() {
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

        Self {
            messages: output,
            span,
        }
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

// Helper for testing - compare MessageChains ignoring spans
impl<'a> PartialEq<MessageChain<'a>> for [Message<'a>] {
    fn eq(&self, other: &MessageChain<'a>) -> bool {
        &other.messages[..] == self
    }
}

impl<'a> Deref for MessageChain<'a> {
    type Target = Vec<Message<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

impl DerefMut for MessageChain<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.messages
    }
}

/// Argument type.
#[derive(Debug, Default, PartialEq)]
pub struct Argument<'a> {
    /// Message chains in this argument
    pub chains: Vec<MessageChain<'a>>,
    /// Source location information
    pub span: Option<Span>,
}

impl<'a> Argument<'a> {
    /// Create a new argument.
    pub fn new(chains: Vec<MessageChain<'a>>) -> Self {
        Self { chains, span: None }
    }

    /// Create a new argument with span information
    pub fn with_span(chains: Vec<MessageChain<'a>>, span: Span) -> Self {
        Self {
            chains,
            span: Some(span),
        }
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
        &self.chains
    }
}

impl DerefMut for Argument<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.chains
    }
}

/// The Message type.
#[derive(Debug, PartialEq)]
pub struct Message<'a> {
    /// The message.
    pub symbol: Symbol<'a>,
    /// Arguments.
    pub args: Vec<Argument<'a>>,
    /// Source location information
    pub span: Option<Span>,
}

impl<'a> Message<'a> {
    /// Create a new message.
    pub fn new(symbol: Symbol<'a>, args: Vec<Argument<'a>>) -> Self {
        Self {
            symbol,
            args: args.into(),
            span: None,
        }
    }

    /// Create a new message with span information
    pub fn with_span(symbol: Symbol<'a>, args: Vec<Argument<'a>>, span: Span) -> Self {
        Self {
            symbol,
            args: args.into(),
            span: Some(span),
        }
    }

    /// Push a message to the first argument.
    pub fn push_to_first_arg(&mut self, msg: Message<'a>) {
        if self.args.is_empty() {
            self.args.push(Argument::from([MessageChain::default()]));
        }

        self.args[0][0].messages.push(msg);
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
    let delimited_parser = delimited(many0(span::wcpad), message_chain, many0(span::wcpad));
    let many_parser = many0(delimited_parser);
    let (rest, chains) = all_consuming(many_parser).parse(input)?;

    Ok((rest, chains.into_par_iter().map(|c| c.sort()).collect()))
}

fn message_chain(input: &str) -> IResult<&str, MessageChain<'_>> {
    let start_input = input;
    let (input, messages) = many1(message).parse(input)?;
    let (input, _) = opt(span::terminator).parse(input)?;

    // Calculate span from the original input to current position
    let span = Span::new(0, start_input.offset(input));

    Ok((input, MessageChain::with_span(messages, span)))
}

fn message(input: &str) -> IResult<&str, Message<'_>> {
    let start_input = input;
    let (rest, _) = many0(span::scpad).parse(input)?;
    let (rest, symbol) = symbol(rest)?;
    let (rest, _) = many0(span::scpad).parse(rest)?;
    let (rest, args) = opt(arguments).parse(rest)?;

    // Calculate span from the original input to current position
    let span = Span::new(0, start_input.offset(rest));

    Ok((
        rest,
        Message::with_span(symbol, args.unwrap_or_default(), span),
    ))
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
    ))
    .parse(input)
}

fn argument(input: &str) -> IResult<&str, Argument<'_>> {
    let start_input = input;
    let (input, _) = many0(span::wcpad).parse(input)?;
    let (input, chains) = many1(message_chain).parse(input)?;
    let (input, _) = many0(span::wcpad).parse(input)?;

    // Calculate span from the original input to current position
    let span = Span::new(0, start_input.offset(input));

    Ok((input, Argument::with_span(chains, span)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to ignore spans when comparing in tests
    fn ignore_spans<'a>(chain: MessageChain<'a>) -> MessageChain<'a> {
        let messages = chain
            .messages
            .into_iter()
            .map(|mut msg| {
                msg.span = None;
                msg.args = msg
                    .args
                    .into_iter()
                    .map(|mut arg| {
                        arg.span = None;
                        arg.chains = arg.chains.into_iter().map(ignore_spans).collect();
                        arg
                    })
                    .collect();
                msg
            })
            .collect();

        MessageChain {
            messages,
            span: None,
        }
    }

    #[test]
    fn test_span_unicode_handling() {
        // Test with ASCII characters
        let ascii_text = "hello\nworld";
        let span1 = Span::new(0, 5); // "hello"
        let span2 = Span::new(6, 11); // "world"

        assert_eq!(span1.location_info(ascii_text), (1, 1)); // Line 1, column 1
        assert_eq!(span2.location_info(ascii_text), (2, 1)); // Line 2, column 1

        // Test with Unicode characters
        let unicode_text = "привет\nмир";
        let span3 = Span::with_length(unicode_text, 0, "привет".len());
        let span4 = Span::with_length(unicode_text, "привет\n".len(), "мир".len());

        assert_eq!(span3.location_info(unicode_text), (1, 1)); // Line 1, column 1
        assert_eq!(span4.location_info(unicode_text), (2, 1)); // Line 2, column 1

        // Test with mixed Unicode and ASCII, and middle-of-line positions
        let mixed_text = "hello привет\nworld мир";
        let span5 = Span::with_length(mixed_text, "hello ".len(), "привет".len());

        assert_eq!(span5.location_info(mixed_text), (1, 7)); // Line 1, column 7

        // Test with emoji characters (which can be multiple bytes per character)
        let emoji_text = "hello 👋\nworld 🌍";
        let span6 = Span::with_length(emoji_text, "hello ".len(), "👋".len());

        assert_eq!(span6.location_info(emoji_text), (1, 7)); // Line 1, column 7
    }

    #[test]
    fn test_parse_arguments() {
        let input = r#"(m, n,
            // comment
            foo bar(1)
            baz(0) qux
          )"#;

        // Parse and ignore spans for comparison
        let result = arguments(input).map(|(rest, args)| {
            // Create a Vec of Arguments with spans removed
            let args_without_spans = args
                .into_iter()
                .map(|arg| {
                    let chains = arg.chains.into_iter().map(ignore_spans).collect();
                    Argument::new(chains)
                })
                .collect();

            (rest, args_without_spans)
        });

        // Create expected arguments without spans
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

        assert_eq!(result, Ok(("", expected)));
    }

    #[test]
    fn test_parse_message() {
        let input = "foo";
        let result = message(input).map(|(rest, msg)| (rest, Message::new(msg.symbol, msg.args)));
        assert_eq!(result, Ok(("", Symbol::Identifier("foo".into()).into())));

        let input = "foo()";
        let result = message(input).map(|(rest, msg)| (rest, Message::new(msg.symbol, msg.args)));
        assert_eq!(result, Ok(("", Symbol::Identifier("foo".into()).into())));

        let input = "foo(1, bar baz)";
        let result = message(input).map(|(rest, msg)| {
            let args = msg
                .args
                .into_iter()
                .map(|arg| {
                    let chains = arg.chains.into_iter().map(ignore_spans).collect();
                    Argument::new(chains)
                })
                .collect();

            (rest, Message::new(msg.symbol, args))
        });

        assert_eq!(
            result,
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
        let result = message_chain(input).map(|(rest, chain)| (rest, ignore_spans(chain)));
        assert_eq!(
            result,
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
        let result = message_chain(input).map(|(rest, chain)| (rest, ignore_spans(chain)));
        assert_eq!(
            result,
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
        let result = message_chain(input).map(|(rest, chain)| (rest, ignore_spans(chain)));

        // Create the expected message chain without spans
        let expected_chain = MessageChain::new(vec![
            Symbol::Identifier("foo".into()).into(),
            (
                Symbol::Identifier("bar".into()),
                [[[Symbol::Number(1.0.into()).into()].into()].into()],
            )
                .into(),
            Symbol::Identifier("baz".into()).into(),
        ]);

        assert_eq!(result, Ok(("", expected_chain)));
    }

    #[test]
    fn test_desugar_operators() {
        let input = "foo bar + baz qux * foo bar";
        let chain = message_chain(input).unwrap().1;
        let expected = message_chain("foo bar +(baz qux) *(foo bar)").unwrap().1;

        // Use ignore_spans for both to compare ignoring span information
        assert_eq!(
            ignore_spans(chain.desugar_operators()),
            ignore_spans(expected)
        );
    }

    #[test]
    fn test_sort_message_chain() {
        let input = "1 >> 2 + 3";
        let expected = message_chain("1 >>(2 +(3))").unwrap().1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );

        let input = "1 * 2 + 3 >> 4";
        let expected = message_chain("1 *(2) +(3) >>(4)").unwrap().1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );

        let input = "1 + 2 * 3 + 4 >> 5";
        let expected = message_chain("1 +(2 *(3)) +(4) >>(5)").unwrap().1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );

        let input = "1 >> 2 + 3 * 4 + 5 >> 6";
        let expected = message_chain("1 >>(2 +(3 *(4)) +(5)) >>(6)").unwrap().1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );

        let input = "1 >> 2 bar + 3 * baz qux + 4 >> 5";
        let expected = message_chain("1 >>(2 bar +(3 *(baz qux)) +(4)) >>(5)")
            .unwrap()
            .1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );

        let input = "1 >> 2 bar + 3 * baz qux(2 + 2 * 2 >> 3) + 4 >> 5";
        let expected = message_chain("1 >>(2 bar +(3 *(baz qux(2 +(2 *(2)) >>(3)))) +(4)) >>(5)")
            .unwrap()
            .1;
        assert_eq!(
            ignore_spans(message_chain(input).unwrap().1.sort()),
            ignore_spans(expected)
        );
    }
}
