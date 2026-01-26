# `wit-bindgen` MoonBit Bindings Generator

This crate implements the MoonBit guest bindings generator for `wit-bindgen`.

## Testing

The repository’s `wit-bindgen test` subcommand is the preferred way to run MoonBit
codegen/runtime tests. See `tests/README.md` for full details.

### Codegen

```sh
cargo run test \
  --languages rust,moonbit \
  --artifacts target/artifacts \
  --rust-wit-bindgen-path ./crates/guest-rust \
  tests/codegen
```

### Runtime (async)

```sh
cargo run test --languages rust,moonbit tests/runtime-async \
  --artifacts target/artifacts \
  --rust-wit-bindgen-path ./crates/guest-rust \
  --runner "wasmtime -W component-model-async"
```

