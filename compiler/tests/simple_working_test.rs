use iowa_compiler::runtime::runtime::{Runtime, Value};

#[test]
fn test_super_simple() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a simple method that returns a constant
    let method_source = "return 42";
    let method_id = runtime.create_method(vec![], method_source).unwrap();
    
    // Assign method to Lobby
    let lobby = runtime.lobby();
    
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("test".to_string(), Value::Object(method_id));
    }
    
    // Set up call frame
    runtime.push_call_frame(method_id, lobby, "test".to_string(), vec![]);
    
    // Call method
    let result = runtime.dispatch_message(Value::Object(lobby), "test", vec![]);
    
    // Check result
    assert_eq!(result, Value::Number(42.0));
}