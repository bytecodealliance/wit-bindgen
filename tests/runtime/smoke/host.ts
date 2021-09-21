import { addImportsToImports, Imports } from "./imports.js";
import { Exports } from "./exports.js";
import { getWasm, addWasiToImports } from "./helpers.js";

function assert(x: boolean, msg: string) {
  if (!x)
    throw new Error(msg);
}

async function run() {
  const importObj = {};
  let hit = false;
  addImportsToImports(importObj, {
    thunk() {
      hit = true;
    }
  });
  const wasi = addWasiToImports(importObj);

  const wasm = new Exports();
  await wasm.instantiate(getWasm(), importObj);
  wasi.start(wasm.instance);

  wasm.thunk();
  assert(hit, "import not called");
}

await run()
