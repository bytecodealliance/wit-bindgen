<div align="center">
  <h1><code>wit-bindgen</code></h1>

  <p>
    <strong>Guest language bindings generator for
    <a href="https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md">WIT</a>
    and the
    <a href="https://github.com/WebAssembly/component-model">Component Model</a>
    </strong>
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

[zulip]: https://bytecodealliance.zulipchat.com/#narrow/stream/327223-wit-bindgen

This project is a suite of bindings generators for languages that are compiled
to WebAssembly and use the [component model]. Bindings are described with
[`*.wit` files][WIT] which specify imports, exports, and facilitate reuse
between bindings definitions.

[WIT]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md
[component model]: https://github.com/WebAssembly/component-model

The `wit-bindgen` repository is currently focused on **guest** programs which
are those compiled to WebAssembly. Executing a component in a host is not
managed in this repository, and some options of how to do so are [described
below][hosts].

## [WIT] as an IDL

The `wit-bindgen` project extensively uses [WIT] definitions to describe imports
and exports. The items supported by [WIT] directly map to the component model
which allows core WebAssembly binaries produced by native compilers to be
transformed into a component. All imports into a WebAssembly binary and all
exports must be described with [WIT]. An example file looks like:

```
default world host {
  import print: func(msg: string)

  export run: func()
}
```

This describes a "world" which describes both imports and exports that the
WebAssembly component will have available. In this case the host will provide a
`print` function and the component itself will provide a `run` function.

Functionality in [WIT] can also be organized into `interface`s:

```
interface my-plugin-api {
  record coord {
    x: u32,
    y: u32,
  }

  get-position: func() -> coord
  set-position: func(pos: coord)

  record monster {
    name: string,
    hp: u32,
    pos: coord,
  }

  monsters: func() -> list<monster>
}

default world my-game {
  import print: func(msg: string)
  import plugin: self.my-plugin-api

  export run: func()
}
```

Here the `my-plugin-api` interface encapsulates a group of functions, types,
etc. This can then be imported wholesale into the `my-game` world via the
`plugin` namespace. The structure of a [WIT] document and world will affect the
generated bindings per-language.

For more information about WIT and its syntax see the [upstream description of
WIT][WIT].

## Creating a Component

The end-goal of `wit-bindgen` is to facilitate creation of a
[component][component model]. Once a component is created it can then be handed
off to any one of a number of [host runtimes][hosts] for execution. Creating a
component is not supported natively by any language today, however, so
`wit-bindgen` is only one of the pieces in the process of creating a component.
The general outline for the build process of a component for a compiled language
is:

1. Using `wit-bindgen` source code for the language is generated representing
   bindings to the specified APIs. This source code is then compiled by the
   native compiler and used by user-written code as well.
2. The native language toolchain is used to emit a core WebAssembly module. This
   core wasm module is the "meat" of a component and contains all user-defined
   code compiled to WebAssembly. The most common native target to use for
   compilation today is the `wasm32-wasi` target.
3. The output core wasm module is transformed into a component using the
   [`wasm-tools`] project, notably the `wasm-tools component new` subcommand.
   This will ingest the native core wasm output and wrap the output into the
   component model binary format.

[`wasm-tools`]: https://github.com/bytecodealliance/wasm-tools

The precise tooling and commands at each of these steps [differs language by
language][guests], but this is the general idea. With a component in-hand the
binary can then be handed off to [a host runtimes][hosts] for execution.

### Creating components: WASI

An important consideration when creating a component today is WASI. All current
native toolchains for languages which have WASI support are using the
`wasi_snapshot_preview1` version of WASI. This definition of WASI was made
with historical `*.witx` files and is not compatible with the component model.
There is, however, a means by which to still create components from modules
that are using `wasi_snapshot_preview1` APIs.

The `wasm-tools component new` subcommand takes an `--adapt` argument which acts
as a way to polyfill non-component-model APIs, like `wasi_snapshot_preview1`,
with component model APIs. The [preview2-prototyping] project is the current
go-to location to to acquire a polyfill from `wasi_snapshot_preview1` to an
in-development version of "wasi preview2" which is specified with [WIT]
and the component model.

Notably you'll want to download [one of the adapter modules][preview1-build]
and name it `wasi_snapshot_preview1.wasm` locally to pass as an `--adapt`
argument to `wasm-tools component new`. Note that there are two modules
provided on the downloads page, [one is for non-commands][non-command] which
don't have a `_start` entrypoint in the generated core wasm module (e.g. the
`cdylib` crate type in Rust) and [one that is for command modules][command]
which has a `_start` entrypoint (e.g. a `src/main.rs` in Rust).

[preview2-prototyping]: https://github.com/bytecodealliance/preview2-prototyping
[preview1-build]: https://github.com/bytecodealliance/preview2-prototyping/releases/tag/latest
[non-command]: https://github.com/bytecodealliance/preview2-prototyping/releases/download/latest/wasi_snapshot_preview1.wasm
[command]: https://github.com/bytecodealliance/preview2-prototyping/releases/download/latest/wasi_snapshot_preview1.command.wasm

## Supported Guest Languages
[guests]: #supported-guest-languages

The `wit-bindgen` project is primarily focused on **guest** languages which are
those compiled to WebAssembly. Each language here already has native support for
execution in WebAssembly at the core wasm layer (e.g. targets the current [core
wasm specification](https://webassembly.github.io/spec/)). Brief instructions
are listed here for each language of how to use it as well.

Each project below will assume the following `*.wit` file in the root of your
project.

```
// wit/host.wit
default world host {
  import print: func(msg: string)

  export run: func()
}
```

### Guest: Rust

The Rust compiler supports a native `wasm32-wasi` target and can be added to any
`rustup`-based toolchain with:

```sh
rustup target add wasm32-wasi
```

Projects can then depend on `wit-bindgen` by executing:

```sh
cargo add --git https://github.com/bytecodealliance/wit-bindgen wit-bindgen-guest-rust
```

WIT files are currently added to a `wit/` folder adjacent to your `Cargo.toml`
file. Example code using this then looks like:

```rust
// src/lib.rs

// Use a procedural macro to generate bindings for the world we specified in
// `host.wit`
wit_bindgen_guest_rust::generate!("host");

// Define a custom type and implement the generated `Host` trait for it which
// represents implementing all the necesssary exported interfaces for this
// component.
struct MyHost;

impl Host for MyHost {
    fn run() {
        print("Hello, world!");
    }
}

export_host!(MyHost);
```

By using [`cargo expand`](https://github.com/dtolnay/cargo-expand) or `cargo
doc` you can also explore the generated code.

This project can then be built with:

```sh
cargo build --target wasm32-wasi
wasm-tools component new ./target/wasm32-wasi/debug/my-project.wasm \
    -o my-component.wasm --adapt ./wasi_snapshot_preview1.wasm
```

This creates a `my-component.wasm` file which is suitable to execute in any
component runtime. Using `wasm-tools` you can inspect the binary as well, for
example inferring the WIT world that is the component:

```sh
wasm-tools component wit my-component.wasm
# default world my-component {
#  import print: func(msg: string)
#  export run: func()
# }
```

which in this case, as expected, is the same as the input world.

### Guest: C/C++

C and C++ code can be compiled for the `wasm32-wasi` target using the [WASI
SDK] project. The releases on that repository have precompiled `clang` binaries
which are pre-configured to compile for WebAssembly.

[WASI SDK]: https://github.com/webassembly/wasi-sdk

To start in C and C++ a `*.c` and `*.h` header file is generated for your
project to use. These files are generated with the [`wit-bindgen` CLI
command][cli-install] in this repository.

```sh
wit-bindgen c ./wit
# Generating "host.c"
# Generating "host.h"
# Generating "host_component_type.o"
```

Some example code using this would then look like

```c
// my-component.c

#include "host.h"

void host_run() {
    host_string_t my_string;
    host_string_set(&my_string, "Hello, world!");

    host_print(&my_string);
}
```

This can then be compiled with `clang` from the [WASI SDK] and assembled into a
component with:

```sh
clang host.c host_component_type.o my-component.c -o my-core.wasm -mexec-model=reactor
wasm-tools component new ./my-core.wasm -o my-component.wasm
```

Like with Rust, you can then inspect the output binary:

```sh
wasm-tools component wit ./my-component.wasm
```


### Guest: Java

Java bytecode can be compiled to WebAssembly using
[TeaVM-WASI](https://github.com/fermyon/teavm-wasi). With this generator,
`wit-bindgen` will emit a `*.java` file which may be used with any JVM language,
e.g. Java, Kotlin, Clojure, Scala, etc.

### Guest: TinyGo

The Go language currently can be compiled to standalone out-of-browser
WebAssembly binaries using the [TinyGo](https://tinygo.org/) toolchain.
Support does not yet exist in this repository for TinyGo but is being worked on
[in a PR](https://github.com/bytecodealliance/wit-bindgen/pull/321) and is
expected to land in this repository at some point in the future.

### Guest: Other Languages

Other languages such as JS, Ruby, Python, etc, are hoped to be supported one day
with `wit-bindgen` or with components in general. It's recommended to reach out
on [zulip] if you're intersted in contributing a generator for one of these
langauges. It's worth noting, however, that turning an interpreted language into
a component is significantly different from how compiled languages currently
work (e.g. Rust or C/C++). It's expected that the first interpreted language
will require a lot of design work, but once that's implemented the others can
ideally relatively quickly follow suit and stay within the confines of the
first design.

## CLI Installation
[cli-install]: #cli-installation

To install the CLI for this tool (which isn't the only way it can be used), run
the following cargo command. This will let you generate the bindings for any
supported language.

```
cargo install --git https://github.com/bytecodealliance/wit-bindgen wit-bindgen-cli
```

This CLI **IS NOT** stable and may change, do not expect it to be or rely on it
being stable. Please reach out to us on [zulip] if you'd like to depend on it,
so we can figure out a better alternative for your use case.


## Host Runtimes for Components
[hosts]: #host-runtimes-for-components

The `wit-bindgen` project is intended to facilitate in generating a component,
but once a component is in your hands the next thing to do is to actually
execute that somewhere. This is not under the purview of `wit-bindgen` itself
but these are some resources and runtimes which can help you work with
components:

* Rust: the [`wasmtime` crate](https://docs.rs/wasmtime) is an implementation of
  a native component runtime that can run any WIT `world`. It additionally comes
  with a [`bindgen!`
  macro](https://docs.rs/wasmtime/latest/wasmtime/component/macro.bindgen.html)
  which acts similar to the `generate!` macro in this repository. This macro
  takes a [WIT] package as input and generates `trait`-based bindings for the
  runtime to implement and use.

* JS: the [`js-component-tools`] project can be used to execute components in JS
  either on the web or outside the browser in a runtime such as `node`. This
  project generates a polyfill for a single concrete component to execute in a
  JS environment by extracting the core WebAssembly modules that make up a
  component and generating JS glue to interact between the host and these
  modules.

* Python: the [`wasmtime`](https://github.com/bytecodealliance/wasmtime-py)
  project [on PyPI](https://pypi.org/project/wasmtime/) has a `bindgen` mode
  that works similar to the JS integration. Given a concrete component this will
  generate Python source code to interact with the component using an embedding
  of Wasmtime for its core WebAssembly support.

* Tooling: the [`wasm-tools`] project can be used to inspect and modify
  low-level details of components. For example as previously mentioned you can
  inspect the WIT-based interface of a component with `wasm-tools component
  wit`. You can link two components together with `wasm-tools compose` as well.

[`js-component-tools`]: https://github.com/bytecodealliance/js-component-tools

Note that the runtimes above are generally intended to work with arbitrary
components, not necessarily only those created by `wit-bindgen`. This is also
not necessarily an exhaustive listing of what can execute a component.
