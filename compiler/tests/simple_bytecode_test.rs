use iowa_compiler::runtime::runtime::{Runtime, Value};
use iowa_compiler::runtime::bytecode::{Bytecode, Instruction, Opcode};

#[test]
fn test_simple_bytecode_execution() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a simple method that returns a number (manually creating bytecode)
    let mut bytecode = Bytecode::new();
    
    // Add constant 42.0 to the constant pool
    bytecode.constant_pool.push(Value::Number(42.0));
    
    // Add instructions: load constant 0, return
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // Serialize the bytecode
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec![], bytecode_data).unwrap();
    
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