# `wit-bindgen-moonbit`

MoonBit language bindings generator for WIT and the Component Model.

## Usage

Generate bindings via the `moonbit` subcommand:

```bash
wit-bindgen moonbit [OPTIONS] <WIT>
```

See `wit-bindgen help moonbit` for available options.

## Testing

From the repo root, run the MoonBit codegen tests:

```bash
cargo run test \
  --languages rust,moonbit \
  --artifacts target/artifacts \
  --rust-wit-bindgen-path ./crates/guest-rust \
  tests/codegen
```

And the async runtime tests (requires an async component-model runner):

```bash
cargo run test --languages rust,moonbit tests/runtime-async \
  --artifacts target/artifacts \
  --rust-wit-bindgen-path ./crates/guest-rust \
  --runner "wasmtime -W component-model-async"
```
