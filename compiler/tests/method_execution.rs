use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_simple_method() {
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
fn test_string_method() {
    let mut runtime = Runtime::new();
    
    // Create a method that returns a string
    let method_source = "return \"hello world\"";
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
    assert_eq!(result, Value::String("hello world".to_string()));
}

#[test]
fn test_if_statement() {
    let mut runtime = Runtime::new();
    
    // Create a method with an if statement
    let method_source = "if(1 < 2, return \"true\", return \"false\")";
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
    assert_eq!(result, Value::String("true".to_string()));
}