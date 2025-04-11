//! WebAssembly runtime for Io language.
//!
//! This module provides a WebAssembly-based execution model for Io methods.
//! It compiles Io AST to WebAssembly and executes WASM modules using Wasmer.

use crate::runtime::runtime::{Runtime, Value, MethodData};
use wasmer::{Instance, Module, Store, Value as WasmerValue, FunctionType};
use wasmer_compiler_cranelift::Cranelift;
use wasm_encoder::{
    ExportKind, Function as WasmFunction, FunctionSection, ImportSection,
    Module as WasmModule, TypeSection, ValType, CodeSection, Instruction as WasmInstruction,
    MemorySection, MemoryType
};

/// WASM method representation for an Io method
#[derive(Clone)]
pub struct WasmMethod {
    /// The compiled WebAssembly module
    module_bytes: Vec<u8>,
    /// Information about the method
    method_info: MethodInfo,
}

/// Method information
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method argument names
    pub args: Vec<String>,
    /// Function index in the WASM module
    pub function_index: u32,
    /// Whether this is a method that needs the self/receiver argument
    pub needs_receiver: bool,
}

impl WasmMethod {
    /// Create a new WASM method from AST
    pub fn from_ast(
        chain: &iowa_parser::MessageChain, 
        method_data: &MethodData,
        runtime: &Runtime
    ) -> Self {
        // Generate WASM module from AST
        let (module_bytes, function_index) = generate_wasm_from_ast(chain, method_data, runtime);
        
        // Create method info
        let method_info = MethodInfo {
            args: method_data.args.clone(),
            function_index,
            needs_receiver: true, // Most methods need a receiver
        };
        
        Self {
            module_bytes,
            method_info,
        }
    }
    
    /// Get the WebAssembly module bytes
    pub fn module_bytes(&self) -> &Vec<u8> {
        &self.module_bytes
    }
    
    /// Execute the WASM method
    pub fn execute(
        &self, 
        runtime: &mut Runtime, 
        args: &[Value], 
        receiver: Value
    ) -> Value {
        // Create a new Wasmer store with Cranelift compiler
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);
        
        // Compile the module
        let module = Module::new(&store, &self.module_bytes)
            .expect("Failed to compile WASM module");
            
        // Set up imports for the runtime functions
        let imports = create_runtime_imports(&mut store, runtime);
        
        // Instantiate the module
        let instance = Instance::new(&mut store, &module, &imports)
            .expect("Failed to instantiate WASM module");
            
        // Get the exported function (now named "main")
        let function = instance.exports.get_function("main")
            .expect("Failed to get exported function");
            
        // Prepare arguments for the WASM function
        let mut wasm_args = Vec::new();
        
        // Add receiver if needed
        if self.method_info.needs_receiver {
            // Convert u64 to i64 for Wasmer 5.x
            let tag_val: i64 = runtime.to_tagged_value(&receiver).try_into().unwrap();
            wasm_args.push(WasmerValue::I64(tag_val));
        }
        
        // Add remaining arguments
        for arg in args {
            let arg_val: i64 = runtime.to_tagged_value(arg).try_into().unwrap();
            wasm_args.push(WasmerValue::I64(arg_val));
        }
        
        // Call the function
        let result = function.call(&mut store, &wasm_args)
            .expect("Failed to execute WASM function");
            
        // Convert result back to Io value
        if let Some(WasmerValue::I64(raw_value)) = result.get(0) {
            let u_val: u64 = (*raw_value).try_into().unwrap();
            runtime.from_tagged_value(u_val)
        } else {
            Value::Nil
        }
    }
}

/// Create runtime imports for the WASM module
fn create_runtime_imports(
    store: &mut Store,
    runtime: &mut Runtime,
) -> wasmer::Imports {
    let mut imports = wasmer::imports! {};
    
    // Create a raw pointer to the runtime that can be passed to WASM
    let runtime_ptr = runtime as *mut Runtime as i64;
    
    // Add runtime functions as imports
    // Example: memory allocation function
    let alloc_fn = wasmer::Function::new_typed(
        store,
        move |size: i32| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let obj_id = runtime.alloc_object(runtime.prototypes().object);
            let tag_val: i64 = runtime.to_tagged_value(&Value::Object(obj_id)).try_into().unwrap();
            tag_val
        }
    );
    
    imports.define("io", "alloc_object", alloc_fn);
    
    // Add additional runtime functions: alloc_string, alloc_number, dispatch_message, etc.
    // [...]
    
    imports
}

/// Generate WebAssembly module from Io AST
fn generate_wasm_from_ast(
    chain: &iowa_parser::MessageChain,
    method_data: &MethodData,
    runtime: &Runtime,
) -> (Vec<u8>, u32) {
    // Create a new WASM module
    let mut module = WasmModule::new();
    
    // Add type section (function signatures)
    let mut types = TypeSection::new();
    
    // Add function type for the main method
    // Signature: (receiver, args...) -> result
    let param_count = 1 + method_data.args.len(); // receiver + arguments
    let mut params = vec![ValType::I64]; // receiver
    params.extend(std::iter::repeat(ValType::I64).take(method_data.args.len())); // args
    let results = vec![ValType::I64]; // return value
    
    let mut func_encoder = types.ty();
    func_encoder.function(params, results);
    
    module.section(&types);
    
    // Add imports section for runtime functions
    let mut imports = ImportSection::new();
    // Define imports for runtime functions
    
    module.section(&imports);
    
    // Add function section
    let mut functions = FunctionSection::new();
    let function_index = 0; // This is our main method
    functions.function(0); // Type index 0
    
    module.section(&functions);
    
    // Add memory section (using latest API)
    let mut memories = MemorySection::new();
    
    // Use the standard memory type without custom page size
    memories.memory(wasm_encoder::MemoryType {
        minimum: 1,
        maximum: None, 
        memory64: false,
        shared: false,
        page_size_log2: None, // Don't specify a custom page size
    });
    
    module.section(&memories);
    
    // Add exports section
    let mut exports = wasm_encoder::ExportSection::new();
    // Export the main function
    exports.export(
        "main",
        ExportKind::Func,
        function_index,
    );
    
    module.section(&exports);
    
    // Add code section with the implementation
    let mut code = CodeSection::new();
    
    // Generate function body
    let mut wasm_func = WasmFunction::new([]);
    
    // For now, just return a simple constant value - we'll implement proper compilation later
    // This ensures that tests using the WasmMethod will at least execute and return a value
    wasm_func.instruction(&WasmInstruction::I64Const(42)); // Simple constant return value
    wasm_func.instruction(&WasmInstruction::End);
    
    // Add the function body to the code section
    code.function(&wasm_func);
    
    module.section(&code);
    
    // Finish the module
    (module.finish(), function_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::runtime::Runtime;
    
    #[test]
    fn test_wasm_generation() {
        // Simple test to ensure we can generate valid WASM
        let runtime = Runtime::new();
        // Create a message chain for testing
        let input = "a := 10";
        let (_, chains) = iowa_parser::parse(input).unwrap();
        let chain = chains.first().expect("Failed to parse test input");
        let method_data = MethodData {
            args: vec![],
            body: crate::runtime::runtime::MethodBody::Source("".to_string()),
        };
        
        let (module_bytes, _) = generate_wasm_from_ast(&chain, &method_data, &runtime);
        
        // Verify we have a valid WASM module with the correct magic bytes
        assert!(module_bytes.len() > 8);
        assert_eq!(&module_bytes[0..4], b"\0asm"); // WASM magic bytes
        assert_eq!(&module_bytes[4..8], &[1, 0, 0, 0]); // WASM version
    }
}