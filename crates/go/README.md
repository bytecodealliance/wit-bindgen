# `wit-bindgen` Go Bindings Generator

This tool generates Go bindings for a chosen WIT world.

## Contributing

If changes need to be made to `wit-bindgen-go`, here are the steps that need to be taken:
- Make the required changes to the [bytecodealliance/go-pkg](https://github.com/bytecodealliance/go-pkg) Go files and tag a release.
- Update the `crates/go/src/pkg` git submodule to reflect the most-recent release of `go-pkg`.
- Update the `REMOTE_PKG_VERSION` constant in [lib.rs](./src/lib.rs) to reflect the most-recent release of `go-pkg`.
- Make the required changes to `wit-bindgen-go`.

## Usage

The easiest way to use `wit-bindgen-go` is through the [componentize-go](https://github.com/bytecodealliance/componentize-go) tool. See below for using `wit-bindgen-go` directly.

To generate bindings with this crate, issue the `go` subcommand to `wit-bindgen`:

```bash
$ wit-bindgen go [OPTIONS] <WIT>
```

See the output of `wit-bindgen help go` for available options.

This command will generate a variable number of files, depending on the WIT
world provided:

- `go.mod`: defines a minimal Go module with the name `wit_component`
    - You can replace this with your own version (e.g. referencing third party dependencies) if desired
- `wit_bindings.go`: defines the `main` package for the module, including low-level, `//go:export`-annotated entrypoint functions corresponding to exported functions
    - These entrypoint functions in turn call high-level functions which must be provided by the application developer
- `go.bytecodealliance.org/pkg/wit/runtime`: defines low-level functions for supporting the component model ABI
- `<name>/wit_bindings.go`: defines any types generated for the interface named `<name>` (or `wit_world` for WIT types defined at the world level), plus any imported functions
    - Note that the types placed in these files include all types for both imported and exported interfaces, except for exported resource types and any types which depend on exported resource types
- `export_<name>/wit_bindings.go`: defines intrinsics for use with any exported resource types generated for the interface named `<name>` (or `wit_world` for WIT types defined at the world level), plus any types which depend on those exported resource types, plus any exported functions
    - The exported resource type definitions must be provided by the application developer
    - The `export_<name>` package is also the place to define any exported functions
- (if needed) `go.bytecodealliance.org/pkg/wit/types`:
  - defines `Tuple<N>` types as required by the WIT world
  - defines an `Option` type as required by the WIT world
  - defines a `Result` type as required by the WIT world
  - defines a `Unit` type as required by the WIT world
  - defines a `StreamReader` and `StreamWriter` types as required by the WIT world
  - defines a `FutureReader` and `FutureWriter` types as required by the WIT world
- (if needed) `go.bytecodealliance.org/pkg/wit/async`: defines low-level functions for integrating the Go scheduler with the component model async ABI

Note that async support currently requires [a patched version of
Go](https://github.com/dicej/go/releases/tag/go1.25.5-wasi-on-idle).  Code
generated for worlds that don't use any async features can be compiled using a
stock release of Go.

## Example

### Prerequisites

- `wit-bindgen`
- `wasm-tools`
- `wasmtime`
- `curl`
- `bash` or similar

Given the following WIT file, we can generate bindings for it, write code to
target the two worlds, and finally compose, build, and run the components.

```shell
cat >world.wit <<EOF
package test:test;

interface foo {
  record thing {
    a: s32,
    b: string,
  }

  echo: func(x: thing) -> tuple<s32, string>;
}

world test {
  export foo;
}

world runner {
  import foo;
  export run: func() -> tuple<s32, string>;
}
EOF

curl -OL https://github.com/bytecodealliance/wasmtime/releases/download/v39.0.1/wasi_snapshot_preview1.reactor.wasm
mkdir test runner

pushd test
wit-bindgen go -w test ../world.wit
mkdir export_test_test_foo
cat >export_test_test_foo/test.go <<EOF
package export_test_test_foo

import . "wit_component/test_test_foo"

func Echo(x Thing) (int32, string) {
    return x.A, x.B
}
EOF
go mod tidy
GOARCH="wasm" GOOS="wasip1" go build -o core.wasm -buildmode=c-shared -ldflags=-checklinkname=0
wasm-tools component embed -w test ../world.wit core.wasm -o core-with-wit.wasm
wasm-tools component new --adapt ../wasi_snapshot_preview1.reactor.wasm core-with-wit.wasm -o component.wasm
popd

pushd runner
wit-bindgen go -w runner ../world.wit
mkdir export_wit_world
cat >export_wit_world/runner.go <<EOF
package export_wit_world

import . "wit_component/test_test_foo"

func Run() (int32, string) {
    return Echo(Thing{42, "hello, world!"})
}
EOF
go mod tidy
GOARCH="wasm" GOOS="wasip1" go build -o core.wasm -buildmode=c-shared -ldflags=-checklinkname=0
wasm-tools component embed -w runner ../world.wit core.wasm -o core-with-wit.wasm
wasm-tools component new --adapt ../wasi_snapshot_preview1.reactor.wasm core-with-wit.wasm -o component.wasm
popd

wasm-tools compose -d test/component.wasm runner/component.wasm -o component.wasm
wasmtime run --invoke 'run()' component.wasm
```

If all goes well, you should see `(42, "hello, world!")`.
