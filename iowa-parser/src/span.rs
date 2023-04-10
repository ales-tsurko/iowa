mod comment;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, line_ending, one_of},
    combinator::{opt, recognize, value},
    sequence::tuple,
    IResult,
};

use comment::comment;

pub(crate) fn scpad(input: &str) -> IResult<&str, ()> {
    value((), alt((separator, comment)))(input)
}

pub(crate) fn wcpad(input: &str) -> IResult<&str, ()> {
    value((), alt((whitespace, comment)))(input)
}

pub(crate) fn terminator(input: &str) -> IResult<&str, ()> {
    value(
        (),
        alt((
            recognize(tuple((opt(separator), tag(";")))),
            recognize(line_ending),
            recognize(tuple((char('\r'), opt(separator)))),
        )),
    )(input)
}

fn separator(input: &str) -> IResult<&str, ()> {
    value((), alt((char(' '), char('\t'), char('\x0c'), char('\x0b'))))(input)
}

pub(crate) fn whitespace(input: &str) -> IResult<&str, ()> {
    value((), one_of(" \t\r\n\x0b\x0c"))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_separator() {
        assert_eq!(separator(" "), Ok(("", ())));
        assert_eq!(separator("\t"), Ok(("", ())));
        assert_eq!(separator("\x0c"), Ok(("", ())));
        assert_eq!(separator("\x0b"), Ok(("", ())));
    }

    #[test]
    fn test_parse_terminator() {
        assert_eq!(terminator(";"), Ok(("", ())));
        assert_eq!(terminator(";\n"), Ok(("\n", ())));
        assert_eq!(terminator("; \r"), Ok((" \r", ())));
        assert_eq!(terminator("\r"), Ok(("", ())));
    }

    #[test]
    fn test_parse_scpad() {
        assert_eq!(scpad(" "), Ok(("", ())));
        assert_eq!(scpad("# comment\n"), Ok(("", ())));
    }

    #[test]
    fn test_parse_wcpad() {
        assert_eq!(wcpad(" "), Ok(("", ())));
        assert_eq!(wcpad("\n"), Ok(("", ())));
        assert_eq!(wcpad("\r"), Ok(("", ())));
        assert_eq!(wcpad("# comment\n"), Ok(("", ())));
    }
}
