import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

function promiseChannel(): [Promise<void>, () => void] {
  let resolveCallback = null;
  const promise = new Promise((resolve, reject) => resolveCallback = resolve);
  // @ts-ignore
  return [promise, resolveCallback];
}

async function run() {
  const importObj = {};
  let hit = false;

  const [concurrentPromise, resolveConcurrent] = promiseChannel();
  const [unblockConcurrent1, resolveUnblockConcurrent1] = promiseChannel();
  const [unblockConcurrent2, resolveUnblockConcurrent2] = promiseChannel();
  const [unblockConcurrent3, resolveUnblockConcurrent3] = promiseChannel();

  const imports: Imports = {
    async thunk() {
      if (hit) {
        console.log('second time in thunk, throwing an error');
        throw new Error('catch me');
      } else {
        console.log('first time in thunk');
        await some_helper();
        console.log('waited on the helper, returning from host thunk');
        hit = true;
      }
    },

    async concurrent1(val) {
      console.log('wasm called concurrent1');
      assert.equal(val, 1);
      resolveUnblockConcurrent1();
      console.log('concurrent1 to reenter back into the host');
      await concurrentPromise;
      console.log('concurrent1 returning to wasm');
      return 11;
    },
    async concurrent2(val) {
      console.log('wasm called concurrent2');
      assert.equal(val, 2);
      resolveUnblockConcurrent2();
      console.log('concurrent2 to reenter back into the host');
      await concurrentPromise;
      console.log('concurrent2 returning to wasm');
      return 12;
    },
    async concurrent3(val) {
      console.log('wasm called concurrent3');
      assert.equal(val, 3);
      resolveUnblockConcurrent3();
      console.log('concurrent3 to reenter back into the host');
      await concurrentPromise;
      console.log('concurrent3 returning to wasm');
      return 13;
    },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  const initBytes = wasm.allocatedBytes();
  console.log("calling initial async function");
  await wasm.thunk();
  assert.ok(hit, "import not called");
  assert.equal(initBytes, wasm.allocatedBytes());

  // Make sure that exceptions on the host make their way back to whomever's
  // doing the actual `await`
  try {
    console.log('executing thunk export a second time');
    await wasm.thunk();
    throw new Error('expected an error to get thrown');
  } catch (e) {
    const err = e as Error;
    console.log('caught error with', err.message);
    assert.equal(err.message, 'catch me');
  }

  console.log('entering wasm');
  const concurrentWasm = wasm.testConcurrent();
  console.log('waiting for wasm to enter the host');
  await unblockConcurrent1;
  await unblockConcurrent2;
  await unblockConcurrent3;
  console.log('allowing host functions to finish');
  resolveConcurrent();
  console.log('waiting on host functions');
  await concurrentWasm;
  console.log('concurrent wasm finished');
}

async function some_helper() {}

await run()
