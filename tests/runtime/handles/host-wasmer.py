from dataclasses import dataclass
from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Tuple, List, Any
import exports.bindings as e
import imports.bindings as i
import sys
import wasmer # type: ignore

@dataclass
class HostState(i.HostState):
    val: int

    def __init__(self, val: int) -> None:
        self.val = val


HOST_STATE2_CLOSED = False


@dataclass
class HostState2(i.HostState2):
    val: int

    def __init__(self, val: int) -> None:
        self.val = val

    def drop(self) -> None:
        global HOST_STATE2_CLOSED
        HOST_STATE2_CLOSED = True


@dataclass
class Markdown(i.Markdown2):
    buf: str = ''

    def append(self, data: str) -> None:
        self.buf += data

    def render(self) -> str:
        return self.buf.replace('red', 'green')


class OddName(i.OddName):
    def frob_the_odd(self) -> None:
        pass


class MyImports:
    def host_state_create(self) -> i.HostState:
        return HostState(100)

    def host_state_get(self, a: i.HostState) -> int:
        assert(isinstance(a, HostState))
        return a.val

    def host_state2_create(self) -> i.HostState2:
        return HostState2(101)

    def host_state2_saw_close(self) -> bool:
        return HOST_STATE2_CLOSED

    def two_host_states(self, a: i.HostState, b: i.HostState2) -> Tuple[i.HostState, i.HostState2]:
        return (b, a)

    def host_state2_param_record(self, a: i.HostStateParamRecord) -> None:
        pass

    def host_state2_param_tuple(self, a: i.HostStateParamTuple) -> None:
        pass

    def host_state2_param_option(self, a: i.HostStateParamOption) -> None:
        pass

    def host_state2_param_result(self, a: i.HostStateParamResult) -> None:
        pass

    def host_state2_param_variant(self, a: i.HostStateParamVariant) -> None:
        pass

    def host_state2_param_list(self, a: List[i.HostState2]) -> None:
        pass

    def host_state2_result_record(self) -> i.HostStateResultRecord:
        return i.HostStateResultRecord(HostState(2))

    def host_state2_result_tuple(self) -> i.HostStateResultTuple:
        return (HostState(2),)

    def host_state2_result_option(self) -> i.HostStateResultOption:
        return HostState(2)

    def host_state2_result_result(self) -> i.HostStateResultResult:
        return i.Ok(HostState2(2))

    def host_state2_result_variant(self) -> i.HostStateResultVariant:
        return i.HostStateResultVariant0(HostState2(2))

    def host_state2_result_list(self) -> List[i.HostState2]:
        return [HostState2(2), HostState2(5)]

    def markdown2_create(self) -> i.Markdown2:
        return Markdown()

    def odd_name_create(self) -> i.OddName:
        return OddName()

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

    # Param/result of a handle works in a simple fashion
    s: e.WasmState = wasm.wasm_state_create()
    assert(wasm.wasm_state_get_val(s) == 100)

    # Deterministic destruction is possible
    assert(wasm.wasm_state2_saw_close() == False)
    s2: e.WasmState2 = wasm.wasm_state2_create()
    assert(wasm.wasm_state2_saw_close() == False)
    s2.drop()
    assert(wasm.wasm_state2_saw_close() == True)

    arg1 = wasm.wasm_state_create()
    arg2 = wasm.wasm_state2_create()
    c, d = wasm.two_wasm_states(arg1, arg2)
    arg1.drop()
    arg2.drop()

    wasm.wasm_state2_param_record(e.WasmStateParamRecord(d))
    wasm.wasm_state2_param_tuple((d,))
    wasm.wasm_state2_param_option(d)
    wasm.wasm_state2_param_option(None)
    wasm.wasm_state2_param_result(e.Ok(d))
    wasm.wasm_state2_param_result(e.Err(2))
    wasm.wasm_state2_param_variant(e.WasmStateParamVariant0(d))
    wasm.wasm_state2_param_variant(e.WasmStateParamVariant1(2))
    wasm.wasm_state2_param_list([])
    wasm.wasm_state2_param_list([d])
    wasm.wasm_state2_param_list([d, d])

    c.drop()
    d.drop()

    wasm.wasm_state2_result_record().a.drop()
    wasm.wasm_state2_result_tuple()[0].drop()
    opt = wasm.wasm_state2_result_option()
    assert(opt is not None)
    opt.drop()
    result = wasm.wasm_state2_result_result()
    assert(isinstance(result, e.Ok))
    result.value.drop()
    variant = wasm.wasm_state2_result_variant()
    print(variant)
    assert(isinstance(variant, e.WasmStateResultVariant0))
    variant.value.drop()
    for val in wasm.wasm_state2_result_list():
        val.drop()

    s.drop()

    md = e.Markdown.create(wasm)
    if md:
        md.append("red is the best color")
        assert(md.render() == "green is the best color")
        md.drop()

if __name__ == '__main__':
    run(sys.argv[1])
