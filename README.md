# test_interactive Component

Generated scaffolding for WIT world `basic-test`.

## Getting Started

1. **Implement the functions** marked with `TODO` in `src/lib.rs`
2. **Build the component**:
   ```bash
   cargo build --target wasm32-wasip2
   ```
3. **Validate your implementation**:
   ```bash
   wit-bindgen validate wit/
   ```

## Development Tips

- Use `show_module_paths: true` in the `wit_bindgen::generate!` macro to see generated module paths
- Test your WIT files with `wit-bindgen validate` before implementing
- Use `cargo expand` to see the generated bindings code

## Project Structure

- `src/lib.rs` - Main component implementation
- `wit/` - WIT interface definitions  
- `Cargo.toml` - Rust project configuration

## Building for Production

```bash
cargo build --target wasm32-wasip2 --release
wasm-tools component new target/wasm32-wasip2/release/test_interactive.wasm -o component.wasm
```
