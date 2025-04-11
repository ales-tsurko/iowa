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

/// Determine if a Cranelift type needs conversion to be compatible with WASM
fn needs_type_conversion(from: ir::Type, to: ir::Type) -> bool {
    // The most common issue is converting between i32 and i64
    (from == ir::types::I32 && to == ir::types::I64)
        || (from == ir::types::I64 && to == ir::types::I32)
}

/// Generate WebAssembly instruction for type conversion
fn insert_type_conversion(func: &mut wasm_encoder::Function, from: ir::Type, to: ir::Type) {
    if from == ir::types::I32 && to == ir::types::I64 {
        // Convert i32 to i64
        func.instruction(&wasm_encoder::Instruction::I64ExtendI32S);
    } else if from == ir::types::I64 && to == ir::types::I32 {
        // Convert i64 to i32 (with possible truncation)
        func.instruction(&wasm_encoder::Instruction::I32WrapI64);
    }
    // Add other conversions as needed (f32/f64 etc.)
}

/// Create a WebAssembly function from a Cranelift IR function
fn create_wasm_function_from_ir(function: &ir::Function) -> Function {
    use cranelift_codegen::ir::Opcode;
    use std::collections::HashMap;
    use wasm_encoder::Instruction as WasmInst;

    // Analyze and allocate locals for the function
    let mut value_map = HashMap::new(); // Maps Cranelift values to WASM locals
    let mut next_local = 0u32;

    // Count locals by type for wasm-encoder's expected format
    let mut i32_locals = 0u32;
    let mut i64_locals = 0u32;
    let mut f32_locals = 0u32;
    let mut f64_locals = 0u32;

    // Collect all local variables needed from the Cranelift IR
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            // Skip instruction types that don't produce values
            let results = function.dfg.inst_results(inst);
            if results.is_empty() {
                continue;
            }

            // For instructions that produce values, allocate a local for each result
            for &value in results {
                let ty = function.dfg.value_type(value);
                value_map.insert(value, next_local);

                // Count locals by type
                match ty {
                    ir::types::I32 => i32_locals += 1,
                    ir::types::I64 => i64_locals += 1,
                    ir::types::F32 => f32_locals += 1,
                    ir::types::F64 => f64_locals += 1,
                    _ => {} // Ignore other types for now
                }

                next_local += 1;
            }
        }
    }

    // Create a Vec of (count, type) pairs for wasm-encoder
    let mut wasm_locals = Vec::new();
    if i32_locals > 0 {
        wasm_locals.push((i32_locals, wasm_encoder::ValType::I32));
    }
    if i64_locals > 0 {
        wasm_locals.push((i64_locals, wasm_encoder::ValType::I64));
    }
    if f32_locals > 0 {
        wasm_locals.push((f32_locals, wasm_encoder::ValType::F32));
    }
    if f64_locals > 0 {
        wasm_locals.push((f64_locals, wasm_encoder::ValType::F64));
    }

    // Create WASM function with locals
    let mut func = Function::new(wasm_locals);

    // Process each block in order they appear in the function
    for block in function.layout.blocks() {
        // Process each instruction in the block
        for inst in function.layout.block_insts(block) {
            // Get the opcode for this instruction
            let opcode = function.dfg.insts[inst].opcode();

            match opcode {
                // Integer constants
                Opcode::Iconst => {
                    // Get the immediate value from the instruction data
                    let imm = match &function.dfg.insts[inst] {
                        cranelift_codegen::ir::InstructionData::UnaryImm { imm, .. } => imm.bits(),
                        _ => continue, // Skip if unexpected format
                    };

                    let result_type = function.dfg.ctrl_typevar(inst);
                    let value = match result_type {
                        ir::types::I32 => WasmInst::I32Const(imm as i32),
                        ir::types::I64 => WasmInst::I64Const(imm),
                        _ => continue, // Skip unsupported types
                    };

                    func.instruction(&value);

                    // Store to local if needed
                    if let Some(result) = function.dfg.inst_results(inst).first() {
                        if let Some(&local) = value_map.get(result) {
                            func.instruction(&WasmInst::LocalSet(local));
                        }
                    }
                }

                // Floating point constants
                Opcode::F32const => {
                    // Get the immediate value from the instruction data
                    let bits = match &function.dfg.insts[inst] {
                        cranelift_codegen::ir::InstructionData::UnaryIeee32 { imm, .. } => {
                            imm.bits()
                        }
                        _ => continue, // Skip if unexpected format
                    };

                    let f = f32::from_bits(bits);
                    func.instruction(&WasmInst::F32Const(f));

                    // Store to local
                    if let Some(result) = function.dfg.inst_results(inst).first() {
                        if let Some(&local) = value_map.get(result) {
                            func.instruction(&WasmInst::LocalSet(local));
                        }
                    }
                }

                Opcode::F64const => {
                    // Get the immediate value from the instruction data
                    let bits = match &function.dfg.insts[inst] {
                        cranelift_codegen::ir::InstructionData::UnaryIeee64 { imm, .. } => {
                            imm.bits()
                        }
                        _ => continue, // Skip if unexpected format
                    };

                    let f = f64::from_bits(bits);
                    func.instruction(&WasmInst::F64Const(f));

                    // Store to local
                    if let Some(result) = function.dfg.inst_results(inst).first() {
                        if let Some(&local) = value_map.get(result) {
                            func.instruction(&WasmInst::LocalSet(local));
                        }
                    }
                }

                // Binary operations
                Opcode::Iadd
                | Opcode::Isub
                | Opcode::Imul
                | Opcode::Udiv
                | Opcode::Sdiv
                | Opcode::Urem
                | Opcode::Srem
                | Opcode::Band
                | Opcode::Bor
                | Opcode::Bxor
                | Opcode::Ishl
                | Opcode::Ushr
                | Opcode::Sshr
                | Opcode::Fadd
                | Opcode::Fsub
                | Opcode::Fmul
                | Opcode::Fdiv
                | Opcode::Fmin
                | Opcode::Fmax => {
                    let args = function.dfg.inst_args(inst);

                    // Skip if we don't have exactly two arguments
                    if args.len() != 2 {
                        continue;
                    }

                    // Load operands from locals if needed and handle type conversions
                    let first_type = function.dfg.value_type(args[0]);
                    let second_type = function.dfg.value_type(args[1]);

                    // We need to ensure both operands are of the same type
                    let target_type = match (first_type, second_type) {
                        // If both are the same, no conversion needed
                        (t1, t2) if t1 == t2 => t1,
                        // If one is i64 and the other is i32, convert i32 to i64
                        (ir::types::I64, ir::types::I32) | (ir::types::I32, ir::types::I64) => {
                            ir::types::I64
                        }
                        // Use the first type as default
                        _ => first_type,
                    };

                    // Load and convert first operand if needed
                    if let Some(&local) = value_map.get(&args[0]) {
                        func.instruction(&WasmInst::LocalGet(local));
                        if needs_type_conversion(first_type, target_type) {
                            insert_type_conversion(&mut func, first_type, target_type);
                        }
                    }

                    // Load and convert second operand if needed
                    if let Some(&local) = value_map.get(&args[1]) {
                        func.instruction(&WasmInst::LocalGet(local));
                        if needs_type_conversion(second_type, target_type) {
                            insert_type_conversion(&mut func, second_type, target_type);
                        }
                    }

                    // Translate the binary operation
                    if let Some(result) = function.dfg.inst_results(inst).first() {
                        let result_type = function.dfg.value_type(*result);

                        let wasm_inst = match (opcode, result_type) {
                            (Opcode::Iadd, ir::types::I32) => Some(WasmInst::I32Add),
                            (Opcode::Iadd, ir::types::I64) => Some(WasmInst::I64Add),
                            (Opcode::Isub, ir::types::I32) => Some(WasmInst::I32Sub),
                            (Opcode::Isub, ir::types::I64) => Some(WasmInst::I64Sub),
                            (Opcode::Imul, ir::types::I32) => Some(WasmInst::I32Mul),
                            (Opcode::Imul, ir::types::I64) => Some(WasmInst::I64Mul),
                            (Opcode::Udiv, ir::types::I32) => Some(WasmInst::I32DivU),
                            (Opcode::Udiv, ir::types::I64) => Some(WasmInst::I64DivU),
                            (Opcode::Sdiv, ir::types::I32) => Some(WasmInst::I32DivS),
                            (Opcode::Sdiv, ir::types::I64) => Some(WasmInst::I64DivS),
                            (Opcode::Urem, ir::types::I32) => Some(WasmInst::I32RemU),
                            (Opcode::Urem, ir::types::I64) => Some(WasmInst::I64RemU),
                            (Opcode::Srem, ir::types::I32) => Some(WasmInst::I32RemS),
                            (Opcode::Srem, ir::types::I64) => Some(WasmInst::I64RemS),
                            (Opcode::Band, ir::types::I32) => Some(WasmInst::I32And),
                            (Opcode::Band, ir::types::I64) => Some(WasmInst::I64And),
                            (Opcode::Bor, ir::types::I32) => Some(WasmInst::I32Or),
                            (Opcode::Bor, ir::types::I64) => Some(WasmInst::I64Or),
                            (Opcode::Bxor, ir::types::I32) => Some(WasmInst::I32Xor),
                            (Opcode::Bxor, ir::types::I64) => Some(WasmInst::I64Xor),
                            (Opcode::Ishl, ir::types::I32) => Some(WasmInst::I32Shl),
                            (Opcode::Ishl, ir::types::I64) => Some(WasmInst::I64Shl),
                            (Opcode::Ushr, ir::types::I32) => Some(WasmInst::I32ShrU),
                            (Opcode::Ushr, ir::types::I64) => Some(WasmInst::I64ShrU),
                            (Opcode::Sshr, ir::types::I32) => Some(WasmInst::I32ShrS),
                            (Opcode::Sshr, ir::types::I64) => Some(WasmInst::I64ShrS),
                            (Opcode::Fadd, ir::types::F32) => Some(WasmInst::F32Add),
                            (Opcode::Fadd, ir::types::F64) => Some(WasmInst::F64Add),
                            (Opcode::Fsub, ir::types::F32) => Some(WasmInst::F32Sub),
                            (Opcode::Fsub, ir::types::F64) => Some(WasmInst::F64Sub),
                            (Opcode::Fmul, ir::types::F32) => Some(WasmInst::F32Mul),
                            (Opcode::Fmul, ir::types::F64) => Some(WasmInst::F64Mul),
                            (Opcode::Fdiv, ir::types::F32) => Some(WasmInst::F32Div),
                            (Opcode::Fdiv, ir::types::F64) => Some(WasmInst::F64Div),
                            (Opcode::Fmin, ir::types::F32) => Some(WasmInst::F32Min),
                            (Opcode::Fmin, ir::types::F64) => Some(WasmInst::F64Min),
                            (Opcode::Fmax, ir::types::F32) => Some(WasmInst::F32Max),
                            (Opcode::Fmax, ir::types::F64) => Some(WasmInst::F64Max),
                            _ => None,
                        };

                        if let Some(binary_inst) = wasm_inst {
                            func.instruction(&binary_inst);

                            // Store result to local
                            if let Some(&local) = value_map.get(result) {
                                func.instruction(&WasmInst::LocalSet(local));
                            }
                        }
                    }
                }

                // Return instruction
                Opcode::Return => {
                    let args = function.dfg.inst_args(inst);

                    // If there are return values, load them from locals
                    if !args.is_empty() {
                        if let Some(&local) = value_map.get(&args[0]) {
                            func.instruction(&WasmInst::LocalGet(local));

                            // Check if we need to convert the type to match function return type
                            let arg_type = function.dfg.value_type(args[0]);
                            if let Some(ret_type) = function.signature.returns.first() {
                                if needs_type_conversion(arg_type, ret_type.value_type) {
                                    insert_type_conversion(
                                        &mut func,
                                        arg_type,
                                        ret_type.value_type,
                                    );
                                }
                            }
                        }
                    }

                    // Add return instruction
                    func.instruction(&WasmInst::Return);
                }

                // Branch and jump instructions - simplified for now
                // Actual implementation would require proper block handling

                // Function calls - simplified for now
                // Actual implementation would require proper function resolving

                // For any other instruction, we simply skip for now
                _ => {}
            }
        }
    }

    // If the function doesn't have a return instruction, add a default return value
    // based on the expected return type from the signature
    if let Some(result) = function.signature.returns.first() {
        match result.value_type {
            ir::types::I32 => {
                func.instruction(&WasmInst::I32Const(0));
            }
            ir::types::I64 => {
                func.instruction(&WasmInst::I64Const(0));
            }
            ir::types::F32 => {
                func.instruction(&WasmInst::F32Const(0.0));
            }
            ir::types::F64 => {
                func.instruction(&WasmInst::F64Const(0.0));
            }
            _ => {
                func.instruction(&WasmInst::Unreachable);
            }
        }
    }

    // End the function
    func.instruction(&WasmInst::End);

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

            // Check if the error might be related to type mismatch
            let error_message = format!("{:?}", err);
            if error_message.contains("type mismatch")
                || error_message.contains("i32")
                || error_message.contains("i64")
            {
                eprintln!(
                    "\nPossible type mismatch between i32 and i64. This might indicate a need for type conversion."
                );
                eprintln!(
                    "Check that all function parameters and return values match the expected types."
                );
                eprintln!("Common issues:");
                eprintln!("  - Memory access functions expect i32 offsets");
                eprintln!("  - Tagged values are i64 but many WASM functions expect i32");
                eprintln!("  - Return values from functions may need explicit type conversion");

                // Attempt to find the exact location of the problem
                if let Some(pos) = error_message.find("offset 0x") {
                    let offset_str = &error_message[pos + 9..];
                    let end = offset_str.find(')').unwrap_or(offset_str.len());
                    let hex = &offset_str[..end];
                    if let Ok(err_offset) = u32::from_str_radix(hex, 16) {
                        eprintln!(
                            "\nTrying to identify function at offset 0x{:x}:",
                            err_offset
                        );

                        // Find the function containing this offset
                        let parser = wasmparser::Parser::new(0);
                        for payload in parser.parse_all(wasm_bytes) {
                            if let Ok(wasmparser::Payload::CodeSectionStart {
                                count, range, ..
                            }) = payload
                            {
                                eprintln!(
                                    "  Code section starts at 0x{:x}, error at 0x{:x}",
                                    range.start, err_offset
                                );

                                // Print info about where the error is relative to the code section
                                let range_start = range.start as u32;
                                let range_end = range.end as u32;
                                if range_start <= err_offset && err_offset < range_end {
                                    eprintln!(
                                        "  Error is in code section, offset 0x{:x} from start",
                                        err_offset - range_start
                                    );

                                    // Print bytes around error
                                    let start_idx = if err_offset >= 10 {
                                        (err_offset - 10) as usize
                                    } else {
                                        0
                                    };
                                    let end_idx =
                                        std::cmp::min((err_offset + 10) as usize, wasm_bytes.len());

                                    eprintln!("  Bytes around error:");
                                    for (i, byte) in
                                        wasm_bytes[start_idx..end_idx].iter().enumerate()
                                    {
                                        let pos = start_idx + i;
                                        let indicator = if pos as u32 == err_offset {
                                            " <-- ERROR"
                                        } else {
                                            ""
                                        };
                                        eprintln!("    0x{:04x}: 0x{:02x}{}", pos, byte, indicator);
                                    }
                                }
                            }
                        }
                    }
                }
            }

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

                        // Print more details about function signatures
                        let types_reader = reader.clone().into_iter();
                        for (i, ty_result) in types_reader.enumerate() {
                            if let Ok(ty) = ty_result {
                                eprintln!("  Function type #{}: {:?}", i, ty);
                            }
                        }
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

                        // Print details about exports
                        let exports = reader.clone().into_iter();
                        for (i, export_result) in exports.enumerate() {
                            if let Ok(export) = export_result {
                                eprintln!(
                                    "  Export #{}: name=\"{}\", kind={:?}, index={}",
                                    i, export.name, export.kind, export.index
                                );
                            }
                        }
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
