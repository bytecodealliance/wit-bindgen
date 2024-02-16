// generate the c# and the component meta module

cargo run c-sharp  --string-encoding utf8 --out-dir testing-csharp tests/codegen/floats.wit

// to run the runtime tests with Native AOT, you need some additional set up

// install emscripten
curl.exe -OL https://github.com/emscripten-core/emsdk/archive/refs/heads/main.zip
unzip main.zip
cd .\emsdk-main\main\emsdk-main
.\emsdk_env.ps1 activate 3.1.23 --permanant

// install wasi-sdk and set env
curl.exe -L https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-20/wasi-sdk-20.0.m-mingw.tar.gz | tar xzvf -
$env:WASI_SDK_PATH="c:\users\jstur\wasi-sdk-20.0+m\"