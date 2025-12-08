# `wit-bindgen` Go Bindings Generator

This tool generates Go bindings for a chosen WIT world.

## Usage

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
- `wit_runtime/wit_runtime.go`: defines low-level functions for supporting the component model ABI
- `<name>/wit_bindings.go`: defines any types generated for the interface named `<name>` (or `wit_world` for WIT types defined at the world level), plus any imported functions
    - Note that the types placed in these files include all types for both imported and exported interfaces, except for exported resource types and any types which depend on exported resource types
- `export_<name>/wit_bindings.go`: defines intrinsics for use with any exported resource types generated for the interface named `<name>` (or `wit_world` for WIT types defined at the world level), plus any types which depend on those exported resource types, plus any exported functions
    - The exported resource type definitions must be provided by the application developer
    - The `export_<name>` package is also the place to define any exported functions
- (if needed) `wit_types/wit_tuples.go`: defines `Tuple<N>` types as required by the WIT world
- (if needed) `wit_types/wit_async.go`: defines low-level functions for integrating the Go scheduler with the component model async ABI
- (if needed) `wit_types/wit_option.go`: defines an `Option` type if required by the WIT world
- (if needed) `wit_types/wit_result.go`: defines an `Result` type if required by the WIT world
- (if needed) `wit_types/wit_unit.go`: defines an `Unit` type if required by the WIT world
- (if needed) `wit_types/wit_stream.go`: defines a `StreamReader` and `StreamWriter` types if required by the WIT world
- (if needed) `wit_types/wit_future.go`: defines a `FutureReader` and `FutureWriter` types if required by the WIT world

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
GOARCH="wasm" GOOS="wasip1" go build -o core.wasm -buildmode=c-shared -ldflags=-checklinkname=0
wasm-tools component embed -w runner ../world.wit core.wasm -o core-with-wit.wasm
wasm-tools component new --adapt ../wasi_snapshot_preview1.reactor.wasm core-with-wit.wasm -o component.wasm
popd

wasm-tools compose -d test/component.wasm runner/component.wasm -o component.wasm
wasmtime run --invoke 'run()' component.wasm
```

If all goes well, you should see `(42, "hello, world!")`.
