<div align="center">
  <h1><code>wit-bindgen</code></h1>

  <p>
    <strong>A language bindings generator for <code>wit</code></strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://github.com/bytecodealliance/wit-bindgen/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/wit-bindgen/workflows/CI/badge.svg" alt="build status" /></a>
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
embeddings of WebAssembly. This works with [`*.wit`](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md), which describe the
interface of a module, either imported or exported. In the future, it will support [profiles](https://github.com/WebAssembly/component-model/pull/83) (`*.world`), which define the collection of possible imports and exports for a module or component.

## Hosts and Guests

When thinking about WebAssembly, it can be useful to talk in terms of **hosts** and **guests**. A **host** exists outside the WebAssembly runtime and can instantiate modules, satisfying their imports and using their exports. The WebAssembly that runs inside the runtime is called the **guest**. Both **hosts** and **guests** can both have imports and exports. For example, a **guest** can import a WASI interface that the **host** exports to it. It's also possible for **guests** to export/import things to/from other **guests**.

## Use Cases

This project can be used in cases such as:

* You're implementing a guest in Rust that will be compiled to WebAssembly and you'd like it to import WASI. The command `wit-bindgen guest rust --import <wit-path>` will generate Rust bindings so your guest can import WASI APIs that are described by a `*.wit` file.

* You're creating a host in Python that uses the wasmtime runtime and wants to provide WASI functionality to guests. The command `wit-bindgen host wasmtime-py --export <wit-path>` will generate the Python stubs needed to implement and pass in the WASI interface.

* You're writing JS host code in the browser that will consume a WebAssembly module and you don't want to deal with funky ABI details. The command `wit-bindgen host js` can generate the JS bindings and a TypeScript interface for you with native JS types.

**Note:** This CLI experience is not the only way wit-bindgen can/will be used. For example it can be used in Rust through procedural macros and potentially in the future Cargo dependencies. Usage in a Web application would probably be through a version of wit-bindgen compiled to WebAssembly and published to NPM.

## Context
The purpose of `wit-bindgen` is to provide a forwards-compatible toolchain and story for modules using the canonical ABI and eventually components in the emerging [Component Model](https://github.com/WebAssembly/component-model). This project was originally based on the [interface types
proposal](https://github.com/webassembly/interface-types) and the [canonical ABI]. The Component Model will eventually "absorb" the interface types proposal, so all references to interface types are effectively to interface types / the component model. This repository will be following upstream changes there, especially for the [`*.wit`](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) syntax.

Currently, generated language bindings all use the [canonical ABI] for communication. This means that any language with supported guest bindings can be consumed in any environment with supported host bindings, which will interoperate through the [canonical ABI].

[canonical ABI]: https://github.com/WebAssembly/interface-types/pull/132

## Demo

[View generated bindings
online!](https://bytecodealliance.github.io/wit-bindgen/)

If you're curious to poke around and see what generated bindings look like for a
given input `*.wit`, you can explore the generated code online to get an idea
of what's being generated and what the glue code looks like.

## CLI Installation

To install the CLI for this tool (which isn't the only way it can be used), run the following cargo command. This will let you generate the bindings for any supported language.

This CLI **IS NOT** stable and may change, do not expect it to be or rely on it being stable. Please rreach out to us on [zulip] if you'd like to depend on it, so we can figure out a better alternative for your use case.

```
cargo install --git https://github.com/bytecodealliance/wit-bindgen wit-bindgen-cli
```

## Supported Generators



### Guests

These generators are for creating guest modules that import/export WIT types.

* `rust` - this is for Rust compiled to WebAssembly, typically using either
  the `wasm32-wasi` or `wasm32-unknown-unknown` targets depending on your use
  case. In this mode you'd probably depend on the `wit-bindgen-guest-rust` crate
  (located at `crates/guest-rust`) and use the `import!` and `export!` macros to
  generate code.

* `c` - this is for C compiled to WebAssembly, using either of the targets above
  for Rust as well. With C the `wit-bindgen` CLI tool will emit a `*.h` and a
  `*.c` file to be compiled into the wasm module.

### Hosts

These generators are for hosts interacting with modules that import/export WIT types.

* `wasmtime-rust` - this is for Rust users using the `wasmtime` crate. This generator 
  can also be is used through the `wit-bindgen-host-wasmtime-rust` crate (located at
  `crates/host-wasmtime-rust`) and, like the guest Rust support, has an
  `import!` and an `export!` macro for generating code.

* `js` - this is for JavaScript users executing WebAssembly modules. This could
  be in a browsers, Node.js, or Deno. In theory this covers browser use cases
  like web workers and such as well. In this mode the `wit-bindgen` CLI tool
  will emit a `*.js` and a `*.d.ts` file describing the interface and providing
  necessary runtime support in JS to implement the canonical ABI. Note that the
  intended long-term integration of this language is to compile `wit-bindgen`
  itself to WebAssembly and publish NPM packages for popular JS build systems to
  integrate `wit-bindgen` into JS build processes.

* `wasmtime-py` - this is for Python users using the `wasmtime` PyPI package.
  This uses Wasmtime under the hood but you get to write Python in providing
  imports to WebAssembly modules or consume modules using interface types. This
  generates a `*.py` file which is annotated with types for usage in `mypy` or
  other type-checkers.

### Other

Finally in a sort of "miscellaneous" category the `wit-bindgen` CLI also
supports:

* `markdown` - generates a `*.md` and a `*.html` file with readable
  documentation rendered from the comments in the source `*.wit` file.

### Arguments
All generators support the `--import` and `--export` flags in the `wit-bindgen`
CLI tool:

```
$ wit-bindgen host js --import browser.wit
$ wit-bindgen guest rust --export my-interface.wit
$ wit-bindgen host rust --import host-functions.wit
```

Here "import" means "I want to import and call the functions in this interface"
and "export" means "I want to define the functions in this interface for others
to call".


### Contributing Bindings

The list of supported languages here is a snapshot in time and is not
final. The purpose of the interface-types proposal is to be language agnostic
both in how WebAssembly modules are written as well as how they are consumed. If
you have a runtime that isn't listed here or you're compiling to WebAssembly and
your language isn't listed here, it doesn't mean that it will never be
supported!

Writing language bindings generators is not trivial, but the crates and tooling in this repository exist to make writing generators as easy as practically possible. If you are interested in support for a language or runtime, please check our issues and file one if there isn't already an issue for it.

Here is a non-exhaustive list of some generators that we don't currently support in `wit-bindgen` today but are possible in the future.

* `host wasmtime-go` - same as for `host wasmtime-py` but for Go.
  Basically for Go users using the [`wasmtime-go` package](https://github.com/bytecodealliance/wasmtime-go) who want to work with interface types rather than raw pointers/memories/etc.

* `host wasmtime-cpp` - again the same as for `host wasmtime-py`, but for users of the
  [`wasmtime-cpp` header file](https://github.com/alexcrichton/wasmtime-cpp) to
  use interface types from C++.

* `guest js` - while host runtime support is provided for JS today it should also be
  supported for [JS compiled to WebAssembly](https://bytecodealliance.org/articles/making-javascript-run-fast-on-webassembly).
  For example a `*.d.ts` file could be generated for what JS projects could
  import and then corresponding glue code for the engine-compiled-to-wasm would
  also be generated. This means that you could use both JS-in-wasm but also JS
  as a host (or more realistically another runtime like Wasmtime since if you're
  running in a JS environment you're probably best off running the JS there
  instead).

There are a plethora of other languages that compile to WebAssembly and runtimes. Since interface types should be able to work with all of them, they're theoretically just some work-hours away from having support in `wit-bindgen`.
