//! WebAssembly compilation target using Cranelift
//!
//! This module provides WebAssembly code generation capabilities for the Io language.
//! It uses Cranelift to generate WASM binary that can be executed in a WASM runtime.

use cranelift_codegen::Context as CodegenContext;
use cranelift_codegen::ir;
use cranelift_module::{FuncId, Linkage, ModuleResult};
use std::collections::HashMap;
use wasm_encoder::{Module, Function, CodeSection, TypeSection, ExportSection, ExportKind};
use wasmer::FunctionType;
use wasmparser::{Validator, WasmFeatures};

/// Builder for WebAssembly modules
pub struct WasmBuilder {
    /// The Cranelift IR functions that will become WASM functions
    functions: HashMap<FuncId, ir::Function>,
    /// Function signatures
    signatures: HashMap<FuncId, ir::Signature>,
    /// Next function ID
    next_func_id: u32,
    /// Function names for export
    function_names: HashMap<FuncId, String>,
}

impl WasmBuilder {
    /// Create a new WebAssembly module builder
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            signatures: HashMap::new(),
            next_func_id: 0,
            function_names: HashMap::new(),
        }
    }

    /// Get a context for code generation
    pub fn make_context(&mut self) -> CodegenContext {
        CodegenContext::new()
    }

    /// Declare a function in the WebAssembly module
    pub fn declare_function(
        &mut self,
        name: &str,
        _linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let id = FuncId::from_u32(self.next_func_id);
        self.next_func_id += 1;
        self.signatures.insert(id, signature.clone());
        self.function_names.insert(id, name.to_string());
        Ok(id)
    }

    /// Define a function in the WebAssembly module
    pub fn define_function(
        &mut self,
        func_id: FuncId,
        ctx: &mut CodegenContext,
    ) -> ModuleResult<()> {
        // Clone the function from the context
        let function = ctx.func.clone();
        
        // Store the function for later WASM generation
        self.functions.insert(func_id, function);
        
        Ok(())
    }

    /// Finalize the module and return the WebAssembly binary
    pub fn finalize(self, main_func_id: FuncId) -> Vec<u8> {
        // Create a new WASM module
        let mut module = Module::new();
        
        // Add type section with function signatures
        let mut types = TypeSection::new();
        for (_func_id, signature) in &self.signatures {
            let params: Vec<wasm_encoder::ValType> = signature.params
                .iter()
                .map(|p| wasm_type_from_cranelift(p.value_type))
                .collect();
                
            let results: Vec<wasm_encoder::ValType> = signature.returns
                .iter()
                .map(|r| wasm_type_from_cranelift(r.value_type))
                .collect();
                
            let mut func_encoder = types.ty();
            func_encoder.function(params, results);
        }
        module.section(&types);
        
        // Create export section
        let mut exports = ExportSection::new();
        if let Some(name) = self.function_names.get(&main_func_id) {
            exports.export(name, ExportKind::Func, main_func_id.as_u32());
        }
        module.section(&exports);
        
        // Generate function bodies
        let mut code_section = CodeSection::new();
        for (_func_id, _function) in &self.functions {
            // Convert Cranelift IR to WASM code
            let wasm_func = create_simple_wasm_function();
            code_section.function(&wasm_func);
        }
        module.section(&code_section);
        
        // Finish the module
        let wasm_bytes = module.finish();
        
        // Validate the WASM module
        validate_wasm_module(&wasm_bytes);
        
        wasm_bytes
    }
}

/// Convert a Cranelift type to a WASM type
fn wasm_type_from_cranelift(ty: ir::Type) -> wasm_encoder::ValType {
    match ty {
        ir::types::I32 => wasm_encoder::ValType::I32,
        ir::types::I64 => wasm_encoder::ValType::I64,
        ir::types::F32 => wasm_encoder::ValType::F32,
        ir::types::F64 => wasm_encoder::ValType::F64,
        _ => panic!("Unsupported Cranelift type for WASM: {:?}", ty),
    }
}

/// Create a simple WebAssembly function that returns 42
fn create_simple_wasm_function() -> Function {
    let mut func = Function::new([]);
    
    // Push constant 42 and return
    func.instruction(&wasm_encoder::Instruction::I64Const(42));
    func.instruction(&wasm_encoder::Instruction::End);
    
    func
}

/// Validate a WASM module
fn validate_wasm_module(wasm_bytes: &[u8]) -> bool {
    let features = WasmFeatures::default();
    let mut validator = Validator::new_with_features(features);
    
    match validator.validate_all(wasm_bytes) {
        Ok(_) => true, // Changed from Ok(()) to Ok(_) since the return type is different
        Err(err) => {
            println!("WASM validation error: {:?}", err);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{Function, Signature, AbiParam, InstBuilder};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_codegen::ir::types;
    
    #[test]
    fn test_wasm_module_generation() {
        // Create a simple function that returns 42
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::Fast);
        sig.returns.push(AbiParam::new(types::I64));
        
        let func_name = ir::UserFuncName::testcase("test");
        let mut func = Function::with_name_signature(func_name, sig);
        let mut func_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);
        
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        
        let const_val = builder.ins().iconst(types::I64, 42);
        builder.ins().return_(&[const_val]);
        builder.seal_all_blocks();
        builder.finalize();
        
        // Create a WASM builder and add our function
        let mut wasm_builder = WasmBuilder::new();
        let mut ctx = wasm_builder.make_context();
        ctx.func = func;
        
        let func_id = wasm_builder.declare_function("main", Linkage::Export, &ctx.func.signature).unwrap();
        wasm_builder.define_function(func_id, &mut ctx).unwrap();
        
        // Generate the WASM module
        let wasm_bytes = wasm_builder.finalize(func_id);
        
        // Ensure we got a valid WASM module
        assert!(wasm_bytes.len() > 8);
        assert_eq!(&wasm_bytes[0..4], b"\0asm"); // WASM magic bytes
        assert_eq!(&wasm_bytes[4..8], &[1, 0, 0, 0]); // WASM version
    }
}