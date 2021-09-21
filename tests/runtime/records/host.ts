import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import * as exports from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    multipleResults() { return [4, 5]; },
    swapTuple([a, b]) { return [b, a]; },
    roundtripFlags1(x) { return x; },
    roundtripFlags2(x) { return x; },
    roundtripFlags3(r0, r1, r2, r3) { return [r0, r1, r2, r3]; },
    roundtripRecord1(x) { return x; },
    tuple0([]) { return []; },
    tuple1([x]) { return [x]; },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  wasm.testImports();
  assert.deepEqual(wasm.multipleResults(), [100, 200]);
  assert.deepStrictEqual(wasm.swapTuple([1, 2]), [2, 1]);
  assert.deepEqual(wasm.roundtripFlags1(exports.F1_A), exports.F1_A);
  assert.deepEqual(wasm.roundtripFlags1(0), 0);
  assert.deepEqual(wasm.roundtripFlags1(exports.F1_A | exports.F1_B), exports.F1_A | exports.F1_B);

  assert.deepEqual(wasm.roundtripFlags2(exports.F2_C), exports.F2_C);
  assert.deepEqual(wasm.roundtripFlags2(0), 0);
  assert.deepEqual(wasm.roundtripFlags2(exports.F2_D), exports.F2_D);
  assert.deepEqual(wasm.roundtripFlags2(exports.F2_C | exports.F2_E), exports.F2_C | exports.F2_E);

  {
    const { a, b } = wasm.roundtripRecord1({ a: 8, b: 0 });
    assert.deepEqual(a, 8);
    assert.deepEqual(b, 0);
  }

  {
    const { a, b } = wasm.roundtripRecord1({ a: 0, b: exports.F1_A | exports.F1_B });
    assert.deepEqual(a, 0);
    assert.deepEqual(b, exports.F1_A | exports.F1_B);
  }

  assert.deepStrictEqual(wasm.tuple0([]), []);
  assert.deepStrictEqual(wasm.tuple1([1]), [1]);
}

await run()
