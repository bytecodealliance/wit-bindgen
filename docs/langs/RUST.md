# Rust

## Host Bindings
The [`wasmtime`](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasmtime) crate provides an API for embedding the `wasmtime` runtime within a Rust program.

Users of the `wasmtime` crate can use the [`wit-bindgen-wasmtime`](https://github.com/bytecodealliance/wit-bindgen/tree/main/crates/wasmtime) crate to generate Rust code for instantiating and invoking modules that implement WIT interfaces.

### Example
```rs
// Bindgen
wit_bindgen_wasmtime::import!("../foobar.wit");

pub use foobar::{Foobar, FoobarData};

// Setup WASMTIME
let engine = Engine::default();
let mut linker = Linker::new(&engine);
let mut store = Store::new(&engine, FoobarData {});

// Load and initialize our module
let module_bytes = ...
let module = Module::new(&engine, module_bytes).unwrap();
let (interface, instance) = Foobar::instantiate(
  		&mut store, &module, &mut linker, get_whole_store
	).unwrap();

// Call interface
interface.do_foo(&mut store, "bar").unwrap();
```

## Guest Bindings
Rust can be compiled to WASM using its compiler `rustc` with either the `wasm32-wasi` or `wasm32-unknown-unknown` target.

The [`wit-bindgen-rust`](https://github.com/bytecodealliance/wit-bindgen/tree/main/crates/rust-wasm) crate provides `import!` and `export!` macros that can be used by a module implementation to bind to imports and exports defined by WIT interfaces.

### Example
<!-- TODO: pick a better example -->
```rs
wit_bindgen_rust::export!("../foobar.wit");

struct Foobar {}

impl foobar::Foobar for Foobar {
  fn do_foo(bar: String) -> parser1::Output {
    ...
  }
}
```