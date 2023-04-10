use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::map,
    sequence::{delimited, preceded},
    IResult,
};

/// Quote (string) token.
#[derive(Debug, PartialEq, Clone)]
pub struct Quote(String);

pub(crate) fn quote(input: &str) -> IResult<&str, Quote> {
    map(alt((tri_quote, mono_quote)), Quote)(input)
}

fn mono_quote(input: &str) -> IResult<&str, String> {
    preceded(tag("\""), unescape)(input)
}

fn unescape(input: &str) -> IResult<&str, String> {
    let mut output = String::new();
    let chars = &mut input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('a') => output.push('\x07'),
                Some('b') => output.push('\x08'),
                Some('e') => output.push('\x1b'),
                Some('f') => output.push('\x0c'),
                Some('n') => output.push('\n'),
                Some('r') => output.push('\r'),
                Some('t') => output.push('\t'),
                Some('v') => output.push('\x0b'),
                Some('\\') => output.push('\\'),
                Some('\'') => output.push('\''),
                Some('"') => output.push('"'),
                Some('0') => output.push('\0'),
                Some('x') => {
                    let hex_str: String = chars.take(2).collect();
                    let byte = u8::from_str_radix(&hex_str, 16).unwrap();
                    output.push(byte as char);
                }
                Some('u') => {
                    let hex_str: String = chars.take(4).collect();
                    let code_point = u32::from_str_radix(&hex_str, 16).unwrap();
                    output.push(std::char::from_u32(code_point).unwrap());
                }
                Some('U') => {
                    let hex_str: String = chars.take(8).collect();
                    let code_point = u32::from_str_radix(&hex_str, 16).unwrap();
                    output.push(std::char::from_u32(code_point).unwrap());
                }
                Some(ch) => output.push(ch),
                None => {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Fail,
                    )))
                }
            }
        } else if ch == '"' {
            return Ok((chars.as_str(), output));
        } else if ch == '\n' {
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Fail,
            )));
        } else {
            output.push(ch);
        }
    }

    Err(nom::Err::Incomplete(nom::Needed::Unknown))
}

fn tri_quote(input: &str) -> IResult<&str, String> {
    delimited(tag("\"\"\""), take_until("\"\"\""), tag("\"\"\""))(input)
        .map(|(i, o)| (i, o.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mono_quote() {
        assert_eq!(mono_quote(r#""test""#), Ok(("", "test".to_string())));
        assert_eq!(mono_quote(r#""\n""#), Ok(("", "\n".to_string())));
        assert_eq!(
            mono_quote(r#""hello, \"world\"""#),
            Ok(("", "hello, \"world\"".to_string()))
        );
        assert_eq!(mono_quote(r#""""#), Ok(("", "".to_string())));
        assert_eq!(mono_quote(r#""test"\n"#), Ok(("\\n", "test".to_string())));
    }

    #[test]
    fn test_parse_tri_quote() {
        assert_eq!(tri_quote(r#""""""""#), Ok(("", String::new())));
        assert_eq!(
            tri_quote(r#""""Hello, world!""""#),
            Ok(("", "Hello, world!".to_string()))
        );
        assert_eq!(
            tri_quote(
                r#""""This is a "test" test,
    hello!""""#
            ),
            Ok(("", "This is a \"test\" test,\n    hello!".to_string()))
        );
    }

    #[test]
    fn test_parse_quote() {
        assert_eq!(quote(r#""test""#), Ok(("", Quote("test".to_string()))));
        assert_eq!(
            quote(r#""hello, \"world\"""#),
            Ok(("", Quote("hello, \"world\"".to_string())))
        );
        assert_eq!(quote(r#""""""""#), Ok(("", Quote(String::new()))));
        assert_eq!(
            quote(r#""""Hello, world!""""#),
            Ok(("", Quote("Hello, world!".to_string())))
        );
    }
}
