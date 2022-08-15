import { readFileSync } from 'fs';
import { WASI } from 'wasi';

export function getWasm() {
  return readFileSync(process.argv[2]);
}

class MyWasi {
  constructor(wasi) {
    this.wasi = wasi;
  }

  start(instance) {
    if ('_start' in instance.exports) {
      this.wasi.start(instance);
    } else {
      this.wasi.initialize(instance);
    }
  }
}

export function addWasiToImports(importObj) {
  const wasi = new WASI();
  importObj.wasi_snapshot_preview1 = wasi.wasiImport;
  return new MyWasi(wasi);
}
