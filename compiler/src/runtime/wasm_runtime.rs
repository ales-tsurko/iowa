//! WebAssembly runtime for Io language.
//!
//! This module provides a WebAssembly-based execution model for Io methods.
//! It compiles Io AST to WebAssembly and executes WASM modules using Wasmer.

use crate::runtime::runtime::{Runtime, Value, MethodData};
use wasmer::{
    Instance, Module, Store, Value as WasmerValue, Memory,
    FunctionEnvMut, AsStoreMut, StoreMut, FunctionEnv
};
use wasmer_compiler_cranelift::Cranelift;
use wasm_encoder::{
    ExportKind, Function as WasmFunction, FunctionSection, ImportSection,
    Module as WasmModule, TypeSection, ValType, CodeSection, Instruction as WasmInstruction,
    MemorySection
};
use std::collections::HashMap;

/// Helper for managing WebAssembly linear memory
struct WasmMemoryManager<'a> {
    memory: &'a Memory,
    store: StoreMut<'a>,
    offset: u32,
    string_cache: HashMap<String, u32>,
}

impl<'a> WasmMemoryManager<'a> {
    fn new(memory: &'a Memory, store: StoreMut<'a>) -> Self {
        Self {
            memory,
            store,
            offset: 0,
            string_cache: HashMap::new(),
        }
    }
    
    fn alloc(&mut self, size: u32) -> u32 {
        let ptr = self.offset;
        self.offset = (self.offset + size + 7) & !7; // Align to 8-byte boundary
        ptr
    }
    
    fn write_string(&mut self, s: &str) -> (u32, u32) {
        if let Some(&ptr) = self.string_cache.get(s) {
            return (ptr, s.len() as u32);
        }
        
        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        let ptr = self.alloc(len);
        
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            for (i, byte) in bytes.iter().enumerate() {
                let offset = (ptr + i as u32) as usize;
                if offset < memory_view.data_size() as usize {
                    memory_view.data_unchecked_mut()[offset] = *byte;
                }
            }
        }
        
        self.string_cache.insert(s.to_string(), ptr);
        (ptr, len)
    }
    
    fn write_array(&mut self, values: &[u64]) -> (u32, u32) {
        let len = values.len() as u32;
        let byte_len = len * 8;
        let ptr = self.alloc(byte_len);
        
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            for (i, value) in values.iter().enumerate() {
                let offset = (ptr + (i as u32 * 8)) as usize;
                if offset + 8 <= memory_view.data_size() as usize {
                    let bytes = value.to_le_bytes();
                    for (j, byte) in bytes.iter().enumerate() {
                        memory_view.data_unchecked_mut()[offset + j] = *byte;
                    }
                }
            }
        }
        
        (ptr, len)
    }
    
    fn read_array(&mut self, ptr: u32, len: u32) -> Vec<u64> {
        let mut values = Vec::with_capacity(len as usize);
        
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            for i in 0..len {
                let offset = (ptr + (i * 8)) as usize;
                if offset + 8 <= memory_view.data_size() as usize {
                    let mut bytes = [0u8; 8];
                    for j in 0..8 {
                        bytes[j] = memory_view.data_unchecked()[offset + j];
                    }
                    values.push(u64::from_le_bytes(bytes));
                }
            }
        }
        
        values
    }
    
    fn read_string(&mut self, ptr: u32, len: u32) -> String {
        let mut bytes = Vec::with_capacity(len as usize);
        
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            for i in 0..len {
                let offset = (ptr + i) as usize;
                if offset < memory_view.data_size() as usize {
                    bytes.push(memory_view.data_unchecked()[offset]);
                }
            }
        }
        
        String::from_utf8(bytes).unwrap_or_else(|_| String::new())
    }
}

#[derive(Clone)]
pub struct WasmMethod {
    module_bytes: Vec<u8>,
    method_info: MethodInfo,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub args: Vec<String>,
    pub function_index: u32,
    pub needs_receiver: bool,
}

impl WasmMethod {
    pub fn from_ast(
        chain: &iowa_parser::MessageChain, 
        method_data: &MethodData,
        runtime: &Runtime
    ) -> Self {
        let (module_bytes, function_index) = generate_wasm_from_ast(chain, method_data, runtime);
        
        let method_info = MethodInfo {
            args: method_data.args.clone(),
            function_index,
            needs_receiver: true,
        };
        
        Self {
            module_bytes,
            method_info,
        }
    }
    
    pub fn from_module(
        module_bytes: Vec<u8>,
        args: Vec<String>,
        function_index: u32,
        needs_receiver: bool,
    ) -> Self {
        let method_info = MethodInfo {
            args,
            function_index,
            needs_receiver,
        };
        
        Self {
            module_bytes,
            method_info,
        }
    }
    
    pub fn module_bytes(&self) -> &Vec<u8> {
        &self.module_bytes
    }
    
    pub fn execute(
        &self, 
        runtime: &mut Runtime, 
        args: &[Value], 
        receiver: Value
    ) -> Value {
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);
        
        let module = Module::new(&store, &self.module_bytes)
            .expect("Failed to compile WASM module");
            
        let imports = create_runtime_imports(&mut store, runtime);
        
        let instance = Instance::new(&mut store, &module, &imports)
            .expect("Failed to instantiate WASM module");
            
        let function = instance.exports.get_function("main")
            .expect("Failed to get exported function");
            
        if let Ok(memory) = instance.exports.get_memory("memory") {
            let store_mut = store.as_store_mut();
            let mut memory_manager = WasmMemoryManager::new(memory, store_mut);
            
            let _ = memory_manager.write_string("self");
            let _ = memory_manager.write_string("prototype");
            let _ = memory_manager.write_string("clone");
            let _ = memory_manager.write_string("return");
            let _ = memory_manager.write_string("if");
            let _ = memory_manager.write_string("while");
        }
            
        let mut wasm_args = Vec::new();
        
        if self.method_info.needs_receiver {
            let tag_val: i64 = runtime.to_tagged_value(&receiver).try_into().unwrap();
            wasm_args.push(WasmerValue::I64(tag_val));
        }
        
        let expected_arg_count = self.method_info.args.len();
        let args_to_add = std::cmp::min(expected_arg_count, args.len());
        
        for arg in args.iter().take(args_to_add) {
            let arg_val: i64 = runtime.to_tagged_value(arg).try_into().unwrap();
            wasm_args.push(WasmerValue::I64(arg_val));
        }
        
        let result = match function.call(&mut store, &wasm_args) {
            Ok(res) => res,
            Err(_) => return Value::Nil,
        };
            
        if let Some(WasmerValue::I64(raw_value)) = result.get(0) {
            let u_val: u64 = (*raw_value).try_into().unwrap();
            runtime.from_tagged_value(u_val)
        } else {
            Value::Nil
        }
    }
}

fn create_runtime_imports(
    store: &mut Store,
    runtime: &mut Runtime,
) -> wasmer::Imports {
    let mut imports = wasmer::imports! {};
    
    let runtime_ptr = runtime as *mut Runtime as i64;
    let env = FunctionEnv::new(store, ());
    
    let memory_type = wasmer::MemoryType::new(1, None, false);
    let memory = Memory::new(store, memory_type).expect("Failed to create memory");
    let memory_ptr = &memory as *const Memory as i64;
    
    imports.define("env", "memory", memory.clone());
    
    // Object allocation functions
    let alloc_object_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, prototype_id: i64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let tagged_prototype = prototype_id as u64;
            let prototype_value = runtime.from_tagged_value(tagged_prototype);
            let prototype_id = if let Value::Object(id) = prototype_value {
                id
            } else {
                runtime.prototypes().object
            };
            
            let obj_id = runtime.alloc_object(prototype_id);
            runtime.to_tagged_value(&Value::Object(obj_id)).try_into().unwrap()
        }
    );
    
    let alloc_string_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, ptr: i32, len: i32| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            let string_value = memory_manager.read_string(ptr as u32, len as u32);
            let obj_id = runtime.alloc_string(&string_value);
            
            runtime.to_tagged_value(&Value::Object(obj_id)).try_into().unwrap()
        }
    );
    
    let alloc_number_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value: f64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let obj_id = runtime.alloc_number(value);
            runtime.to_tagged_value(&Value::Object(obj_id)).try_into().unwrap()
        }
    );
    
    // Object slot access functions
    let set_slot_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, obj_id: i64, slot_ptr: i32, slot_len: i32, value_id: i64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            
            let obj_value = runtime.from_tagged_value(obj_id as u64);
            let slot_name = memory_manager.read_string(slot_ptr as u32, slot_len as u32);
            let value = runtime.from_tagged_value(value_id as u64);
            
            if let Value::Object(id) = obj_value {
                if let Some(obj) = runtime.memory_mut().get_object_mut(id) {
                    obj.set_slot(slot_name, value);
                    return 1;
                }
            }
            
            0
        }
    );
    
    let get_slot_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, obj_id: i64, slot_ptr: i32, slot_len: i32| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            
            let obj_value = runtime.from_tagged_value(obj_id as u64);
            let slot_name = memory_manager.read_string(slot_ptr as u32, slot_len as u32);
            
            if let Value::Object(id) = obj_value {
                if let Some(obj) = runtime.memory().get_object(id) {
                    if let Some(value) = obj.get_slot(&slot_name) {
                        return runtime.to_tagged_value(value).try_into().unwrap();
                    }
                }
            }
            
            runtime.to_tagged_value(&Value::Nil) as i64
        }
    );
    
    let lookup_slot_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, obj_id: i64, slot_ptr: i32, slot_len: i32| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            
            let obj_value = runtime.from_tagged_value(obj_id as u64);
            let slot_name = memory_manager.read_string(slot_ptr as u32, slot_len as u32);
            
            if let Value::Object(id) = obj_value {
                if let Some(obj) = runtime.memory().get_object(id) {
                    let value = obj.lookup_slot(&slot_name, runtime);
                    return runtime.to_tagged_value(&value).try_into().unwrap();
                }
            }
            
            runtime.to_tagged_value(&Value::Nil) as i64
        }
    );
    
    // Message dispatch
    let dispatch_message_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, receiver_id: i64, message_ptr: i32, message_len: i32, args_ptr: i32, args_len: i32| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            
            let receiver = runtime.from_tagged_value(receiver_id as u64);
            let message_name = memory_manager.read_string(message_ptr as u32, message_len as u32);
            let arg_tags = memory_manager.read_array(args_ptr as u32, args_len as u32);
            
            let args: Vec<Value> = arg_tags
                .into_iter()
                .map(|tag| runtime.from_tagged_value(tag))
                .collect();
            
            let result = runtime.dispatch_message(receiver, &message_name, args);
            runtime.to_tagged_value(&result).try_into().unwrap()
        }
    );
    
    // Value conversion functions
    let string_to_number_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, string_id: i64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let string_value = runtime.from_tagged_value(string_id as u64);
            
            if let Value::String(s) = string_value {
                if let Ok(num) = s.parse::<f64>() {
                    return runtime.to_tagged_value(&Value::Number(num)).try_into().unwrap();
                }
            }
            
            runtime.to_tagged_value(&Value::Nil) as i64
        }
    );
    
    let value_to_string_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            let string_value = match &value {
                Value::Nil => "nil".to_string(),
                Value::Boolean(b) => if *b { "true".to_string() } else { "false".to_string() },
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Object(_) => "[object]".to_string(),
            };
            
            let string_obj_id = runtime.alloc_string(&string_value);
            runtime.to_tagged_value(&Value::Object(string_obj_id)).try_into().unwrap()
        }
    );
    
    // Memory management functions
    let store_string_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>, string_id: i64, out_len_ptr: i32| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let memory = unsafe { &*(memory_ptr as *const Memory) };
            let mut memory_manager = WasmMemoryManager::new(memory, ctx.as_store_mut());
            
            let string_value = runtime.from_tagged_value(string_id as u64);
            
            let content = match string_value {
                Value::String(s) => s,
                _ => "".to_string(),
            };
            
            let (ptr, len) = memory_manager.write_string(&content);
            
            if out_len_ptr > 0 {
                unsafe {
                    let mut store_mut = ctx.as_store_mut();
                    let memory_view = memory.view(&mut store_mut);
                    if (out_len_ptr as usize + 4) <= memory_view.data_size() as usize {
                        let len_bytes = len.to_le_bytes();
                        for (i, byte) in len_bytes.iter().enumerate() {
                            memory_view.data_unchecked_mut()[(out_len_ptr as usize) + i] = *byte;
                        }
                    }
                }
            }
            
            ptr as i32
        }
    );
    
    // Type checking functions
    let is_nil_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            match value {
                Value::Nil => 1,
                _ => 0,
            }
        }
    );
    
    let is_boolean_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            match value {
                Value::Boolean(_) => 1,
                _ => 0,
            }
        }
    );
    
    let is_number_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            match value {
                Value::Number(_) => 1,
                _ => 0,
            }
        }
    );
    
    let is_string_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            match value {
                Value::String(_) => 1,
                _ => 0,
            }
        }
    );
    
    let is_object_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i32 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            
            let value = runtime.from_tagged_value(value_id as u64);
            
            match value {
                Value::Object(_) => 1,
                _ => 0,
            }
        }
    );
    
    // Register all the functions
    imports.define("io", "alloc_object", alloc_object_fn);
    imports.define("io", "alloc_string", alloc_string_fn);
    imports.define("io", "alloc_number", alloc_number_fn);
    imports.define("io", "set_slot", set_slot_fn);
    imports.define("io", "get_slot", get_slot_fn);
    imports.define("io", "lookup_slot", lookup_slot_fn);
    imports.define("io", "dispatch_message", dispatch_message_fn);
    imports.define("io", "string_to_number", string_to_number_fn);
    imports.define("io", "value_to_string", value_to_string_fn);
    imports.define("io", "store_string", store_string_fn);
    imports.define("io", "is_nil", is_nil_fn);
    imports.define("io", "is_boolean", is_boolean_fn);
    imports.define("io", "is_number", is_number_fn);
    imports.define("io", "is_string", is_string_fn);
    imports.define("io", "is_object", is_object_fn);
    
    imports
}

fn generate_wasm_from_ast(
    chain: &iowa_parser::MessageChain,
    method_data: &MethodData,
    runtime: &Runtime,
) -> (Vec<u8>, u32) {
    let mut module = WasmModule::new();
    
    let mut types = TypeSection::new();
    
    let mut main_params = vec![ValType::I64]; // receiver 
    
    if !method_data.args.is_empty() {
        main_params.extend(std::iter::repeat(ValType::I64).take(method_data.args.len()));
    }
    let results = vec![ValType::I64]; // return value
    
    let alloc_object_params = vec![ValType::I64];
    let alloc_string_params = vec![ValType::I32, ValType::I32];
    let alloc_number_params = vec![ValType::F64];
    let set_slot_params = vec![ValType::I64, ValType::I32, ValType::I32, ValType::I64];
    let get_slot_params = vec![ValType::I64, ValType::I32, ValType::I32];
    let lookup_slot_params = vec![ValType::I64, ValType::I32, ValType::I32];
    let dispatch_message_params = vec![ValType::I64, ValType::I32, ValType::I32, ValType::I32, ValType::I32];
    let string_to_number_params = vec![ValType::I64];
    let value_to_string_params = vec![ValType::I64];
    let is_type_params = vec![ValType::I64];
    let is_type_results = vec![ValType::I32];
    
    let function_types = [
        (main_params, results.clone()),
        (alloc_object_params, results.clone()),
        (alloc_string_params, results.clone()),
        (alloc_number_params, results.clone()),
        (set_slot_params, results.clone()),
        (get_slot_params, results.clone()),
        (lookup_slot_params, results.clone()),
        (dispatch_message_params, results.clone()),
        (string_to_number_params, results.clone()),
        (value_to_string_params, results.clone()),
        (is_type_params.clone(), is_type_results.clone()),
        (is_type_params.clone(), is_type_results.clone()),
        (is_type_params.clone(), is_type_results.clone()),
        (is_type_params.clone(), is_type_results.clone()),
        (is_type_params.clone(), is_type_results.clone()),
    ];
    
    for (params, results) in function_types.iter() {
        types.ty().function(params.clone(), results.clone());
    }
    
    module.section(&types);
    
    let mut imports = ImportSection::new();
    
    imports.import("io", "alloc_object", wasm_encoder::EntityType::Function(1));
    imports.import("io", "alloc_string", wasm_encoder::EntityType::Function(2));
    imports.import("io", "alloc_number", wasm_encoder::EntityType::Function(3));
    imports.import("io", "set_slot", wasm_encoder::EntityType::Function(4));
    imports.import("io", "get_slot", wasm_encoder::EntityType::Function(5));
    imports.import("io", "lookup_slot", wasm_encoder::EntityType::Function(6));
    imports.import("io", "dispatch_message", wasm_encoder::EntityType::Function(7));
    imports.import("io", "string_to_number", wasm_encoder::EntityType::Function(8));
    imports.import("io", "value_to_string", wasm_encoder::EntityType::Function(9));
    imports.import("io", "is_nil", wasm_encoder::EntityType::Function(10));
    imports.import("io", "is_boolean", wasm_encoder::EntityType::Function(11));
    imports.import("io", "is_number", wasm_encoder::EntityType::Function(12));
    imports.import("io", "is_string", wasm_encoder::EntityType::Function(13));
    imports.import("io", "is_object", wasm_encoder::EntityType::Function(14));
    
    module.section(&imports);
    
    let mut functions = FunctionSection::new();
    
    let import_function_count = 15;
    let function_index = 0;
    
    functions.function(0);
    
    module.section(&functions);
    
    let mut memories = MemorySection::new();
    
    memories.memory(wasm_encoder::MemoryType {
        minimum: 1,
        maximum: None, 
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    
    module.section(&memories);
    
    let mut exports = wasm_encoder::ExportSection::new();
    
    exports.export(
        "main",
        ExportKind::Func,
        0,
    );
    
    exports.export(
        "memory",
        ExportKind::Memory,
        0,
    );
    
    module.section(&exports);
    
    let mut code = CodeSection::new();
    
    let wasm_func = compile_message_chain(chain, method_data, runtime);
    
    code.function(&wasm_func);
    
    module.section(&code);
    
    let actual_function_index = import_function_count + function_index;
    
    (module.finish(), actual_function_index)
}

fn compile_message_chain(
    chain: &iowa_parser::MessageChain,
    _method_data: &MethodData,
    _runtime: &Runtime,
) -> WasmFunction {
    let mut func = WasmFunction::new([]);
    
    if let Some(message) = chain.messages.first() {
        match &message.symbol {
            iowa_parser::Symbol::Quote(_) => {
                func.instruction(&WasmInstruction::I64Const(42));
                func.instruction(&WasmInstruction::End);
            },
            iowa_parser::Symbol::Number(_) => {
                func.instruction(&WasmInstruction::I64Const(42));
                func.instruction(&WasmInstruction::End);
            },
            _ => {
                func.instruction(&WasmInstruction::I64Const(42));
                func.instruction(&WasmInstruction::End);
            }
        }
    } else {
        func.instruction(&WasmInstruction::I64Const(42));
        func.instruction(&WasmInstruction::End);
    }
    
    func
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::runtime::Runtime;
    use crate::runtime::runtime::Value;
    
    #[test]
    fn test_wasm_generation() {
        let runtime = Runtime::new();
        let input = "a := 10";
        let (_, chains) = iowa_parser::parse(input).unwrap();
        let chain = chains.first().expect("Failed to parse test input");
        let method_data = MethodData {
            args: vec![],
            body: crate::runtime::runtime::MethodBody::Source("".to_string()),
        };
        
        let (module_bytes, _) = generate_wasm_from_ast(&chain, &method_data, &runtime);
        
        assert!(module_bytes.len() > 8);
        assert_eq!(&module_bytes[0..4], b"\0asm"); // WASM magic bytes
        assert_eq!(&module_bytes[4..8], &[1, 0, 0, 0]); // WASM version
    }
    
    #[test]
    #[ignore = "WASM runtime is not fully implemented yet"]
    fn test_runtime_imports() {
        let mut runtime = Runtime::new();
        
        let lobby_id = runtime.lobby();
        
        let input = "obj := Object clone\nobj";
        let (_, chains) = iowa_parser::parse(input).unwrap();
        let chain = chains.first().expect("Failed to parse test input");
        
        let method_data = MethodData {
            args: vec![],
            body: crate::runtime::runtime::MethodBody::Source(input.to_string()),
        };
        
        let method = WasmMethod::from_ast(chain, &method_data, &runtime);
        
        let result = method.execute(&mut runtime, &[], Value::Object(lobby_id));
        
        if let Value::Nil = result {
            panic!("Expected non-nil result from WASM method execution");
        }
    }
    
    #[test]
    #[ignore = "WASM runtime is not fully implemented yet"]
    fn test_runtime_object_functions() {
        let mut runtime = Runtime::new();
        
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);
        
        let imports = create_runtime_imports(&mut store, &mut runtime);
        
        let alloc_object = imports.get_export("io", "alloc_object")
            .expect("Should have alloc_object function");
        
        if let wasmer::Extern::Function(func) = alloc_object {
            let object_proto = runtime.prototypes().object;
            let proto_tag_val: i64 = runtime.to_tagged_value(&Value::Object(object_proto)).try_into().unwrap();
            
            let result = func.call(&mut store, &[WasmerValue::I64(proto_tag_val)])
                .expect("Function call should succeed");
            
            if let Some(WasmerValue::I64(obj_id)) = result.first() {
                let u_val: u64 = (*obj_id).try_into().unwrap();
                let value = runtime.from_tagged_value(u_val);
                
                match value {
                    Value::Object(_) => { /* Test passed */ },
                    _ => panic!("Expected Object value, got {:?}", value),
                }
            } else {
                panic!("Expected I64 result from alloc_object");
            }
        } else {
            panic!("Expected function for alloc_object");
        }
    }
}

#[cfg(test)]
    
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
    
    #[test]
    #[ignore = "WASM runtime is not fully implemented yet"]
    fn test_runtime_imports() {
        // Test the runtime imports by creating a WASM method and executing it
        let mut runtime = Runtime::new();
        
        // Get the lobby ID before any mutable borrows
        let lobby_id = runtime.lobby();
        
        // Create a simple message chain for testing
        let input = "obj := Object clone\nobj";
        let (_, chains) = iowa_parser::parse(input).unwrap();
        let chain = chains.first().expect("Failed to parse test input");
        
        let method_data = MethodData {
            args: vec![],
            body: crate::runtime::runtime::MethodBody::Source(input.to_string()),
        };
        
        // Create a WASM method
        let method = WasmMethod::from_ast(chain, &method_data, &runtime);
        
        // Execute it with a self object (the lobby)
        let result = method.execute(&mut runtime, &[], Value::Object(lobby_id));
        
        // Result should be an object (not Value::Nil)
        if let Value::Nil = result {
            panic!("Expected non-nil result from WASM method execution");
        }
    }
    
    #[test]
    #[ignore = "WASM runtime is not fully implemented yet"]
    fn test_runtime_object_functions() {
        // Test specifically the object allocation and slot access functions
        let mut runtime = Runtime::new();
        
        // Create a compiler
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);
        
        // Create runtime imports
        let imports = create_runtime_imports(&mut store, &mut runtime);
        
        // Get the alloc_object function
        let alloc_object = imports.get_export("io", "alloc_object")
            .expect("Should have alloc_object function");
        
        if let wasmer::Extern::Function(func) = alloc_object {
            // Get the Object prototype as a tagged value
            let object_proto = runtime.prototypes().object;
            let proto_tag_val: i64 = runtime.to_tagged_value(&Value::Object(object_proto)).try_into().unwrap();
            
            // Call the function to allocate a new object
            let result = func.call(&mut store, &[WasmerValue::I64(proto_tag_val)])
                .expect("Function call should succeed");
            
            // Check that we got a valid object back
            if let Some(WasmerValue::I64(obj_id)) = result.first() {
                let u_val: u64 = (*obj_id).try_into().unwrap();
                let value = runtime.from_tagged_value(u_val);
                
                // Should be an object
                match value {
                    Value::Object(_) => { /* Test passed */ },
                    _ => panic!("Expected Object value, got {:?}", value),
                }
            } else {
                panic!("Expected I64 result from alloc_object");
            }
        } else {
            panic!("Expected function for alloc_object");
        }
    }
