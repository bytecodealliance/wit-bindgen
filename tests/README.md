# Testing witx-bindgen

A brief overview of tests in `witx-bindgen`:

* Any `*.witx` file placed in this directory will get imports/exports generated
  for Rust/Wasmtime.

* Codegen tests happen through:
  * `crates/gen-rust-wasm/tests/run.rs`
  * `crates/gen-wasmtime/tests/run.rs`
  * `crates/test-codegen/src/lib.rs`

* Codegen tests generate `rustfmt`-d output into `$OUT_DIR` so error messages
  have filenames and line numbers so you can go inspect the source and see
  what's up.

* Codegen tests (`tests/run.rs`) can be edited to test only one or two `*.witx`
  files at a time by editing the macro invocations.

Not included in this directory are some other tests:

* `test-wasmtime`, when run, executes `host.witx` and `wasm.witx` to ensure
  runtime-correctness of bindings

  ```
  $ cargo run -p test-wasmtime
  ```
