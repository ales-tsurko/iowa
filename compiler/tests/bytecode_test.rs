use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_method_execution_with_bytecode() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a simple method that returns a number
    let method_source = "return 42";
    let method_id = runtime.create_method(vec![], method_source).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("test".to_string(), Value::Object(method_id));
    }
    
    // Call the method
    runtime.push_call_frame(method_id, lobby, "test".to_string(), vec![]);
    let result = runtime.dispatch_message(Value::Object(lobby), "test", vec![]);
    
    // Check result
    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_method_with_args() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that returns the sum of its arguments
    let method_source = "a := arg1; b := arg2; return a + b";
    let method_id = runtime.create_method(vec!["arg1".to_string(), "arg2".to_string()], method_source).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("add".to_string(), Value::Object(method_id));
    }
    
    // Call the method with arguments
    let args = vec![Value::Number(3.0), Value::Number(4.0)];
    runtime.push_call_frame(method_id, lobby, "add".to_string(), args.clone());
    let result = runtime.dispatch_message(Value::Object(lobby), "add", args);
    
    // Check result
    assert_eq!(result, Value::Number(7.0));
}

#[test]
fn test_method_with_if_statement() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method with an if statement
    let method_source = r#"
        a := x;
        if(a > 5,
            return "big",
            return "small"
        )
    "#;
    let method_id = runtime.create_method(vec!["x".to_string()], method_source).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("checkSize".to_string(), Value::Object(method_id));
    }
    
    // Call the method with a big number
    let args_big = vec![Value::Number(10.0)];
    runtime.push_call_frame(method_id, lobby, "checkSize".to_string(), args_big.clone());
    let result_big = runtime.dispatch_message(Value::Object(lobby), "checkSize", args_big);
    
    // Check result
    assert_eq!(result_big, Value::String("big".to_string()));
    
    // Call the method with a small number
    let args_small = vec![Value::Number(3.0)];
    runtime.push_call_frame(method_id, lobby, "checkSize".to_string(), args_small.clone());
    let result_small = runtime.dispatch_message(Value::Object(lobby), "checkSize", args_small);
    
    // Check result
    assert_eq!(result_small, Value::String("small".to_string()));
}

#[test]
fn test_method_with_assignment() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that assigns and uses a local variable
    let method_source = "a := 10; b := 2; return a * b";
    let method_id = runtime.create_method(vec![], method_source).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("compute".to_string(), Value::Object(method_id));
    }
    
    // Call the method
    runtime.push_call_frame(method_id, lobby, "compute".to_string(), vec![]);
    let result = runtime.dispatch_message(Value::Object(lobby), "compute", vec![]);
    
    // Check result
    assert_eq!(result, Value::Number(20.0));
}