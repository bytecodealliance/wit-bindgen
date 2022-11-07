// Flags: --base64 --compat --map testwasi=./helpers.js,imports=./host.js
function assert(x: boolean, msg: string) {
  if (!x)
    throw new Error(msg);
}

let hit = false;

export function thunk () {
  hit = true;
}

async function run() {
  const wasm = await import('./smoke.js');

  await wasm.$init;

  wasm.thunk();
  assert(hit, "import not called");
}

// Async cycle handling
setTimeout(run);
