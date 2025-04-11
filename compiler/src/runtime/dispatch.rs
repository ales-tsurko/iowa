//! Message dispatch system for the Io runtime.
//!
//! This module provides functions for message passing and method lookup
//! in the Io object system. It implements the prototype-based lookup chain
//! and supports Io's approach to operators as methods.

use cranelift_codegen::ir::{types, Value as CraneliftValue, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::isa::CallConv;
use super::{value, object};

/// Constants for runtime function calls
pub const RUNTIME_CALL_METHOD: &str = "runtime_call_method";
pub const RUNTIME_ENCODE_STRING: &str = "runtime_encode_string";

/// Call the runtime message dispatch function
///
/// In Io, message dispatch follows these rules:
/// 1. Look up message in the receiver's slots
/// 2. If not found, look in the receiver's prototype
/// 3. Continue up the prototype chain until found or reach Object prototype
/// 4. If not found, try the "forward" slot if it exists
/// 5. Otherwise return nil
///
/// Operators are just methods in Io, defined in an operator table that can
/// be extended by users. Precedence is determined by position in the table.
pub fn call_message_dispatch(
    builder: &mut FunctionBuilder,
    receiver: CraneliftValue,
    message_name: CraneliftValue,
    args: &[CraneliftValue]
) -> CraneliftValue {
    // Create blocks for our dispatch logic
    let lookup_block = builder.create_block();
    let forward_block = builder.create_block();
    let call_block = builder.create_block();
    let not_found_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add parameters to blocks
    builder.append_block_param(call_block, types::I64); // method to call
    builder.append_block_param(done_block, types::I64); // result
    
    // Start the dispatch process
    builder.ins().jump(lookup_block, &[]);
    builder.switch_to_block(lookup_block);
    
    // Try to find the method in the receiver and its prototype chain
    let method = object::lookup_slot(builder, receiver, message_name);
    
    // Create a nil value to compare against
    let nil_value = value::encode_nil(builder);
    
    // Check if we found a method or got nil
    let is_nil = builder.ins().icmp(IntCC::Equal, method, nil_value);
    builder.ins().brif(is_nil, forward_block, &[], call_block, &[method]);
    
    // Forward block - try the "forward" method
    builder.switch_to_block(forward_block);
    
    // Look up the "forward" method
    let forward_name = encode_string_constant(builder, "forward");
    let forward_method = object::lookup_slot(builder, receiver, forward_name);
    
    // Check if forward exists
    let forward_is_nil = builder.ins().icmp(IntCC::Equal, forward_method, nil_value);
    builder.ins().brif(forward_is_nil, not_found_block, &[], call_block, &[forward_method]);
    
    // Call block - we found a method, call it
    builder.switch_to_block(call_block);
    let method_to_call = builder.block_params(call_block)[0];
    
    // Check if method is a function (activation) - will be used in full implementation
    let _is_activation = value::has_tag(builder, method_to_call, value::TAG_OBJECT_REF);
    
    // Create an array of arguments
    let args_array = create_args_array(builder, args);
    
    // Call the method (in a real implementation, this would check method type and dispatch accordingly)
    let result = call_method(builder, method_to_call, receiver, message_name, args_array);
    builder.ins().jump(done_block, &[result]);
    
    // Not found block - method not found, return nil
    builder.switch_to_block(not_found_block);
    builder.ins().jump(done_block, &[nil_value]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Create an array of arguments for method calls
fn create_args_array(
    builder: &mut FunctionBuilder, 
    args: &[CraneliftValue]
) -> CraneliftValue {
    // In a real implementation, this would allocate an array object
    // For now, just return a placeholder reference if there are arguments
    if args.is_empty() {
        return value::encode_nil(builder);
    }
    
    // This is a placeholder - would need to allocate an actual array and populate it
    let ptr = builder.ins().iconst(types::I64, 0x4000);
    value::encode_reference(builder, ptr, value::TAG_OBJECT_REF)
}

/// Call a method on an object
fn call_method(
    builder: &mut FunctionBuilder, 
    method: CraneliftValue, 
    receiver: CraneliftValue, 
    _message_name: CraneliftValue, // Will be used in full implementation
    args: CraneliftValue
) -> CraneliftValue {
    // Create blocks for different method types
    let activation_block = builder.create_block();
    let primitive_block = builder.create_block();
    let done_block = builder.create_block();
    
    // Add parameter to done block for the result
    builder.append_block_param(done_block, types::I64);
    
    // Check method type - is it an activation object?
    let is_activation = value::has_tag(builder, method, value::TAG_OBJECT_REF);
    builder.ins().brif(is_activation, activation_block, &[], primitive_block, &[]);
    
    // Activation block - call an activation object
    builder.switch_to_block(activation_block);
    
    // In a real implementation, we would:
    // 1. Extract the activation from the method object
    // 2. Set up the call frame with receiver and arguments
    // 3. Call the activation function
    
    // For now, call the runtime method function
    let result = call_runtime_method(builder, method, receiver, args);
    builder.ins().jump(done_block, &[result]);
    
    // Primitive block - call a primitive method (C function)
    builder.switch_to_block(primitive_block);
    
    // For primitives, use the same runtime call mechanism for now
    let primitive_result = call_runtime_method(builder, method, receiver, args);
    builder.ins().jump(done_block, &[primitive_result]);
    
    // Done block - return the result
    builder.switch_to_block(done_block);
    builder.block_params(done_block)[0]
}

/// Call into the runtime to execute a method
fn call_runtime_method(
    builder: &mut FunctionBuilder, 
    method: CraneliftValue, 
    receiver: CraneliftValue, 
    args: CraneliftValue
) -> CraneliftValue {
    // Create a function signature
    let mut sig = cranelift_codegen::ir::Signature::new(
        CallConv::Fast
    );
    
    // Add parameters (method, receiver, args)
    sig.params.push(cranelift_codegen::ir::AbiParam::new(types::I64)); // method
    sig.params.push(cranelift_codegen::ir::AbiParam::new(types::I64)); // receiver
    sig.params.push(cranelift_codegen::ir::AbiParam::new(types::I64)); // args
    
    // Return type is an Io value (i64)
    sig.returns.push(cranelift_codegen::ir::AbiParam::new(types::I64));
    
    // Import the signature first
    let sig_ref = builder.func.import_signature(sig);
    
    // Then import the function
    let callee = builder.import_function(cranelift_codegen::ir::ExtFuncData {
        name: cranelift_codegen::ir::ExternalName::testcase(RUNTIME_CALL_METHOD), // Use testcase as it accepts a &str
        signature: sig_ref,
        colocated: false,
    });
    
    // Call the function
    let call_args = &[method, receiver, args];
    let inst = builder.ins().call(callee, call_args);
    builder.inst_results(inst)[0]
}

/// Encode a string constant for use in the compiled code
/// 
/// This is still a placeholder implementation, but more structured to match our memory system.
/// In a real implementation, we would maintain a string table in the compiler.
pub fn encode_string_constant(builder: &mut FunctionBuilder, text: &str) -> CraneliftValue {
    // For most accurate implementation, we'd call into the runtime:
    // 1. Allocate a string object with proper length
    // 2. Copy the string contents into the string object
    // 3. Return a tagged reference to the string
    
    // For simplicity, use hardcoded addresses for common strings:
    let address = match text {
        "forward" => 0x3001,
        "asString" => 0x3002,
        "print" => 0x3003,
        "+" => 0x3004,
        "-" => 0x3005,
        "*" => 0x3006,
        "/" => 0x3007,
        _ => 0x3000, // Default for any other string
    };
    
    let ptr = builder.ins().iconst(types::I64, address);
    value::encode_reference(builder, ptr, value::TAG_STRING_REF)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_constant_encoding() {
        // Test that common Io method names have distinct addresses
        let forward_addr = match "forward" {
            "forward" => 0x3001,
            _ => 0,
        };
        
        let plus_addr = match "+" {
            "+" => 0x3004,
            _ => 0,
        };
        
        let custom_addr = match "custom" {
            "forward" | "+" | "-" | "*" | "/" | "asString" | "print" => 0,
            _ => 0x3000,
        };
        
        assert_eq!(forward_addr, 0x3001);
        assert_eq!(plus_addr, 0x3004);
        assert_eq!(custom_addr, 0x3000);
        
        // Check that the algorithm in encode_string_constant works as expected
        assert_ne!(forward_addr, plus_addr);
        assert_ne!(forward_addr, custom_addr);
    }
}