from dataclasses import dataclass
from exports.bindings import Wasm
from imports.bindings import add_host_to_linker, Host, Expected
from typing import Tuple, Optional, List
import exports.bindings as e
import imports.bindings as i
import math
import sys
import wasmtime


@dataclass
class HostState(i.HostState):
    val: int

    def __init__(self, val: int) -> None:
        self.val = val


HOST_STATE2_CLOSED = False


@dataclass
class HostState2(i.HostState2):
    val: int

    def __init__(self, val: int) -> None:
        self.val = val

    def drop(self) -> None:
        global HOST_STATE2_CLOSED
        HOST_STATE2_CLOSED = True


@dataclass
class Markdown(i.Markdown2):
    buf: str = ''

    def append(self, data: str) -> None:
        self.buf += data

    def render(self) -> str:
        return self.buf.replace('red', 'green')


class HostImpl(Host):

    def roundtrip_option(self, a: Optional[float]) -> Optional[int]:
        if a:
            return int(a)
        return None

    def roundtrip_result(self, a: i.Expected[int, float]) -> i.Expected[float, int]:
        if isinstance(a, i.Ok):
            return i.Ok(float(a.value))
        return i.Err(int(a.value))

    def roundtrip_enum(self, a: i.E1) -> i.E1:
        return a

    def invert_bool(self, a: bool) -> bool:
        return not a

    def variant_casts(self, a: i.Casts) -> i.Casts:
        return a

    def variant_zeros(self, a: i.Zeros) -> i.Zeros:
        return a

    def variant_typedefs(self, a: i.OptionTypedef, b: i.BoolTypedef, c: i.ResultTypedef) -> None:
        pass

    def variant_enums(self, a: bool, b: Expected[None, None], c: i.MyErrno) -> Tuple[bool, Expected[None, None], i.MyErrno]:
        assert(a)
        assert(isinstance(b, i.Ok))
        assert(c == i.MyErrno.SUCCESS)
        return (False, i.Err(None), i.MyErrno.A)

    def list_param(self, a: bytes) -> None:
        assert(a == b'\x01\x02\x03\x04')

    def list_param2(self, a: str) -> None:
        assert(a == 'foo')

    def list_param3(self, a: List[str]) -> None:
        assert(a == ['foo', 'bar', 'baz'])

    def list_param4(self, a: List[List[str]]) -> None:
        assert(a == [['foo', 'bar'], ['baz']])

    def list_result(self) -> bytes:
        return b'\x01\x02\x03\x04\x05'

    def list_result2(self) -> str:
        return 'hello!'

    def list_result3(self) -> List[str]:
        return ['hello,', 'world!']

    def string_roundtrip(self, a: str) -> str:
        return a

    def unaligned_roundtrip1(self, a: List[int], b: List[int], c: List[int], d: List[i.Flag32], e: List[i.Flag64]) -> None:
        assert(a == [1])
        assert(b == [2])
        assert(c == [3])
        assert(d == [i.Flag32.B8])
        assert(e == [i.Flag64.B9])

    def unaligned_roundtrip2(self, a: List[i.UnalignedRecord], b: List[float], c: List[float], d: List[str], e: List[bytes]) -> None:
          assert(a == [i.UnalignedRecord(a=10, b=11)])
          assert(b == [100.0])
          assert(c == [101.0])
          assert(d == ['foo'])
          assert(e == [b'\x66'])

    def list_minmax8(self, a: bytes, b: List[int]) -> Tuple[bytes, List[int]]:
        assert(a == b'\x00\xff')
        assert(b == [-(1 << (8 - 1)), (1 << (8 - 1)) - 1])
        return (a, b)

    def list_minmax16(self, a: List[int], b: List[int]) -> Tuple[List[int], List[int]]:
        assert(a == [0, (1 << 16) - 1])
        assert(b == [-(1 << (16 - 1)), (1 << (16 - 1)) - 1])
        return (a, b)

    def list_minmax32(self, a: List[int], b: List[int]) -> Tuple[List[int], List[int]]:
        assert(a == [0, (1 << 32) - 1])
        assert(b == [-(1 << (32 - 1)), (1 << (32 - 1)) - 1])
        return (a, b)

    def list_minmax64(self, a: List[int], b: List[int]) -> Tuple[List[int], List[int]]:
        assert(a == [0, (1 << 64) - 1])
        assert(b == [-(1 << (64 - 1)), (1 << (64 - 1)) - 1])
        return (a, b)

    def list_minmax_float(self, a: List[float], b: List[float]) -> Tuple[List[float], List[float]]:
        assert(a == [-3.4028234663852886e+38, 3.4028234663852886e+38, -float('inf'), float('inf')])
        assert(b == [-sys.float_info.max, sys.float_info.max, -float('inf'), float('inf')])
        return (a, b)

    def host_state_create(self) -> i.HostState:
        return HostState(100)

    def host_state_get(self, a: i.HostState) -> int:
        assert(isinstance(a, HostState))
        return a.val

    def host_state2_create(self) -> i.HostState2:
        return HostState2(101)

    def host_state2_saw_close(self) -> bool:
        return HOST_STATE2_CLOSED

    def two_host_states(self, a: i.HostState, b: i.HostState2) -> Tuple[i.HostState, i.HostState2]:
        return (b, a)

    def host_state2_param_record(self, a: i.HostStateParamRecord) -> None:
        pass

    def host_state2_param_tuple(self, a: i.HostStateParamTuple) -> None:
        pass

    def host_state2_param_option(self, a: i.HostStateParamOption) -> None:
        pass

    def host_state2_param_result(self, a: i.HostStateParamResult) -> None:
        pass

    def host_state2_param_variant(self, a: i.HostStateParamVariant) -> None:
        pass

    def host_state2_param_list(self, a: List[i.HostState2]) -> None:
        pass

    def host_state2_result_record(self) -> i.HostStateResultRecord:
        return i.HostStateResultRecord(HostState(2))

    def host_state2_result_tuple(self) -> i.HostStateResultTuple:
        return (HostState(2),)

    def host_state2_result_option(self) -> i.HostStateResultOption:
        return HostState(2)

    def host_state2_result_result(self) -> i.HostStateResultResult:
        return i.Ok(HostState2(2))

    def host_state2_result_variant(self) -> i.HostStateResultVariant:
        return i.HostStateResultVariant0(HostState2(2))

    def host_state2_result_list(self) -> List[i.HostState2]:
        return [HostState2(2), HostState2(5)]

    def markdown2_create(self) -> i.Markdown2:
        return Markdown()

    def buffer_u8(self, a: i.PullBuffer[int], b: i.PushBuffer[int]) -> int:
        assert(len(a) == 1)
        assert(len(b) == 10)
        assert(a.pull() == 0)
        assert(a.pull() == None)
        b.push(1)
        b.push(2)
        b.push(3)
        return 3

    def buffer_u32(self, a: i.PullBuffer[int], b: i.PushBuffer[int]) -> int:
        assert(len(a) == 1)
        assert(len(b) == 10)
        assert(a.pull() == 0)
        assert(a.pull() == None)
        b.push(1)
        b.push(2)
        b.push(3)
        return 3

    def buffer_bool(self, a: i.PullBuffer[bool], b: i.PushBuffer[bool]) -> int:
        assert(len(a) <= len(b))
        n = 0
        while True:
            val = a.pull()
            if val is None:
                break
            b.push(not val)
            n += 1
        return n

    def buffer_mutable1(self, x: List[i.PullBuffer[bool]]) -> None:
        assert(len(x) == 1)
        assert(len(x[0]) == 5)
        assert(x[0].pull() == True)
        assert(x[0].pull() == False)
        assert(x[0].pull() == True)
        assert(x[0].pull() == True)
        assert(x[0].pull() == False)
        assert(x[0].pull() == None)

    def buffer_mutable2(self, a: List[i.PushBuffer[int]]) -> int:
        assert(len(a) == 1)
        assert(len(a[0]) > 4)
        a[0].push(1)
        a[0].push(2)
        a[0].push(3)
        a[0].push(4)
        return 4

    def buffer_mutable3(self, a: List[i.PushBuffer[bool]]) -> int:
        assert(len(a) == 1)
        assert(len(a[0]) > 3)
        a[0].push(False)
        a[0].push(True)
        a[0].push(False)
        return 3

    def buffer_in_record(self, a: i.BufferInRecord) -> None:
        pass

    def buffer_typedef(self, a: i.ParamInBufferU8, b: i.ParamOutBufferU8, c: i.ParamInBufferBool, d: i.ParamOutBufferBool) -> None:
        pass

    def list_in_record1(self, a: i.ListInRecord1) -> None:
        pass

    def list_in_record2(self) -> i.ListInRecord2:
        return i.ListInRecord2('list_in_record2')

    def list_in_record3(self, a: i.ListInRecord3) -> i.ListInRecord3:
        assert(a.a == 'list_in_record3 input')
        return i.ListInRecord3('list_in_record3 output')

    def list_in_record4(self, a: i.ListInAlias) -> i.ListInAlias:
        assert(a.a == 'input4')
        return i.ListInRecord4('result4')

    def list_in_variant1(self, a: i.ListInVariant11, b: i.ListInVariant12, c: i.ListInVariant13) -> None:
        assert(a == 'foo')
        assert(b == i.Err('bar'))
        assert(c == i.ListInVariant130('baz'))

    def list_in_variant2(self) -> i.ListInVariant2:
        return 'list_in_variant2'

    def list_in_variant3(self, a: i.ListInVariant3) -> i.ListInVariant3:
        assert(a == 'input3')
        return 'output3'

    def errno_result(self) -> i.Expected[None, i.MyErrno]:
        return i.Err(i.MyErrno.B)

    def list_typedefs(self, a: i.ListTypedef, c: i.ListTypedef3) -> Tuple[i.ListTypedef2, i.ListTypedef3]:
        assert(a == 'typedef1')
        assert(c == ['typedef2'])
        return (b'typedef3', ['typedef4'])

    def list_of_variants(self, a: List[bool], b: List[i.Expected[None, None]], c: List[i.MyErrno]) -> Tuple[List[bool], List[i.Expected[None, None]], List[i.MyErrno]]:
          assert(a == [True, False])
          assert(b == [i.Ok(None), i.Err(None)])
          assert(c == [i.MyErrno.SUCCESS, i.MyErrno.A])
          return (
                [False, True],
                [i.Err(None), i.Ok(None)],
                [i.MyErrno.A, i.MyErrno.B],
          )


def run(wasm_file: str) -> None:
    print('Running', wasm_file)
    store = wasmtime.Store()
    module = wasmtime.Module.from_file(store.engine, wasm_file)
    linker = wasmtime.Linker(store.engine)
    linker.define_wasi()
    wasi = wasmtime.WasiConfig()
    wasi.inherit_stdout()
    wasi.inherit_stderr()
    store.set_wasi(wasi)

    # Define state imported from python and register it with our linker
    host = HostImpl()
    add_host_to_linker(linker, store, host)

    # Using the linker, instantiate the module and wrap it up in the export
    # bindings.
    wasm = Wasm(store, linker, module)

    # Run all the tests!

    allocated_bytes = wasm.allocated_bytes(store)
    wasm.run_import_tests(store)
    test_scalars(wasm, store)
    test_records(wasm, store)
    test_variants(wasm, store)
    test_lists(wasm, store)
    test_flavorful(wasm, store)
    test_invalid(wasm, store)
    test_handles(wasm, store)

    # Ensure that we properly called `free` everywhere in all the glue that we
    # needed to.
    assert(allocated_bytes == wasm.allocated_bytes(store))



def test_variants(wasm: Wasm, store: wasmtime.Store) -> None:
    assert(wasm.roundtrip_option(store, 1.) == 1)
    assert(wasm.roundtrip_option(store, None) == None)
    assert(wasm.roundtrip_option(store, 2.) == 2)
    assert(wasm.roundtrip_result(store, e.Ok(2)) == e.Ok(2))
    assert(wasm.roundtrip_result(store, e.Ok(4)) == e.Ok(4))
    assert(wasm.roundtrip_result(store, e.Err(5)) == e.Err(5))

    assert(wasm.roundtrip_enum(store, e.E1.A) == e.E1.A)
    assert(wasm.roundtrip_enum(store, e.E1.B) == e.E1.B)

    assert(wasm.invert_bool(store, True) == False)
    assert(wasm.invert_bool(store, False) == True)

    a1, a2, a3, a4, a5, a6 = wasm.variant_casts(store, (
        e.C1A(1),
        e.C2A(2),
        e.C3A(3),
        e.C4A(4),
        e.C5A(5),
        e.C6A(6.),
    ))
    assert(a1 == e.C1A(1))
    assert(a2 == e.C2A(2))
    assert(a3 == e.C3A(3))
    assert(a4 == e.C4A(4))
    assert(a5 == e.C5A(5))
    assert(a6 == e.C6A(6))

    b1, b2, b3, b4, b5, b6 = wasm.variant_casts(store, (
        e.C1B(1),
        e.C2B(2),
        e.C3B(3),
        e.C4B(4),
        e.C5B(5),
        e.C6B(6.),
    ))
    assert(b1 == e.C1B(1))
    assert(b2 == e.C2B(2))
    assert(b3 == e.C3B(3))
    assert(b4 == e.C4B(4))
    assert(b5 == e.C5B(5))
    assert(b6 == e.C6B(6))

    z1, z2, z3, z4 = wasm.variant_zeros(store, (
        e.Z1A(1),
        e.Z2A(2),
        e.Z3A(3.),
        e.Z4A(4.),
    ))
    assert(z1 == e.Z1A(1))
    assert(z2 == e.Z2A(2))
    assert(z3 == e.Z3A(3))
    assert(z4 == e.Z4A(4))

    wasm.variant_typedefs(store, None, False, e.Err(None))


def test_lists(wasm: Wasm, store: wasmtime.Store) -> None:
    wasm.list_param(store, b'\x01\x02\x03\x04')
    wasm.list_param2(store, "foo")
    wasm.list_param3(store, ["foo", "bar", "baz"])
    wasm.list_param4(store, [["foo", "bar"], ["baz"]])
    assert(wasm.list_result(store) == b'\x01\x02\x03\x04\x05')
    assert(wasm.list_result2(store) == "hello!")
    assert(wasm.list_result3(store) == ["hello,", "world!"])

    assert(wasm.string_roundtrip(store, "x") == "x")
    assert(wasm.string_roundtrip(store, "") == "")
    assert(wasm.string_roundtrip(store, "hello ⚑ world") == "hello ⚑ world")


def test_flavorful(wasm: Wasm, store: wasmtime.Store) -> None:
    wasm.list_in_record1(store, e.ListInRecord1("list_in_record1"))
    assert(wasm.list_in_record2(store) == e.ListInRecord2(a="list_in_record2"))

    assert(wasm.list_in_record3(store, e.ListInRecord3("list_in_record3 input")).a == "list_in_record3 output")
    assert(wasm.list_in_record4(store, e.ListInRecord4("input4")).a == "result4")

    wasm.list_in_variant1(store, "foo", e.Err("bar"), e.ListInVariant130('baz'))
    assert(wasm.list_in_variant2(store) == "list_in_variant2")
    assert(wasm.list_in_variant3(store, "input3") == "output3")

    assert(isinstance(wasm.errno_result(store), e.Err))

    r1, r2 = wasm.list_typedefs(store, "typedef1", ["typedef2"])
    assert(r1 == b'typedef3')
    assert(r2 == ['typedef4'])


def test_invalid(wasm: Wasm, store: wasmtime.Store) -> None:
    def assert_throws(name: str, msg: str) -> None:
        export = wasm.instance.exports(store)[name]
        assert(isinstance(export, wasmtime.Func))
        try:
            export(store)
            raise RuntimeError('expected exception')
        except TypeError as e:
            actual = str(e)
        except OverflowError as e:
            actual = str(e)
        except ValueError as e:
            actual = str(e)
        except IndexError as e:
            actual = str(e)
        if not msg in actual:
            print(actual)
            assert(msg in actual)

    assert_throws('invalid_bool', 'invalid variant discriminant for bool')
    assert_throws('invalid_u8', 'must be between')
    assert_throws('invalid_s8', 'must be between')
    assert_throws('invalid_u16', 'must be between')
    assert_throws('invalid_s16', 'must be between')
    assert_throws('invalid_char', 'not a valid char')
    assert_throws('invalid_e1', 'not a valid E1')
    assert_throws('invalid_handle', 'handle index not valid')
    assert_throws('invalid_handle_close', 'handle index not valid')


def test_handles(wasm: Wasm, store: wasmtime.Store) -> None:
    # Param/result of a handle works in a simple fashion
    s: e.WasmState = wasm.wasm_state_create(store)
    assert(wasm.wasm_state_get_val(store, s) == 100)

    # Deterministic destruction is possible
    assert(wasm.wasm_state2_saw_close(store) == False)
    s2: e.WasmState2 = wasm.wasm_state2_create(store)
    assert(wasm.wasm_state2_saw_close(store) == False)
    s2.drop(store)
    assert(wasm.wasm_state2_saw_close(store) == True)

    arg1 = wasm.wasm_state_create(store)
    arg2 = wasm.wasm_state2_create(store)
    c, d = wasm.two_wasm_states(store, arg1, arg2)
    arg1.drop(store)
    arg2.drop(store)

    wasm.wasm_state2_param_record(store, e.WasmStateParamRecord(d))
    wasm.wasm_state2_param_tuple(store, (d,))
    wasm.wasm_state2_param_option(store, d)
    wasm.wasm_state2_param_option(store, None)
    wasm.wasm_state2_param_result(store, e.Ok(d))
    wasm.wasm_state2_param_result(store, e.Err(2))
    wasm.wasm_state2_param_variant(store, e.WasmStateParamVariant0(d))
    wasm.wasm_state2_param_variant(store, e.WasmStateParamVariant1(2))
    wasm.wasm_state2_param_list(store, [])
    wasm.wasm_state2_param_list(store, [d])
    wasm.wasm_state2_param_list(store, [d, d])

    c.drop(store)
    d.drop(store)

    wasm.wasm_state2_result_record(store).a.drop(store)
    wasm.wasm_state2_result_tuple(store)[0].drop(store)
    opt = wasm.wasm_state2_result_option(store)
    assert(opt is not None)
    opt.drop(store)
    result = wasm.wasm_state2_result_result(store)
    assert(isinstance(result, e.Ok))
    result.value.drop(store)
    variant = wasm.wasm_state2_result_variant(store)
    print(variant)
    assert(isinstance(variant, e.WasmStateResultVariant0))
    variant.value.drop(store)
    for val in wasm.wasm_state2_result_list(store):
        val.drop(store)

    s.drop(store)

    md = e.Markdown.create(store, wasm)
    if md:
        md.append(store, "red is the best color")
        assert(md.render(store) == "green is the best color")
        md.drop(store)

if __name__ == '__main__':
    run(sys.argv[1])
