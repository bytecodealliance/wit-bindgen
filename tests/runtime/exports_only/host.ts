// Flags: --valid-lifting-optimization --base64-cutoff=0
// @ts-ignore
import { ok, strictEqual } from 'assert';
// @ts-ignore
import { readFile } from 'fs/promises';
// @ts-ignore
import { fileURLToPath } from 'url';
import { thunk } from './exports_only.js';

const result = thunk();
strictEqual(result, 'test');

// Verify the inlined file size does not regress
const url = new URL('./exports_only.js', import.meta.url);
const jsSource = await readFile(url);
const max_size = 1200;
ok(jsSource.byteLength <= max_size, `JS inlined bytelength ${jsSource.byteLength} is greater than ${max_size} bytes, at ${fileURLToPath(url)}`);

