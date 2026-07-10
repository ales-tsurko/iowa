//! Number sign integration tests.

use gg_parser::{Message, Number, Symbol, parse};

fn messages(input: &str) -> Vec<Message<'_>> {
    let (_, chains) = parse(input).expect("number expression should parse");
    assert_eq!(chains.len(), 1);
    chains.into_iter().next().expect("one chain").messages
}

#[test]
fn leading_signs_are_operator_messages() {
    let cases = [
        ("-2.5", "-", Number::Float(2.5)),
        ("+2.5", "+", Number::Float(2.5)),
        ("-42", "-", Number::Int(42)),
        ("+0b10", "+", Number::Binary(2)),
        ("-0xFF", "-", Number::Hex(255)),
    ];

    for (input, operator, number) in cases {
        let parsed = messages(input);
        let mut parsed = parsed.iter();

        assert!(
            matches!(
                parsed.next().map(|message| &message.symbol),
                Some(Symbol::Operator(value)) if value.name() == operator
            ),
            "{input}"
        );
        assert_eq!(
            parsed.next().map(|message| &message.symbol),
            Some(&Symbol::Number(number)),
            "{input}"
        );
        assert_eq!(parsed.next(), None, "{input}");
    }
}

#[test]
fn exponent_signs_remain_in_float_literals() {
    let parsed = messages("1e-3");

    assert_eq!(
        parsed.first().map(|message| &message.symbol),
        Some(&Symbol::Number(Number::Float(0.001)))
    );
    assert_eq!(parsed.len(), 1);
}
