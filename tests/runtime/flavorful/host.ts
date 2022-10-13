import { loadWasm, testwasi } from "./helpers.js";
import { instantiate } from "./flavorful.js";

// @ts-ignore
import * as assert from 'assert';

async function run() {
  const wasm = await instantiate(loadWasm, {
    testwasi,
    imports: {
      fListInRecord1(x) {},
      fListInRecord2() { return { a: 'list_in_record2' }; },
      fListInRecord3(x) {
        assert.strictEqual(x.a, 'list_in_record3 input');
        return { a: 'list_in_record3 output' };
      },
      fListInRecord4(x) {
        assert.strictEqual(x.a, 'input4');
        return { a: 'result4' };
      },
      fListInVariant1(a, b, c) {
        assert.strictEqual(a, 'foo');
        assert.deepStrictEqual(b, { tag: 'err', val: 'bar' });
        assert.deepStrictEqual(c, { tag: 0, val: 'baz' });
      },
      fListInVariant2() { return 'list_in_variant2'; },
      fListInVariant3(x) {
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
    },
  });

  wasm.testImports();
  wasm.fListInRecord1({ a: "list_in_record1" });
  assert.deepStrictEqual(wasm.fListInRecord2(), { a: "list_in_record2" });

  assert.deepStrictEqual(
    wasm.fListInRecord3({ a: "list_in_record3 input" }),
    { a: "list_in_record3 output" },
  );

  assert.deepStrictEqual(
    wasm.fListInRecord4({ a: "input4" }),
    { a: "result4" },
  );

  wasm.fListInVariant1("foo", { tag: 'err', val: 'bar' }, { tag: 0, val: 'baz' });

  assert.deepStrictEqual(wasm.fListInVariant2(), "list_in_variant2");
  assert.deepStrictEqual(wasm.fListInVariant3("input3"), "output3");

  assert.deepStrictEqual(wasm.errnoResult().tag, 'err');

  const [r1, r2] = wasm.listTypedefs("typedef1", ["typedef2"]);
  assert.deepStrictEqual(r1, (new TextEncoder()).encode('typedef3'));
  assert.deepStrictEqual(r2, ['typedef4']);
}

await run()
