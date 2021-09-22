import { readFileSync } from 'fs';
import * as assert from 'assert';
import * as imports from './imports/bindings.js';
import * as exports from './exports/bindings.js';
import { WASI } from 'wasi';

async function run() {
  const wasm = readFileSync(process.argv[2]);
  const wasi = new WASI({
    args: process.argv,
    env: process.env,
  });
  const importObj = {
    wasi_snapshot_preview1: wasi.wasiImport,
  };
  let instance: WebAssembly.Instance | null = null;
  imports.addHostToImports(importObj, host(), name => {
    if (instance === null)
      throw new Error("instance not ready yet");
    return instance.exports[name];
  });
  const wasmObj = new exports.Wasm();
  await wasmObj.instantiate(wasm, importObj);
  instance = wasmObj.instance;
  wasi.initialize(instance);

  runTests(wasmObj);

  // test other methods of creating a wasm wrapper
  (new exports.Wasm()).instantiate(wasm.buffer, importObj);
  (new exports.Wasm()).instantiate(new Uint8Array(wasm), importObj);
  (new exports.Wasm()).instantiate(new WebAssembly.Module(wasm), importObj);
  {
    const obj = new exports.Wasm();
    obj.addToImports(importObj);
    obj.instantiate(new WebAssembly.Instance(new WebAssembly.Module(wasm), importObj));
  }
}

function host(): imports.Host {
  return {


    listInRecord1(x) {},
    listInRecord2() { return { a: 'list_in_record2' }; },
    listInRecord3(x) {
      assert.strictEqual(x.a, 'list_in_record3 input');
      return { a: 'list_in_record3 output' };
    },
    listInRecord4(x) {
      assert.strictEqual(x.a, 'input4');
      return { a: 'result4' };
    },
    listInVariant1(a, b, c) {
      assert.strictEqual(a, 'foo');
      assert.deepStrictEqual(b, { tag: 'err', val: 'bar' });
      assert.deepStrictEqual(c, { tag: '0', val: 'baz' });
    },
    listInVariant2() { return 'list_in_variant2'; },
    listInVariant3(x) {
      assert.strictEqual(x, 'input3');
      return 'output3';
    },

    errnoResult() { return { tag: 'err', val: imports.MyErrno.B }; },
    listTypedefs(x, y) {
      assert.strictEqual(x, 'typedef1');
      assert.deepStrictEqual(y, ['typedef2']);
      return [(new TextEncoder).encode('typedef3'), ['typedef4']];
    },

    listOfVariants(bools, results, enums) {
      assert.deepStrictEqual(bools, [true, false]);
      assert.deepStrictEqual(results, [{ tag: 'ok' }, { tag: 'err' }]);
      assert.deepStrictEqual(enums, [imports.MyErrno.Success, imports.MyErrno.A]);
      return [
        [false, true],
        [{ tag: 'err', val: undefined }, { tag: 'ok', val: undefined }],
        [imports.MyErrno.A, imports.MyErrno.B],
      ];
    },

  };
}

function runTests(wasm: exports.Wasm) {
  wasm.runImportTests();
  testScalars(wasm);
  testRecords(wasm);
  testVariants(wasm);
  testLists(wasm);
  testFlavorful(wasm);
  testInvalid(wasm);
  testHandles(wasm);
  // buffers(wasm);

}


function testFlavorful(wasm: exports.Wasm) {
  wasm.listInRecord1({ a: "list_in_record1" });
  assert.deepStrictEqual(wasm.listInRecord2(), { a: "list_in_record2" });

  assert.deepStrictEqual(
    wasm.listInRecord3({ a: "list_in_record3 input" }),
    { a: "list_in_record3 output" },
  );

  assert.deepStrictEqual(
    wasm.listInRecord4({ a: "input4" }),
    { a: "result4" },
  );

  wasm.listInVariant1("foo", { tag: 'err', val: 'bar' }, { tag: '0', val: 'baz' });

  assert.deepStrictEqual(wasm.listInVariant2(), "list_in_variant2");
  assert.deepStrictEqual(wasm.listInVariant3("input3"), "output3");

  assert.deepStrictEqual(wasm.errnoResult().tag, 'err');

  const [r1, r2] = wasm.listTypedefs("typedef1", ["typedef2"]);
  assert.deepStrictEqual(r1, (new TextEncoder()).encode('typedef3'));
  assert.deepStrictEqual(r2, ['typedef4']);
}


// fn buffers(wasm: &Wasm) -> Result<()> {
//     let mut out = [0; 10];
//     let n = wasm.buffer_u8(&[0u8], &mut out)? as usize;
//     assert_eq!(n, 3);
//     assert_eq!(&out[..n], [1, 2, 3]);
//     assert!(out[n..].iter().all(|x| *x == 0));

//     let mut out = [0; 10];
//     let n = wasm.buffer_u32(&[0], &mut out)? as usize;
//     assert_eq!(n, 3);
//     assert_eq!(&out[..n], [1, 2, 3]);
//     assert!(out[n..].iter().all(|x| *x == 0));

//     assert_eq!(wasm.buffer_bool(&mut iter::empty(), &mut Vec::new())?, 0);
//     assert_eq!(wasm.buffer_string(&mut iter::empty(), &mut Vec::new())?, 0);
//     assert_eq!(
//         wasm.buffer_list_bool(&mut iter::empty(), &mut Vec::new())?,
//         0
//     );

//     let mut bools = [true, false, true].iter().copied();
//     let mut out = Vec::with_capacity(4);
//     let n = wasm.buffer_bool(&mut bools, &mut out)?;
//     assert_eq!(n, 3);
//     assert_eq!(out, [false, true, false]);

//     let mut strings = ["foo", "bar", "baz"].iter().copied();
//     let mut out = Vec::with_capacity(3);
//     let n = wasm.buffer_string(&mut strings, &mut out)?;
//     assert_eq!(n, 3);
//     assert_eq!(out, ["FOO", "BAR", "BAZ"]);

//     let a = &[true, false, true][..];
//     let b = &[false, false][..];
//     let list = [a, b];
//     let mut lists = list.iter().copied();
//     let mut out = Vec::with_capacity(4);
//     let n = wasm.buffer_list_bool(&mut lists, &mut out)?;
//     assert_eq!(n, 2);
//     assert_eq!(out, [vec![false, true, false], vec![true, true]]);

//     let a = [true, false, true, true, false];
//     // let mut bools = a.iter().copied();
//     // let mut list = [&mut bools as &mut dyn ExactSizeIterator<Item = _>];
//     // let mut buffers = list.iter_mut().map(|b| &mut **b);
//     // wasm.buffer_buffer_bool(&mut buffers)?;

//     let mut bools = a.iter().copied();
//     wasm.buffer_mutable1(&mut [&mut bools])?;

//     let mut dst = [0; 10];
//     let n = wasm.buffer_mutable2(&mut [&mut dst])? as usize;
//     assert_eq!(n, 4);
//     assert_eq!(&dst[..n], [1, 2, 3, 4]);

//     let mut out = Vec::with_capacity(10);
//     let n = wasm.buffer_mutable3(&mut [&mut out])?;
//     assert_eq!(n, 3);
//     assert_eq!(out, [false, true, false]);

//     Ok(())
// }

function testInvalid(wasm: exports.Wasm) {
  const exports = wasm.instance.exports as any;
  assert.throws(exports.invalid_bool, /invalid variant discriminant for bool/);
  assert.throws(exports.invalid_u8, /must be between/);
  assert.throws(exports.invalid_s8, /must be between/);
  assert.throws(exports.invalid_u16, /must be between/);
  assert.throws(exports.invalid_s16, /must be between/);
  assert.throws(exports.invalid_char, /not a valid char/);
  assert.throws(exports.invalid_e1, /invalid discriminant specified for E1/);
  assert.throws(exports.invalid_handle, /handle index not valid/);
  assert.throws(exports.invalid_handle_close, /handle index not valid/);
}

await run()
