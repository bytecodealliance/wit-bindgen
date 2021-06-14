import { readFileSync } from 'fs';
import * as assert from 'assert';
import * as bindings from './bindings.js';
import { WASI } from 'wasi';

function run() {
  const wasm = readFileSync(process.argv[2]);
  const module = new WebAssembly.Module(wasm);
  const wasi = new WASI({
    args: process.argv,
    env: process.env,
  });
  const imports = {
    wasi_snapshot_preview1: wasi.wasiImport,
  };
  let instance: WebAssembly.Instance | null = null;
  bindings.add_host_to_imports(imports, host(), name => {
    return instance ? instance.exports[name] : null;
  });
  instance = new WebAssembly.Instance(module, imports);
  wasi.initialize(instance);
  let run = instance.exports.run_import_tests as CallableFunction;
  run();
}

function host(): bindings.Host {
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
    roundtrip_usize(x) { return x; },
    multiple_results() { return { a: 4, b: 5 }; },
    set_scalar(x) { scalar = x; },
    get_scalar() { return scalar; },
    swap_tuple([a, b]) { return [b, a]; },
    roundtrip_flags1(x) { return x; },
    roundtrip_flags2(x) { return x; },
    roundtrip_flags3(r0, r1, r2, r3) { return { r0, r1, r2, r3 }; },
    legacy_flags1(x) { return x; },
    legacy_flags2(x) { return x; },
    legacy_flags3(x) { return x; },
    legacy_flags4(x) { return x; },
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
    legacy_params(a, b, c) {},
    legacy_result(succeed) {
      if (succeed) {
        return {
          tag: 'ok',
          val: [
            1,
            2,
            3,
            4,
            5,
            6,
            7n,
            8n,
            9.,
            10.,
            {
              a: 0,
              b: 0,
            },
          ],
        };
      } else {
        return {
          tag: 'err',
          val: bindings.E1.B,
        };
      }
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
    host_state_create() { return 100; },
    host_state_get(x) { return x; },
    host_state2_create() { return 101; },
    host_state2_saw_close() { return saw_close; },
    drop_host_state2(state) { saw_close = true; },
    two_host_states(a, b) { return { c: b, d: a }; },
    host_state2_param_record(x) {},
    host_state2_param_tuple(x) {},
    host_state2_param_option(x) {},
    host_state2_param_result(x) {},
    host_state2_param_variant(x) {},
    host_state2_param_list(x) {},

    host_state2_result_record() { return { a: null }; },
    host_state2_result_tuple() { return [null]; },
    host_state2_result_option() { return 102; },
    host_state2_result_result() { return { tag: 'ok', val: null }; },
    host_state2_result_variant() { return { tag: '0', val: null }; },
    host_state2_result_list() { return [null, 3]; },

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
      assert.ok(x.length < out.length);
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

    errno_result() { return { tag: 'err', val: bindings.MyErrno.B }; },
    list_typedefs(x, y) {
      assert.strictEqual(x, 'typedef1');
      assert.deepStrictEqual(y, ['typedef2']);
      return { r1: (new TextEncoder).encode('typedef3'), r2: ['typedef4'] };
    },
  };
}

run()
