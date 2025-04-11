use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_simple_method_execution() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that returns a number using source code
    let source = "return 42";
    let method_id = runtime.create_method(vec![], source).unwrap();
    
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