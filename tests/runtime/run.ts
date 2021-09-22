import { readFileSync } from 'fs';
import * as assert from 'assert';
import * as imports from './imports/bindings.js';
import * as exports from './exports/bindings.js';
import { WASI } from 'wasi';

async function run() {
  const wasm = readFileSync(process.argv[2]);
  const wasi = new WASI({
    args: process.argv,
    env: process.env,
  });
  const importObj = {
    wasi_snapshot_preview1: wasi.wasiImport,
  };
  let instance: WebAssembly.Instance | null = null;
  imports.addHostToImports(importObj, host(), name => {
    if (instance === null)
      throw new Error("instance not ready yet");
    return instance.exports[name];
  });
  const wasmObj = new exports.Wasm();
  await wasmObj.instantiate(wasm, importObj);
  instance = wasmObj.instance;
  wasi.initialize(instance);

  runTests(wasmObj);

  // test other methods of creating a wasm wrapper
  (new exports.Wasm()).instantiate(wasm.buffer, importObj);
  (new exports.Wasm()).instantiate(new Uint8Array(wasm), importObj);
  (new exports.Wasm()).instantiate(new WebAssembly.Module(wasm), importObj);
  {
    const obj = new exports.Wasm();
    obj.addToImports(importObj);
    obj.instantiate(new WebAssembly.Instance(new WebAssembly.Module(wasm), importObj));
  }
}


function testFlavorful(wasm: exports.Wasm) {



function testInvalid(wasm: exports.Wasm) {
  const exports = wasm.instance.exports as any;
  assert.throws(exports.invalid_bool, /invalid variant discriminant for bool/);
  assert.throws(exports.invalid_u8, /must be between/);
  assert.throws(exports.invalid_s8, /must be between/);
  assert.throws(exports.invalid_u16, /must be between/);
  assert.throws(exports.invalid_s16, /must be between/);
  assert.throws(exports.invalid_char, /not a valid char/);
  assert.throws(exports.invalid_e1, /invalid discriminant specified for E1/);
  assert.throws(exports.invalid_handle, /handle index not valid/);
  assert.throws(exports.invalid_handle_close, /handle index not valid/);
}

await run()
