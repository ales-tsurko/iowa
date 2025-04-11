use iowa_compiler::runtime::runtime::{Runtime, Value};
use iowa_compiler::runtime::bytecode::{Bytecode, Instruction, Opcode};

#[test]
fn test_manual_bytecode() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Manually create bytecode that loads a constant and returns it
    let mut bytecode = Bytecode::new();
    
    // Add 42.0 to the constant pool at index 0
    bytecode.constant_pool.push(Value::Number(42.0));
    
    // Instructions: load the constant and then return
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // Serialize the bytecode to bytes
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec![], bytecode_data).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("test".to_string(), Value::Object(method_id));
    }
    
    // Set up call frame for test
    runtime.push_call_frame(method_id, lobby, "test".to_string(), vec![]);
    
    // Call the method
    let result = runtime.dispatch_message(Value::Object(lobby), "test", vec![]);
    
    // Check the result
    assert_eq!(result, Value::Number(42.0));
}