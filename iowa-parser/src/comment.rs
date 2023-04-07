use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::{line_ending, one_of},
    combinator::value,
    sequence::{pair, tuple},
    IResult,
};

pub(crate) fn comment(input: &str) -> IResult<&str, ()> {
    alt((line_comment, block_comment))(input)
}

fn line_comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        tuple((alt((tag("#"), tag("//"))), is_not("\n\r"), line_ending)),
    )(input)
}

fn block_comment(input: &str) -> IResult<&str, ()> {
    value((), tuple((tag("/*"), take_until("*/"), tag("*/"))))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment() {
        assert_eq!(line_comment("# comment\n"), Ok(("", ())));
        assert_eq!(line_comment("// comment\n"), Ok(("", ())));
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
