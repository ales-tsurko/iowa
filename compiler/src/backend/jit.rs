//! JIT compilation target using Cranelift and WebAssembly
//!
//! This module provides JIT compilation capabilities for Io language.
//! It uses WebAssembly as the intermediate representation, then compiles
//! the WASM to native code via Cranelift for execution.

use cranelift_codegen::Context as CodegenContext;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder as CraneliftJITBuilder, JITModule};
use cranelift_module::Module;
use cranelift_native as native;
use std::mem;
use std::sync::{Arc, Mutex};
use wasmer::{Function, Instance, Module as WasmerModule, Store, Value as WasmerValue};
use wasmer_compiler_cranelift::Cranelift;

/// Function signature for JIT-compiled Io functions
pub type IoJitFunction = unsafe extern "C" fn() -> u64;

/// Wrapper around a JIT-compiled function with a runtime instance
pub struct JitFunction {
    /// The JIT module that contains the function
    _module: JITModule, // Keep module alive while function exists
    /// Function pointer to the compiled code
    pub func: IoJitFunction,
    /// The runtime instance used by this function
    runtime: Arc<Mutex<crate::runtime::runtime::Runtime>>,
    /// The compiled WASM module (if we're using WASM-based execution)
    wasm_module: Option<Vec<u8>>,
}

impl JitFunction {
    /// Create a new JIT function
    fn new(
        module: JITModule,
        func_ptr: *const u8,
        runtime: Arc<Mutex<crate::runtime::runtime::Runtime>>,
        wasm_module: Option<Vec<u8>>,
    ) -> Self {
        // Convert raw function pointer to the expected signature
        let func = unsafe { mem::transmute::<*const u8, IoJitFunction>(func_ptr) };

        Self {
            _module: module,
            func,
            runtime,
            wasm_module,
        }
    }

    /// Execute the JIT-compiled function
    pub fn execute(&self) -> u64 {
        // If we have a WASM module, we could execute it via Wasmer instead
        // For now, just use the JIT-compiled function directly
        unsafe { (self.func)() }
    }

    /// Execute using the WASM module directly via Wasmer
    pub fn execute_wasm(&self) -> u64 {
        if let Some(wasm_bytes) = &self.wasm_module {
            // Create a Wasmer store with Cranelift compiler
            let mut store = Store::new(Cranelift::default());

            // Compile the WASM module
            let module =
                WasmerModule::new(&store, wasm_bytes).expect("Failed to compile WASM module");

            // Set up runtime imports for the WASM module
            let imports = self.create_runtime_imports(&mut store);

            // Instantiate the module
            let instance = Instance::new(&mut store, &module, &imports)
                .expect("Failed to instantiate WASM module");

            // Get the main function
            let main_func = instance
                .exports
                .get_function("main")
                .expect("Failed to get main function");

            // Call the function with no arguments
            let result = main_func
                .call(&mut store, &[])
                .expect("Failed to call main function");

            // Extract the result and convert i64 to u64
            if let Some(WasmerValue::I64(val)) = result.first() {
                (*val).try_into().unwrap_or(0)
            } else {
                0 // Default to nil
            }
        } else {
            // Fall back to the JIT-compiled function
            self.execute()
        }
    }

    /// Create Wasmer imports for runtime functions
    fn create_runtime_imports(&self, store: &mut Store) -> wasmer::Imports {
        let mut imports = wasmer::imports! {};

        // We'll need to add imports for all runtime functions here
        // For now, just a placeholder

        imports
    }

    /// Get a reference to the runtime
    pub fn runtime(&self) -> &Arc<Mutex<crate::runtime::runtime::Runtime>> {
        &self.runtime
    }

    /// Get the WASM module bytes, if available
    pub fn wasm_module(&self) -> Option<&Vec<u8>> {
        self.wasm_module.as_ref()
    }
}

/// Builder for JIT-compiled functions
pub struct JitBuilder {
    /// The underlying Cranelift JIT module
    pub module: JITModule,
    /// Execution context for the JIT
    ctx: CodegenContext,
    /// WebAssembly module being built (primary IR)
    wasm_module: Option<Vec<u8>>,
}

impl JitBuilder {
    /// Create a new JIT builder
    pub fn new() -> Self {
        // Set up JIT builder
        let mut flag_builder = settings::builder();
        flag_builder.enable("use_colocated_libcalls").unwrap();
        flag_builder.enable("is_pic").unwrap();
        let isa_builder = native::builder().unwrap();
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        // Create JIT module
        let builder = CraneliftJITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        let ctx = module.make_context();

        Self {
            module,
            ctx,
            wasm_module: None,
        }
    }

    /// Get a context for code generation
    pub fn make_context(&mut self) -> CodegenContext {
        self.module.make_context()
    }

    /// Set the WebAssembly module to use as the primary IR
    pub fn set_wasm_module(&mut self, wasm_bytes: Vec<u8>) {
        self.wasm_module = Some(wasm_bytes);
    }

    /// Finalize the compilation and return a callable function
    pub fn finalize(
        mut self,
        func_id: cranelift_module::FuncId,
        runtime: Arc<Mutex<crate::runtime::runtime::Runtime>>,
    ) -> JitFunction {
        // Finalize the module
        self.module.finalize_definitions().unwrap();

        // Get the function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Create and return the JIT function
        JitFunction::new(self.module, code_ptr, runtime, self.wasm_module)
    }
}

// Implementation removed - we're using the methods on JitBuilder directly

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::runtime::Runtime;
    use cranelift_codegen::ir::types;
    use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::Linkage;

    #[test]
    #[ignore = "JIT test is platform-dependent"]
    fn test_jit_execution() {
        // Create a simple function that returns 42
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::Fast);
        sig.returns.push(AbiParam::new(types::I64));

        let func_name = cranelift_codegen::ir::UserFuncName::testcase("test");
        let mut func = Function::with_name_signature(func_name, sig);
        let mut func_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);

        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        // Return constant 42
        let const_val = builder.ins().iconst(types::I64, 42);
        builder.ins().return_(&[const_val]);
        builder.seal_all_blocks();
        builder.finalize();

        // Create JIT builder
        let mut jit_builder = JitBuilder::new();
        let mut ctx = jit_builder.make_context();
        ctx.func = func;

        // Add the function
        let func_id = jit_builder
            .module
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();

        jit_builder
            .module
            .define_function(func_id, &mut ctx)
            .unwrap();

        // Create runtime
        let runtime = Arc::new(Mutex::new(Runtime::new()));

        // Finalize and get function
        let jit_func = jit_builder.finalize(func_id, runtime);

        // Execute and check result
        let result = jit_func.execute();
        assert_eq!(result, 42);
    }
}
