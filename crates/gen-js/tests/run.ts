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
  imports.add_host_to_imports(importObj, host(), name => {
    if (instance === null)
      throw new Error("instance not ready yet");
    return instance.exports[name];
  });
  const wasmObj = new exports.Wasm();
  await wasmObj.instantiate(wasm, importObj);
  instance = wasmObj.instance;
  wasi.initialize(instance);

  run_tests(wasmObj);

  // test other methods of creating a wasm wrapper
  (new exports.Wasm()).instantiate(wasm.buffer, importObj);
  (new exports.Wasm()).instantiate(new Uint8Array(wasm), importObj);
  (new exports.Wasm()).instantiate(new WebAssembly.Module(wasm), importObj);
  {
    const obj = new exports.Wasm();
    obj.add_to_imports(importObj);
    obj.instantiate(new WebAssembly.Instance(new WebAssembly.Module(wasm), importObj));
  }
}

function host(): imports.Host {
  let scalar = 0;
  let saw_close = false;
  return {
    roundtrip_u8(x) { return x; },
    roundtrip_s8(x) { return x; },
    roundtrip_u16(x) { return x; },
    roundtrip_s16(x) { return x; },
    roundtrip_u32(x) { return x; },
    roundtrip_s32(x) { return x; },
    roundtrip_u64(x) { return x; },
    roundtrip_s64(x) { return x; },
    roundtrip_f32(x) { return x; },
    roundtrip_f64(x) { return x; },
    roundtrip_char(x) { return x; },
    multiple_results() { return [4, 5]; },
    set_scalar(x) { scalar = x; },
    get_scalar() { return scalar; },
    swap_tuple([a, b]) { return [b, a]; },
    roundtrip_flags1(x) { return x; },
    roundtrip_flags2(x) { return x; },
    roundtrip_flags3(r0, r1, r2, r3) { return [r0, r1, r2, r3]; },
    roundtrip_record1(x) { return x; },
    tuple0([]) { return []; },
    tuple1([x]) { return [x]; },
    roundtrip_option(x) { return x; },
    roundtrip_result(x) {
      if (x.tag == 'ok') {
        return { tag: 'ok', val: x.val };
      } else {
        return { tag: 'err', val: Math.round(x.val) };
      }
    },
    roundtrip_enum(x) { return x; },
    invert_bool(x) { return !x; },
    variant_casts(x) { return x; },
    variant_zeros(x) { return x; },
    variant_typedefs(x, y, z) {},
    variant_enums(a, b, c) {
      assert.deepStrictEqual(a, true);
      assert.deepStrictEqual(b, { tag: 'ok' });
      assert.deepStrictEqual(c, imports.MyErrno.Success);
      return [
        false,
        { tag: 'err', val: undefined },
        imports.MyErrno.A,
      ];
    },
    list_param(a) {
      assert.deepStrictEqual(Array.from(a), [1, 2, 3, 4]);
    },
    list_param2(a) {
      assert.strictEqual(a, 'foo');
    },
    list_param3(a) {
      assert.deepStrictEqual(a, ['foo', 'bar', 'baz']);
    },
    list_param4(a) {
      assert.deepStrictEqual(a, [['foo', 'bar'], ['baz']]);
    },
    list_result() {
      return new Uint8Array([1, 2, 3, 4, 5]);
    },
    list_result2() { return 'hello!'; },
    list_result3() { return ['hello,', 'world!']; },
    string_roundtrip(x) { return x; },
    host_state_create() { return 100; },
    host_state_get(x) { return x as number; },
    host_state2_create() { return 101; },
    host_state2_saw_close() { return saw_close; },
    drop_host_state2(state) { saw_close = true; },
    two_host_states(a, b) { return [b, a]; },
    host_state2_param_record(x) {},
    host_state2_param_tuple(x) {},
    host_state2_param_option(x) {},
    host_state2_param_result(x) {},
    host_state2_param_variant(x) {},
    host_state2_param_list(x) {},

    host_state2_result_record() { return { a: {} }; },
    host_state2_result_tuple() { return [{}]; },
    host_state2_result_option() { return 102; },
    host_state2_result_result() { return { tag: 'ok', val: {} }; },
    host_state2_result_variant() { return { tag: '0', val: {} }; },
    host_state2_result_list() { return [{}, 3]; },

    buffer_u8(x, out) {
      assert.deepStrictEqual(Array.from(x), [0]);
      assert.deepStrictEqual(out.length, 10);
      out[0] = 1;
      out[1] = 2;
      out[2] = 3;
      return 3;
    },
    buffer_u32(x, out) {
      assert.deepStrictEqual(Array.from(x), [0]);
      assert.deepStrictEqual(out.length, 10);
      out[0] = 1;
      out[1] = 2;
      out[2] = 3;
      return 3;
    },
    buffer_bool(x, out) {
      assert.ok(x.length <= out.length);
      let amt = 0;
      while (true) {
        const val = x.pull();
        if (val === undefined)
          break;
        out.push(!val);
        amt += 1;
      }
      return amt;
    },
    buffer_mutable1(x) {
      assert.strictEqual(x.length, 1);
      assert.strictEqual(x[0].length, 5);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), false);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), true);
      assert.strictEqual(x[0].pull(), false);
      assert.strictEqual(x[0].pull(), undefined);
    },
    buffer_mutable2(x) {
      assert.strictEqual(x.length, 1);
      assert.ok(x[0].length > 4);
      x[0].set([1, 2, 3, 4]);
      return 4;
    },
    buffer_mutable3(x) {
      assert.strictEqual(x.length, 1);
      assert.ok(x[0].length > 3);
      x[0].push(false);
      x[0].push(true);
      x[0].push(false);
      return 3;
    },
    buffer_in_record(x) { },
    buffer_typedef(a, b, c, d) {},

    list_in_record1(x) {},
    list_in_record2() { return { a: 'list_in_record2' }; },
    list_in_record3(x) {
      assert.strictEqual(x.a, 'list_in_record3 input');
      return { a: 'list_in_record3 output' };
    },
    list_in_record4(x) {
      assert.strictEqual(x.a, 'input4');
      return { a: 'result4' };
    },
    list_in_variant1(a, b, c) {
      assert.strictEqual(a, 'foo');
      assert.deepStrictEqual(b, { tag: 'err', val: 'bar' });
      assert.deepStrictEqual(c, { tag: '0', val: 'baz' });
    },
    list_in_variant2() { return 'list_in_variant2'; },
    list_in_variant3(x) {
      assert.strictEqual(x, 'input3');
      return 'output3';
    },

    errno_result() { return { tag: 'err', val: imports.MyErrno.B }; },
    list_typedefs(x, y) {
      assert.strictEqual(x, 'typedef1');
      assert.deepStrictEqual(y, ['typedef2']);
      return [(new TextEncoder).encode('typedef3'), ['typedef4']];
    },

    list_of_variants(bools, results, enums) {
      assert.deepStrictEqual(bools, [true, false]);
      assert.deepStrictEqual(results, [{ tag: 'ok' }, { tag: 'err' }]);
      assert.deepStrictEqual(enums, [imports.MyErrno.Success, imports.MyErrno.A]);
      return [
        [false, true],
        [{ tag: 'err', val: undefined }, { tag: 'ok', val: undefined }],
        [imports.MyErrno.A, imports.MyErrno.B],
      ];
    },

    unaligned_roundtrip1(u16, u32, u64, flag32, flag64) {
      assert.deepStrictEqual(Array.from(u16), [1]);
      assert.deepStrictEqual(Array.from(u32), [2]);
      assert.deepStrictEqual(Array.from(u64), [3n]);
      assert.deepStrictEqual(flag32, [imports.FLAG32_B8]);
      assert.deepStrictEqual(flag64, [imports.FLAG64_B9]);
    },
    unaligned_roundtrip2(record, f32, f64, string, list) {
      assert.deepStrictEqual(Array.from(record), [{ a: 10, b: 11n }]);
      assert.deepStrictEqual(Array.from(f32), [100]);
      assert.deepStrictEqual(Array.from(f64), [101]);
      assert.deepStrictEqual(string, ['foo']);
      assert.deepStrictEqual(list, [new Uint8Array([102])]);
    },
  };
}

function run_tests(wasm: exports.Wasm) {
  const bytes = wasm.allocated_bytes();
  wasm.run_import_tests();
  test_scalars(wasm);
  test_records(wasm);
  test_variants(wasm);
  test_lists(wasm);
  test_flavorful(wasm);
  test_invalid(wasm);
  test_handles(wasm);
  // buffers(wasm);

  // Ensure that we properly called `free` everywhere in all the glue that we
  // needed to.
  assert.strictEqual(bytes, wasm.allocated_bytes());
}

function test_scalars(wasm: exports.Wasm) {
  assert.strictEqual(wasm.roundtrip_u8(1), 1);
  assert.strictEqual(wasm.roundtrip_u8((1 << 8) - 1), (1 << 8) - 1);

  assert.strictEqual(wasm.roundtrip_s8(1), 1);
  assert.strictEqual(wasm.roundtrip_s8((1 << 7) - 1), (1 << 7) - 1);
  assert.strictEqual(wasm.roundtrip_s8(-(1 << 7)), -(1 << 7));

  assert.strictEqual(wasm.roundtrip_u16(1), 1);
  assert.strictEqual(wasm.roundtrip_u16((1 << 16) - 1), (1 << 16) - 1);

  assert.strictEqual(wasm.roundtrip_s16(1), 1);
  assert.strictEqual(wasm.roundtrip_s16((1 << 15) - 1), (1 << 15) - 1);
  assert.strictEqual(wasm.roundtrip_s16(-(1 << 15)), -(1 << 15));

  assert.strictEqual(wasm.roundtrip_u32(1), 1);
  assert.strictEqual(wasm.roundtrip_u32((1 << 32) - 1), (1 << 32) - 1);

  assert.strictEqual(wasm.roundtrip_s32(1), 1);
  assert.strictEqual(wasm.roundtrip_s32(((1 << 31) - 1) >>> 0), ((1 << 31) - 1) >>> 0);
  assert.strictEqual(wasm.roundtrip_s32(1 << 31), 1 << 31);

  assert.strictEqual(wasm.roundtrip_u64(1n), 1n);
  assert.strictEqual(wasm.roundtrip_u64((1n << 64n) - 1n), (1n << 64n) - 1n);

  assert.strictEqual(wasm.roundtrip_s64(1n), 1n);
  assert.strictEqual(wasm.roundtrip_s64((1n << 63n) - 1n), (1n << 63n) - 1n);
  assert.strictEqual(wasm.roundtrip_s64(-(1n << 63n)), -(1n << 63n));

  assert.deepEqual(wasm.multiple_results(), [100, 200]);

  assert.strictEqual(wasm.roundtrip_f32(1), 1);
  assert.strictEqual(wasm.roundtrip_f32(Infinity), Infinity);
  assert.strictEqual(wasm.roundtrip_f32(-Infinity), -Infinity);
  assert.ok(Number.isNaN(wasm.roundtrip_f32(NaN)));

  assert.strictEqual(wasm.roundtrip_f64(1), 1);
  assert.strictEqual(wasm.roundtrip_f64(Infinity), Infinity);
  assert.strictEqual(wasm.roundtrip_f64(-Infinity), -Infinity);
  assert.ok(Number.isNaN(wasm.roundtrip_f64(NaN)));

  assert.strictEqual(wasm.roundtrip_char('a'), 'a');
  assert.strictEqual(wasm.roundtrip_char(' '), ' ');
  assert.strictEqual(wasm.roundtrip_char('🚩'), '🚩');

  wasm.set_scalar(2);
  assert.strictEqual(wasm.get_scalar(), 2);
  wasm.set_scalar(4);
  assert.strictEqual(wasm.get_scalar(), 4);
}

function test_records(wasm: exports.Wasm) {
  assert.deepStrictEqual(wasm.swap_tuple([1, 2]), [2, 1]);
  assert.deepEqual(wasm.roundtrip_flags1(exports.F1_A), exports.F1_A);
  assert.deepEqual(wasm.roundtrip_flags1(0), 0);
  assert.deepEqual(wasm.roundtrip_flags1(exports.F1_A | exports.F1_B), exports.F1_A | exports.F1_B);

  assert.deepEqual(wasm.roundtrip_flags2(exports.F2_C), exports.F2_C);
  assert.deepEqual(wasm.roundtrip_flags2(0), 0);
  assert.deepEqual(wasm.roundtrip_flags2(exports.F2_D), exports.F2_D);
  assert.deepEqual(wasm.roundtrip_flags2(exports.F2_C | exports.F2_E), exports.F2_C | exports.F2_E);

  {
    const { a, b } = wasm.roundtrip_record1({ a: 8, b: 0 });
    assert.deepEqual(a, 8);
    assert.deepEqual(b, 0);
  }

  {
    const { a, b } = wasm.roundtrip_record1({ a: 0, b: exports.F1_A | exports.F1_B });
    assert.deepEqual(a, 0);
    assert.deepEqual(b, exports.F1_A | exports.F1_B);
  }

  assert.deepStrictEqual(wasm.tuple0([]), []);
  assert.deepStrictEqual(wasm.tuple1([1]), [1]);
}

function test_variants(wasm: exports.Wasm) {
  assert.deepStrictEqual(wasm.roundtrip_option(1), 1);
  assert.deepStrictEqual(wasm.roundtrip_option(null), null);
  assert.deepStrictEqual(wasm.roundtrip_option(2), 2);
  assert.deepStrictEqual(wasm.roundtrip_result({ tag: 'ok', val: 2 }), { tag: 'ok', val: 2 });
  assert.deepStrictEqual(wasm.roundtrip_result({ tag: 'ok', val: 4 }), { tag: 'ok', val: 4 });
  const f = Math.fround(5.2);
  assert.deepStrictEqual(wasm.roundtrip_result({ tag: 'err', val: f }), { tag: 'err', val: 5 });

  assert.deepStrictEqual(wasm.roundtrip_enum(exports.E1.A), exports.E1.A);
  assert.deepStrictEqual(wasm.roundtrip_enum(exports.E1.B), exports.E1.B);

  assert.deepStrictEqual(wasm.invert_bool(true), false);
  assert.deepStrictEqual(wasm.invert_bool(false), true);

  {
    const a: exports.E1.A = exports.E1.A;
    const b: exports.E1.B = exports.E1.B;
  }

  {
    const [a1, a2, a3, a4, a5, a6] = wasm.variant_casts([
      { tag: 'a', val: 1 },
      { tag: 'a', val: 2 },
      { tag: 'a', val: 3 },
      { tag: 'a', val: 4n },
      { tag: 'a', val: 5n },
      { tag: 'a', val: 6 },
    ]);
    assert.deepStrictEqual(a1, { tag: 'a', val: 1 });
    assert.deepStrictEqual(a2, { tag: 'a', val: 2 });
    assert.deepStrictEqual(a3, { tag: 'a', val: 3 });
    assert.deepStrictEqual(a4, { tag: 'a', val: 4n });
    assert.deepStrictEqual(a5, { tag: 'a', val: 5n });
    assert.deepStrictEqual(a6, { tag: 'a', val: 6 });
  }
  {
    const [b1, b2, b3, b4, b5, b6] = wasm.variant_casts([
      { tag: 'b', val: 1n },
      { tag: 'b', val: 2 },
      { tag: 'b', val: 3 },
      { tag: 'b', val: 4 },
      { tag: 'b', val: 5 },
      { tag: 'b', val: 6 },
    ]);
    assert.deepStrictEqual(b1, { tag: 'b', val: 1n });
    assert.deepStrictEqual(b2, { tag: 'b', val: 2 });
    assert.deepStrictEqual(b3, { tag: 'b', val: 3 });
    assert.deepStrictEqual(b4, { tag: 'b', val: 4 });
    assert.deepStrictEqual(b5, { tag: 'b', val: 5 });
    assert.deepStrictEqual(b6, { tag: 'b', val: 6 });
  }

  {
    const [a1, a2, a3, a4] = wasm.variant_zeros([
      { tag: 'a', val: 1 },
      { tag: 'a', val: 2n },
      { tag: 'a', val: 3 },
      { tag: 'a', val: 4 },
    ]);
    assert.deepStrictEqual(a1, { tag: 'a', val: 1 });
    assert.deepStrictEqual(a2, { tag: 'a', val: 2n });
    assert.deepStrictEqual(a3, { tag: 'a', val: 3 });
    assert.deepStrictEqual(a4, { tag: 'a', val: 4 });
  }

  wasm.variant_typedefs(null, false, { tag: 'err' });
}

function test_lists(wasm: exports.Wasm) {
    wasm.list_param(new Uint8Array([1, 2, 3, 4]));
    wasm.list_param2("foo");
    wasm.list_param3(["foo", "bar", "baz"]);
    wasm.list_param4([["foo", "bar"], ["baz"]]);
    assert.deepStrictEqual(Array.from(wasm.list_result()), [1, 2, 3, 4, 5]);
    assert.deepStrictEqual(wasm.list_result2(), "hello!");
    assert.deepStrictEqual(wasm.list_result3(), ["hello,", "world!"]);

    assert.deepStrictEqual(wasm.string_roundtrip("x"), "x");
    assert.deepStrictEqual(wasm.string_roundtrip(""), "");
    assert.deepStrictEqual(wasm.string_roundtrip("hello ⚑ world"), "hello ⚑ world");
}

function test_flavorful(wasm: exports.Wasm) {
  wasm.list_in_record1({ a: "list_in_record1" });
  assert.deepStrictEqual(wasm.list_in_record2(), { a: "list_in_record2" });

  assert.deepStrictEqual(
    wasm.list_in_record3({ a: "list_in_record3 input" }),
    { a: "list_in_record3 output" },
  );

  assert.deepStrictEqual(
    wasm.list_in_record4({ a: "input4" }),
    { a: "result4" },
  );

  wasm.list_in_variant1("foo", { tag: 'err', val: 'bar' }, { tag: '0', val: 'baz' });

  assert.deepStrictEqual(wasm.list_in_variant2(), "list_in_variant2");
  assert.deepStrictEqual(wasm.list_in_variant3("input3"), "output3");

  assert.deepStrictEqual(wasm.errno_result().tag, 'err');

  const [r1, r2] = wasm.list_typedefs("typedef1", ["typedef2"]);
  assert.deepStrictEqual(r1, (new TextEncoder()).encode('typedef3'));
  assert.deepStrictEqual(r2, ['typedef4']);
}

function test_handles(wasm: exports.Wasm) {
  const bytes = wasm.allocated_bytes();

  // Param/result of a handle works in a simple fashion
  const s: exports.WasmState = wasm.wasm_state_create();
  assert.strictEqual(wasm.wasm_state_get_val(s), 100);

  // Deterministic destruction is possible
  assert.strictEqual(wasm.wasm_state2_saw_close(), false);
  const s2: exports.WasmState2 = wasm.wasm_state2_create();
  assert.strictEqual(wasm.wasm_state2_saw_close(), false);
  s2.drop();
  assert.strictEqual(wasm.wasm_state2_saw_close(), true);

  const arg1 = wasm.wasm_state_create();
  const arg2 = wasm.wasm_state2_create();
  const [c, d] = wasm.two_wasm_states(arg1, arg2);
  arg1.drop();
  arg2.drop();

  wasm.wasm_state2_param_record({ a: d });
  wasm.wasm_state2_param_tuple([d]);
  wasm.wasm_state2_param_option(d);
  wasm.wasm_state2_param_option(null);
  wasm.wasm_state2_param_result({ tag: 'ok', val: d });
  wasm.wasm_state2_param_result({ tag: 'err', val: 2 });
  wasm.wasm_state2_param_variant({ tag: '0', val: d });
  wasm.wasm_state2_param_variant({ tag: '1', val: 2 });
  wasm.wasm_state2_param_list([]);
  wasm.wasm_state2_param_list([d]);
  wasm.wasm_state2_param_list([d, d]);

  c.drop();
  d.drop();

  wasm.wasm_state2_result_record().a.drop();
  wasm.wasm_state2_result_tuple()[0].drop();
  const opt = wasm.wasm_state2_result_option();
  if (opt === null)
    throw new Error('should be some');
  opt.drop();
  const result = wasm.wasm_state2_result_result();
  if (result.tag === 'err')
    throw new Error('should be ok');
  result.val.drop();
  const variant = wasm.wasm_state2_result_variant();
  if (variant.tag === '1')
    throw new Error('should be 0');
  variant.val.drop();
  for (let val of wasm.wasm_state2_result_list())
    val.drop();

  s.drop();
  assert.strictEqual(bytes, wasm.allocated_bytes());
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

function test_invalid(wasm: exports.Wasm) {
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
