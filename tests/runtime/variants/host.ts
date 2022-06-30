import { addImportsToImports, Imports, MyErrno } from "./imports.js";
import { Exports } from "./exports.js";
import * as exports from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    roundtripOption(x) { return x; },
    roundtripResult(x) {
      if (x.tag == 'ok') {
        return { tag: 'ok', val: x.val };
      } else {
        return { tag: 'err', val: Math.round(x.val) };
      }
    },
    roundtripEnum(x) { return x; },
    invertBool(x) { return !x; },
    variantCasts(x) { return x; },
    variantZeros(x) { return x; },
    variantTypedefs(x, y, z) {},
    variantEnums(a, b, c) {
      assert.deepStrictEqual(a, true);
      assert.deepStrictEqual(b, { tag: 'ok', val: undefined });
      assert.deepStrictEqual(c, "success");
      return [
        false,
        { tag: 'err', val: undefined },
        "a",
      ];
    },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  wasm.testImports();
  assert.deepStrictEqual(wasm.roundtripOption(1), 1);
  assert.deepStrictEqual(wasm.roundtripOption(null), null);
  assert.deepStrictEqual(wasm.roundtripOption(2), 2);
  assert.deepStrictEqual(wasm.roundtripResult({ tag: 'ok', val: 2 }), { tag: 'ok', val: 2 });
  assert.deepStrictEqual(wasm.roundtripResult({ tag: 'ok', val: 4 }), { tag: 'ok', val: 4 });
  const f = Math.fround(5.2);
  assert.deepStrictEqual(wasm.roundtripResult({ tag: 'err', val: f }), { tag: 'err', val: 5 });

  assert.deepStrictEqual(wasm.roundtripEnum("a"), "a");
  assert.deepStrictEqual(wasm.roundtripEnum("b"), "b");

  assert.deepStrictEqual(wasm.invertBool(true), false);
  assert.deepStrictEqual(wasm.invertBool(false), true);

  {
    const [a1, a2, a3, a4, a5, a6] = wasm.variantCasts([
      { tag: 'a', val: 1 },
      { tag: 'a', val: 2 },
      { tag: 'a', val: 3 },
      { tag: 'a', val: 4n },
      { tag: 'a', val: 5n },
      { tag: 'a', val: 6 },
    ]);
    assert.deepStrictEqual(a1, { tag: 'a', val: 1 });
    assert.deepStrictEqual(a2, { tag: 'a', val: 2 });
    assert.deepStrictEqual(a3, { tag: 'a', val: 3 });
    assert.deepStrictEqual(a4, { tag: 'a', val: 4n });
    assert.deepStrictEqual(a5, { tag: 'a', val: 5n });
    assert.deepStrictEqual(a6, { tag: 'a', val: 6 });
  }
  {
    const [b1, b2, b3, b4, b5, b6] = wasm.variantCasts([
      { tag: 'b', val: 1n },
      { tag: 'b', val: 2 },
      { tag: 'b', val: 3 },
      { tag: 'b', val: 4 },
      { tag: 'b', val: 5 },
      { tag: 'b', val: 6 },
    ]);
    assert.deepStrictEqual(b1, { tag: 'b', val: 1n });
    assert.deepStrictEqual(b2, { tag: 'b', val: 2 });
    assert.deepStrictEqual(b3, { tag: 'b', val: 3 });
    assert.deepStrictEqual(b4, { tag: 'b', val: 4 });
    assert.deepStrictEqual(b5, { tag: 'b', val: 5 });
    assert.deepStrictEqual(b6, { tag: 'b', val: 6 });
  }

  {
    const [a1, a2, a3, a4] = wasm.variantZeros([
      { tag: 'a', val: 1 },
      { tag: 'a', val: 2n },
      { tag: 'a', val: 3 },
      { tag: 'a', val: 4 },
    ]);
    assert.deepStrictEqual(a1, { tag: 'a', val: 1 });
    assert.deepStrictEqual(a2, { tag: 'a', val: 2n });
    assert.deepStrictEqual(a3, { tag: 'a', val: 3 });
    assert.deepStrictEqual(a4, { tag: 'a', val: 4 });
  }

  wasm.variantTypedefs(null, false, { tag: 'err', val: undefined });
}

await run()
