// @ts-ignore
import { readFile } from 'node:fs/promises';
// @ts-ignore
import { argv, stdout, stderr } from 'node:process';

// This is a helper function used from `host.ts` test in the `tests/runtime/*`
// directory to pass as the `instantiateCore` argument to the `instantiate`
// function generated by `wit-bindgen`.
//
// This function loads the module named by `path` and instantiates it with the
// `imports` object provided. The `path` is a relative path to a wasm file
// within the generated directory which for tests is passed as argv 2.
export async function loadWasm(path: string) {
  const root = argv[2];
  return await WebAssembly.compile(await readFile(root + '/' + path))
}

// Export a WASI interface directly for instance imports
export function log (bytes: Uint8Array) {
  stdout.write(bytes);
}
export function logErr (bytes: Uint8Array) {
  stderr.write(bytes);
}
