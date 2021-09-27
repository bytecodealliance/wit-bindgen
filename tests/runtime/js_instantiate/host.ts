import { Exports } from "./exports.js";
import { getWasm } from "./helpers.js";

async function run() {
  const importObj = {};
  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);

  // test other methods of creating a wasm wrapper
  (new Exports()).instantiate(getWasm().buffer, importObj);
  (new Exports()).instantiate(new Uint8Array(getWasm()), importObj);
  (new Exports()).instantiate(new WebAssembly.Module(getWasm()), importObj);
  {
    const obj = new Exports();
    obj.addToImports(importObj);
    obj.instantiate(new WebAssembly.Instance(new WebAssembly.Module(getWasm()), importObj));
  }
}

await run()
