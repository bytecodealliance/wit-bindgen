# `wit-bindgen-moonbit`

MoonBit language bindings generator for WIT and the Component Model.

## Usage

Generate bindings via the `moonbit` subcommand:

```bash
wit-bindgen moonbit [OPTIONS] <WIT>
```

See `wit-bindgen help moonbit` for available options.

## Local async usage

For pure MoonBit code (no FFI), you can create local future/stream pairs.

Future + Promise:

```mbt
let (f, p) = @async.Future::new[Int]()
@async.spawn(async fn() { p.write(42) })
let value = f.get()
```

Stream + Sink (batched reads/writes):

```mbt
let (s, sink) = @async.Stream::new[Byte]()
@async.spawn(async fn() {
  let chunk : Array[Byte] = [1, 2, 3, 4]
  let _ = sink.write(chunk[:])
  sink.close()
})
let chunk = s.read(4096)
match chunk {
  None => ()
  Some(bytes) => {
    let _ = bytes.length()
  }
}
```

`Stream::read(count)` returns up to `count` elements; `Sink::write` accepts
`ArrayView[T]` so byte streams can batch data efficiently. `Stream::new`
accepts an optional `capacity` (<= 0 means unbounded).

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
