
This folder contains examples on how to use the caninical ABI without
a wasm32 target.

The `native_strings` folder contains an example of passing strings, with
the guest in C++ and Rust, the host in C++, and in the w2c folder an
example of a wasm component transpiled to C and then executed natively.

Sadly the [w2c2](https://github.com/turbolent/w2c2) bridge code generation isn't yet complete.

The `native_resources` folder shows a more complex example using resources,
both guest and host defined ones. This doesn't include a wasm2c deployment.
