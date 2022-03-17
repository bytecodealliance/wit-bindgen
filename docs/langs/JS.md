# JavaScript

## Host Bindings ✔️
JavaScript code run in the browser, Node.js, or Deno may be able to execute WebAssembly modules since those runtimes provide WebAssembly support. In theory this covers browser use cases like web workers and such as well.

The wit-bindgen CLI tool can emit a `*.js` and `*.d.ts` file describing the interface and providing necessary runtime support in JS to implement the canonical ABI.

**Note:** The intended long-term integration of this language is to compile wit-bindgen itself to WebAssembly and publish NPM packages for popular JS build systems to integrate wit-bindgen into JS build processes.