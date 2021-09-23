from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
from typing import Tuple
import exports.bindings as e
import imports.bindings as i
import sys
import wasmtime

class MyImports:
    def multiple_results(self) -> Tuple[int, int]:
        return (4, 5)

    def swap_tuple(self, a: Tuple[int, int]) -> Tuple[int, int]:
        return (a[1], a[0])

    def roundtrip_flags1(self, a: i.F1) -> i.F1:
        return a

    def roundtrip_flags2(self, a: i.F2) -> i.F2:
        return a

    def roundtrip_flags3(self, a: i.Flag8, b: i.Flag16, c: i.Flag32, d: i.Flag64) -> Tuple[i.Flag8, i.Flag16, i.Flag32, i.Flag64]:
        return (a, b, c, d)

    def roundtrip_record1(self, a: i.R1) -> i.R1:
        return a

    def tuple0(self, a: None) -> None:
        pass

    def tuple1(self, a: Tuple[int]) -> Tuple[int]:
        return (a[0],)

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
    assert(wasm.multiple_results(store) == (100, 200))
    assert(wasm.swap_tuple(store, (1, 2)) == (2, 1))
    assert(wasm.roundtrip_flags1(store, e.F1.A) == e.F1.A)
    assert(wasm.roundtrip_flags1(store, e.F1(0)) == e.F1(0))
    assert(wasm.roundtrip_flags1(store, e.F1.A | e.F1.B) == (e.F1.A | e.F1.B))

    assert(wasm.roundtrip_flags2(store, e.F2.C) == e.F2.C)
    assert(wasm.roundtrip_flags2(store, e.F2(0)) == e.F2(0))
    assert(wasm.roundtrip_flags2(store, e.F2.D) == e.F2.D)
    assert(wasm.roundtrip_flags2(store, e.F2.C | e.F2.E) == (e.F2.C | e.F2.E))

    r = wasm.roundtrip_record1(store, e.R1(8, e.F1(0)))
    assert(r.a == 8)
    assert(r.b == e.F1(0))

    r = wasm.roundtrip_record1(store, e.R1(a=0, b=e.F1.A | e.F1.B))
    assert(r.a == 0)
    assert(r.b == (e.F1.A | e.F1.B))

    wasm.tuple0(store, None)
    assert(wasm.tuple1(store, (1,)) == (1,))

if __name__ == '__main__':
    run(sys.argv[1])
