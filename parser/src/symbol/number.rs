use nom::{
    Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, hex_digit1, one_of},
    combinator::{map_res, opt, recognize},
    multi::many1,
    sequence::{pair, preceded},
};

use crate::{Input, ParseResult};

/// The Number type.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Number {
    /// Binary number.
    Binary(u64),
    /// Hexadecimal number.
    Hex(u64),
    /// Integer number.
    Int(u64),
    /// Floating-point number.
    Float(f64),
}

impl From<f64> for Number {
    fn from(num: f64) -> Self {
        Self::Float(num)
    }
}

impl From<u64> for Number {
    fn from(num: u64) -> Self {
        Self::Int(num)
    }
}

pub(crate) fn number(input: Input<'_>) -> ParseResult<'_, Number> {
    alt((binary_number, hex_number, float_number, int_number)).parse(input)
}

// Define a parser for binary digits
fn binary_number(input: Input<'_>) -> ParseResult<'_, Number> {
    let binary_tag = alt((tag("0b"), tag("0B")));
    let binary_digits = recognize(many1(one_of("01")));
    let preceded_parser = preceded(binary_tag, binary_digits);

    map_res(preceded_parser, |out: Input<'_>| {
        u64::from_str_radix(out.fragment(), 2).map(Number::Binary)
    })
    .parse(input)
}

// Define a parser for hexadecimal digits
fn hex_number(input: Input<'_>) -> ParseResult<'_, Number> {
    let hex_tag = alt((tag("0x"), tag("0X")));
    let hex_digits = recognize(many1(hex_digit1));
    let preceded_parser = preceded(hex_tag, hex_digits);

    map_res(preceded_parser, |out: Input<'_>| {
        u64::from_str_radix(out.fragment(), 16).map(Number::Hex)
    })
    .parse(input)
}

fn int_number(input: Input<'_>) -> ParseResult<'_, Number> {
    map_res(digit1, |out: Input<'_>| {
        out.fragment().parse::<u64>().map(Number::Int)
    })
    .parse(input)
}

// Define a parser for floating-point decimal numbers
fn float_number(input: Input<'_>) -> ParseResult<'_, Number> {
    // .42
    let decimal_point_digits = recognize((
        char('.'),
        digit1,
        opt((one_of("eE"), opt(one_of("+-")), digit1)),
    ));

    // 42e42 and 42.42e42
    let sci_notation = recognize((
        digit1,
        opt(preceded(char('.'), digit1)),
        one_of("eE"),
        opt(one_of("+-")),
        digit1,
    ));

    // 42. and 42.42
    let digits_decimal_point = recognize((digit1, char('.'), opt(digit1)));

    let number_formats = alt((decimal_point_digits, sci_notation, digits_decimal_point));

    let sign_and_number = recognize(pair(opt(one_of("+-")), number_formats));

    map_res(sign_and_number, |out: Input<'_>| {
        out.fragment().parse::<f64>().map(Number::Float)
    })
    .parse(input)
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
    fn test_parse_binary_number() {
        assert_eq!(
            parsed(binary_number(Input::new("0b1010"))),
            Ok(("", Number::Binary(0b1010)))
        );
        assert_eq!(
            parsed(binary_number(Input::new("0B1111"))),
            Ok(("", Number::Binary(0b1111)))
        );
    }

    #[test]
    fn test_parse_hex_number() {
        assert_eq!(
            parsed(hex_number(Input::new("0x1234"))),
            Ok(("", Number::Hex(0x1234)))
        );
        assert_eq!(
            parsed(hex_number(Input::new("0Xabcd"))),
            Ok(("", Number::Hex(0xABCD)))
        );
        assert_eq!(
            parsed(hex_number(Input::new("0x1a2b3c4d"))),
            Ok(("", Number::Hex(0x1A2B3C4D)))
        );
    }

    #[test]
    fn test_parse_int_number() {
        assert_eq!(
            parsed(int_number(Input::new("42"))),
            Ok(("", Number::Int(42)))
        );
    }

    #[test]
    fn test_parse_float_number() {
        assert_eq!(
            parsed(float_number(Input::new("3.125"))),
            Ok(("", Number::Float(3.125)))
        );
        assert_eq!(
            parsed(float_number(Input::new("123.456e+10"))),
            Ok(("", Number::Float(1234560000000.0)))
        );
        assert_eq!(
            parsed(float_number(Input::new("0.5e-3"))),
            Ok(("", Number::Float(0.0005)))
        );
        assert_eq!(
            parsed(float_number(Input::new("-2.5e-3"))),
            Ok(("", Number::Float(-0.0025)))
        );
    }
}
