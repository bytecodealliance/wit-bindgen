import { addImportsToImports, Imports, MyErrno } from "./imports.js";
import { Exports } from "./exports.js";
import * as exports from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  const imports: Imports = {
    listInRecord1(x) {},
    listInRecord2() { return { a: 'list_in_record2' }; },
    listInRecord3(x) {
      assert.strictEqual(x.a, 'list_in_record3 input');
      return { a: 'list_in_record3 output' };
    },
    listInRecord4(x) {
      assert.strictEqual(x.a, 'input4');
      return { a: 'result4' };
    },
    listInVariant1(a, b, c) {
      assert.strictEqual(a, 'foo');
      assert.deepStrictEqual(b, { tag: 'err', val: 'bar' });
      assert.deepStrictEqual(c, { tag: 0, val: 'baz' });
    },
    listInVariant2() { return 'list_in_variant2'; },
    listInVariant3(x) {
      assert.strictEqual(x, 'input3');
      return 'output3';
    },

    errnoResult() { return { tag: 'err', val: "b" }; },
    listTypedefs(x, y) {
      assert.strictEqual(x, 'typedef1');
      assert.deepStrictEqual(y, ['typedef2']);
      return [(new TextEncoder).encode('typedef3'), ['typedef4']];
    },

    listOfVariants(bools, results, enums) {
      assert.deepStrictEqual(bools, [true, false]);
      assert.deepStrictEqual(results, [{ tag: 'ok', val: undefined }, { tag: 'err', val: undefined }]);
      assert.deepStrictEqual(enums, ["success", "a"]);
      return [
        [false, true],
        [{ tag: 'err', val: undefined }, { tag: 'ok', val: undefined }],
        ["a", "b"],
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
  wasm.listInRecord1({ a: "list_in_record1" });
  assert.deepStrictEqual(wasm.listInRecord2(), { a: "list_in_record2" });

  assert.deepStrictEqual(
    wasm.listInRecord3({ a: "list_in_record3 input" }),
    { a: "list_in_record3 output" },
  );

  assert.deepStrictEqual(
    wasm.listInRecord4({ a: "input4" }),
    { a: "result4" },
  );

  wasm.listInVariant1("foo", { tag: 'err', val: 'bar' }, { tag: 0, val: 'baz' });

  assert.deepStrictEqual(wasm.listInVariant2(), "list_in_variant2");
  assert.deepStrictEqual(wasm.listInVariant3("input3"), "output3");

  assert.deepStrictEqual(wasm.errnoResult().tag, 'err');

  const [r1, r2] = wasm.listTypedefs("typedef1", ["typedef2"]);
  assert.deepStrictEqual(r1, (new TextEncoder()).encode('typedef3'));
  assert.deepStrictEqual(r2, ['typedef4']);
}

await run()
