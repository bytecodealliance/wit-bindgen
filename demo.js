import { log as lowering0Callee, error as lowering1Callee } from './console.js';

const instantiateCore = WebAssembly.instantiate;

const utf8Decoder = new TextDecoder();

function toString(val) {
  if (typeof val === 'symbol') throw new TypeError('symbols cannot be converted to strings');
  return String(val);
}

const utf8Encoder = new TextEncoder();

let utf8EncodedLen = 0;
function utf8Encode(s, realloc, memory) {
  if (typeof s !== 'string') throw new TypeError('expected a string');
  if (s.length === 0) {
    utf8EncodedLen = 0;
    return 1;
  }
  let allocLen = 0;
  let ptr = 0;
  let writtenTotal = 0;
  while (s.length > 0) {
    ptr = realloc(ptr, allocLen, 1, allocLen + s.length);
    allocLen += s.length;
    const { read, written } = utf8Encoder.encodeInto(
    s,
    new Uint8Array(memory.buffer, ptr + writtenTotal, allocLen - writtenTotal),
    );
    writtenTotal += written;
    s = s.slice(read);
  }
  if (allocLen > writtenTotal)
  ptr = realloc(ptr, allocLen, 1, writtenTotal);
  utf8EncodedLen = writtenTotal;
  return ptr;
}

let dv = new DataView(new ArrayBuffer());
const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);

class ComponentError extends Error {
  constructor (value) {
    const enumerable = typeof value !== 'string';
    super(enumerable ? `${String(value)} (see error.payload)` : value);
    Object.defineProperty(this, 'payload', { value, enumerable });
  }
}

const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
let _fs;
async function fetchCompile (url) {
  if (isNode) {
    _fs = _fs || await import('fs/promises');
    return WebAssembly.compile(await _fs.readFile(url));
  }
  return fetch(url).then(WebAssembly.compileStreaming);
}

const base64Compile = str => WebAssembly.compile(typeof Buffer !== 'undefined' ? Buffer.from(str, 'base64') : Uint8Array.from(atob(str), b => b.charCodeAt(0)));

let exports0;
let exports1;
let memory0;
let exports2;
let realloc0;
let postReturn0;
export const demo = {
  render(arg0, arg1, arg2) {
    const val0 = toString(arg0);
    let enum0;
    switch (val0) {
      case 'js': {
        enum0 = 0;
        break;
      }
      case 'rust': {
        enum0 = 1;
        break;
      }
      case 'java': {
        enum0 = 2;
        break;
      }
      case 'c': {
        enum0 = 3;
        break;
      }
      case 'markdown': {
        enum0 = 4;
        break;
      }
      default: {
        throw new TypeError(`"${val0}" is not one of the cases of lang`);
      }
    }
    const ptr1 = utf8Encode(arg1, realloc0, memory0);
    const len1 = utf8EncodedLen;
    const {rustUnchecked: v2_0, jsCompat: v2_1, jsInstantiation: v2_2 } = arg2;
    const ret = exports1['demo#render'](enum0, ptr1, len1, v2_0 ? 1 : 0, v2_1 ? 1 : 0, v2_2 ? 1 : 0);
    let variant7;
    switch (dataView(memory0).getUint8(ret + 0, true)) {
      case 0: {
        const len5 = dataView(memory0).getInt32(ret + 8, true);
        const base5 = dataView(memory0).getInt32(ret + 4, true);
        const result5 = [];
        for (let i = 0; i < len5; i++) {
          const base = base5 + i * 16;
          const ptr3 = dataView(memory0).getInt32(base + 0, true);
          const len3 = dataView(memory0).getInt32(base + 4, true);
          const result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
          const ptr4 = dataView(memory0).getInt32(base + 8, true);
          const len4 = dataView(memory0).getInt32(base + 12, true);
          const result4 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr4, len4));
          result5.push([result3, result4]);
        }
        variant7= {
          tag: 'ok',
          val: result5
        };
        break;
      }
      case 1: {
        const ptr6 = dataView(memory0).getInt32(ret + 4, true);
        const len6 = dataView(memory0).getInt32(ret + 8, true);
        const result6 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr6, len6));
        variant7= {
          tag: 'err',
          val: result6
        };
        break;
      }
      default: {
        throw new TypeError('invalid variant discriminant for expected');
      }
    }
    postReturn0(ret);
    if (variant7.tag === 'err') {
      throw new ComponentError(variant7.val);
    }
    return variant7.val;
  },
  
};

const $init = (async() => {
  const module0 = fetchCompile(new URL('./demo.core.wasm', import.meta.url));
  const module1 = base64Compile('AGFzbQEAAAABBgFgAn9/AAMDAgAABAUBcAECAgcUAwEwAAABMQABCCRpbXBvcnRzAQAKGQILACAAIAFBABEAAAsLACAAIAFBAREAAAsANgRuYW1lAS8CABRpbmRpcmVjdC1jb25zb2xlLWxvZwEWaW5kaXJlY3QtY29uc29sZS1lcnJvcg==');
  const module2 = base64Compile('AGFzbQEAAAABBgFgAn9/AAIaAwABMAAAAAExAAAACCRpbXBvcnRzAXABAgIJCAEAQQALAgAB');
  Promise.all([module0, module1, module2]).catch(() => {});
  ({ exports: exports0 } = await instantiateCore(await module1));
  ({ exports: exports1 } = await instantiateCore(await module0, {
    console: {
      error: exports0['1'],
      log: exports0['0'],
    },
  }));
  memory0 = exports1.memory;
  
  function lowering0(arg0, arg1) {
    const ptr0 = arg0;
    const len0 = arg1;
    const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    lowering0Callee(result0);
  }
  
  function lowering1(arg0, arg1) {
    const ptr0 = arg0;
    const len0 = arg1;
    const result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    lowering1Callee(result0);
  }
  ({ exports: exports2 } = await instantiateCore(await module2, {
    '': {
      $imports: exports0.$imports,
      '0': lowering0,
      '1': lowering1,
    },
  }));
  realloc0 = exports1.cabi_realloc;
  postReturn0 = exports1['cabi_post_demo#render'];
})();

await $init;
