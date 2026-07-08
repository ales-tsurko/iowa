mod comment;

use comment::comment;
use nom::{
    Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, line_ending, one_of},
    combinator::{opt, recognize, value},
};

use crate::{Input, ParseResult};

pub(crate) fn scpad(input: Input<'_>) -> ParseResult<'_, ()> {
    value((), alt((separator, comment))).parse(input)
}

pub(crate) fn wcpad(input: Input<'_>) -> ParseResult<'_, ()> {
    value((), alt((whitespace, comment))).parse(input)
}

pub(crate) fn terminator(input: Input<'_>) -> ParseResult<'_, ()> {
    value(
        (),
        alt((
            recognize((opt(separator), tag(";"))),
            recognize(line_ending),
            recognize((char('\r'), opt(separator))),
        )),
    )
    .parse(input)
}

fn separator(input: Input<'_>) -> ParseResult<'_, ()> {
    value((), alt((char(' '), char('\t'), char('\x0c'), char('\x0b')))).parse(input)
}

pub(crate) fn whitespace(input: Input<'_>) -> ParseResult<'_, ()> {
    value((), one_of(" \t\r\n\x0b\x0c")).parse(input)
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
    fn test_parse_separator() {
        assert_eq!(parsed(separator(Input::new(" "))), Ok(("", ())));
        assert_eq!(parsed(separator(Input::new("\t"))), Ok(("", ())));
        assert_eq!(parsed(separator(Input::new("\x0c"))), Ok(("", ())));
        assert_eq!(parsed(separator(Input::new("\x0b"))), Ok(("", ())));
    }

    #[test]
    fn test_parse_terminator() {
        assert_eq!(parsed(terminator(Input::new(";"))), Ok(("", ())));
        assert_eq!(parsed(terminator(Input::new(";\n"))), Ok(("\n", ())));
        assert_eq!(parsed(terminator(Input::new("; \r"))), Ok((" \r", ())));
        assert_eq!(parsed(terminator(Input::new("\r"))), Ok(("", ())));
    }

    #[test]
    fn test_parse_scpad() {
        assert_eq!(parsed(scpad(Input::new(" "))), Ok(("", ())));
        assert_eq!(parsed(scpad(Input::new("# comment\n"))), Ok(("", ())));
    }

    #[test]
    fn test_parse_wcpad() {
        assert_eq!(parsed(wcpad(Input::new(" "))), Ok(("", ())));
        assert_eq!(parsed(wcpad(Input::new("\n"))), Ok(("", ())));
        assert_eq!(parsed(wcpad(Input::new("\r"))), Ok(("", ())));
        assert_eq!(parsed(wcpad(Input::new("# comment\n"))), Ok(("", ())));
    }
}
