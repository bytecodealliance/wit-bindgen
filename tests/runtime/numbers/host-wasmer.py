from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Any
import math;
import sys
import wasmer # type: ignore

class MyImports:
    def roundtrip_u8(self, a: int) -> int:
        return a

    def roundtrip_s8(self, a: int) -> int:
        return a

    def roundtrip_u16(self, a: int) -> int:
        return a

    def roundtrip_s16(self, a: int) -> int:
        return a

    def roundtrip_u32(self, a: int) -> int:
        return a

    def roundtrip_s32(self, a: int) -> int:
        return a

    def roundtrip_u64(self, a: int) -> int:
        return a

    def roundtrip_s64(self, a: int) -> int:
        return a

    def roundtrip_f32(self, a: float) -> float:
        return a

    def roundtrip_f64(self, a: float) -> float:
        return a

    def roundtrip_char(self, a: str) -> str:
        return a

    def set_scalar(self, a: int) -> None:
        self.scalar = a

    def get_scalar(self) -> int:
        return self.scalar


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
    assert(wasm.roundtrip_u8(1) == 1)
    assert(wasm.roundtrip_u8((1 << 8) - 1) == (1 << 8) - 1)
    assert(wasm.roundtrip_u16(1) == 1)
    assert(wasm.roundtrip_u16((1 << 16) - 1) == (1 << 16) - 1)
    assert(wasm.roundtrip_u32(1) == 1)
    assert(wasm.roundtrip_u32((1 << 32) - 1) == (1 << 32) - 1)
    assert(wasm.roundtrip_u64(1) == 1)
    assert(wasm.roundtrip_u64((1 << 64) - 1) == (1 << 64) - 1)

    assert(wasm.roundtrip_s8(1) == 1)
    assert(wasm.roundtrip_s8((1 << (8 - 1) - 1)) == (1 << (8 - 1) - 1))
    assert(wasm.roundtrip_s8(-(1 << (8 - 1))) == -(1 << (8 - 1)))
    assert(wasm.roundtrip_s16(1) == 1)
    assert(wasm.roundtrip_s16((1 << (16 - 1) - 1)) == (1 << (16 - 1) - 1))
    assert(wasm.roundtrip_s16(-(1 << (16 - 1))) == -(1 << (16 - 1)))
    assert(wasm.roundtrip_s32(1) == 1)
    assert(wasm.roundtrip_s32((1 << (32 - 1) - 1)) == (1 << (32 - 1) - 1))
    assert(wasm.roundtrip_s32(-(1 << (32 - 1))) == -(1 << (32 - 1)))
    assert(wasm.roundtrip_s64(1) == 1)
    assert(wasm.roundtrip_s64((1 << (64 - 1) - 1)) == (1 << (64 - 1) - 1))
    assert(wasm.roundtrip_s64(-(1 << (64 - 1))) == -(1 << (64 - 1)))

    inf = float('inf')
    assert(wasm.roundtrip_f32(1.0) == 1.0)
    assert(wasm.roundtrip_f32(inf) == inf)
    assert(wasm.roundtrip_f32(-inf) == -inf)
    assert(math.isnan(wasm.roundtrip_f32(float('nan'))))

    assert(wasm.roundtrip_f64(1.0) == 1.0)
    assert(wasm.roundtrip_f64(inf) == inf)
    assert(wasm.roundtrip_f64(-inf) == -inf)
    assert(math.isnan(wasm.roundtrip_f64(float('nan'))))

    assert(wasm.roundtrip_char('a') == 'a')
    assert(wasm.roundtrip_char(' ') == ' ')
    assert(wasm.roundtrip_char('🚩') == '🚩')

    wasm.set_scalar(2)
    assert(wasm.get_scalar() == 2)
    wasm.set_scalar(4)
    assert(wasm.get_scalar() == 4)

if __name__ == '__main__':
    run(sys.argv[1])
