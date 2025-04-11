use iowa_compiler::runtime::runtime::{Runtime, Value};
use iowa_compiler::runtime::bytecode::{Bytecode, Instruction, Opcode};

#[test]
fn test_bytecode_addition() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that adds two numbers (load constants, add, return)
    let mut bytecode = Bytecode::new();
    
    // Add constants to the constant pool
    bytecode.constant_pool.push(Value::Number(3.0));  // Constant 0
    bytecode.constant_pool.push(Value::Number(4.0));  // Constant 1
    bytecode.constant_pool.push(Value::String("+".to_string())); // Constant 2
    
    // Add instructions:
    // 1. Load constant 0 (3.0)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
    
    // 2. Load constant 1 (4.0)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 1));
    
    // 3. Send message "+" with 1 argument
    bytecode.instructions.push(Instruction::new(Opcode::SendMessage, 2, 1));
    
    // 4. Return
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // Serialize the bytecode
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec![], bytecode_data).unwrap();
    
    // Create a receiver object
    let lobby = runtime.lobby();
    
    // Add the method to the lobby
    if let Some(obj) = runtime.memory_mut().get_object_mut(lobby) {
        obj.set_slot("add".to_string(), Value::Object(method_id));
    }
    
    // Call the method
    runtime.push_call_frame(method_id, lobby, "add".to_string(), vec![]);
    let result = runtime.dispatch_message(Value::Object(lobby), "add", vec![]);
    
    // Check result
    assert_eq!(result, Value::Number(7.0));
}

#[test]
fn test_bytecode_with_local_variables() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that uses local variables
    let mut bytecode = Bytecode::new();
    
    // Add constants to the constant pool
    bytecode.constant_pool.push(Value::Number(10.0));  // Constant 0
    bytecode.constant_pool.push(Value::Number(2.0));   // Constant 1
    bytecode.constant_pool.push(Value::String("*".to_string())); // Constant 2
    
    // Set local count
    bytecode.local_count = 2;
    
    // Add instructions:
    // 1. Load constant 0 (10.0)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
    
    // 2. Store in local variable 0
    bytecode.instructions.push(Instruction::with_operand1(Opcode::StoreLocal, 0));
    
    // 3. Load constant 1 (2.0)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 1));
    
    // 4. Store in local variable 1
    bytecode.instructions.push(Instruction::with_operand1(Opcode::StoreLocal, 1));
    
    // 5. Load local variable 0
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadLocal, 0));
    
    // 6. Load local variable 1
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadLocal, 1));
    
    // 7. Send message "*" with 1 argument
    bytecode.instructions.push(Instruction::new(Opcode::SendMessage, 2, 1));
    
    // 8. Return
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // Serialize the bytecode
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec![], bytecode_data).unwrap();
    
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

#[test]
fn test_bytecode_with_args() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method that adds its two arguments
    let mut bytecode = Bytecode::new();
    
    // Add constants to the constant pool
    bytecode.constant_pool.push(Value::String("+".to_string())); // Constant 0
    
    // Set local count to match arg count
    bytecode.local_count = 2;
    
    // Add instructions:
    // 1. Load local variable 0 (arg1)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadLocal, 0));
    
    // 2. Load local variable 1 (arg2)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadLocal, 1));
    
    // 3. Send message "+" with 1 argument
    bytecode.instructions.push(Instruction::new(Opcode::SendMessage, 0, 1));
    
    // 4. Return
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // Serialize the bytecode
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec!["arg1".to_string(), "arg2".to_string()], bytecode_data).unwrap();
    
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
fn test_bytecode_with_if_statement() {
    // Create a new runtime
    let mut runtime = Runtime::new();
    
    // Create a method with conditional logic
    let mut bytecode = Bytecode::new();
    
    // Add constants to the constant pool
    bytecode.constant_pool.push(Value::Number(5.0));  // Constant 0
    bytecode.constant_pool.push(Value::String(">".to_string())); // Constant 1
    bytecode.constant_pool.push(Value::String("big".to_string())); // Constant 2
    bytecode.constant_pool.push(Value::String("small".to_string())); // Constant 3
    
    // Set local count
    bytecode.local_count = 1;
    
    // Add instructions:
    // 1. Load local variable 0 (x)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadLocal, 0));
    
    // 2. Load constant 0 (5.0)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 0));
    
    // 3. Send message ">" with 1 argument
    bytecode.instructions.push(Instruction::new(Opcode::SendMessage, 1, 1));
    
    // 4. Jump if false to small branch
    bytecode.instructions.push(Instruction::with_operand1(Opcode::JumpIfFalse, 4));
    
    // 5. Load "big" constant
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 2));
    
    // 6. Return
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // 7. Jump to end
    bytecode.instructions.push(Instruction::with_operand1(Opcode::Jump, 2));
    
    // 8. Push nil as placeholder
    bytecode.instructions.push(Instruction::simple(Opcode::PushNil));
    
    // 9. Load "small" constant (replacing nil)
    bytecode.instructions.push(Instruction::with_operand1(Opcode::LoadConst, 3));
    
    // 10. Return the "small" constant
    bytecode.instructions.push(Instruction::simple(Opcode::Return));
    
    // 11. End (not reachable)
    bytecode.instructions.push(Instruction::simple(Opcode::End));
    
    // Serialize the bytecode
    let bytecode_data = bytecode.serialize();
    
    // Create a method with this bytecode
    let method_id = runtime.create_method_with_bytecode(vec!["x".to_string()], bytecode_data).unwrap();
    
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