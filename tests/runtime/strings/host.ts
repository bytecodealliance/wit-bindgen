import { loadWasm, testwasi } from "./helpers.js";
import { instantiate } from "./strings.js";

// @ts-ignore
import * as assert from 'assert';

async function run() {
  const wasm = await instantiate(loadWasm, {
    testwasi,
    imports: {
      f1 (s: string) {
        assert.strictEqual(s, 'latin utf16');
      },
      f2 () {
        return '🚀🚀🚀 𠈄𓀀';
      }
    }
  });

  wasm.testImports();
  assert.strictEqual(wasm.f2(), '🚀🚀🚀 𠈄𓀀');
  wasm.f1('str');
  assert.strictEqual(wasm.f2(), 'str');
}

await run()
