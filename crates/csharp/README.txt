## generate the c# and the component module

The following will generate the c# code given a wit file:

```bash
cargo run csharp --string-encoding utf8 --out-dir testing-csharp tests/codegen/floats.wit
```

## Setup
To run the runtime tests with Native AOT, you need some additional set up

```bash
// install wasi-sdk and set env
curl.exe -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-22/wasi-sdk-22.0.m-mingw64.tar.gz | tar xzvf -
$env:WASI_SDK_PATH="c:\users\jstur\wasi-sdk-22.0+m\"
```
