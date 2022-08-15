# Testing wit-bindgen - `codegen

Any tests placed into the `tests/codegen` directory should be raw `*.wit`
files. These files will be executed in all code generators by default most
likely, and the purpose of these files is to execute language-specific
validation for each bindings generator. Basically if there's a bug where
something generates invalid code then this is probably where the test should go.
Note that this directory can have whatever it wants since nothing implements the
interfaces or tries to call them.

# Testing wit-bindgen - `runtime`

Otherwise tests are organized in `tests/runtime/*`. Inside this directory is a
directory-per-test. These tests are somewhat heavyweight so you may want to
extend existing tests, but it's also fine to add new tests at any time.

The purpose of this directory is to contain code that's actually compiled to
wasm and executed on hosts. The code compiled-to-wasm can be one of:

* `wasm.rs` - compiled with Rust to WebAssembly
* `wasm.c` - compiled with Clang

Existence of these files indicates that the language should be supported for the
test, and if a file is missing then it's skipped when running other tests. Each
`wasm.*` file is run inside each of the host files:

* `host.rs` - executes wasms with Wasmtime
* `host.js` - executes WebAssembly with node.js
* `host.py` - executes with `wasmtime`'s PyPI package.

Each of these hosts can also be omitted if the host doesn't implement the test
or something like that. Otherwise for each host that exists when the host's
crate generator crate is tested it will run all these tests.

# Testing Layout

If you're adding a test, all you should generally have to do is edit files in
`tests/runtime/*`. If you're adding a new test it *should* be along the lines of
just dropping some files in there, but currently if you're adding a
Rust-compiled-to-wasm binary you'll need to edit
`crates/test-rust-wasm/Cargo.toml` and add a corresponding binary to
`crates/test-rust-wasm/src/bin/*.rs` (in the same manner as the other tests).
Other than this though all other generators should automatically pick up new
tests.

The actual way tests are hooked up looks roughly like:

* All generator crates have a `codegen.rs` and a `runtime.rs` integration test
  file (typically defined in the crate's own `tests/*.rs` directory).
* All generator crates depend on `crates/test-helpers`. This crate will walk the
  appropriate directory in the top-level `tests/*` directory.
* The `test-helpers` crate will generate appropriate `#[test]` functions to
  execute tests. For example the JS generator will run `eslint` or `tsc`. Rust
  tests for `codegen` are simply that they compile.
* The `test-helpers` crate also builds wasm files at build time, both the Rust
  and C versions. These are then encoded into the generated `#[test]` functions
  to get executed when testing.

The general layout is then that if you want to run the JS host tests you run:

```
$ cargo test -p wit-bindgen-gen-host-js
```

and if you want to run all tests you can execute:

```
$ cargo test --workspace
```

It's all a bit convoluted so feel free to ask questions on Zulip or open an
issue if you're lost.
