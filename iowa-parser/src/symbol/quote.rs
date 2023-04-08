use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, none_of},
    combinator::{map, recognize},
    multi::many0,
    sequence::{delimited, preceded},
    IResult,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Quote<'a>(&'a str);

pub(crate) fn quote(input: &str) -> IResult<&str, Quote<'_>> {
    map(alt((tri_quote, mono_quote)), Quote)(input)
}

fn mono_quote(input: &str) -> IResult<&str, &str> {
    let inner = recognize(many0(alt((
        none_of(r#""\"#),
        preceded(char('\\'), alt((char('"'), char('\\')))),
    ))));
    delimited(char('"'), inner, char('"'))(input)
}

fn tri_quote(input: &str) -> IResult<&str, &str> {
    delimited(tag("\"\"\""), take_until("\"\"\""), tag("\"\"\""))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mono_quote() {
        assert_eq!(mono_quote(r#""""#), Ok(("", "")));
        assert_eq!(mono_quote(r#""test""#), Ok(("", "test")));
        assert_eq!(
            mono_quote(r#""hello, \"world\"""#),
            Ok(("", "hello, \\\"world\\\""))
        );
    }

    #[test]
    fn test_parse_tri_quote() {
        assert_eq!(tri_quote(r#""""""""#), Ok(("", "")));
        assert_eq!(
            tri_quote(r#""""Hello, world!""""#),
            Ok(("", "Hello, world!"))
        );
        assert_eq!(
            tri_quote(
                r#""""This is a "test" test,
    hello!""""#
            ),
            Ok(("", "This is a \"test\" test,\n    hello!"))
        );
    }

    #[test]
    fn test_parse_quote() {
        assert_eq!(quote(r#""test""#), Ok(("", Quote("test"))));
        assert_eq!(
            quote(r#""hello, \"world\"""#),
            Ok(("", Quote("hello, \\\"world\\\"")))
        );
        assert_eq!(quote(r#""""""""#), Ok(("", Quote(""))));
        assert_eq!(
            quote(r#""""Hello, world!""""#),
            Ok(("", Quote("Hello, world!")))
        );
    }
}
