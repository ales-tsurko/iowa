use iowa_compiler::runtime::runtime::{MethodBody, MethodData, Runtime, Value};
use iowa_compiler::runtime::wasm_runtime::WasmMethod;

#[test]
fn test_minimal_method_execution() {
    // Create a runtime instance
    let mut runtime = Runtime::new();

    // Create a simple test object
    let object_proto = runtime.prototypes().object;
    let test_obj = runtime.alloc_object(object_proto);

    // Set a value on the object
    if let Some(obj) = runtime.memory_mut().get_object_mut(test_obj) {
        obj.set_slot("value".to_string(), Value::Number(42.0));
    }

    // Create a method that returns the value
    let source = "method(return self value)";

    // Parse the source code
    let (_, chains) = iowa_parser::parse(source).unwrap();

    // Create method data
    let method_data = MethodData {
        args: vec![],
        body: MethodBody::Source(source.to_string()),
    };

    // Create a WasmMethod
    let wasm_method = WasmMethod::from_ast(&chains[0], &method_data, &runtime);

    // Execute the method
    let result = wasm_method.execute(&mut runtime, &[], Value::Object(test_obj));

    // We should get the value back
    assert_eq!(result, Value::Number(42.0));
}
