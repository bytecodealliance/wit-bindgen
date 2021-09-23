from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
import sys
import wasmtime

class MyImports:
    def thunk(self) -> None:
        self.hit = True

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
    wasm.thunk(store)
    assert(imports.hit)

if __name__ == '__main__':
    run(sys.argv[1])
