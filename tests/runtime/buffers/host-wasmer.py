from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Tuple, List, Any
import exports.bindings as e
import imports.bindings as i
import sys
import wasmer # type: ignore

class MyImports:
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

    wasm.test_imports()

if __name__ == '__main__':
    run(sys.argv[1])
