# Gobbledygook Design Guide

Gobbledygook is an Io-inspired compiled language.

It keeps Io's minimal message syntax and message-based metaprogramming, but
replaces prototype inheritance with concrete types, traits, composition, and
optional static typing.

Primary targets:

- native binaries
- WebAssembly
- JIT / embedded scripting
- WASI component interoperability

Memory is garbage-collected by default.

## Core Idea

Everything starts as a message.

```io
"hello" println
user name println
add(1, 2)
```

The parser does not understand functions, loops, types, or traits as syntax.
It only creates message trees.

Every message is sent to a receiver. A message chain has an implicit first
receiver: the current evaluation context.

At top level, the current context is the current module:

```io
user name println
```

means:

```text
currentModule user -> name -> println
```

Inside a method, the current context resolves locals first, then `self`, then
module/import/prelude bindings.

Conceptually:

```io
user name println
```

is a chain:

```text
Message("user") -> Message("name") -> Message("println")
```

Arguments are also message trees:

```io
add(1 + 2, value println)
```

## Messages

A message has:

```text
name
arguments
next
source location
```

This is the central data structure of the language.

Runtime code can inspect messages. Compile-time code can transform messages.

```io
call message name
call message argAt(0)
receiver doMessage(message)
```

## Parsing

Parsing is intentionally small.

```text
source
-> tokens
-> raw Message tree
```

Operator precedence is not part of parsing. It is a later rewrite.

```io
1 + 2 * 3
```

starts as a flat message chain, then becomes:

```io
1 +(2 *(3))
```

## Operators

Operators are messages with precedence.

```io
1 + 2
```

means:

```io
1 +(2)
```

The active `Compiler` controls operator precedence.

```io
Compiler addOperator("!!", 3)
```

Operator changes affect later compiled code in the same compile-time context.

## Modules

Top-level code runs in a `Module`, not in Io's `Lobby`.

A module is a lexical namespace and import boundary.

```io
User ::= type(
  name str
)

main := method(i32,
  0
)
```

Top-level messages are resolved against the current module, imports, prelude,
and compile-time environment.

## Assignment

Assignment is a message rewrite.

```io
name := "Ann"
name ::= "Public Ann"
name = "Bob"
```

Core meanings:

```io
name := value   // define private binding in current scope
name ::= value  // define public/exported binding in current scope
name = value    // update existing binding
```

In local method/block scope, `:=` and `::=` both define locals; visibility only
matters for module/type/trait members.

```io
Greeter ::= trait(
  greet ::= method(str)
)
```

## Types

Types are concrete data shapes.

```io
User := type(
  name str
  age i32
)
```

There is no type inheritance.

Reuse is by composition:

```io
Engine := type(
  power i32
)

Car := type(
  engine Engine
)
```

## Optional Typing

Types are optional.

Untyped values default to `any`.

```io
log := method(value,
  value println
)
```

means roughly:

```text
log(value: any) -> any
```

Typed code is checked and optimized.

```io
add := method(a i32, b i32, i32,
  a + b
)
```

Primitive type names are lowercase:

```text
any bool char str
i8 i16 i32 i64
u8 u16 u32 u64
f32 f64
```

User-defined types and traits use capitalized names:

```io
User
Greeter
HttpClient
```

## Methods

Methods are receiver-scoped.

```io
User greet := method(str,
  "hi " .. name
)
```

Arguments use message-style type annotations:

```io
User greet := method(prefix str, str,
  prefix .. " " .. name
)
```

A bare type in signature position marks the return type.

```io
method(str)
method(name str, str)
method(name str, age i32, str)
```

Typed arguments must be named. A bare type is only allowed once and must be the
last signature item before the body.

```io
method(str)                    // () -> str
method(name str)               // (name: str) -> any
method(name str, str)          // (name: str) -> str
method(a i32, b i32, i32, ...) // (a: i32, b: i32) -> i32
```

If no return type is given, it defaults to `any`.

## Blocks

`block(...)` is lexical.

```io
makeCounter := method(
  n := 0
  block(i32,
    n = n + 1
  )
)
```

Use `method(...)` for receiver methods.
Use `block(...)` for closures.

## Traits

Traits are interfaces.

```io
Greeter ::= trait(
  greet ::= method(str)
)
```

A type implements a trait by providing matching methods.

```io
User implements(Greeter)

User greet := method(str,
  "hi " .. name
)
```

A type may implement many traits.

Trait dispatch is dynamic interface dispatch. Concrete type dispatch is static
when types are known.

## Dispatch

Message dispatch depends on receiver type.

```text
Concrete T  -> static method call
Trait T     -> trait/vtable dispatch
any         -> dynamic message lookup
CompileTime -> compiler message evaluation
```

This preserves Io-like message sending while allowing compiled performance.

## WASI Interoperability

Gobbledygook should import WASI components as Gobbledygook modules.

```io
Image ::= importWasi("image.component.wasm")

pixels := Image decodePng(bytes)
```

`importWasi(...)` is a compiler message. It reads the component's WIT world,
then generates a Gobbledygook module with typed bindings.

```text
WASI component + WIT
-> Gobbledygook module
-> typed functions, records, variants, resources
```

WIT functions become Gobbledygook methods/functions:

```wit
resize: func(image: image, width: u32, height: u32) -> image;
```

becomes:

```io
Image resize(image Image, width u32, height u32, Image)
```

WIT types map to Gobbledygook types:

```text
bool          -> bool
string        -> str
s32 / u32     -> i32 / u32
f32 / f64     -> f32 / f64
list<T>       -> list(T)
option<T>     -> option(T)
result<T, E>  -> result(T, E)
record        -> type(...)
variant/enum  -> variant type
resource      -> GC-managed external handle
```

Resources are handles owned by the component runtime. Gobbledygook stores them as
GC-managed external objects, with explicit `close`/`drop` available for prompt
release.

```io
Db ::= importWasi("sqlite.component.wasm")

conn := Db open("app.db")
rows := conn query("select * from users")
conn close
```

Imported components do not get ambient host access. Filesystem, network, clocks,
randomness, and other capabilities are provided by the embedding runtime.

Plain core wasm or old WASI Preview 1 modules can be supported through an
adapter or explicit WIT file:

```io
Math ::= importWasm("math.wasm", wit "math.wit")
```

First-class interop target is WASI Component Model modules. Raw wasm without WIT
is not enough to generate safe, typed Gobbledygook bindings automatically.

## Metaprogramming

Metaprogramming works on messages.

The compiler exposes a compile-time environment. Core language constructs are
compiler messages in that environment.

The compile-time receiver is `Compiler`.

```io
Compiler type(...)
Compiler trait(...)
Compiler method(...)
Compiler block(...)
Compiler addOperator(...)
```

These are not parser keywords. They are compile-time messages that lower message
trees into core IR.

Conceptually:

```text
type: Message -> TypeDecl
trait: Message -> TraitDecl
method: Message -> MethodDecl
```

Normal code usually omits the explicit receiver because top-level resolution
finds compiler messages in the compile-time environment:

```io
User ::= type(
  name str
)
```

This is equivalent to the compiler resolving `type` through `Compiler`.

Type checking happens after compile-time messages produce declarations and typed
core IR. Raw messages are not type-checked directly.

```text
Message tree
-> Compiler type/method/trait messages
-> TypeDecl / MethodDecl / TraitDecl
-> type checker
```

User metaprogramming should use the same model:

```io
unless := compilerMethod(
  condition := call message argAt(0)
  body := call message argAt(1)

  quote(if(condition not, body))
)
```

The exact API is not fixed yet, but the substrate is fixed: `Message`.

## Compile-Time vs Runtime

The same message syntax is used in both phases.

The difference is the receiver/context.

```text
compile-time context -> builds declarations and rewrites messages
runtime context      -> executes program values
```

Example:

```io
User := type(
  name str
)
```

`type(...)` is evaluated in the compile-time context and produces a type
declaration.

## Compilation Pipeline

```text
source
-> raw Message tree
-> compile-time message evaluation
-> operator / assignment rewrite where requested by Compiler
-> core IR
-> type checking
-> optimization
-> native / wasm / JIT
```

The type checker is compiler-owned. Macros and compiler messages may generate
typed declarations, but they do not replace the type checker.

## Concurrency

Gobbledygook should have one concurrency model: async/await with tasks.

Native builds use Tokio under the hood. WASM builds map to the host event loop.

```io
fetch := async method(url str, str,
  Http get(url) await
)

main := async method(i32,
  html := fetch("https://example.com") await
  html println
  0
)
```

`spawn(...)` starts a task.

```io
task := spawn(fetch(url))
body := task await
```

No actors, transparent futures, continuations, or multiple competing concurrency
systems in the first core.

## Memory

GC is the default memory model.

Reasons:

- dynamic `any` values need uniform representation
- message trees and metaprogramming allocate many objects
- embedded scripting is easier
- wasm works well with a managed heap

Future low-level features may add value/native/unmanaged types, but they are
not part of the first core.

## What Gobbledygook Removes From Io

Gobbledygook intentionally removes:

- prototype inheritance
- `Lobby` as the root global object
- runtime mutation of inheritance chains
- object protos as namespaces and locals

Gobbledygook keeps:

- message syntax
- message trees as code
- `method` and `block`
- operator rewrites
- `call message`
- `doMessage`
- strong metaprogramming

## First Core

The first implementation should support:

- raw parser preserving message chains
- operator rewrite as a separate phase
- module scope
- `:=`, `::=`, and `=`
- `method(...)`
- `block(...)`
- `type(...)`
- `trait(...)`
- optional annotations with bare return type
- `any`
- GC runtime values
- a small compiler-message environment
- async/await backed by Tokio
- WASI component import through WIT

Avoid initially:

- actors
- futures
- continuations
- full reflection
- package system
- ownership / borrow checking
- advanced optimizer
