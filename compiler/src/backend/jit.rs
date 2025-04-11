//! JIT compilation target using Cranelift
//!
//! This module provides JIT compilation capabilities for Io language.

use cranelift_codegen::Context as CodegenContext;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder as CraneliftJITBuilder, JITModule};
use cranelift_module::Module;
use cranelift_native as native;
use std::mem;
use std::sync::{Arc, Mutex};

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
}

impl JitFunction {
    /// Create a new JIT function
    fn new(
        module: JITModule, 
        func_ptr: *const u8, 
        runtime: Arc<Mutex<crate::runtime::runtime::Runtime>>
    ) -> Self {
        // Convert raw function pointer to the expected signature
        let func = unsafe { mem::transmute::<*const u8, IoJitFunction>(func_ptr) };

        Self {
            _module: module,
            func,
            runtime,
        }
    }

    /// Execute the JIT-compiled function
    pub fn execute(&self) -> u64 {
        unsafe { (self.func)() }
    }
    
    /// Get a reference to the runtime
    pub fn runtime(&self) -> &Arc<Mutex<crate::runtime::runtime::Runtime>> {
        &self.runtime
    }
}

/// Builder for JIT-compiled functions
pub struct JitBuilder {
    /// The underlying Cranelift JIT module
    pub module: JITModule,
    /// Execution context for the JIT
    ctx: CodegenContext,
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

        Self { module, ctx }
    }

    /// Get a context for code generation
    pub fn make_context(&mut self) -> CodegenContext {
        self.module.make_context()
    }

    /// Finalize the compilation and return a callable function
    pub fn finalize(
        mut self, 
        func_id: cranelift_module::FuncId,
        runtime: Arc<Mutex<crate::runtime::runtime::Runtime>>
    ) -> JitFunction {
        // Finalize the module
        self.module.finalize_definitions().unwrap();

        // Get the function pointer
        let code_ptr = self.module.get_finalized_function(func_id);

        // Create and return the JIT function
        JitFunction::new(self.module, code_ptr, runtime)
    }
}

// Implementation removed - we're using the methods on JitBuilder directly
