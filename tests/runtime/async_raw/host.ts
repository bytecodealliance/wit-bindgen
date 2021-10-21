import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    async thunk() {}
  };

  async function instantiate() {
    let instance: WebAssembly.Instance;
    addImportsToImports(importObj, imports, name => instance.exports[name]);
    const wasi = addWasiToImports(importObj);

    const wasm = new Exports();
    await wasm.instantiate(getWasm(), importObj);
    wasi.start(wasm.instance);
    instance = wasm.instance;
    return wasm;
  }

  let wasm = await instantiate();
  await wasm.completeImmediately();
  await wasm.assertCoroutineIdZero();
  await wasm.assertCoroutineIdZero();

  wasm = await instantiate();
  await assert.rejects(wasm.completionNotCalled(), /blocked coroutine with 0 pending callbacks/);

  wasm = await instantiate();
  await assert.rejects(wasm.completeTwice(), /cannot complete coroutine twice/);

  wasm = await instantiate();
  await assert.rejects(wasm.completeThenTrap(), /unreachable/);

  wasm = await instantiate();
  assert.throws(() => wasm.notAsyncExportDone(), /invalid coroutine index/);

  wasm = await instantiate();
  await assert.rejects(wasm.importCallbackNull(), /table index is a null function/);

  // TODO: this is the wrong error from this, but it's not clear how best to do
  // type-checks in JS...
  wasm = await instantiate();
  await assert.rejects(wasm.importCallbackWrongType(), /0 pending callbacks/);

  wasm = await instantiate();
  await assert.rejects(wasm.importCallbackBadIndex(), RangeError);
}

await run()
