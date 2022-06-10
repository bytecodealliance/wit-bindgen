<div align="center">
  <h1><code>wit-bindgen-wasmtime</code></h1>

  <p>
    <strong>A WebAssembly bindings generator for Rust users using the `wasmtime` crate.</strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://github.com/bytecodealliance/wit-bindgen/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/wit-bindgen/workflows/CI/badge.svg" alt="build status" /></a>
    <img src="https://img.shields.io/badge/rustc-stable+-green.svg" alt="supported rustc stable" />
  </p>
</div>

# wit-bindgen-wasmtime

_Please note: this is currently an experimental project._

`wit-bindgen-wasmtime` is a prototype [WebAssembly](https://webassembly.org/) bindings generator for Rust users using the `wasmtime` crate. It has an `import!` and an `export!` macro for generating code.

## Demo

This demo demonstrates the use of `import!`. The goal is to make an application that, using the `wasmtime` crate, runs Wasm modules that implement some `*.wit` file. Therefore, in this application, the `*wit` file acts as a public API for plugins that are distributed as Wasm modules.

### Prerequisites

The demo requires [cargo-wasi](https://github.com/bytecodealliance/cargo-wasi), so install it using `cargo`:

```text
$ cargo install cargo-wasi
```

### The `*.wit` file

The `renderer.wit` file specifies functions that should be implemented by "plugins" of the application. The plugin can be written in any language that compiles to Wasm, as only the Wasm module is run by the application. 

The interface for plugins is:

```wit
/// Name of the plugin
name: function() -> string

/// Render texts
render: function(text: string) -> string
```

### Building the plugin

In this case, we will build a plugin using Rust, but it could be other language too. To do so, we make use of the `wit-bindgen-rust` crate. 

To build the `markdown` plugin, go into the `markdown` directory and run

```text
$ cargo wasi build
```

This generates the wasm module under `markdown/target/wasm32-wasi/debug/markdown.wasm`. To make it available to the app, copy it and paste it in `app/plugins` directory.

*Note:* Intentionally, this is a separated crate from the app, and depends only on the `renderer.wit` file.

### Running the app

The application uses the `wasmtime` crate to run the plugin `markdown.wasm`. To make of interface types, it needs to import it using the `import!` macro in the `wit-bindgen-wasmtime` crate.

To run the application, go to the `app` directory and run:

```text
$ cargo run
```

If everything worked correctly, a loop should start where each input line is rendered by the markdown plugin.

To understand better the code of the `app` directory, we recommend to expand the `import!` macro using the [online wit-bindgen demo](https://bytecodealliance.github.io/wit-bindgen/).

