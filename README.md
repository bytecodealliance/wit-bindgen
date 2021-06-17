# WebAssembly Module Linker

_Please note: this is an experimental project._

`wasmlink` is a prototype [WebAssembly](https://webassembly.org/) module linker that can link together a module and its dependencies using [module linking](https://github.com/WebAssembly/module-linking) and the [Canonical Interface Types ABI](https://github.com/WebAssembly/interface-types/pull/132).

When used in combination with [witx-bindgen](https://github.com/bytecodealliance/witx-bindgen), it is capable of generating interface adapter functions in WebAssembly that enables WebAssembly modules to exchange interface types such as strings, records, and lists.

## Building

To build `wasmlink`:

```text
$ cargo build
```

## Running

To run `wasmlink`:

```text
$ cargo run
```

## Testing

To run tests, first install the `wasm32-wasi` target:

```text
$ rustup target install wasm32-wasi
```

With the target installed, run the tests:

```text
$ cargo test --all
```

## Demo

### Prerequisites

The demo requires [cargo-wasi](https://github.com/bytecodealliance/cargo-wasi), so install it using `cargo`:

```text
$ cargo install cargo-wasi
```

### Building the `markdown` module

The `markdown` module exposes an interface consisting of a `render` function that takes a string (the [Markdown](https://en.wikipedia.org/wiki/Markdown)) as an argument and returns a string (the rendered HTML).

The interface for the `markdown` module is:

```witx
render: function(markdown: string) -> string
```

To build the `markdown` module:

```text
$ cd demo/markdown
$ cargo wasi build
$ cp markdown.witx target/wasm32-wasi/debug/markdown.witx
```

_Note: the linker currently expects either an embedded witx file in a custom section of the module or a witx file of the same name next to the input wasm module, so we copy the witx file to the target directory above._

### Building the `renderer` module

The `renderer` module will read input via `stdin`, pass the input as a string to the `render` function from the `markdown` module, and then print the returned HTML to `stdout`.

To build the `renderer` module:

```text
$ cd demo/renderer
$ cargo wasi build
```

### Linking the two modules together

With the two modules now built, it is time to link them together so that they can be run directly with [Wasmtime](https://github.com/bytecodealliance/wasmtime):

```text
$ cargo run --release -- -i markdown=demo/markdown/target/wasm32-wasi/debug/markdown.wasm -p wasmtime -o linked.wasm demo/renderer/target/wasm32-wasi/debug/renderer.wasm
```

This command produces a linked module named `linked.wasm` in the current directory.

### Running the linked module

As the linked module uses features from both the [module linking](https://github.com/WebAssembly/module-linking) and [multi-memory](https://github.com/WebAssembly/multi-memory) WebAssembly proposals, support has to be explicitly enabled in Wasmtime to enable the module to run.

To run the linked module:

```text
$ echo '# Hello\nworld' | wasmtime --enable-module-linking --enable-multi-memory linked.wasm
```

If everything worked correctly, this should render the Markdown echoed on the command line:

```markdown
# Hello
world
```

as the following HTML:

```html
<h1>Hello</h1>
<p>world</p>
```
