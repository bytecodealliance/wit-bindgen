# Overview
WIT Bindgen

## Host Bindings Support
Host bindings in a language allows code in that language to talk to WASM modules using bindings generated from a WIT interface. This requires that that language has the ability to either embed a WASM runtime like `wasmtime` or interact with one built in to the language runtime (like V8).

## Guest Bindings Support
Guest bindings in a language allow WASM modules written in that language to implement interfaces defined by WIT interfaces. This requires that the language can be compiled to WASM.

# WIT Support Table
<!-- Whenever you update the status table, -->
<!-- make sure the corresponding document is also updated. -->

| Language | Host Bindings | Guest Bindings |
| - | - | - |
| Rust | ✔️ ([details](./langs/RUST.md#host-bindings))| ✔️ ([details](./langs/RUST.md#guest-bindings))|
| JS | ✔️ ([details](./langs/JS.md)) | ❌ |
| Python | ✔️ ([details](./langs/PYTHON.md)) | ❌ |
| Go | ⏳ ([details](./langs/GO.md)) | ❌ |
| C++ | ⏳ ([details](./langs/CPP.md)) | ❌ |

## Key
| Symbol | Description |
| - | - |
| ✔️ | Support for WIT |
| ⏳ | Partial/WIP support for WIT |
| ❌ | No support for WIT |
