import { loadWasm, testwasi } from "./helpers.js";
import { instantiate } from "./smoke.js";

function assert(x: boolean, msg: string) {
  if (!x)
    throw new Error(msg);
}

async function run() {
  let hit = false;

  const wasm = await instantiate(loadWasm, {
    testwasi,
    imports: {
      thunk() {
        hit = true;
      },
    },
  });

  wasm.thunk();
  assert(hit, "import not called");
}

await run()
