//! Number parser.

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, one_of, hex_digit1, digit1},
    combinator::{map_res, opt, recognize},
    multi::many1,
    sequence::{pair, preceded, tuple},
    IResult,
};

/// The Number type.
#[derive(Debug, PartialEq)]
pub enum Number {
    /// Hexadecimal number.
    Hex(u64),
    /// Decimal number.
    Decimal(f64),
}

/// Parse number.
pub fn number(input: &str) -> IResult<&str, Number> {
    alt((hex_number, decimal_number))(input)
}

// Define a parser for hexadecimal digits
fn hex_number(input: &str) -> IResult<&str, Number> {
    map_res(
        preceded(alt((tag("0x"), tag("0X"))), recognize(many1(hex_digit1))),
        |out: &str| u64::from_str_radix(out, 16).map(Number::Hex),
    )(input)
}

// Define a parser for decimal numbers
fn decimal_number(input: &str) -> IResult<&str, Number> {
    let num = recognize(pair(
        opt(one_of("+-")),
        alt((
            // .42
            recognize(tuple((
                char('.'),
                digit1,
                opt(tuple((one_of("eE"), opt(one_of("+-")), digit1))),
            ))), // 42e42 and 42.42e42
            recognize(tuple((
                digit1,
                opt(preceded(char('.'), digit1)),
                one_of("eE"),
                opt(one_of("+-")),
                digit1,
            ))), // 42. and 42.42
            recognize(tuple((digit1, char('.'), opt(digit1)))),
            // 42
            recognize(digit1),
        )),
    ));
    map_res(num, |out: &str| out.parse::<f64>().map(Number::Decimal))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_number() {
        assert_eq!(hex_number("0x1234"), Ok(("", Number::Hex(0x1234))));
        assert_eq!(hex_number("0Xabcd"), Ok(("", Number::Hex(0xABCD))));
        assert_eq!(hex_number("0x1a2b3c4d"), Ok(("", Number::Hex(0x1A2B3C4D))));
    }

    #[test]
    fn test_parse_decimal_number() {
        assert_eq!(decimal_number("42"), Ok(("", Number::Decimal(42.0))));
        assert_eq!(decimal_number("3.1415"), Ok(("", Number::Decimal(3.1415))));
        assert_eq!(
            decimal_number("123.456e+10"),
            Ok(("", Number::Decimal(1234560000000.0)))
        );
        assert_eq!(decimal_number("0.5e-3"), Ok(("", Number::Decimal(0.0005))));
        assert_eq!(
            decimal_number("-2.5e-3"),
            Ok(("", Number::Decimal(-0.0025)))
        );
    }
}
