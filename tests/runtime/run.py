from dataclasses import dataclass
from exports.bindings import Wasm
from imports.bindings import add_host_to_linker, Host, Expected
from typing import Tuple, Optional, List
import exports.bindings as e
import imports.bindings as i
import math
import sys
import wasmtime




class HostImpl(Host):



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

if __name__ == '__main__':
    run(sys.argv[1])
