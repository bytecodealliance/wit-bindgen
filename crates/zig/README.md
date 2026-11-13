# `wit-bindgen-zig`

Zig bindings generator for WIT and the WebAssembly Component Model.

## Usage

```sh
wit-bindgen zig ./wit --out-dir src/bindings
```

By default, this generates:
- `wit.zig`: Root module re-exporting all interfaces, `exports`, and `abi`.
- `wit/abi.zig`: Self-contained comptime Canonical ABI runtime.
- `wit/<interface>.zig`: Types, resource wrappers, and imported functions for each WIT interface.
- `wit/exports.zig`: `@export` registration helpers for world exports.

### Options

- `--name <NAME>`: Base name for the generated root `.zig` file and directory (defaults to `wit`).
- `--abi-path <PATH>`: Custom import path for `abi.zig` relative to `<name>/<interface>.zig`. When provided, `<name>/abi.zig` is not emitted and generated modules import `<PATH>` instead.
