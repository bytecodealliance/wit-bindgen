import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import * as exports from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const importObj = {};
  let sawClose = false;
  const imports: Imports = {
    hostStateCreate() { return 100; },
    hostStateGet(x) { return x as number; },
    hostState2Create() { return 101; },
    hostState2SawClose() { return sawClose; },
    dropHostState2(state) { sawClose = true; },
    twoHostStates(a, b) { return [b, a]; },
    hostState2ParamRecord(x) {},
    hostState2ParamTuple(x) {},
    hostState2ParamOption(x) {},
    hostState2ParamResult(x) {},
    hostState2ParamVariant(x) {},
    hostState2ParamList(x) {},

    hostState2ResultRecord() { return { a: {} }; },
    hostState2ResultTuple() { return [{}]; },
    hostState2ResultOption() { return 102; },
    hostState2ResultResult() { return { tag: 'ok', val: {} }; },
    hostState2ResultVariant() { return { tag: 0, val: {} }; },
    hostState2ResultList() { return [{}, 3]; },

    markdown2Create() {
      class Markdown {
        buf: string;

        constructor() {
          this.buf = '';
        }
        append(extra: string) {
          this.buf += extra;
        }
        render() {
          return this.buf.replace('red', 'green');
        }
      }

      return new Markdown();
    },

    oddNameCreate() {
      class OddName {
        frobTheOdd() {}
      }
      return new OddName();
    }
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  wasm.testImports();

  // Param/result of a handle works in a simple fashion
  const s: exports.WasmState = wasm.wasmStateCreate();
  assert.strictEqual(wasm.wasmStateGetVal(s), 100);

  // Deterministic destruction is possible
  assert.strictEqual(wasm.wasmState2SawClose(), false);
  const s2: exports.WasmState2 = wasm.wasmState2Create();
  assert.strictEqual(wasm.wasmState2SawClose(), false);
  s2.drop();
  assert.strictEqual(wasm.wasmState2SawClose(), true);

  const arg1 = wasm.wasmStateCreate();
  const arg2 = wasm.wasmState2Create();
  const [c, d] = wasm.twoWasmStates(arg1, arg2);
  arg1.drop();
  arg2.drop();

  wasm.wasmState2ParamRecord({ a: d });
  wasm.wasmState2ParamTuple([d]);
  wasm.wasmState2ParamOption(d);
  wasm.wasmState2ParamOption(null);
  wasm.wasmState2ParamResult({ tag: 'ok', val: d });
  wasm.wasmState2ParamResult({ tag: 'err', val: 2 });
  wasm.wasmState2ParamVariant({ tag: 0, val: d });
  wasm.wasmState2ParamVariant({ tag: 1, val: 2 });
  wasm.wasmState2ParamList([]);
  wasm.wasmState2ParamList([d]);
  wasm.wasmState2ParamList([d, d]);

  c.drop();
  d.drop();

  wasm.wasmState2ResultRecord().a?.drop();
  wasm.wasmState2ResultTuple()[0].drop();
  const opt = wasm.wasmState2ResultOption();
  if (opt === null)
    throw new Error('should be some');
  opt.drop();
  const result = wasm.wasmState2ResultResult();
  if (result.tag === 'err')
    throw new Error('should be ok');
  result.val.drop();
  const variant = wasm.wasmState2ResultVariant();
  if (variant.tag === 1)
    throw new Error('should be 0');
  variant.val.drop();
  for (let val of wasm.wasmState2ResultList())
    val.drop();

  s.drop();

  const md = exports.Markdown.create(wasm);
  if (md) {
    md.append("red is the best color");
    assert.strictEqual(md.render(), "green is the best color");
    md.drop();
  }
}

await run()
