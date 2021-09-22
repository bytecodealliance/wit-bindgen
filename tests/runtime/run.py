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

    wasm.run_import_tests(store)
    test_scalars(wasm, store)
    test_records(wasm, store)
    test_variants(wasm, store)
    test_lists(wasm, store)
    test_flavorful(wasm, store)
    test_invalid(wasm, store)
    test_handles(wasm, store)






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
