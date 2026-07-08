//! Integration tests for the Io parser

use gg_parser::{MessageChain, Symbol, parse};

// Helper function to unwrap parse result and get just the AST
fn parse_and_get_ast(input: &str) -> Vec<MessageChain<'_>> {
    let (_, ast) = parse(input).expect("Failed to parse input");
    ast
}

// Helper for accessing symbol values
fn symbol_value<'a>(symbol: &'a Symbol<'_>) -> &'a str {
    match symbol {
        Symbol::Identifier(id) => id.name(),
        Symbol::Operator(op) => op.name(),
        Symbol::Quote(q) => q.content(),
        Symbol::Number(_) => "<number>",
    }
}

// Helper to find identifier in AST
fn contains_identifier(ast: &[MessageChain<'_>], name: &str) -> bool {
    ast.iter().any(|chain| {
        chain.messages.iter().any(|msg| {
            if let Symbol::Identifier(_) = &msg.symbol {
                symbol_value(&msg.symbol) == name
            } else {
                false
            }
        })
    })
}

// Helper to find operator in AST
fn contains_operator(ast: &[MessageChain<'_>], op_name: &str) -> bool {
    ast.iter().any(|chain| {
        chain.messages.iter().any(|msg| {
            if let Symbol::Operator(_) = &msg.symbol {
                symbol_value(&msg.symbol) == op_name
            } else {
                false
            }
        })
    })
}

#[test]
fn test_ackermann() {
    let input = r#"
    #!/usr/bin/env io

    ack := method(m, n,
      //writeln("ack(", m, ",", n, ")")
      if (m < 1, return n + 1)
      if (n < 1, return ack(m - 1, 1))
      return ack(m - 1, ack(m, n - 1))
    )

    ack(3, 4) print
    "\n" print
    "#;

    parse(input).expect("Ackermann example should parse");
}

#[test]
fn test_variable() {
    let input = "ack := foo";

    // Validate AST structure
    let ast = parse_and_get_ast(input);

    // Check that we have one message chain (assignment)
    assert_eq!(ast.len(), 1, "Should have exactly one message chain");

    // Check for specific tokens
    assert!(
        contains_identifier(&ast, "ack"),
        "Should have 'ack' identifier"
    );
    assert!(contains_operator(&ast, ":="), "Should have ':=' operator");
}

#[test]
fn test_data_types() {
    let input = "number := 42; string := \"Hello, world!\"";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 2);

    // Check for data types
    assert!(
        contains_identifier(&ast, "number"),
        "AST should contain 'number'"
    );
    assert!(
        contains_identifier(&ast, "string"),
        "AST should contain 'string'"
    );
    assert!(
        contains_operator(&ast, ":="),
        "AST should contain ':=' operator"
    );
}

#[test]
fn test_object_system() {
    let input = "Person := foo";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 1);

    // Check object concepts
    assert!(
        contains_identifier(&ast, "Person"),
        "AST should contain 'Person'"
    );
}

#[test]
fn test_control_structures() {
    let input = "if(x, y, z); for(i, 1, 10, println)";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 2);

    // Check for control flow keywords
    assert!(contains_identifier(&ast, "if"), "AST should contain 'if'");
    assert!(contains_identifier(&ast, "for"), "AST should contain 'for'");
}

#[test]
fn test_exception_handling() {
    let input = "Exception raise(\"Error\"); try(foo, e println)";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 2);

    // Check for exception handling keywords
    assert!(
        contains_identifier(&ast, "Exception"),
        "AST should contain 'Exception'"
    );
    assert!(
        contains_identifier(&ast, "raise"),
        "AST should contain 'raise'"
    );
    assert!(contains_identifier(&ast, "try"), "AST should contain 'try'");
}

#[test]
fn test_advanced_features() {
    let input = "getSlot; Vector + := foo";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 2);

    // Verify features
    assert!(
        contains_identifier(&ast, "getSlot"),
        "AST should contain 'getSlot'"
    );
    assert!(
        contains_identifier(&ast, "Vector"),
        "AST should contain 'Vector'"
    );

    // Verify operators
    assert!(
        contains_operator(&ast, "+"),
        "AST should contain '+' operator"
    );
    assert!(
        contains_operator(&ast, ":="),
        "AST should contain ':=' operator"
    );
}

#[test]
fn test_edge_cases() {
    // Simpler test than originally planned - we'll add more comprehensive tests later
    let input = "a";

    let ast = parse_and_get_ast(input);

    assert_eq!(ast.len(), 1);

    // Just check for an identifier
    assert!(contains_identifier(&ast, "a"), "AST should contain 'a'");
}

#[test]
fn test_ast_validation() {
    // Simple message chain
    let input = "foo(1) bar";
    let ast = parse_and_get_ast(input);

    // Check for identifiers
    assert!(contains_identifier(&ast, "foo"), "AST should contain 'foo'");
    assert!(contains_identifier(&ast, "bar"), "AST should contain 'bar'");

    // Check we have one message chain
    assert_eq!(ast.len(), 1, "Should have exactly one message chain");

    // Check foo with argument
    let chain = ast.first().expect("AST should contain one chain");
    assert_eq!(chain.messages.len(), 2);

    // First message should be foo with one argument
    assert_eq!(
        chain
            .messages
            .first()
            .expect("chain should contain foo")
            .args
            .len(),
        1,
        "foo should have one argument"
    );

    // Second message should be bar with no arguments
    assert_eq!(
        chain
            .messages
            .get(1)
            .expect("chain should contain bar")
            .args
            .len(),
        0,
        "bar should have no arguments"
    );
}
