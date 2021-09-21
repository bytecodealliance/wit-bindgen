import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    roundtripU8(x) { throw new Error('unreachable'); },
    roundtripS8(x) { throw new Error('unreachable'); },
    roundtripU16(x) { throw new Error('unreachable'); },
    roundtripS16(x) { throw new Error('unreachable'); },
    roundtripBool(x) { throw new Error('unreachable'); },
    roundtripChar(x) { throw new Error('unreachable'); },
    roundtripEnum(x) { throw new Error('unreachable'); },
    getInternal(x) { throw new Error('unreachable'); },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  assert.throws(() => wasm.invalidBool(), /invalid variant discriminant for bool/);
  assert.throws(() => wasm.invalidU8(), /must be between/);
  assert.throws(() => wasm.invalidS8(), /must be between/);
  assert.throws(() => wasm.invalidU16(), /must be between/);
  assert.throws(() => wasm.invalidS16(), /must be between/);
  assert.throws(() => wasm.invalidChar(), /not a valid char/);
  assert.throws(() => wasm.invalidEnum(), /invalid discriminant specified for E/);
  assert.throws(() => wasm.invalidHandle(), /handle index not valid/);
  assert.throws(() => wasm.invalidHandleClose(), /handle index not valid/);
}

await run()
