from smoke import Smoke, SmokeImports
from helpers import TestWasi
import wasmtime

class MyImports:
    def thunk(self) -> None:
        self.hit = True

def run() -> None:
    store = wasmtime.Store()

    imports = MyImports()
    wasm = Smoke(store, SmokeImports(imports, TestWasi()))

    wasm.thunk(store)
    assert(imports.hit)

if __name__ == '__main__':
    run()
