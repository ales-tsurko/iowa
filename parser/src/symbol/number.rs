use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, hex_digit1, one_of},
    combinator::{map_res, opt, recognize},
    multi::many1,
    sequence::{pair, preceded},
};

/// The Number type.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Number {
    /// Hexadecimal number.
    Hex(u64),
    /// Decimal number.
    Decimal(f64),
}

impl From<f64> for Number {
    fn from(num: f64) -> Self {
        Self::Decimal(num)
    }
}

impl From<u64> for Number {
    fn from(num: u64) -> Self {
        Self::Hex(num)
    }
}

pub(crate) fn number(input: &str) -> IResult<&str, Number> {
    alt((hex_number, decimal_number)).parse(input)
}

// Define a parser for hexadecimal digits
fn hex_number(input: &str) -> IResult<&str, Number> {
    let hex_tag = alt((tag("0x"), tag("0X")));
    let hex_digits = recognize(many1(hex_digit1));
    let preceded_parser = preceded(hex_tag, hex_digits);

    map_res(preceded_parser, |out: &str| {
        u64::from_str_radix(out, 16).map(Number::Hex)
    })
    .parse(input)
}

// Define a parser for decimal numbers
fn decimal_number(input: &str) -> IResult<&str, Number> {
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

    // 42
    let just_digits = recognize(digit1);

    let number_formats = alt((
        decimal_point_digits,
        sci_notation,
        digits_decimal_point,
        just_digits,
    ));

    let sign_and_number = recognize(pair(opt(one_of("+-")), number_formats));

    map_res(sign_and_number, |out: &str| {
        out.parse::<f64>().map(Number::Decimal)
    })
    .parse(input)
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
        assert_eq!(decimal_number("3.125"), Ok(("", Number::Decimal(3.125))));
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
