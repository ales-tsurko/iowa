use nom::{
    Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::line_ending,
    combinator::value,
};

use crate::{Input, ParseResult};

pub(crate) fn comment(input: Input<'_>) -> ParseResult<'_, ()> {
    alt((line_comment, block_comment)).parse(input)
}

fn line_comment(input: Input<'_>) -> ParseResult<'_, ()> {
    value(
        (),
        (alt((tag("#"), tag("//"))), is_not("\n\r"), line_ending),
    )
    .parse(input)
}

fn block_comment(input: Input<'_>) -> ParseResult<'_, ()> {
    value((), (tag("/*"), take_until("*/"), tag("*/"))).parse(input)
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
    fn test_comment() {
        assert_eq!(
            parsed(line_comment(Input::new("# comment\n"))),
            Ok(("", ()))
        );
        assert_eq!(
            parsed(line_comment(Input::new("// comment\n"))),
            Ok(("", ()))
        );
    }

    #[test]
    fn test_block_comment() {
        let comment = r#"/* comment
                            on
                            multiple
                            lines
                            */"#;
        assert_eq!(parsed(block_comment(Input::new(comment))), Ok(("", ())));
    }
}
