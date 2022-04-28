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
  const imports: Imports = {
    manyArguments(
      a1,
      a2,
      a3,
      a4,
      a5,
      a6,
      a7,
      a8,
      a9,
      a10,
      a11,
      a12,
      a13,
      a14,
      a15,
      a16,
      a17,
      a18,
      a19,
      a20,
    ) {
      assertEq(a1, 1n);
      assertEq(a2, 2n);
      assertEq(a3, 3n);
      assertEq(a4, 4n);
      assertEq(a5, 5n);
      assertEq(a6, 6n);
      assertEq(a7, 7n);
      assertEq(a8, 8n);
      assertEq(a9, 9n);
      assertEq(a10, 10n);
      assertEq(a11, 11n);
      assertEq(a12, 12n);
      assertEq(a13, 13n);
      assertEq(a14, 14n);
      assertEq(a15, 15n);
      assertEq(a16, 16n);
      assertEq(a17, 17n);
      assertEq(a18, 18n);
      assertEq(a19, 19n);
      assertEq(a20, 20n);
    },
  };
  let instance: WebAssembly.Instance;
  addImportsToImports(importObj, imports, name => instance.exports[name]);
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);
  instance = wasm.instance;

  wasm.manyArguments(
    1n,
    2n,
    3n,
    4n,
    5n,
    6n,
    7n,
    8n,
    9n,
    10n,
    11n,
    12n,
    13n,
    14n,
    15n,
    16n,
    17n,
    18n,
    19n,
    20n,
  );
}

await run()
