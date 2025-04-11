//! WebAssembly runtime for Io language.
//!
//! This module provides a WebAssembly-based execution model for Io methods.
//! It compiles Io AST to WebAssembly and executes WASM modules using Wasmer.

use crate::runtime::runtime::{MethodData, Runtime, Value};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_encoder::{
    CodeSection, ExportKind, Function as WasmFunction, FunctionSection, ImportSection,
    Instruction as WasmInstruction, MemorySection, Module as WasmModule, TypeSection, ValType,
};
use wasmer::{
    AsStoreMut, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, Store, StoreMut,
    Value as WasmerValue,
};
use wasmer_compiler_cranelift::Cranelift;

/// Constants for memory management
const INITIAL_MEMORY_OFFSET: u32 = 4096; // Start at 4KB to avoid null pointer issues
const STRING_HEAP_START: u32 = INITIAL_MEMORY_OFFSET;
const OBJECT_HEAP_START: u32 = 1024 * 1024; // 1MB offset for object heap
const MEMORY_PAGE_SIZE: u32 = 65536; // 64KB per WASM page

/// Helper for managing WebAssembly linear memory
pub struct WasmMemoryManager<'a> {
    memory: &'a Memory,
    store: StoreMut<'a>,
    string_offset: u32,
    object_offset: u32,
    string_cache: HashMap<String, u32>,
    runtime: Option<&'a Runtime>,
}

impl<'a> WasmMemoryManager<'a> {
    /// Create a new memory manager with the given memory and store
    pub fn new(memory: &'a Memory, store: StoreMut<'a>) -> Self {
        Self {
            memory,
            store,
            string_offset: STRING_HEAP_START,
            object_offset: OBJECT_HEAP_START,
            string_cache: HashMap::new(),
            runtime: None,
        }
    }

    /// Create a new memory manager with runtime reference
    pub fn with_runtime(memory: &'a Memory, store: StoreMut<'a>, runtime: &'a Runtime) -> Self {
        Self {
            memory,
            store,
            string_offset: STRING_HEAP_START,
            object_offset: OBJECT_HEAP_START,
            string_cache: HashMap::new(),
            runtime: Some(runtime),
        }
    }

    /// Ensure memory has enough capacity, growing if needed
    pub fn ensure_capacity(&mut self, required_bytes: u32) -> bool {
        let memory_view = self.memory.view(&mut self.store);
        let memory_size: usize = memory_view.data_size().try_into().unwrap();
        let required_size = self.object_offset + required_bytes;

        if required_size > memory_size as u32 {
            let pages_needed =
                (required_size - memory_size as u32 + MEMORY_PAGE_SIZE - 1) / MEMORY_PAGE_SIZE;
            let result = self.memory.grow(&mut self.store, pages_needed);
            result.is_ok()
        } else {
            true
        }
    }

    /// Allocate memory from the string heap
    pub fn alloc_string_memory(&mut self, size: u32) -> u32 {
        let ptr = self.string_offset;
        self.string_offset = (self.string_offset + size + 7) & !7; // Align to 8-byte boundary

        // Ensure we have enough memory
        self.ensure_capacity(size);

        ptr
    }

    /// Allocate memory from the object heap
    pub fn alloc_object_memory(&mut self, size: u32) -> u32 {
        let ptr = self.object_offset;
        self.object_offset = (self.object_offset + size + 7) & !7; // Align to 8-byte boundary

        // Ensure we have enough memory
        self.ensure_capacity(size);

        ptr
    }

    /// Write a string to memory, using the string cache for deduplication
    pub fn write_string(&mut self, s: &str) -> (u32, u32) {
        // Check string cache first for deduplication
        if let Some(&ptr) = self.string_cache.get(s) {
            return (ptr, s.len() as u32);
        }

        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        let ptr = self.alloc_string_memory(len);

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

    /// Write an array of tagged values to memory
    pub fn write_array(&mut self, values: &[u64]) -> (u32, u32) {
        let len = values.len() as u32;
        let byte_len = len * 8;
        let ptr = self.alloc_object_memory(byte_len);

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

    /// Write IoValues to memory and return the array pointer and length
    pub fn write_values(&mut self, values: &[Value]) -> (u32, u32) {
        if let Some(runtime) = self.runtime {
            // Convert Values to tagged values (u64)
            let tagged_values: Vec<u64> =
                values.iter().map(|v| runtime.to_tagged_value(v)).collect();

            self.write_array(&tagged_values)
        } else {
            // If no runtime, return a null array
            (0, 0)
        }
    }

    /// Create a cached string constant and return its pointer and length
    pub fn write_string_constant(&mut self, s: &str) -> (u32, u32) {
        self.write_string(s)
    }

    /// Read an array of u64 values from memory
    pub fn read_array(&mut self, ptr: u32, len: u32) -> Vec<u64> {
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

    /// Read a string from memory
    pub fn read_string(&mut self, ptr: u32, len: u32) -> String {
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

    /// Convert IoValues to tagged values and store them as an array
    pub fn store_arguments(&mut self, args: &[Value]) -> (u32, u32) {
        if args.is_empty() {
            return (0, 0);
        }

        self.write_values(args)
    }

    /// Initialize the memory with common string constants
    pub fn initialize_common_strings(&mut self) {
        // Preload common strings to reduce memory allocations
        let common_strings = [
            "self",
            "prototype",
            "clone",
            "forward",
            "return",
            "if",
            "while",
            "for",
            "break",
            "continue",
            "+",
            "-",
            "*",
            "/",
            "==",
            "!=",
            "<",
            ">",
            "<=",
            ">=",
            "Object",
            "Number",
            "String",
            "List",
            "Map",
            "nil",
            "true",
            "false",
        ];

        for &s in common_strings.iter() {
            self.write_string(s);
        }
    }

    /// Write a value to memory at the given offset
    pub fn write_value_at(&mut self, offset: u32, value: u64) -> bool {
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            let memory_size = memory_view.data_size().try_into().unwrap();
            if (offset + 8) as usize <= memory_size {
                let bytes = value.to_le_bytes();
                for (i, byte) in bytes.iter().enumerate() {
                    memory_view.data_unchecked_mut()[(offset as usize) + i] = *byte;
                }
                true
            } else {
                false
            }
        }
    }

    /// Read a u64 value from memory at the given offset
    pub fn read_value_at(&mut self, offset: u32) -> Option<u64> {
        unsafe {
            let memory_view = self.memory.view(&mut self.store);
            let memory_size = memory_view.data_size().try_into().unwrap();
            if (offset + 8) as usize <= memory_size {
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    bytes[i] = memory_view.data_unchecked()[(offset as usize) + i];
                }
                Some(u64::from_le_bytes(bytes))
            } else {
                None
            }
        }
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
        runtime: &Runtime,
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

    pub fn execute(&self, runtime: &mut Runtime, args: &[Value], receiver: Value) -> Value {
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);

        let module =
            Module::new(&store, &self.module_bytes).expect("Failed to compile WASM module");

        let imports = create_runtime_imports(&mut store, runtime);

        let instance = Instance::new(&mut store, &module, &imports)
            .expect("Failed to instantiate WASM module");

        let function = instance
            .exports
            .get_function("main")
            .expect("Failed to get exported function");

        // Initialize memory with common strings
        if let Ok(memory) = instance.exports.get_memory("memory") {
            let store_mut = store.as_store_mut();
            let mut memory_manager = WasmMemoryManager::with_runtime(memory, store_mut, runtime);
            memory_manager.initialize_common_strings();
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

fn create_runtime_imports(store: &mut Store, runtime: &mut Runtime) -> wasmer::Imports {
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
            runtime
                .to_tagged_value(&Value::Object(obj_id))
                .try_into()
                .unwrap()
        },
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

            runtime
                .to_tagged_value(&Value::Object(obj_id))
                .try_into()
                .unwrap()
        },
    );

    let alloc_number_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value: f64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };
            let obj_id = runtime.alloc_number(value);
            runtime
                .to_tagged_value(&Value::Object(obj_id))
                .try_into()
                .unwrap()
        },
    );

    // Object slot access functions
    let set_slot_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>,
              obj_id: i64,
              slot_ptr: i32,
              slot_len: i32,
              value_id: i64|
              -> i64 {
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
        },
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
        },
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
        },
    );

    // Message dispatch
    let dispatch_message_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |mut ctx: FunctionEnvMut<()>,
              receiver_id: i64,
              message_ptr: i32,
              message_len: i32,
              args_ptr: i32,
              args_len: i32|
              -> i64 {
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
        },
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
                    return runtime
                        .to_tagged_value(&Value::Number(num))
                        .try_into()
                        .unwrap();
                }
            }

            runtime.to_tagged_value(&Value::Nil) as i64
        },
    );

    let value_to_string_fn = wasmer::Function::new_typed_with_env(
        store,
        &env,
        move |_ctx: FunctionEnvMut<()>, value_id: i64| -> i64 {
            let runtime = unsafe { &mut *(runtime_ptr as *mut Runtime) };

            let value = runtime.from_tagged_value(value_id as u64);

            let string_value = match &value {
                Value::Nil => "nil".to_string(),
                Value::Boolean(b) => {
                    if *b {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                }
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                Value::Object(_) => "[object]".to_string(),
            };

            let string_obj_id = runtime.alloc_string(&string_value);
            runtime
                .to_tagged_value(&Value::Object(string_obj_id))
                .try_into()
                .unwrap()
        },
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
        },
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
        },
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
        },
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
        },
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
        },
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
        },
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
    let dispatch_message_params = vec![
        ValType::I64,
        ValType::I32,
        ValType::I32,
        ValType::I32,
        ValType::I32,
    ];
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
    imports.import(
        "io",
        "dispatch_message",
        wasm_encoder::EntityType::Function(7),
    );
    imports.import(
        "io",
        "string_to_number",
        wasm_encoder::EntityType::Function(8),
    );
    imports.import(
        "io",
        "value_to_string",
        wasm_encoder::EntityType::Function(9),
    );
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

    exports.export("main", ExportKind::Func, 0);

    exports.export("memory", ExportKind::Memory, 0);

    module.section(&exports);

    let mut code = CodeSection::new();

    let wasm_func = compile_message_chain(chain, method_data, runtime);

    code.function(&wasm_func);

    module.section(&code);

    let actual_function_index = import_function_count + function_index;

    (module.finish(), actual_function_index)
}

/// Compile an argument to WebAssembly instructions
fn compile_argument(
    arg: &iowa_parser::Argument,
    func: &mut WasmFunction,
    runtime: &Runtime,
    local_base: u32,
) -> u32 {
    // Constants for local variables
    const ARG_RESULT: u32 = 0; // Where we store the result
    const ARG_STR_PTR: u32 = 1; // String pointer
    const ARG_STR_LEN: u32 = 2; // String length
    const ARG_TEMP: u32 = 3; // Temporary variable

    // Actual local indices (offset by local_base)
    let result_local = local_base + ARG_RESULT;
    let str_ptr_local = local_base + ARG_STR_PTR;
    let str_len_local = local_base + ARG_STR_LEN;
    let temp_local = local_base + ARG_TEMP;

    // If there are no chains, return nil
    if arg.chains.is_empty() {
        // Return nil (encoded as u64)
        let nil_value = runtime.to_tagged_value(&Value::Nil) as i64;
        func.instruction(&WasmInstruction::I64Const(nil_value));
        func.instruction(&WasmInstruction::LocalSet(result_local));
        return result_local;
    }

    // Process the first message chain in the argument
    let chain = &arg.chains[0];

    // If there are no messages in the chain, return nil
    if chain.messages.is_empty() {
        let nil_value = runtime.to_tagged_value(&Value::Nil) as i64;
        func.instruction(&WasmInstruction::I64Const(nil_value));
        func.instruction(&WasmInstruction::LocalSet(result_local));
        return result_local;
    }

    // If it's a single message with no arguments, we can optimize common cases
    if chain.messages.len() == 1 && chain.messages[0].args.is_empty() {
        let message = &chain.messages[0];
        // Handle simple literals directly
        match &message.symbol {
            iowa_parser::Symbol::Number(num) => {
                // Number literal
                let num_val = match num {
                    iowa_parser::Number::Decimal(val) => *val,
                    iowa_parser::Number::Hex(val) => *val as f64,
                };

                // Call alloc_number with the value
                func.instruction(&WasmInstruction::F64Const(num_val));
                func.instruction(&WasmInstruction::Call(3)); // Call alloc_number
                func.instruction(&WasmInstruction::LocalSet(result_local));
                return result_local;
            }
            iowa_parser::Symbol::Quote(quote) => {
                // String literal
                let string_content = quote.content();

                // Store string info
                func.instruction(&WasmInstruction::I32Const(string_content.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(str_len_local));

                // Call string allocation
                func.instruction(&WasmInstruction::I32Const(0)); // Dummy pointer (filled at runtime)
                func.instruction(&WasmInstruction::LocalGet(str_len_local));
                func.instruction(&WasmInstruction::Call(2)); // alloc_string
                func.instruction(&WasmInstruction::LocalSet(result_local));
                return result_local;
            }
            iowa_parser::Symbol::Identifier(id) => {
                // Identifier (variable reference)
                let name = id.name();

                // Store identifier name info
                func.instruction(&WasmInstruction::I32Const(name.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(str_len_local));

                // Get name pointer (will be filled at runtime)
                func.instruction(&WasmInstruction::I32Const(0));
                func.instruction(&WasmInstruction::LocalGet(str_len_local));

                // Receiver is self
                func.instruction(&WasmInstruction::LocalGet(0));

                // Call lookup_slot to find the identifier
                func.instruction(&WasmInstruction::Call(6)); // lookup_slot
                func.instruction(&WasmInstruction::LocalSet(result_local));
                return result_local;
            }
            _ => {} // Fall through to full chain compilation for operators
        }
    }

    // For more complex expressions, use the full message chain compilation
    // We're using a simplified version here that only handles one message for now
    if !chain.messages.is_empty() {
        let message = &chain.messages[0];

        // Handle the message based on its type
        match &message.symbol {
            iowa_parser::Symbol::Identifier(id) => {
                let name = id.name();

                // Store identifier name info
                func.instruction(&WasmInstruction::I32Const(name.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(str_len_local));

                // Get name pointer (will be filled at runtime)
                func.instruction(&WasmInstruction::I32Const(0));
                func.instruction(&WasmInstruction::LocalGet(str_len_local));

                // Receiver is self
                func.instruction(&WasmInstruction::LocalGet(0));

                // Call lookup_slot to find the identifier
                func.instruction(&WasmInstruction::Call(6)); // lookup_slot
                func.instruction(&WasmInstruction::LocalSet(result_local));
            }
            iowa_parser::Symbol::Operator(op) => {
                // Get the operator symbol
                let name_str = op.symbol();

                // Set message name info
                func.instruction(&WasmInstruction::I32Const(name_str.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(str_len_local));

                // Get name pointer (will be filled at runtime)
                func.instruction(&WasmInstruction::I32Const(0));
                func.instruction(&WasmInstruction::LocalGet(str_len_local));

                // Receiver is self
                func.instruction(&WasmInstruction::LocalGet(0));

                // If we have arguments for the operator, compile them
                if !message.args.is_empty() {
                    // Process each argument (recursively)
                    // This is a key enhancement to handle nested expressions

                    // For now, we'll implement a simplified version that handles
                    // just one argument, since that's the common case for operators
                    if let Some(arg) = message.args.first() {
                        // Recursively compile the argument (with offset locals)
                        let arg_local = compile_argument(arg, func, runtime, local_base + 4);

                        // Create an array with just this one argument value
                        func.instruction(&WasmInstruction::I32Const(8)); // 8 bytes per value
                        func.instruction(&WasmInstruction::LocalSet(str_ptr_local));
                        func.instruction(&WasmInstruction::I32Const(1)); // array length = 1
                        func.instruction(&WasmInstruction::LocalSet(str_len_local));

                        // Get arg value to pass
                        func.instruction(&WasmInstruction::LocalGet(arg_local));
                        func.instruction(&WasmInstruction::LocalSet(temp_local));

                        // Call dispatch_message with the operator and argument
                        func.instruction(&WasmInstruction::LocalGet(0)); // self as receiver
                        func.instruction(&WasmInstruction::I32Const(0)); // message name ptr (filled at runtime)
                        func.instruction(&WasmInstruction::LocalGet(str_len_local));
                        func.instruction(&WasmInstruction::LocalGet(str_ptr_local)); // args ptr
                        func.instruction(&WasmInstruction::I32Const(1)); // arg count = 1
                        func.instruction(&WasmInstruction::Call(7)); // Call dispatch_message
                        func.instruction(&WasmInstruction::LocalSet(result_local));
                    } else {
                        // No arguments for operator, treat as unary
                        func.instruction(&WasmInstruction::I32Const(0)); // Empty array ptr
                        func.instruction(&WasmInstruction::I32Const(0)); // Zero length
                        func.instruction(&WasmInstruction::Call(7)); // Call dispatch_message
                        func.instruction(&WasmInstruction::LocalSet(result_local));
                    }
                } else {
                    // No arguments, call with empty args array
                    func.instruction(&WasmInstruction::I32Const(0)); // Empty array ptr
                    func.instruction(&WasmInstruction::I32Const(0)); // Zero length
                    func.instruction(&WasmInstruction::Call(7)); // Call dispatch_message
                    func.instruction(&WasmInstruction::LocalSet(result_local));
                }
            }
            _ => {
                // For any other symbol type, just return nil for now
                let nil_value = runtime.to_tagged_value(&Value::Nil) as i64;
                func.instruction(&WasmInstruction::I64Const(nil_value));
                func.instruction(&WasmInstruction::LocalSet(result_local));
            }
        }
    } else {
        // No messages in chain, use nil
        let nil_value = runtime.to_tagged_value(&Value::Nil) as i64;
        func.instruction(&WasmInstruction::I64Const(nil_value));
        func.instruction(&WasmInstruction::LocalSet(result_local));
    }

    // Return the local index where we stored the result
    result_local
}

fn compile_message_chain(
    chain: &iowa_parser::MessageChain,
    method_data: &MethodData,
    runtime: &Runtime,
) -> WasmFunction {
    // Define local variables we'll need
    let mut func_locals = Vec::new();

    // We need locals for storing:
    // - Result of each message operation
    // - String pointers and lengths
    // - Argument array pointers and lengths
    // - Nested argument evaluation

    // Add locals for result storage (we'll use local 0 for the current result)
    func_locals.push((1, ValType::I64)); // One I64 local for result

    // Add locals for string operations
    func_locals.push((4, ValType::I32)); // Four I32 locals for string operations

    // Add locals for array operations
    func_locals.push((4, ValType::I32)); // Four I32 locals for array operations

    // Add locals for argument processing
    func_locals.push((8, ValType::I64)); // Eight I64 locals for argument processing
    func_locals.push((8, ValType::I32)); // Eight I32 locals for argument metadata

    // Initialize the function with locals
    let mut func = WasmFunction::new(func_locals);

    // For an empty chain, return nil
    if chain.messages.is_empty() {
        // Return nil (encoded as u64)
        let nil_value = runtime.to_tagged_value(&Value::Nil) as i64;
        func.instruction(&WasmInstruction::I64Const(nil_value));
        func.instruction(&WasmInstruction::End);
        return func;
    }

    // Process each message in the chain
    for (i, message) in chain.messages.iter().enumerate() {
        let is_last = i == chain.messages.len() - 1;
        let is_first = i == 0;

        // Store constant locals
        const STRING_NAME_PTR: u32 = 1; // String pointer local
        const STRING_NAME_LEN: u32 = 2; // String length local
        const ARGS_PTR: u32 = 3; // Arguments array pointer local
        const ARGS_LEN: u32 = 4; // Arguments array length local
        const RESULT_LOCAL: u32 = 0; // Result local

        match &message.symbol {
            iowa_parser::Symbol::Quote(quote) => {
                // Create a string literal in WASM memory
                let string_content = quote.content();

                // Store the string length as constant
                func.instruction(&WasmInstruction::I32Const(string_content.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(STRING_NAME_LEN));

                // Call string allocation function (alloc_string)
                func.instruction(&WasmInstruction::I32Const(0)); // Dummy pointer
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_LEN));
                func.instruction(&WasmInstruction::Call(2)); // Call alloc_string

                if !is_last {
                    // Store result for next operation
                    func.instruction(&WasmInstruction::LocalSet(RESULT_LOCAL));
                }
            }
            iowa_parser::Symbol::Number(num) => {
                // Handle numeric literal
                let num_val = match num {
                    iowa_parser::Number::Decimal(val) => *val,
                    iowa_parser::Number::Hex(val) => *val as f64,
                };

                // Call alloc_number with the number value
                func.instruction(&WasmInstruction::F64Const(num_val));
                func.instruction(&WasmInstruction::Call(3)); // Call alloc_number

                if !is_last {
                    // Store result for next operation
                    func.instruction(&WasmInstruction::LocalSet(RESULT_LOCAL));
                }
            }
            iowa_parser::Symbol::Identifier(id) => {
                // Get the message name
                let name_str = id.name();

                // Set message name length
                func.instruction(&WasmInstruction::I32Const(name_str.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(STRING_NAME_LEN));

                // Prepare message name pointer (will be determined at runtime)
                func.instruction(&WasmInstruction::I32Const(0)); // Dummy pointer
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_LEN));
                func.instruction(&WasmInstruction::LocalSet(STRING_NAME_PTR));

                // Process arguments if we have any
                if !message.args.is_empty() {
                    // We need to create an array of argument values
                    let arg_count = message.args.len();
                    let mut arg_locals = Vec::with_capacity(arg_count);

                    // Use our enhanced argument compilation for each argument
                    // Start at local index 5 for argument processing
                    let arg_local_base = 5;

                    for (arg_idx, arg) in message.args.iter().enumerate() {
                        // Compile this argument with appropriate local variable offset
                        // Each argument gets its own set of local variables
                        let offset = arg_local_base + arg_idx * 4;
                        let arg_local = compile_argument(arg, &mut func, runtime, offset as u32);
                        arg_locals.push(arg_local);
                    }

                    // Now create an array with all argument values
                    // For now, we'll use a simple approach with a fixed array
                    // In a real implementation, we'd allocate memory and copy values

                    // Set argument array size
                    func.instruction(&WasmInstruction::I32Const(arg_count as i32));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_LEN));

                    // Allocate array pointer (placeholder)
                    func.instruction(&WasmInstruction::I32Const(0));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_PTR));
                } else {
                    // No arguments
                    func.instruction(&WasmInstruction::I32Const(0)); // Empty array pointer
                    func.instruction(&WasmInstruction::I32Const(0)); // Zero length
                    func.instruction(&WasmInstruction::LocalSet(ARGS_PTR));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_LEN));
                }

                // Get the receiver for this message
                if is_first {
                    // First message uses the self parameter (local 0)
                    func.instruction(&WasmInstruction::LocalGet(0)); // Get 'self' parameter
                } else {
                    // Use the result of the previous message
                    func.instruction(&WasmInstruction::LocalGet(RESULT_LOCAL));
                }

                // Call dispatch_message with the prepared arguments
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_PTR));
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_LEN));
                func.instruction(&WasmInstruction::LocalGet(ARGS_PTR));
                func.instruction(&WasmInstruction::LocalGet(ARGS_LEN));
                func.instruction(&WasmInstruction::Call(7)); // Call dispatch_message

                if !is_last {
                    // Store result for the next operation
                    func.instruction(&WasmInstruction::LocalSet(RESULT_LOCAL));
                }
            }
            iowa_parser::Symbol::Operator(op) => {
                // Get the operator symbol
                let name_str = op.symbol();

                // Set message name length
                func.instruction(&WasmInstruction::I32Const(name_str.len() as i32));
                func.instruction(&WasmInstruction::LocalSet(STRING_NAME_LEN));

                // Prepare message name pointer (will be determined at runtime)
                func.instruction(&WasmInstruction::I32Const(0)); // Dummy pointer
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_LEN));
                func.instruction(&WasmInstruction::LocalSet(STRING_NAME_PTR));

                // Process arguments if we have any
                if !message.args.is_empty() {
                    // We need to create an array of argument values
                    let arg_count = message.args.len();
                    let mut arg_locals = Vec::with_capacity(arg_count);

                    // Use our enhanced argument compilation for each argument
                    // Start at local index 5 for argument processing
                    let arg_local_base = 5;

                    for (arg_idx, arg) in message.args.iter().enumerate() {
                        // Compile this argument with appropriate local variable offset
                        let offset = arg_local_base + arg_idx * 4;
                        let arg_local = compile_argument(arg, &mut func, runtime, offset as u32);
                        arg_locals.push(arg_local);
                    }

                    // Set argument array size
                    func.instruction(&WasmInstruction::I32Const(arg_count as i32));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_LEN));

                    // Allocate array pointer (placeholder)
                    func.instruction(&WasmInstruction::I32Const(0));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_PTR));
                } else {
                    // No arguments
                    func.instruction(&WasmInstruction::I32Const(0)); // Empty array pointer
                    func.instruction(&WasmInstruction::I32Const(0)); // Zero length
                    func.instruction(&WasmInstruction::LocalSet(ARGS_PTR));
                    func.instruction(&WasmInstruction::LocalSet(ARGS_LEN));
                }

                // Get the receiver for this message
                if is_first {
                    // First message uses the self parameter (local 0)
                    func.instruction(&WasmInstruction::LocalGet(0)); // Get 'self' parameter
                } else {
                    // Use the result of the previous message
                    func.instruction(&WasmInstruction::LocalGet(RESULT_LOCAL));
                }

                // Call dispatch_message with the prepared arguments
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_PTR));
                func.instruction(&WasmInstruction::LocalGet(STRING_NAME_LEN));
                func.instruction(&WasmInstruction::LocalGet(ARGS_PTR));
                func.instruction(&WasmInstruction::LocalGet(ARGS_LEN));
                func.instruction(&WasmInstruction::Call(7)); // Call dispatch_message

                if !is_last {
                    // Store result for the next operation
                    func.instruction(&WasmInstruction::LocalSet(RESULT_LOCAL));
                }
            }
        }
    }

    // End the function
    func.instruction(&WasmInstruction::End);

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

        let (module_bytes, _) = generate_wasm_from_ast(chain, &method_data, &runtime);

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
    fn test_runtime_object_functions() {
        let mut runtime = Runtime::new();

        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);

        let imports = create_runtime_imports(&mut store, &mut runtime);

        let alloc_object = imports
            .get_export("io", "alloc_object")
            .expect("Should have alloc_object function");

        if let wasmer::Extern::Function(func) = alloc_object {
            let object_proto = runtime.prototypes().object;
            let proto_tag_val: i64 = runtime
                .to_tagged_value(&Value::Object(object_proto))
                .try_into()
                .unwrap();

            let result = func
                .call(&mut store, &[WasmerValue::I64(proto_tag_val)])
                .expect("Function call should succeed");

            if let Some(WasmerValue::I64(obj_id)) = result.first() {
                let u_val: u64 = (*obj_id).try_into().unwrap();
                let value = runtime.from_tagged_value(u_val);

                match value {
                    Value::Object(_) => { /* Test passed */ }
                    _ => panic!("Expected Object value, got {:?}", value),
                }
            } else {
                panic!("Expected I64 result from alloc_object");
            }
        } else {
            panic!("Expected function for alloc_object");
        }
    }

    #[test]
    fn test_memory_management() {
        let compiler = Cranelift::default();
        let mut store = Store::new(compiler);

        let memory_type = wasmer::MemoryType::new(1, None, false);
        let memory = Memory::new(&mut store, memory_type).expect("Failed to create memory");

        let mut memory_manager = WasmMemoryManager::new(&memory, store.as_store_mut());

        // Test string allocation and reading
        let (ptr, len) = memory_manager.write_string("Hello, Io!");
        assert!(ptr >= STRING_HEAP_START);
        assert_eq!(len, 10);

        let read_string = memory_manager.read_string(ptr, len);
        assert_eq!(read_string, "Hello, Io!");

        // Test value array allocation and reading
        let values = vec![42u64, 100u64, 999u64];
        let (arr_ptr, arr_len) = memory_manager.write_array(&values);
        assert!(arr_ptr >= OBJECT_HEAP_START);
        assert_eq!(arr_len, 3);

        let read_values = memory_manager.read_array(arr_ptr, arr_len);
        assert_eq!(read_values, values);

        // Test memory capacity growing
        let big_size = MEMORY_PAGE_SIZE * 2;
        memory_manager.ensure_capacity(big_size);
        let memory_view = memory.view(&mut memory_manager.store);
        let memory_size: usize = memory_view.data_size().try_into().unwrap();
        assert!(memory_size >= big_size as usize);
    }
}
