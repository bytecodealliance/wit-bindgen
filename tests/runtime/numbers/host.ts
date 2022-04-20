import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";

function assertEq(x: any, y: any) {
  if (x !== y)
    throw new Error(`${x} != ${y}`);
}

function assert(x: boolean) {
  if (!x)
    throw new Error("assert failed");
}

async function run() {
  const importObj = {};
  let scalar = 0;
  addImportsToImports(importObj, {
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
  });
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);

  wasm.testImports();

  assertEq(wasm.roundtripU8(1), 1);
  assertEq(wasm.roundtripU8((1 << 8) - 1), (1 << 8) - 1);

  assertEq(wasm.roundtripS8(1), 1);
  assertEq(wasm.roundtripS8((1 << 7) - 1), (1 << 7) - 1);
  assertEq(wasm.roundtripS8(-(1 << 7)), -(1 << 7));

  assertEq(wasm.roundtripU16(1), 1);
  assertEq(wasm.roundtripU16((1 << 16) - 1), (1 << 16) - 1);

  assertEq(wasm.roundtripS16(1), 1);
  assertEq(wasm.roundtripS16((1 << 15) - 1), (1 << 15) - 1);
  assertEq(wasm.roundtripS16(-(1 << 15)), -(1 << 15));

  assertEq(wasm.roundtripU32(1), 1);
  assertEq(wasm.roundtripU32(~0 >>> 0), ~0 >>> 0);

  assertEq(wasm.roundtripS32(1), 1);
  assertEq(wasm.roundtripS32(((1 << 31) - 1) >>> 0), ((1 << 31) - 1) >>> 0);
  assertEq(wasm.roundtripS32(1 << 31), 1 << 31);

  assertEq(wasm.roundtripU64(1n), 1n);
  assertEq(wasm.roundtripU64((1n << 64n) - 1n), (1n << 64n) - 1n);

  assertEq(wasm.roundtripS64(1n), 1n);
  assertEq(wasm.roundtripS64((1n << 63n) - 1n), (1n << 63n) - 1n);
  assertEq(wasm.roundtripS64(-(1n << 63n)), -(1n << 63n));

  assertEq(wasm.roundtripFloat32(1), 1);
  assertEq(wasm.roundtripFloat32(Infinity), Infinity);
  assertEq(wasm.roundtripFloat32(-Infinity), -Infinity);
  assert(Number.isNaN(wasm.roundtripFloat32(NaN)));

  assertEq(wasm.roundtripFloat64(1), 1);
  assertEq(wasm.roundtripFloat64(Infinity), Infinity);
  assertEq(wasm.roundtripFloat64(-Infinity), -Infinity);
  assert(Number.isNaN(wasm.roundtripFloat64(NaN)));

  assertEq(wasm.roundtripChar('a'), 'a');
  assertEq(wasm.roundtripChar(' '), ' ');
  assertEq(wasm.roundtripChar('🚩'), '🚩');

  wasm.setScalar(2);
  assertEq(wasm.getScalar(), 2);
  wasm.setScalar(4);
  assertEq(wasm.getScalar(), 4);
}

await run()
