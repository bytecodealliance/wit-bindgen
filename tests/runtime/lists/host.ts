import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import * as exports from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    emptyListParam(a) {
      assert.deepStrictEqual(Array.from(a), []);
    },
    emptyStringParam(a) {
      assert.strictEqual(a, '');
    },
    emptyListResult() {
      return new Uint8Array([]);
    },
    emptyStringResult() { return ''; },
    listParam(a) {
      assert.deepStrictEqual(Array.from(a), [1, 2, 3, 4]);
    },
    listParam2(a) {
      assert.strictEqual(a, 'foo');
    },
    listParam3(a) {
      assert.deepStrictEqual(a, ['foo', 'bar', 'baz']);
    },
    listParam4(a) {
      assert.deepStrictEqual(a, [['foo', 'bar'], ['baz']]);
    },
    listResult() {
      return new Uint8Array([1, 2, 3, 4, 5]);
    },
    listResult2() { return 'hello!'; },
    listResult3() { return ['hello,', 'world!']; },
    listRoundtrip(x) { return x; },
    stringRoundtrip(x) { return x; },

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
    listMinmax8(u, s) {
      assert.deepEqual(u.length, 2);
      assert.deepEqual(u[0], 0);
      assert.deepEqual(u[1], (1 << 8) - 1);
      assert.deepEqual(s.length, 2);
      assert.deepEqual(s[0], -(1 << 7));
      assert.deepEqual(s[1], (1 << 7) - 1);

      return [u, s];
    },

    listMinmax16(u, s) {
      assert.deepEqual(u.length, 2);
      assert.deepEqual(u[0], 0);
      assert.deepEqual(u[1], (1 << 16) - 1);
      assert.deepEqual(s.length, 2);
      assert.deepEqual(s[0], -(1 << 15));
      assert.deepEqual(s[1], (1 << 15) - 1);

      return [u, s];
    },

    listMinmax32(u, s) {
      assert.deepEqual(u.length, 2);
      assert.deepEqual(u[0], 0);
      assert.deepEqual(u[1], ~0 >>> 0);
      assert.deepEqual(s.length, 2);
      assert.deepEqual(s[0], 1 << 31);
      assert.deepEqual(s[1], ((1 << 31) - 1) >>> 0);

      return [u, s];
    },

    listMinmax64(u, s) {
      assert.deepEqual(u.length, 2);
      assert.deepEqual(u[0], 0n);
      assert.deepEqual(u[1], (2n ** 64n) - 1n);
      assert.deepEqual(s.length, 2);
      assert.deepEqual(s[0], -(2n ** 63n));
      assert.deepEqual(s[1], (2n ** 63n) - 1n);

      return [u, s];
    },

    listMinmaxFloat(f, d) {
      assert.deepEqual(f.length, 4);
      assert.deepEqual(f[0], -3.4028234663852886e+38);
      assert.deepEqual(f[1], 3.4028234663852886e+38);
      assert.deepEqual(f[2], Number.NEGATIVE_INFINITY);
      assert.deepEqual(f[3], Number.POSITIVE_INFINITY);

      assert.deepEqual(d.length, 4);
      assert.deepEqual(d[0], -Number.MAX_VALUE);
      assert.deepEqual(d[1], Number.MAX_VALUE);
      assert.deepEqual(d[2], Number.NEGATIVE_INFINITY);
      assert.deepEqual(d[3], Number.POSITIVE_INFINITY);

      return [f, d];
    },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  const bytes = wasm.allocatedBytes();
  wasm.testImports();
  wasm.emptyListParam(new Uint8Array([]));
  wasm.emptyStringParam('');
  wasm.listParam(new Uint8Array([1, 2, 3, 4]));
  wasm.listParam2("foo");
  wasm.listParam3(["foo", "bar", "baz"]);
  wasm.listParam4([["foo", "bar"], ["baz"]]);
  assert.deepStrictEqual(Array.from(wasm.emptyListResult()), []);
  assert.deepStrictEqual(wasm.emptyStringResult(), "");
  assert.deepStrictEqual(Array.from(wasm.listResult()), [1, 2, 3, 4, 5]);
  assert.deepStrictEqual(wasm.listResult2(), "hello!");
  assert.deepStrictEqual(wasm.listResult3(), ["hello,", "world!"]);

  const buffer = new ArrayBuffer(8);
  (new Uint8Array(buffer)).set(new Uint8Array([1, 2, 3, 4]), 2);
  // Create a view of the four bytes in the middle of the buffer
  const view = new Uint8Array(buffer, 2, 4);
  assert.deepStrictEqual(Array.from(wasm.listRoundtrip(view)), [1, 2, 3, 4]);

  assert.deepStrictEqual(wasm.stringRoundtrip("x"), "x");
  assert.deepStrictEqual(wasm.stringRoundtrip(""), "");
  assert.deepStrictEqual(wasm.stringRoundtrip("hello ⚑ world"), "hello ⚑ world");

  // Ensure that we properly called `free` everywhere in all the glue that we
  // needed to.
  assert.strictEqual(bytes, wasm.allocatedBytes());
}

await run()
