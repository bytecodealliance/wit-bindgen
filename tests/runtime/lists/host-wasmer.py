from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Tuple, List, Any
import exports.bindings as e
import imports.bindings as i
import sys
import wasmer # type: ignore

class MyImports:
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
        # Wasmer's python API doesn't support unaligned memory views
        pass
        #assert(a == [1])
        #assert(b == [2])
        #assert(c == [3])
        #assert(d == [i.Flag32.B8])
        #assert(e == [i.Flag64.B9])

    def unaligned_roundtrip2(self, a: List[i.UnalignedRecord], b: List[float], c: List[float], d: List[str], e: List[bytes]) -> None:
        # Wasmer's python API doesn't support unaligned memory views
        pass
        #assert(a == [i.UnalignedRecord(a=10, b=11)])
        #assert(b == [100.0])
        #assert(c == [101.0])
        #assert(d == ['foo'])
        #assert(e == [b'\x66'])

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

def run(wasm_file: str) -> None:
    store = wasmer.Store()
    module = wasmer.Module(store, open(wasm_file, 'rb').read())
    wasi_version = wasmer.wasi.get_version(module, strict=False)
    if wasi_version is None:
        import_object = {}
    else:
        wasi_env = wasmer.wasi.StateBuilder('test').finalize()
        import_object = wasi_env.generate_imports(store, wasi_version)

    wasm: Exports
    def get_export(name: str) -> Any:
        return wasm.instance.exports.__getattribute__(name)

    imports = MyImports()
    add_imports_to_imports(store, import_object, imports, get_export)
    wasm = Exports(store, import_object, module)

    allocated_bytes = wasm.allocated_bytes()
    wasm.test_imports()
    wasm.list_param(b'\x01\x02\x03\x04')
    wasm.list_param2("foo")
    wasm.list_param3(["foo", "bar", "baz"])
    wasm.list_param4([["foo", "bar"], ["baz"]])
    assert(wasm.list_result() == b'\x01\x02\x03\x04\x05')
    assert(wasm.list_result2() == "hello!")
    assert(wasm.list_result3() == ["hello,", "world!"])

    assert(wasm.string_roundtrip("x") == "x")
    assert(wasm.string_roundtrip("") == "")
    assert(wasm.string_roundtrip("hello ⚑ world") == "hello ⚑ world")

    # Ensure that we properly called `free` everywhere in all the glue that we
    # needed to.
    assert(allocated_bytes == wasm.allocated_bytes())

if __name__ == '__main__':
    run(sys.argv[1])
