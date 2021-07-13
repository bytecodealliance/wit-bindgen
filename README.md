<div align="center">
  <h1><code>witx-bindgen</code></h1>

  <p>
    <strong>A language bindings generator for `witx`</strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://github.com/bytecodealliance/witx-bindgen/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/witx-bindgen/workflows/CI/badge.svg" alt="build status" /></a>
    <img src="https://img.shields.io/badge/rustc-stable+-green.svg" alt="supported rustc stable" />
  </p>
</div>

## About

> **Note**: this project is still relatively young and active development with
> large changes is still happening. If you're considering depending on this at
> this time it's recommended to reach out to the authors on [zulip] and get more
> information first.

[zulip]: https://bytecodealliance.zulipchat.com/

This project is a bindings generator framework for WebAssembly programs and
embeddings of WebAssembly. This works with `*.witx` files which describe the
interface of a module, either imported or exported. For example this project can
be used in cases such as:

* Your language (say, Rust) is compiled to WebAssembly and you'd like to import
  WASI. This project will generate Rust bindings to import WASI APIs that are
  described with `*.witx`.

* Your runtime (say, Wasmtime) wants to then provide WASI functionality to guest
  programs. This project will generate a Rust `trait` for you to implement for
  the WASI interface.

* You're consuming a WebAssembly module (say, in a browser) and you don't want
  to deal with funky ABI details. You'd use this project to generate JS bindings
  which give you a TypeScript interface dealing with native JS types for the
  WebAssembly module described by `*.witx`.

This project is based on the [interface types
proposal](https://github.com/webassembly/interface-types) and the [canonical
ABI](https://github.com/WebAssembly/interface-types/pull/132), both of which are
at the time of this writing a work in progress. This repository will be
following upstream changes. The purpose of `witx-bindgen` is to provide a
forwards-compatible toolchain and story for interface types and a canonical ABI.
Generated language bindings all use the canonical ABI for communication,
enabling WebAssembly modules to be written in any language with support and for
WebAssembly modules to be consumed in any environment with language support.

## Demo

[View generated bindings
online!](https://bytecodealliance.github.io/witx-bindgen/)

If you're curious to poke around and see what generated bindings look like for a
given input `*.witx`, you can explore the generated code online to get an idea
of what's being generated and what the glue code looks like.

## Installation

At this time a CLI tool is provided mostly for debugging and exploratory
purposes. It can be installed with:

```
$ cargo install --git https://github.com/bytecodealliance/witx-bindgen
```

This tool is not necessarily intended to be integrated into toolchains. For
example usage in Rust would more likely be done through procedural macros and
Cargo dependencies. Usage in a Web application would probably use a version of
`witx-bindgen` compiled to WebAssembly and published to NPM.

For now, though, you can explore what bindings look like in each language
through the CLI.

## Supported Languages

The currently supported languages for `witx-bindgen` are:

#### `witx-bindgen rust-wasm --import *.witx`

This generation mode is intended for Rust programs compiled to WebAssembly. This
will generate the bindings necessary to import the provided `*.witx` files. Safe
functions are exposed with Rust-native types.

#### `witx-bindgen rust-wasm --export *.witx`

This generation mode is intended for Rust programs compiled to WebAssembly. This
will generate the bindings necessary to have the final WebAssembly module export
the interface described in the `*.witx` files. This mode will generate a trait
that the Rust code needs to implement, and the Rust code will also need to
implement a function returning a singleton of this trait for exported functions
to call and use.

#### `witx-bindgen wasmtime --import *.witx`

This generation mode is intended for Rust programs compiled against the
[`wasmtime`] crate, typically running WebAssembly programs. This will generate
bindings for the host to provide the specified `*.witx` files as imports to the
WebAssembly modules. This generates a function which adds host-defined functions
to a `Linker`, and provides a trait for the host to implement to interact with
the guest (using Rust-native types rather than wasm primitive types).

[`wasmtime`]: https://github.com/bytecodealliance/wasmtime

#### `witx-bindgen wasmtime --export *.witx`

This generation mode is intended for Rust programs compiled against the
[`wasmtime`] crate, typically running WebAssembly programs. This will generate
bindings for the host to consume a WebAssembly module which has the interface
specified in the `*.witx` file. This generates a `struct` which is a typed
representation of the WebAssembly instance and typed methods using native Rust
types are provided to interact with the WebAssembly module.

#### `witx-bindgen js --import *.witx`

This generation mode is intended for consuming a WebAssembly module in a JS
environment (in theory either Node or the Web). This mode generates bindings
suitable for supplying as part of an import object to satisfy the imports of a
wasm module. JS is responsible for creating an object with all the appropriate
functions. This also generates a TypeScript file for type definitions by
default.

#### `witx-bindgen js --export *.witx`

This is intended for calling the exports of a WebAssembly module in a JS
environment. This generates a wrapper class which will perform instantiation
with a provided import object and then provides typed view of the wasm module's
exports (dealing in JS types as well, automatically converting for you). This
also generates a TypeScript file by default.

#### `witx-bindgen c --import *.witx`

This is the same as `rust-wasm --import`, but intended for C. C code compiled to
WebAssembly can use this to import APIs.

#### `witx-bindgen c --export *.witx`

This is the same as `rust-wasm --export`, but intended for C. C code compiled to
WebAssembly can use this to export specific APIs.

## Format of `*.witx` files

This repository supports the s-expression-based `*.witx` format pioneered in the
upstream [WASI repository](https://github.com/webassembly/wasi), but it is also
experimenting with a different syntax that is not based on s-expressions. This
new syntax can be seen throughout the `tests` directory.
