# `wit-bindgen-test`

This folder contains the `wit-bindgen-test` crate which is used to power the
`wit-bindgen test` subcommand of the `wit-bindgen` CLI. The purpose of this
document is to document what this subcommand does and how it enables testing.

## Testing `wit-bindgen`

The goal of the `test` subcommand is to make it as easy as possible to test
bindings generators and their functionality. It's also intended to enable
testing various kinds of functionality of a bindings generator in terms of
code generator flags, language flags, etc. This is a pretty generic problem
though since this isn't just one bindings generator but instead a bindings
generator for many languages. There's additional overlap where component runtime
hosts may want similar tests as everything, if you squint hard enough, looks
like it's all testing in a similar fashion.

To help scope this problem down, the goals of this tool are:

* Enable easily testing **guest** bindings generators, aka those that compile
  code to WebAssembly and produce a WebAssembly component.
* One category of tests are **codegen tests**. Codegen tests take a `*.wit` file
  as input and assert that both bindings can be generated and the generated
  bindings are valid. Validity of the generated bindings is determined by the
  guest language itself, and this step typically doesn't involve creating a
  WebAssembly component binary.
* The second category of tests are **runtime tests**. Runtime tests are
  goverened by a `test.wit` which contains a `runner` world and a `test` world.
  There are then "runner" components defined with the file prefix `runner`, for
  example `runner.rs`. To complement these there are "test" components, such as
  `test.rs`. The runner is composed with the test to create a single component
  binary which looks like a WASI CLI program. This program is then run to
  completion.

Built on these goals the `wit-bindgen test` subcommand has a number of sub-goals
such as parallelizing tests, making iteration fast, good error messages, etc.
This is all feeding towards the highest-level goal of making it easy to write
tests in any language that has a bindings generator and can compile to
components.

## CLI Interface

The `wit-bindgen test` subcommand can be explored with:

```
$ wit-bindgen test -h
```

The main arguments to the subcommand are directories that contain tests and a
`--artifacts` path which contains where to store temporary build artifacts, such
as compiled component binaries. For example:

```
$ wit-bindgen test ./tests --artifacts ./target/artifacts
```

This will look recursively in the `./tests` directory for tests. Test discovery
is detailed below in runtime and codegen tests. During test execution any
intermediate artifacts are present in `./target/artifacts` at a per-test stable
location to assist with debugging. For example if invalid code is generated it
can inspected within `./target/artifacts`.

Some other basic flags are:

* `-f` or `--filter` - a regex-based filter on which tests to run, used to run
  only a single test if desired. Note that running a single test can also be
  done by passing a narrower `./tests` directory, such as
  `./tests/codegen/my-test.wit`.

* `-i` or `--inherit-stderr` - this is used to have subprocesses inherit stderr
  from the calling process which can be useful when guest language compilers
  produce colored error messages for example as otherwise stderr is captured
  from subcommands by default meaning that colors won't show up.

* `-l` or `--languages` - By default all languages that `wit-bindgen test`
  supports are enabled. This means that if you don't have development toolchains
  installed locally tests may fail. This flag can be used to filter languages to
  test (e.g. `--languages rust`) or to disable specific languages (e.g.
  `--languages=-rust`).

* `--runner` - Runtime tests are executed within a WebAssembly component runtime
  and this is the path to a custom runtime to use. By default `wasmtime` is used
  but any other runtime can be supplied.

## Codegen Tests

The first category of tests that `wit-bindgen test` supports are called "codegen
tests". These tests are a `*.wit` file which contains a single `world` within
it. These files are used as input to a language bindings generator and then the
output is compiled by the target language to ensure that valid bindings were
generated.

These tests do not produce a complete component. Instead the validity of the
generated bindings are up to the target language. For example in Rust the
bindings are compiled with `--crate-type rlib`, in C the bindings are compiled
to an object file, and interpreted languages might run various lints for
example.

Codegen tests are discovered inside of a directory called `codegen`. Internally
all `codegen/*.wit` files are then run as tests. By default all supported
languages of `wit-bindgen test` are run for each `codegen/*.wit` test file.

#### Testing code generator options

By default each language only tests the default settings of the bindings
generator. To have all tests also tested with more options you'll want to update
the `LanguageMethods::codegen_test_variants` method. If this is a non-empty
array then each entry will run each codegen test through those options as well,
effectively testing codegen tests in more than one configuration.

#### Ignoring classes tests

Ignoring classes of tests can be done in the CLI tool by updating a few
locations:

* Update `WitConfig` to contain a field for this class of test
  that needs to be ignored (if it's not already present).
* Tag tests as belonging to this class of tests by adding a comment at the top
  such as `//@ async = true` which would indicate that this uses async features.
* Update `LanguageMethods::should_fail_verify` for your language to ignore this
  class of tests by checking the `WitConfig` config option and returning
  `true` for "should fail"

This will still run the test but an error will be expected. If an error is
generated then the test will be considered to have passed. If the test instead
passes, however, then the test will be considered to have failed and the
`should_fail_verify` method will need to be updated.

#### Ignoring a single test

If a single test is problematic and doesn't fall into a "class" of tests like
above then the `LanguageMethods::should_fail_verify` method should be updated
and the `name` field should be consulted. This is the name of the test itself
and that can be used to expect failure in individual tests.

## Runtime Tests

The second class of tests supported by `wit-bindgen test` is what are called
"runtime tests". The goal of runtime tests is to not only test that generated
code is valid but it additionally produces a valid component that works at
runtime. These tests have the following structure:

```
my-test/
    test.wit        # WIT `test` and `runner` worlds
    runner.rs       # Implementation of `runner` in Rust
    runner.c        # Implementation of `runner` in C
    test.go         # Implementation of `test` in Go
    test2.go        # Another implementation of `test` in Go
```

Each folder must contain a `test.wit` file. This WIT file must contain at least
two worlds: `runner` and `test`. The `runner` world imports functionality and
the `test` world exports functionality.

Each `runner*` file is compiled, using the language-specific toolchain and
bindings, into a component. This is then additionally done for the `test*`
files. Bindings are automatically generated and provided to the compilation
phase and each language has its own conventions of how to assemble everything
into a component.

Once components are produced the matrix of `runner x test` is produced to
compose together. Each runner and test are composed to produce a single
component which is a test case. For example the above example would have four
test components produced:

* `runner.rs x test.go`
* `runner.rs x test2.go`
* `runner.c x test.go`
* `runner.c x test2.go`

Each test component is then run with the `--runner` CLI option (or `wasmtime` by
default).

The `runner` component is expected to export a `wasi:cli/run` interface
according to language specific conventions (e.g. it has `fn main() { .. }` for
Rust). Both the `runner` and `test` component can access other WASI APIs such as
printing to standard out/err for debugging.

It's recommended to write both "runner" and "test" components in the language
that you want to test. The "runner" component exercises the ability to import
WIT interfaces and call them while the "test" component exercises the ability to
export interfaces and have them called.

#### Test Configuration

Each source language file can be annotated with arguments to pass to the
bindings generation phase. This is done by having a comment at the top of the
file such as:

```rust
//@ args = '--custom --arguments'

fn main() {
    // ...
}
```

This `//@` prefix indicates that test configuration present. The test
configuration deserializes via TOML to `RuntimeTestConfig`. The field used here
is `args` which are the CLI arguments to pass to `wit-bindgen rust ...` in this
case. This can be used to have specific source files test various options of a
bindings generator.

Note that multiple runners are supported, so for example in one test Rust might
have `runner-std.rs` and `runner-nostd.rs` to test with and without the
`--std-feature` flag to the Rust bindings generator. Note that regardless of
bindings generator flags it's expected that the original `runner` or `test`
worlds are still adhered to.

#### Test Configuration: World Names

By default `runner` and `test` worlds are expected, but this can be configured
with:

```wit
//@ runner = "other-runner"
//@ dependencies = ["other-test"]

package foo:bar;

world other-runner {
    // ...
}

world other-test {
    // ...
}
```

This will then expect `other-runner.rs` for example as a test file, so test
files are still named after their worlds.

#### Test Configuration: Fancy Compositions

The `wac` tooling is available for composing components together. This can be
configured with `dependencies` and `wac` keys:

```wit
//@ dependencies = ["intermediate", "leaf"]
//@ wac = "./compose.wac"

package foo:bar;

world runner {
    // ...
}

world intermediate {
    // ...
}

world leaf {
    // ...
}
```

This would then require a `compose.wac` file in the test directory. Components
named `test:{world}` are made available to the script to perform compositions
with:

```wac
package example:composition;

let leaf = new test:leaf { ... };
let intermediate = new test:intermediate { ...leaf, ... };
let runner = new test:runner { ...intermediate, ... };

export runner...;
```

## Language Support

Currently the `wit-bindgen test` CLI comes with built-in language support for a
few languages. Note that this does not include built-in toolchain support. For
example `wit-bindgen test` will still need access to a Rust toolchain to compile
Rust source files.

* Rust
* C
* C++
* Go (via TinyGo)
* WebAssembly Text (`*.wat`)

Tests written in these languages can use `wit-bindgen test` natively and don't
need to otherwise provide anything else. Custom language support is
additionally supported at this time via the `--custom` CLI flag to
`wit-bindgen test`. For example if the CLI didn't natively have support for Rust
it could be specified as:

```
$ wit-bindgen test ./tests --artifacts-dir ./artifacts \
    --custom rs=./wit-bindgen-rust-runner
```

This would recognize the `rs` file extension and use the
`./wit-bindgen-rust-runner` script or binary to execute tests. The exact
interface to the tests is documented as part of `wit-bindgen test --help` for
the `--custom` argument.

#### Configuration: Rust

Rust configuration supports a few keys at the top of files in addition to the
default `args` option for bindings generator options

```rust
//@ [lang]
//@ rustflags = '-O'
//@ externs = ['./other.rs']
```

Here the crate will be compiled with `-O` and `./other.rs` will be compiled as a
separate crate and passed as `--extern`

#### Configuration: C

C/C++ configuration supports configuring compilation flags at this time:

```rust
//@ [lang]
//@ cflags = '-O'
```
