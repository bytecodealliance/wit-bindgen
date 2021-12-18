from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Any
import sys
import wasmer # type: ignore

class MyImports:
    def thunk(self) -> None:
        self.hit = True

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
    wasm.thunk()
    assert(imports.hit)

if __name__ == '__main__':
    run(sys.argv[1])
