use nom::{
    Input as _, Parser,
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::map,
    sequence::{delimited, preceded},
};

use crate::{Input, ParseResult, ParserError, ParserErrorKind};

/// Quote (string) token.
#[derive(Debug, PartialEq, Clone)]
pub struct Quote(String);

impl Quote {
    /// Get the content of the quote
    pub fn content(&self) -> &str {
        &self.0
    }
}

pub(crate) fn quote(input: Input<'_>) -> ParseResult<'_, Quote> {
    let quote_parser = alt((tri_quote, mono_quote));
    map(quote_parser, Quote).parse(input)
}

fn mono_quote(input: Input<'_>) -> ParseResult<'_, String> {
    preceded(tag("\""), unescape).parse(input)
}

fn unescape(input: Input<'_>) -> ParseResult<'_, String> {
    let mut output = String::new();
    let input_text = *input.fragment();
    let chars = &mut input_text.chars();

    while let Some(ch) = chars.next() {
        let consumed = input_text.len() - chars.as_str().len() - ch.len_utf8();
        let current_input = input.take_from(consumed);

        if ch == '\\' {
            let escape_input = input.take_from(input_text.len() - chars.as_str().len());

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
                    if hex_str.len() != 2 {
                        return Err(ParserError::failure(
                            escape_input,
                            ParserErrorKind::InvalidHexEscape,
                        ));
                    }
                    let byte = u8::from_str_radix(&hex_str, 16).map_err(|_parse_error| {
                        ParserError::failure(escape_input, ParserErrorKind::InvalidHexEscape)
                    })?;
                    output.push(byte as char);
                }
                Some('u') => {
                    let hex_str: String = chars.take(4).collect();
                    if hex_str.len() != 4 {
                        return Err(ParserError::failure(
                            escape_input,
                            ParserErrorKind::InvalidUnicodeEscape,
                        ));
                    }
                    let code_point = u32::from_str_radix(&hex_str, 16).map_err(|_parse_error| {
                        ParserError::failure(escape_input, ParserErrorKind::InvalidUnicodeEscape)
                    })?;
                    let ch = std::char::from_u32(code_point).ok_or_else(|| {
                        ParserError::failure(escape_input, ParserErrorKind::InvalidUnicodeEscape)
                    })?;
                    output.push(ch);
                }
                Some('U') => {
                    let hex_str: String = chars.take(8).collect();
                    if hex_str.len() != 8 {
                        return Err(ParserError::failure(
                            escape_input,
                            ParserErrorKind::InvalidUnicodeEscape,
                        ));
                    }
                    let code_point = u32::from_str_radix(&hex_str, 16).map_err(|_parse_error| {
                        ParserError::failure(escape_input, ParserErrorKind::InvalidUnicodeEscape)
                    })?;
                    let ch = std::char::from_u32(code_point).ok_or_else(|| {
                        ParserError::failure(escape_input, ParserErrorKind::InvalidUnicodeEscape)
                    })?;
                    output.push(ch);
                }
                Some(ch) => output.push(ch),
                None => {
                    return Err(ParserError::failure(
                        escape_input,
                        ParserErrorKind::MissingEscape,
                    ));
                }
            }
        } else if ch == '"' {
            let rest = input.take_from(input_text.len() - chars.as_str().len());
            return Ok((rest, output));
        } else if ch == '\n' {
            return Err(ParserError::failure(
                current_input,
                ParserErrorKind::NewlineInString,
            ));
        } else {
            output.push(ch);
        }
    }

    Err(ParserError::failure(
        input,
        ParserErrorKind::UnterminatedString,
    ))
}

fn tri_quote(input: Input<'_>) -> ParseResult<'_, String> {
    delimited(tag("\"\"\""), take_until("\"\"\""), tag("\"\"\""))
        .parse(input)
        .map(|(i, o)| (i, o.fragment().to_string()))
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
    fn test_parse_mono_quote() {
        assert_eq!(
            parsed(mono_quote(Input::new(r#""test""#))),
            Ok(("", "test".to_string()))
        );
        assert_eq!(
            parsed(mono_quote(Input::new(r#""\n""#))),
            Ok(("", "\n".to_string()))
        );
        assert_eq!(
            parsed(mono_quote(Input::new(r#""hello, \"world\"""#))),
            Ok(("", "hello, \"world\"".to_string()))
        );
        assert_eq!(
            parsed(mono_quote(Input::new(r#""""#))),
            Ok(("", "".to_string()))
        );
        assert_eq!(
            parsed(mono_quote(Input::new(r#""test"\n"#))),
            Ok(("\\n", "test".to_string()))
        );
    }

    #[test]
    fn test_reject_invalid_hex_escape() {
        assert!(matches!(
            mono_quote(Input::new(r#""\xZZ""#)),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::InvalidHexEscape
                    && error.line() == 1
                    && error.column() == 3
        ));
    }

    #[test]
    fn test_reject_invalid_unicode_escape() {
        assert!(matches!(
            mono_quote(Input::new(r#""\uD800""#)),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::InvalidUnicodeEscape
                    && error.line() == 1
                    && error.column() == 3
        ));
        assert!(matches!(
            mono_quote(Input::new(r#""\U00110000""#)),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::InvalidUnicodeEscape
                    && error.line() == 1
                    && error.column() == 3
        ));
    }

    #[test]
    fn test_reject_invalid_string_boundaries() {
        assert!(matches!(
            mono_quote(Input::new("\"\\")),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::MissingEscape
                    && error.line() == 1
                    && error.column() == 3
        ));
        assert!(matches!(
            mono_quote(Input::new("\"\n\"")),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::NewlineInString
                    && error.line() == 1
                    && error.column() == 2
        ));
        assert!(matches!(
            mono_quote(Input::new("\"unterminated")),
            Err(nom::Err::Failure(error))
                if error.kind == ParserErrorKind::UnterminatedString
                    && error.line() == 1
                    && error.column() == 2
        ));
    }

    #[test]
    fn test_parse_tri_quote() {
        assert_eq!(
            parsed(tri_quote(Input::new(r#""""""""#))),
            Ok(("", String::new()))
        );
        assert_eq!(
            parsed(tri_quote(Input::new(r#""""Hello, world!""""#))),
            Ok(("", "Hello, world!".to_string()))
        );
        assert_eq!(
            parsed(tri_quote(Input::new(
                r#""""This is a "test" test,
    hello!""""#
            ))),
            Ok(("", "This is a \"test\" test,\n    hello!".to_string()))
        );
    }

    #[test]
    fn test_parse_quote() {
        assert_eq!(
            parsed(quote(Input::new(r#""test""#))),
            Ok(("", Quote("test".to_string())))
        );
        assert_eq!(
            parsed(quote(Input::new(r#""hello, \"world\"""#))),
            Ok(("", Quote("hello, \"world\"".to_string())))
        );
        assert_eq!(
            parsed(quote(Input::new(r#""""""""#))),
            Ok(("", Quote(String::new())))
        );
        assert_eq!(
            parsed(quote(Input::new(r#""""Hello, world!""""#))),
            Ok(("", Quote("Hello, world!".to_string())))
        );
    }
}
