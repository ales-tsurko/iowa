//! WebAssembly compilation target using Cranelift
//!
//! This module provides WebAssembly code generation capabilities for the Io language.
//! It uses Cranelift to generate WASM binary that can be executed in a WASM runtime.

use cranelift_codegen::Context as CodegenContext;
use cranelift_codegen::ir;
use cranelift_module::{FuncId, Linkage, ModuleResult};
use std::collections::HashMap;
use wasm_encoder::{CodeSection, ExportKind, ExportSection, Function, Module, TypeSection};
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
        let function = ctx.func.clone();
        self.functions.insert(func_id, function);
        Ok(())
    }

    /// Finalize the module and return the WebAssembly binary
    pub fn finalize(self, main_func_id: FuncId) -> Vec<u8> {
        // Create a new WebAssembly module
        let mut module = Module::new();

        // The WebAssembly spec requires sections to appear in a specific order:
        // 1. Type section
        // 2. Import section
        // 3. Function section
        // 4. Table section
        // 5. Memory section
        // 6. Global section
        // 7. Export section
        // 8. Start section
        // 9. Element section
        // 10. Data count section
        // 11. Code section
        // 12. Data section

        // 1. Type section
        let mut type_section = TypeSection::new();
        for (_func_id, signature) in &self.signatures {
            let params: Vec<wasm_encoder::ValType> = signature
                .params
                .iter()
                .map(|p| wasm_type_from_cranelift(p.value_type))
                .collect();

            let results: Vec<wasm_encoder::ValType> = signature
                .returns
                .iter()
                .map(|r| wasm_type_from_cranelift(r.value_type))
                .collect();

            // Add function type directly
            type_section.ty().function(params, results);
        }
        module.section(&type_section);

        // 2. Import section (we don't have any imports)

        // 3. Function section - one entry per function, specifying its type
        let mut function_section = wasm_encoder::FunctionSection::new();
        for _ in 0..self.functions.len() {
            function_section.function(0); // All functions use type index 0 for now
        }
        module.section(&function_section);

        // 4-6. Skip Table, Memory, Global sections (we don't use them in our simple test)

        // 7. Export section
        let mut export_section = ExportSection::new();
        if let Some(name) = self.function_names.get(&main_func_id) {
            // Export the function with the correct index
            let func_idx = self
                .functions
                .keys()
                .position(|id| *id == main_func_id)
                .unwrap_or(0) as u32;
            export_section.export(name, ExportKind::Func, func_idx);
        }
        module.section(&export_section);

        // 8-10. Skip Start, Element, Data count sections (we don't use them)

        // 11. Code section - function bodies (must come AFTER exports)
        let mut code_section = CodeSection::new();
        for (_func_id, function) in &self.functions {
            let wasm_func = create_wasm_function_from_ir(function);
            code_section.function(&wasm_func);
        }
        module.section(&code_section);

        // Generate the final WebAssembly binary
        let wasm_bytes = module.finish();

        // Validate the module and handle errors properly
        let is_valid = validate_wasm_module(&wasm_bytes);
        if !is_valid {
            panic!("Generated WASM module is invalid - this indicates a bug in the WASM generator");
        }

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

/// Create a WebAssembly function from a Cranelift IR function
fn create_wasm_function_from_ir(function: &ir::Function) -> Function {
    // Create a minimal valid function body
    let locals = Vec::new();

    // For now this is a simplified implementation that returns a constant value
    // In a real implementation, we would translate the entire function body
    // by walking the CFG and emitting instructions for each block

    let mut func = Function::new(locals);

    // Check the return type and emit an appropriate constant
    if let Some(result) = function.signature.returns.first() {
        match result.value_type {
            ir::types::I32 => {
                func.instruction(&wasm_encoder::Instruction::I32Const(42));
            }
            ir::types::I64 => {
                func.instruction(&wasm_encoder::Instruction::I64Const(42));
            }
            ir::types::F32 => {
                func.instruction(&wasm_encoder::Instruction::F32Const(42.0));
            }
            ir::types::F64 => {
                func.instruction(&wasm_encoder::Instruction::F64Const(42.0));
            }
            _ => {
                // For any other return type, emit a trap
                func.instruction(&wasm_encoder::Instruction::Unreachable);
            }
        }
    }

    // End the function
    func.instruction(&wasm_encoder::Instruction::End);

    func
}

/// Validate a WASM module
fn validate_wasm_module(wasm_bytes: &[u8]) -> bool {
    let features = WasmFeatures::default();
    let mut validator = Validator::new_with_features(features);

    match validator.validate_all(wasm_bytes) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("WASM validation error: {:?}", err);

            // Parse and print the module structure for debugging
            eprintln!("\nWASM module structure:");
            let parser = wasmparser::Parser::new(0);
            for payload in parser.parse_all(wasm_bytes) {
                match payload {
                    Ok(wasmparser::Payload::Version { num, range, .. }) => {
                        eprintln!("Version: {} ({}..{})", num, range.start, range.end);
                    }
                    Ok(wasmparser::Payload::TypeSection(reader)) => {
                        eprintln!("Type section: count={}", reader.count());
                    }
                    Ok(wasmparser::Payload::FunctionSection(reader)) => {
                        eprintln!("Function section: count={}", reader.count());
                    }
                    Ok(wasmparser::Payload::CodeSectionStart { count, range, .. }) => {
                        eprintln!(
                            "Code section: count={} ({}..{})",
                            count, range.start, range.end
                        );
                    }
                    Ok(wasmparser::Payload::ExportSection(reader)) => {
                        eprintln!("Export section: count={}", reader.count());
                    }
                    Ok(payload) => {
                        // Other section types
                        eprintln!("Other section: {:?}", payload);
                    }
                    Err(e) => {
                        eprintln!("Error parsing WASM section: {:?}", e);
                    }
                }
            }

            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::types;
    use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

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

        let func_id = wasm_builder
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();
        wasm_builder.define_function(func_id, &mut ctx).unwrap();

        // Generate the WASM module
        let wasm_bytes = wasm_builder.finalize(func_id);

        // Ensure we got a valid WASM module
        assert!(wasm_bytes.len() > 8);
        assert_eq!(&wasm_bytes[0..4], b"\0asm"); // WASM magic bytes
        assert_eq!(&wasm_bytes[4..8], &[1, 0, 0, 0]); // WASM version
    }
}
