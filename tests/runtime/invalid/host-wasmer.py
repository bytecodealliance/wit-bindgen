from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Callable, Any
import imports.bindings as i
import sys
import wasmer # type: ignore

class MyImports(Imports):
    def roundtrip_u8(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_s8(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_u16(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_s16(self, x: int) -> int:
        raise Exception('unreachable')

    def roundtrip_bool(self, x: bool) -> bool:
        raise Exception('unreachable')

    def roundtrip_char(self, x: str) -> str:
        raise Exception('unreachable')

    def roundtrip_enum(self, x: i.E) -> i.E:
        raise Exception('unreachable')

    def get_internal(self, x: i.HostState) -> int:
        raise Exception('unreachable')

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

    def assert_throws(f: Callable, msg: str) -> None:
        try:
            f()
            raise RuntimeError('expected exception')
        except TypeError as e:
            actual = str(e)
        except OverflowError as e:
            actual = str(e)
        except ValueError as e:
            actual = str(e)
        except IndexError as e:
            actual = str(e)
        if not msg in actual:
            print(actual)
            assert(msg in actual)

    assert_throws(lambda: wasm.invalid_bool(), 'invalid variant discriminant for bool')
    assert_throws(lambda: wasm.invalid_u8(), 'must be between')
    assert_throws(lambda: wasm.invalid_s8(), 'must be between')
    assert_throws(lambda: wasm.invalid_u16(), 'must be between')
    assert_throws(lambda: wasm.invalid_s16(), 'must be between')
    assert_throws(lambda: wasm.invalid_char(), 'not a valid char')
    assert_throws(lambda: wasm.invalid_enum(), 'not a valid E')
    assert_throws(lambda: wasm.invalid_handle(), 'handle index not valid')
    assert_throws(lambda: wasm.invalid_handle_close(), 'handle index not valid')

if __name__ == '__main__':
    run(sys.argv[1])
