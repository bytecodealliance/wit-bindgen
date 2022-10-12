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
      /*
    unalignedRoundtrip1(u16, u32, u64, flag32, flag64) {
      assert.deepStrictEqual(Array.from(u16), [1]);
      assert.deepStrictEqual(Array.from(u32), [2]);
      assert.deepStrictEqual(Array.from(u64), [3n]);
      assert.deepStrictEqual(flag32, [{
        b0: false, b1: false, b2: false, b3: false, b4: false, b5: false, b6: false, b7: false,
        b8: true, b9: false, b10: false, b11: false, b12: false, b13: false, b14: false, b15: false,
        b16: false, b17: false, b18: false, b19: false, b20: false, b21: false, b22: false, b23: false,
        b24: false, b25: false, b26: false, b27: false, b28: false, b29: false, b30: false, b31: false,
      }]);
      assert.deepStrictEqual(flag64, [{
        b0: false, b1: false, b2: false, b3: false, b4: false, b5: false, b6: false, b7: false,
        b8: false, b9: true, b10: false, b11: false, b12: false, b13: false, b14: false, b15: false,
        b16: false, b17: false, b18: false, b19: false, b20: false, b21: false, b22: false, b23: false,
        b24: false, b25: false, b26: false, b27: false, b28: false, b29: false, b30: false, b31: false,
        b32: false, b33: false, b34: false, b35: false, b36: false, b37: false, b38: false, b39: false,
        b40: false, b41: false, b42: false, b43: false, b44: false, b45: false, b46: false, b47: false,
        b48: false, b49: false, b50: false, b51: false, b52: false, b53: false, b54: false, b55: false,
        b56: false, b57: false, b58: false, b59: false, b60: false, b61: false, b62: false, b63: false,
      }]);
    },
    unalignedRoundtrip2(record, f32, f64, string, list) {
      assert.deepStrictEqual(Array.from(record), [{ a: 10, b: 11n }]);
      assert.deepStrictEqual(Array.from(f32), [100]);
      assert.deepStrictEqual(Array.from(f64), [101]);
      assert.deepStrictEqual(string, ['foo']);
      assert.deepStrictEqual(list, [new Uint8Array([102])]);
    },
      */

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
    /*
  assert.throws(() => wasm.testUnaligned(), /is not aligned/);
    */
}

await run()
