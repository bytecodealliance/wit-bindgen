from dataclasses import dataclass
from exports.bindings import Wasm
from imports.bindings import add_host_to_linker, Host, Expected
from typing import Tuple, Optional, List
import exports.bindings as e
import imports.bindings as i
import math
import sys
import wasmtime





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
