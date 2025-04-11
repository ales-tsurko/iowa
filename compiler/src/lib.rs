//! Compiler for Io programming language.
//!
//! This module uses Cranelift to compile Io code to machine code for JIT execution
//! and to WebAssembly modules for portable distribution.

mod backend;
pub mod runtime;

use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module};
use iowa_parser::{Message, MessageChain, Number, Symbol};

/// Runtime value type for Io objects
#[derive(Debug, Clone)]
pub enum IoValue {
    /// Nil value
    Nil,
    /// Boolean value
    Boolean(bool),
    /// Number value
    Number(f64),
    /// String value
    String(String),
    /// Object with slots
    Object(usize), // Reference to runtime object table
    /// Message
    Message(usize), // Reference to runtime message table
}

/// Compile the Io message chain for JIT execution
/// 
/// This compiles Io code to WebAssembly first, then uses Cranelift to JIT-compile
/// the WASM module for execution.
pub fn compile_jit(chain: &MessageChain) -> backend::jit::JitFunction {
    // Create a new runtime instance - thread-safe with Arc<Mutex<>>
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(runtime::runtime::Runtime::new()));
    
    // First generate WASM as our primary IR
    let wasm_module = compile_wasm(chain);
    
    // Create JIT backend
    let mut jit_builder = backend::jit::JitBuilder::new();
    
    // Set the WASM module to use (source of truth)
    jit_builder.set_wasm_module(wasm_module);

    // Compile the message chain to Cranelift IR (for JIT execution)
    let function_id = compile_to_jit_module(chain, &mut jit_builder, runtime.clone());

    // Finalize and return the compiled function with runtime reference
    jit_builder.finalize(function_id, runtime)
}

/// Compile the Io message chain to a WebAssembly module
/// 
/// This is the primary compilation path - all Io code is compiled to WASM.
pub fn compile_wasm(chain: &MessageChain) -> Vec<u8> {
    // Create a new runtime instance - thread-safe with Arc<Mutex<>>
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(runtime::runtime::Runtime::new()));
    
    // Create WASM backend
    let mut wasm_builder = backend::wasm::WasmBuilder::new();

    // Compile the message chain to WASM
    let function_id = compile_to_wasm_module(chain, &mut wasm_builder, runtime.clone());

    // Finalize and return the WASM module
    wasm_builder.finalize(function_id)
}

/// Compile the Io message chain and return a WASM module with a Wasmer runtime instance
pub fn compile_wasmer(chain: &MessageChain) -> (Vec<u8>, std::sync::Arc<std::sync::Mutex<runtime::runtime::Runtime>>) {
    // Create a new runtime instance - thread-safe with Arc<Mutex<>>
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(runtime::runtime::Runtime::new()));
    
    // Compile to WASM
    let wasm_bytes = compile_wasm(chain);
    
    // Return both the WASM module and the runtime
    (wasm_bytes, runtime)
}

/// Shared compilation logic for JIT backend
fn compile_to_jit_module(
    chain: &MessageChain,
    jit_builder: &mut backend::jit::JitBuilder,
    runtime: std::sync::Arc<std::sync::Mutex<runtime::runtime::Runtime>>,
) -> cranelift_module::FuncId {
    // Create function context and builder
    let mut ctx = jit_builder.make_context();
    let mut func_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

    // Set up entry block
    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);

    // Compile the message chain to Cranelift IR
    let result = compile_message_chain(chain, &mut builder);

    // Return the computed value
    builder.ins().return_(&[result]);

    // Finalize function
    builder.seal_all_blocks();
    builder.finalize();

    // Add to module and return function ID using the JITModule
    let func_id = jit_builder
        .module
        .declare_function("main", Linkage::Export, &ctx.func.signature)
        .unwrap();

    jit_builder
        .module
        .define_function(func_id, &mut ctx)
        .unwrap();

    func_id
}

/// Shared compilation logic for WASM backend
fn compile_to_wasm_module(
    chain: &MessageChain,
    wasm_builder: &mut backend::wasm::WasmBuilder,
    runtime: std::sync::Arc<std::sync::Mutex<runtime::runtime::Runtime>>,
) -> cranelift_module::FuncId {
    // Create function context and builder
    let mut ctx = wasm_builder.make_context();
    let mut func_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

    // Set up entry block
    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);

    // Compile the message chain to Cranelift IR
    let result = compile_message_chain(chain, &mut builder);

    // Return the computed value
    builder.ins().return_(&[result]);

    // Finalize function
    builder.seal_all_blocks();
    builder.finalize();

    // Add to module and return function ID
    let func_id = wasm_builder
        .declare_function("main", Linkage::Export, &ctx.func.signature)
        .unwrap();

    wasm_builder.define_function(func_id, &mut ctx).unwrap();

    func_id
}

/// Compile a message chain to Cranelift IR
fn compile_message_chain(chain: &MessageChain, builder: &mut FunctionBuilder) -> Value {
    // For an empty chain, return nil
    if chain.messages.is_empty() {
        return runtime::value::encode_nil(builder);
    }

    // Evaluate each message in sequence
    let mut result = compile_message(&chain.messages[0], builder);

    for message in chain.messages.iter().skip(1) {
        // Each subsequent message uses the previous result as the receiver
        result = compile_message_with_receiver(message, result, builder);
    }

    result
}

/// Compile a single message to Cranelift IR
fn compile_message(message: &Message, builder: &mut FunctionBuilder) -> Value {
    match &message.symbol {
        // For literals, encode them directly
        Symbol::Number(n) => match n {
            Number::Decimal(value) => {
                let f64_val = builder.ins().f64const(*value);
                runtime::memory::allocation::alloc_number(builder, f64_val)
            }
            Number::Hex(value) => {
                let f64_val = builder.ins().f64const(*value as f64);
                runtime::memory::allocation::alloc_number(builder, f64_val)
            }
        },
        Symbol::Quote(quote) => {
            // Allocate a string object for the quoted content
            let content = quote.content();
            let length = builder.ins().iconst(types::I32, content.len() as i64);
            let string_ptr = runtime::memory::allocation::alloc_string(builder, length);

            // In a real implementation, we would copy the content to the string object
            // Here we just return the tagged string reference
            runtime::value::encode_reference(builder, string_ptr, runtime::value::TAG_STRING_REF)
        }
        // Identifiers without a receiver refer to slots in the current context (lobby)
        Symbol::Identifier(id) => {
            // Get the Lobby object (would be passed in or defined globally)
            let lobby = get_lobby(builder);

            // Create a string object for the identifier name
            let name_str = id.name();
            let name_value = runtime::dispatch::encode_string_constant(builder, name_str);

            // Look up the slot in the Lobby
            runtime::object::lookup_slot(builder, lobby, name_value)
        }
        // For operators without a receiver, we treat the input value as the receiver
        // and look for a unary operator implementation
        Symbol::Operator(op) => {
            // Get the Lobby object
            let receiver = get_lobby(builder);

            // Create a string for the operator name
            let op_name = runtime::dispatch::encode_string_constant(builder, op.symbol());

            // Compile arguments
            let mut arg_values = Vec::new();
            for arg in &message.args {
                if !arg.chains.is_empty() {
                    let arg_value = compile_message_chain(&arg.chains[0], builder);
                    arg_values.push(arg_value);
                }
            }

            // Dispatch the message
            runtime::dispatch::call_message_dispatch(builder, receiver, op_name, &arg_values)
        }
    }
}

/// Compile a message with an explicit receiver
fn compile_message_with_receiver(
    message: &Message,
    receiver: Value,
    builder: &mut FunctionBuilder,
) -> Value {
    // Compile all arguments
    let mut arg_values = Vec::new();
    for arg in &message.args {
        if !arg.chains.is_empty() {
            let arg_value = compile_message_chain(&arg.chains[0], builder);
            arg_values.push(arg_value);
        }
    }

    // Create a string object for the message name
    let message_name = match &message.symbol {
        Symbol::Identifier(id) => runtime::dispatch::encode_string_constant(builder, id.name()),
        Symbol::Operator(op) => runtime::dispatch::encode_string_constant(builder, op.symbol()),
        // For other symbols as messages, convert to string first
        Symbol::Number(n) => {
            let num_str = match n {
                Number::Decimal(value) => format!("{}", value),
                Number::Hex(value) => format!("0x{:X}", value),
            };
            runtime::dispatch::encode_string_constant(builder, &num_str)
        }
        Symbol::Quote(q) => runtime::dispatch::encode_string_constant(builder, q.content()),
    };

    // Dispatch the message - this will look up methods in the prototype chain
    runtime::dispatch::call_message_dispatch(builder, receiver, message_name, &arg_values)
}

/// Get the Lobby object (global context)
fn get_lobby(builder: &mut FunctionBuilder) -> Value {
    // For now, return a placeholder reference
    // In a real implementation, this would be passed in or defined globally
    let ptr = builder.ins().iconst(types::I64, 0x5000);
    runtime::value::encode_reference(builder, ptr, runtime::value::TAG_OBJECT_REF)
}

// In Io, operators are just methods, so we don't need special handling for them.
// The runtime dispatch mechanism will handle looking up methods in prototypes.
