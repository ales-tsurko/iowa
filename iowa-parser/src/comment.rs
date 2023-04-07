//! Comment parser.

use nom::{
    branch::alt,
    bytes::complete::is_not,
    bytes::complete::{tag, take_until},
    character::complete::one_of,
    combinator::value,
    sequence::{pair, tuple},
    IResult,
};

/// Parse comment.
pub fn comment(input: &str) -> IResult<&str, ()> {
    alt((line_comment, block_comment))(input)
}

fn line_comment(input: &str) -> IResult<&str, ()> {
    value((), pair(one_of("#/"), is_not("\n\r")))(input)
}

fn block_comment(input: &str) -> IResult<&str, ()> {
    value((), tuple((tag("/*"), take_until("*/"), tag("*/"))))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment() {
        assert_eq!(line_comment("# comment"), Ok(("", ())));
        assert_eq!(line_comment("/ comment"), Ok(("", ())));
    }

    #[test]
    fn test_block_comment() {
        let comment = r#"/* comment
                            on
                            multiple
                            lines
                            */"#;
        assert_eq!(block_comment(comment), Ok(("", ())));
    }
}