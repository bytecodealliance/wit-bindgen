export function getWasm(): Uint8Array;

export interface Wasi {
  start(instance: WebAssembly.Instance): void;
}

export function addWasiToImports(importObj: any): Wasi;
