// Flags: --instantiation

import * as helpers from "./helpers.js";
import { instantiate } from "./numbers.js";

function assertEq(x: any, y: any) {
  if (x !== y)
    throw new Error(`${x} != ${y}`);
}

function assert(x: boolean) {
  if (!x)
    throw new Error("assert failed");
}

async function run() {
  let scalar = 0;
  const wasm = await instantiate(helpers.loadWasm, {
    testwasi: helpers,
    imports: {
      roundtripU8(x) { return x; },
      roundtripS8(x) { return x; },
      roundtripU16(x) { return x; },
      roundtripS16(x) { return x; },
      roundtripU32(x) { return x; },
      roundtripS32(x) { return x; },
      roundtripU64(x) { return x; },
      roundtripS64(x) { return x; },
      roundtripFloat32(x) { return x; },
      roundtripFloat64(x) { return x; },
      roundtripChar(x) { return x; },
      setScalar(x) { scalar = x; },
      getScalar() { return scalar; },
    },
  });

  wasm.testImports();

  assertEq(wasm.exports.roundtripU8(1), 1);
  assertEq(wasm.exports.roundtripU8((1 << 8) - 1), (1 << 8) - 1);

  assertEq(wasm.exports.roundtripS8(1), 1);
  assertEq(wasm.exports.roundtripS8((1 << 7) - 1), (1 << 7) - 1);
  assertEq(wasm.exports.roundtripS8(-(1 << 7)), -(1 << 7));

  assertEq(wasm.exports.roundtripU16(1), 1);
  assertEq(wasm.exports.roundtripU16((1 << 16) - 1), (1 << 16) - 1);

  assertEq(wasm.exports.roundtripS16(1), 1);
  assertEq(wasm.exports.roundtripS16((1 << 15) - 1), (1 << 15) - 1);
  assertEq(wasm.exports.roundtripS16(-(1 << 15)), -(1 << 15));

  assertEq(wasm.exports.roundtripU32(1), 1);
  assertEq(wasm.exports.roundtripU32(~0 >>> 0), ~0 >>> 0);

  assertEq(wasm.exports.roundtripS32(1), 1);
  assertEq(wasm.exports.roundtripS32(((1 << 31) - 1) >>> 0), ((1 << 31) - 1) >>> 0);
  assertEq(wasm.exports.roundtripS32(1 << 31), 1 << 31);

  assertEq(wasm.exports.roundtripU64(1n), 1n);
  assertEq(wasm.exports.roundtripU64((1n << 64n) - 1n), (1n << 64n) - 1n);

  assertEq(wasm.exports.roundtripS64(1n), 1n);
  assertEq(wasm.exports.roundtripS64((1n << 63n) - 1n), (1n << 63n) - 1n);
  assertEq(wasm.exports.roundtripS64(-(1n << 63n)), -(1n << 63n));

  assertEq(wasm.exports.roundtripFloat32(1), 1);
  assertEq(wasm.exports.roundtripFloat32(Infinity), Infinity);
  assertEq(wasm.exports.roundtripFloat32(-Infinity), -Infinity);
  assert(Number.isNaN(wasm.exports.roundtripFloat32(NaN)));

  assertEq(wasm.exports.roundtripFloat64(1), 1);
  assertEq(wasm.exports.roundtripFloat64(Infinity), Infinity);
  assertEq(wasm.exports.roundtripFloat64(-Infinity), -Infinity);
  assert(Number.isNaN(wasm.exports.roundtripFloat64(NaN)));

  assertEq(wasm.exports.roundtripChar('a'), 'a');
  assertEq(wasm.exports.roundtripChar(' '), ' ');
  assertEq(wasm.exports.roundtripChar('🚩'), '🚩');

  wasm.exports.setScalar(2);
  assertEq(wasm.exports.getScalar(), 2);
  wasm.exports.setScalar(4);
  assertEq(wasm.exports.getScalar(), 4);
}

await run()
