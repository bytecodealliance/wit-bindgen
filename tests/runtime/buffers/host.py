from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
from typing import Tuple, List
import exports.bindings as e
import imports.bindings as i
import sys
import wasmtime

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

    wasm.test_imports(store)

if __name__ == '__main__':
    run(sys.argv[1])
