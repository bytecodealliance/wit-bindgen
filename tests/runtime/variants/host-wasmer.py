from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Optional, Tuple, Any
import exports.bindings as e
import imports.bindings as i
import sys
import wasmer # type: ignore

class MyImports:
    def roundtrip_option(self, a: Optional[float]) -> Optional[int]:
        if a:
            return int(a)
        return None

    def roundtrip_result(self, a: i.Expected[int, float]) -> i.Expected[float, int]:
        if isinstance(a, i.Ok):
            return i.Ok(float(a.value))
        return i.Err(int(a.value))

    def roundtrip_enum(self, a: i.E1) -> i.E1:
        return a

    def invert_bool(self, a: bool) -> bool:
        return not a

    def variant_casts(self, a: i.Casts) -> i.Casts:
        return a

    def variant_zeros(self, a: i.Zeros) -> i.Zeros:
        return a

    def variant_typedefs(self, a: i.OptionTypedef, b: i.BoolTypedef, c: i.ResultTypedef) -> None:
        pass

    def variant_enums(self, a: bool, b: i.Expected[None, None], c: i.MyErrno) -> Tuple[bool, i.Expected[None, None], i.MyErrno]:
        assert(a)
        assert(isinstance(b, i.Ok))
        assert(c == i.MyErrno.SUCCESS)
        return (False, i.Err(None), i.MyErrno.A)

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

    assert(wasm.roundtrip_option(1.) == 1)
    assert(wasm.roundtrip_option(None) == None)
    assert(wasm.roundtrip_option(2.) == 2)
    assert(wasm.roundtrip_result(e.Ok(2)) == e.Ok(2))
    assert(wasm.roundtrip_result(e.Ok(4)) == e.Ok(4))
    assert(wasm.roundtrip_result(e.Err(5)) == e.Err(5))

    assert(wasm.roundtrip_enum(e.E1.A) == e.E1.A)
    assert(wasm.roundtrip_enum(e.E1.B) == e.E1.B)

    assert(wasm.invert_bool(True) == False)
    assert(wasm.invert_bool(False) == True)

    a1, a2, a3, a4, a5, a6 = wasm.variant_casts((
        e.C1A(1),
        e.C2A(2),
        e.C3A(3),
        e.C4A(4),
        e.C5A(5),
        e.C6A(6.),
    ))
    assert(a1 == e.C1A(1))
    assert(a2 == e.C2A(2))
    assert(a3 == e.C3A(3))
    assert(a4 == e.C4A(4))
    assert(a5 == e.C5A(5))
    assert(a6 == e.C6A(6))

    b1, b2, b3, b4, b5, b6 = wasm.variant_casts((
        e.C1B(1),
        e.C2B(2),
        e.C3B(3),
        e.C4B(4),
        e.C5B(5),
        e.C6B(6.),
    ))
    assert(b1 == e.C1B(1))
    assert(b2 == e.C2B(2))
    assert(b3 == e.C3B(3))
    assert(b4 == e.C4B(4))
    assert(b5 == e.C5B(5))
    assert(b6 == e.C6B(6))

    z1, z2, z3, z4 = wasm.variant_zeros((
        e.Z1A(1),
        e.Z2A(2),
        e.Z3A(3.),
        e.Z4A(4.),
    ))
    assert(z1 == e.Z1A(1))
    assert(z2 == e.Z2A(2))
    assert(z3 == e.Z3A(3))
    assert(z4 == e.Z4A(4))

    wasm.variant_typedefs(None, False, e.Err(None))

if __name__ == '__main__':
    run(sys.argv[1])
