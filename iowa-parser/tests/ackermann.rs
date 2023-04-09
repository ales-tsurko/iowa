use iowa_parser::parse;

#[test]
fn test_parser() {
    let input = r#"

    ack := method(m, n,
      //writeln("ack(", m, ",", n, ")")
      if (m < 1, return n + 1)
      if (n < 1, return ack(m - 1, 1))
      return ack(m - 1, ack(m, n - 1))
    )

    ack(3, 4) print
    #"\n" print
    "#;

    let result = parse(input);

    dbg!(result);
}