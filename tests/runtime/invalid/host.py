from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
from typing import Callable
import imports.bindings as i
import sys
import wasmtime

class MyImports(Imports):
    def roundtrip_u8(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_s8(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_u16(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_s16(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_bool(self, x: bool) -> bool:
        raise Exception('unreachable')

    def roundtrip_char(self, x: str) -> str:
        raise Exception('unreachable')

    def roundtrip_enum(self, x: i.E) -> i.E:
        raise Exception('unreachable')

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


def run(wasm_file: str) -> None:
    store = wasmtime.Store()
    module = wasmtime.Module.from_file(store.engine, wasm_file)
    linker = wasmtime.Linker(store.engine)
    linker.define_wasi()
    wasi = wasmtime.WasiConfig()
    wasi.inherit_stdout()
    wasi.inherit_stderr()
    store.set_wasi(wasi)

    imports = MyImports()
    add_imports_to_linker(linker, store, imports)
    wasm = Exports(store, linker, module)

    def assert_throws(f: Callable, msg: str) -> None:
        try:
            f()
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

    assert_throws(lambda: wasm.invalid_bool(store), 'invalid variant discriminant for bool')
    assert_throws(lambda: wasm.invalid_u8(store), 'must be between')
    assert_throws(lambda: wasm.invalid_s8(store), 'must be between')
    assert_throws(lambda: wasm.invalid_u16(store), 'must be between')
    assert_throws(lambda: wasm.invalid_s16(store), 'must be between')
    assert_throws(lambda: wasm.invalid_char(store), 'not a valid char')
    assert_throws(lambda: wasm.invalid_enum(store), 'not a valid E')
    assert_throws(lambda: wasm.test_unaligned(store), 'is not aligned');

if __name__ == '__main__':
    run(sys.argv[1])
