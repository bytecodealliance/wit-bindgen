import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    bufferU8(x, out) {
      assert.deepStrictEqual(Array.from(x), [0]);
      assert.deepStrictEqual(out.length, 10);
      out[0] = 1;
      out[1] = 2;
      out[2] = 3;
      return 3;
    },
    bufferU32(x, out) {
      assert.deepStrictEqual(Array.from(x), [0]);
      assert.deepStrictEqual(out.length, 10);
      out[0] = 1;
      out[1] = 2;
      out[2] = 3;
      return 3;
    },
    bufferBool(x, out) {
      assert.ok(x.length <= out.length);
      let amt = 0;
      while (true) {
        const val = x.pull();
        if (val === undefined)
          break;
        out.push(!val);
        amt += 1;
      }
      return amt;
    },
    bufferMutable1(x) {
      assert.strictEqual(x.length, 1);
      assert.strictEqual(x[0].length, 5);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), false);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), false);
      assert.strictEqual(x[0].pull(), undefined);
    },
    bufferMutable2(x) {
      assert.strictEqual(x.length, 1);
      assert.ok(x[0].length > 4);
      x[0].set([1, 2, 3, 4]);
      return 4;
    },
    bufferMutable3(x) {
      assert.strictEqual(x.length, 1);
      assert.ok(x[0].length > 3);
      x[0].push(false);
      x[0].push(true);
      x[0].push(false);
      return 3;
    },
    bufferInRecord(x) { },
    bufferTypedef(a, b, c, d) {},
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  wasm.testImports();
}

await run()
