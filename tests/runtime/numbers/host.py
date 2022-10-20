import math
import wasmtime
from numbers import Numbers, NumbersImports
from helpers import TestWasi

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

    def roundtrip_float32(self, a: float) -> float:
        return a

    def roundtrip_float64(self, a: float) -> float:
        return a

    def roundtrip_char(self, a: str) -> str:
        return a

    def set_scalar(self, a: int) -> None:
        self.scalar = a

    def get_scalar(self) -> int:
        return self.scalar

def run() -> None:
    store = wasmtime.Store()
    wasm = Numbers(store, NumbersImports(MyImports(), TestWasi()))

    wasm.test_imports(store)
    assert(wasm.roundtrip_u8(store, 1) == 1)
    assert(wasm.roundtrip_u8(store, (1 << 8) - 1) == (1 << 8) - 1)
    assert(wasm.roundtrip_u16(store, 1) == 1)
    assert(wasm.roundtrip_u16(store, (1 << 16) - 1) == (1 << 16) - 1)
    assert(wasm.roundtrip_u32(store, 1) == 1)
    assert(wasm.roundtrip_u32(store, (1 << 32) - 1) == (1 << 32) - 1)
    assert(wasm.roundtrip_u64(store, 1) == 1)
    assert(wasm.roundtrip_u64(store, (1 << 64) - 1) == (1 << 64) - 1)

    assert(wasm.roundtrip_s8(store, 1) == 1)
    assert(wasm.roundtrip_s8(store, (1 << (8 - 1) - 1)) == (1 << (8 - 1) - 1))
    assert(wasm.roundtrip_s8(store, -(1 << (8 - 1))) == -(1 << (8 - 1)))
    assert(wasm.roundtrip_s16(store, 1) == 1)
    assert(wasm.roundtrip_s16(store, (1 << (16 - 1) - 1)) == (1 << (16 - 1) - 1))
    assert(wasm.roundtrip_s16(store, -(1 << (16 - 1))) == -(1 << (16 - 1)))
    assert(wasm.roundtrip_s32(store, 1) == 1)
    assert(wasm.roundtrip_s32(store, (1 << (32 - 1) - 1)) == (1 << (32 - 1) - 1))
    assert(wasm.roundtrip_s32(store, -(1 << (32 - 1))) == -(1 << (32 - 1)))
    assert(wasm.roundtrip_s64(store, 1) == 1)
    assert(wasm.roundtrip_s64(store, (1 << (64 - 1) - 1)) == (1 << (64 - 1) - 1))
    assert(wasm.roundtrip_s64(store, -(1 << (64 - 1))) == -(1 << (64 - 1)))

    inf = float('inf')
    assert(wasm.roundtrip_float32(store, 1.0) == 1.0)
    assert(wasm.roundtrip_float32(store, inf) == inf)
    assert(wasm.roundtrip_float32(store, -inf) == -inf)
    assert(math.isnan(wasm.roundtrip_float32(store, float('nan'))))

    assert(wasm.roundtrip_float64(store, 1.0) == 1.0)
    assert(wasm.roundtrip_float64(store, inf) == inf)
    assert(wasm.roundtrip_float64(store, -inf) == -inf)
    assert(math.isnan(wasm.roundtrip_float64(store, float('nan'))))

    assert(wasm.roundtrip_char(store, 'a') == 'a')
    assert(wasm.roundtrip_char(store, ' ') == ' ')
    assert(wasm.roundtrip_char(store, '🚩') == '🚩')

    wasm.set_scalar(store, 2)
    assert(wasm.get_scalar(store) == 2)
    wasm.set_scalar(store, 4)
    assert(wasm.get_scalar(store) == 4)

if __name__ == '__main__':
    run()
