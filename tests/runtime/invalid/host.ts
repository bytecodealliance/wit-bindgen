// Flags: --instantiation

import { instantiate } from "./invalid.js";
import * as helpers from "./helpers.js";
// @ts-ignore
import * as assert from 'assert';

async function run() {
  const wasm = await instantiate(helpers.loadWasm, {
    testwasi: helpers,
    imports: {
      roundtripU8(x) { throw new Error('unreachable'); },
      roundtripS8(x) { throw new Error('unreachable'); },
      roundtripU16(x) { throw new Error('unreachable'); },
      roundtripS16(x) { throw new Error('unreachable'); },
      roundtripBool(x) { throw new Error('unreachable'); },
      roundtripChar(x) { throw new Error('unreachable'); },
      roundtripEnum(x) { throw new Error('unreachable'); },
      unaligned1(x) { throw new Error('unreachable'); },
      unaligned2(x) { throw new Error('unreachable'); },
      unaligned3(x) { throw new Error('unreachable'); },
      unaligned4(x) { throw new Error('unreachable'); },
      unaligned5(x) { throw new Error('unreachable'); },
      unaligned6(x) { throw new Error('unreachable'); },
      unaligned7(x) { throw new Error('unreachable'); },
      unaligned8(x) { throw new Error('unreachable'); },
      unaligned9(x) { throw new Error('unreachable'); },
      unaligned10(x) { throw new Error('unreachable'); },
    },
  });

  // FIXME(#376) these should succeed
  assert.throws(() => wasm.invalidBool(), /invalid variant discriminant for bool/);
  assert.throws(() => wasm.invalidU8(), /must be between/);
  assert.throws(() => wasm.invalidS8(), /must be between/);
  assert.throws(() => wasm.invalidU16(), /must be between/);
  assert.throws(() => wasm.invalidS16(), /must be between/);

  // FIXME(#375) these should require a new instantiation
  assert.throws(() => wasm.invalidChar(), /not a valid char/);
  assert.throws(() => wasm.invalidEnum(), /invalid discriminant specified for E/);

    /*
  assert.throws(() => wasm.testUnaligned(), /is not aligned/);
    */
}

await run()
