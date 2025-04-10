use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::line_ending,
    combinator::value,
};

pub(crate) fn comment(input: &str) -> IResult<&str, ()> {
    alt((line_comment, block_comment)).parse(input)
}

fn line_comment(input: &str) -> IResult<&str, ()> {
    value(
        (),
        (alt((tag("#"), tag("//"))), is_not("\n\r"), line_ending),
    )
    .parse(input)
}

fn block_comment(input: &str) -> IResult<&str, ()> {
    value((), (tag("/*"), take_until("*/"), tag("*/"))).parse(input)
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
