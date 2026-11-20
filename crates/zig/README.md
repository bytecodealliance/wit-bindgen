# `wit-bindgen-zig`

Guest bindings generator for [Zig](https://ziglang.org/) targeting [WIT](https://component-model.bytecodealliance.org/design/wit.html) and the [WebAssembly Component Model](https://component-model.bytecodealliance.org/).

`wit-bindgen-zig` emits pure Zig source files backed by a comptime-driven Canonical ABI layer (`abi.zig`), with no C headers or external runtime dependencies required.

## Prerequisites

- **Zig `0.16.0`** (or compatible)
- **`wit-bindgen-cli`**: `cargo install wit-bindgen-cli`
- **`wasm-tools`**: `cargo install wasm-tools` (for embedding WIT metadata and wrapping core Wasm modules into components)
- **`wasi_snapshot_preview1.reactor.wasm`** (optional, only required when targeting `wasm32-wasi` and importing WASI Preview 1 functions from Zig's standard library; available from [Wasmtime releases](https://github.com/bytecodealliance/wasmtime/releases/latest))

---

## Quickstart

Suppose you have the following WIT world in `wit/world.wit`:

```wit
package example:greeter;

interface host {
    log: func(msg: string);
}

interface guest {
    greet: func(name: string) -> string;
}

world greeter {
    import host;
    export guest;
}
```

### 1. Generate bindings

Run `wit-bindgen zig` pointing to your `wit` directory:

```sh
wit-bindgen zig ./wit --out-dir src/bindings
```

This produces:

```text
src/bindings/
├── wit.zig
└── wit/
    ├── abi.zig
    ├── exports.zig
    ├── guest.zig
    └── host.zig
```

### 2. Implement the guest component

Create `src/main.zig`, import the generated `wit` module, and register your exported functions inside a `comptime` block:

```zig
const std = @import("std");
const wit = @import("wit");

comptime {
    wit.exports.greet(greet);
}

fn greet(name: []const u8) []const u8 {
    wit.host.log("Formatting greeting...");
    return std.fmt.allocPrint(wit.abi.arena(), "Hello, {s}!", .{name}) catch @panic("OOM");
}
```

### 3. Compile to a WebAssembly Component

#### Option A: Using `zig build-exe` directly

Compile the guest as a Wasm reactor and wrap it into a component with `wasm-tools`:

```sh
zig build-exe \
  -target wasm32-wasi \
  -O ReleaseSmall \
  -fno-entry \
  -rdynamic \
  -mexec-model=reactor \
  --dep wit \
  -Mroot=src/main.zig \
  -Mwit=src/bindings/wit.zig \
  -femit-bin=greeter.core.wasm

wasm-tools component embed ./wit greeter.core.wasm -o greeter.embedded.wasm
wasm-tools component new greeter.embedded.wasm \
  --adapt wasi_snapshot_preview1.reactor.wasm \
  -o greeter.component.wasm
```

#### Option B: Using `build.zig`

In a standard Zig project (`build.zig`), expose `src/bindings/wit.zig` as a module import on your WebAssembly executable target:

```zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseSmall,
    });

    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .wasi,
    });

    const wit_mod = b.createModule(.{
        .root_source_file = b.path("src/bindings/wit.zig"),
        .target = target,
        .optimize = optimize,
    });

    const root_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
        .imports = &.{
            .{ .name = "wit", .module = wit_mod },
        },
    });

    const exe = b.addExecutable(.{
        .name = "greeter",
        .root_module = root_mod,
    });
    exe.entry = .disabled;
    exe.rdynamic = true;
    exe.wasi_exec_model = .reactor;

    b.installArtifact(exe);
}
```

After running `zig build`, run `wasm-tools component embed` and `wasm-tools component new` on `zig-out/bin/greeter.wasm` as shown above.

---

## CLI Options

```sh
wit-bindgen zig [OPTIONS] <WIT>
```

| Option | Default | Description |
| :--- | :--- | :--- |
| `--name <NAME>` | `wit` | Base name for the generated root `.zig` file (`<name>.zig`) and directory (`<name>/`). |
| `--abi-path <PATH>` | *(bundled)* | Custom `@import(...)` path for `abi.zig` relative to `<name>/<interface>.zig`. When specified, `<name>/abi.zig` is omitted and generated modules import `<PATH>` instead. |
| `--world <WORLD>` | *(default world)* | WIT world to generate bindings for. |
| `--out-dir <DIR>` | `.` | Output directory for the generated files. |

---

## Generated File Layout

For a base name `wit` (the default), `wit-bindgen zig` generates:

- **`wit.zig`**: Root namespace module re-exporting `abi`, `root` (if the world has top-level types or imported functions), every interface module, and `exports` (if the world has exports).
- **`wit/abi.zig`**: Self-contained comptime Canonical ABI implementation (`cabi_realloc`, flattening, linear memory layout/load/store, per-call arena, and resource handle tracking).
- **`wit/root.zig`**: Emitted when the world defines top-level types or imports top-level functions outside of an interface.
- **`wit/<interface>.zig`**: One file per imported or exported WIT interface containing its Zig types, resource types, and imported function wrappers. If two interfaces share the same short name, they are disambiguated with their package prefix (`<namespace>_<package>_<interface>.zig`).
- **`wit/exports.zig`**: Emitted when the world exports functions or interfaces. Each exported function has a registration helper `pub fn <funcName>(comptime impl: fn (...) ...) void` that wires up `@export` for both the core entry point and any required `cabi_post_*` function.

---

## Type Mapping

WIT types map directly to idiomatic Zig types:

| WIT Type | Zig Type | Notes |
| :--- | :--- | :--- |
| `bool` | `bool` | |
| `u8`, `u16`, `u32`, `u64` | `u8`, `u16`, `u32`, `u64` | |
| `s8`, `s16`, `s32`, `s64` | `i8`, `i16`, `i32`, `i64` | |
| `f32`, `f64` | `f32`, `f64` | |
| `char` | `u21` | Unicode scalar value |
| `string` | `[]const u8` | UTF-8 slice |
| `list<T>` | `[]const T` | |
| `list<T, N>` | `[N]T` | Fixed-length array |
| `tuple<T1, T2, ...>` | `struct { T1, T2, ... }` | Anonymous tuple struct |
| `option<T>` | `?T` | |
| `result<T, E>` | `wit.abi.Result(T, E)` | `union(enum) { ok: T, err: E }` (`void` when `_` is omitted) |
| `record` | `struct { ... }` | `snake_case` field names |
| `variant` | `union(enum) { ... }` | Tagged union with `snake_case` cases |
| `enum` | `enum(u8)` / `enum(u16)` / `enum(u32)` | Tag width chosen from case count |
| `flags` | `packed struct(u8\|u16\|u32)` | `bool` fields defaulting to `false`, plus padding `_` |
| `own<R>` | `R` (`enum(u32) { _, ... }`) | Non-exhaustive `u32` handle enum with methods |
| `borrow<R>` | `R` (in import parameters) | Automatically wrapped via `abi.borrow(...)` at call sites |

### Naming Conventions

- **Types** (`record`, `variant`, `enum`, `flags`, `resource`, `type` aliases): `UpperCamelCase` (`record player-pos` -> `pub const PlayerPos = struct { ... };`).
- **Functions & Methods**: `lowerCamelCase` (`get-position` -> `pub fn getPosition(...)`). Resource `[constructor]` functions are named `init`.
- **Fields, Enum/Variant Cases, Flag Members & Parameters**: `snake_case`. Identifiers that collide with Zig keywords or primitives are escaped with `@"..."`.

---

## Memory & Resource Lifetimes

### Per-Call Arena (`wit.abi.arena()`)

All values lifted from the host during an exported function call (strings, slices, nested aggregates) and temporary buffers allocated by `cabi_realloc` live in a per-call arena accessed via `wit.abi.arena()`.

- When the outermost export finishes (including its `cabi_post_*` callback for indirect returns), `abi.endCall()` resets the arena while retaining capacity.
- **Returning slices/strings from exports**: Allocate return data on `wit.abi.arena()` (or return static slices); `cabi_post_*` frees the arena after the host has read the returned memory.
- **Persisting data across export calls**: Data allocated on `wit.abi.arena()` does not survive past the current export call. To store data in global state across calls, clone it using `wit.abi.heap` (`std.heap.wasm_allocator` on `wasm32`).

### Imported Resource Handles

Owned resource handles (`own<R>`) received during an export call—whether passed as arguments or returned from imported functions/constructors—are automatically tracked for the duration of the export call:

- **Automatic cleanup**: Any owned handle still tracked when the export returns is automatically dropped via `[resource-drop]R`.
- **Consuming calls**: Passing an owned handle to an imported function that takes `own<R>` untracks it so ownership transfers cleanly to the host.
- **Borrowing**: Imported functions and resource methods that take `borrow<R>` accept `R` directly in Zig and lend the handle without consuming it.
- **Early release (`.deinit()`)**: Call `handle.deinit()` to drop an owned handle immediately before the export returns.
- **Persisting across calls (`.keep()`)**: Call `handle.keep()` to remove a handle from automatic per-call cleanup so it can be stored in guest state across exports. Once kept, you are responsible for eventually releasing it with `handle.deinit()`.

---

## Current Limitations

- **Guest-exported resources**: Importing host resources (`own<R>`, `borrow<R>`, constructors, methods, and static functions) is supported, but exporting guest-defined resources is not yet implemented.
- **Component Model Async**: `stream<T>`, `future<T>`, `error-context`, and `async` functions are not yet supported.
