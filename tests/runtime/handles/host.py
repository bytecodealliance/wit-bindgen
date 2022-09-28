from dataclasses import dataclass
from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
from typing import Tuple, List
import exports.bindings as e
import imports.bindings as i
import sys
import wasmtime

@dataclass
class HostState(i.HostState):
    val: int

    def __init__(self, val: int) -> None:
        self.val = val

    def drop(self) -> None:
        pass


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

    def drop(self) -> None:
        pass


class OddName(i.OddName):
    def frob_the_odd(self) -> None:
        pass

    def drop(self) -> None:
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
        return HostState2(2)

    def host_state2_result_list(self) -> List[i.HostState2]:
        return [HostState2(2), HostState2(5)]

    def markdown2_create(self) -> i.Markdown2:
        return Markdown()

    def odd_name_create(self) -> i.OddName:
        return OddName()

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

    # Param/result of a handle works in a simple fashion
    s: e.WasmState = wasm.wasm_state_create(store)
    assert(wasm.wasm_state_get_val(store, s) == 100)

    # Deterministic destruction is possible
    assert(wasm.wasm_state2_saw_close(store) == False)
    s2: e.WasmState2 = wasm.wasm_state2_create(store)
    assert(wasm.wasm_state2_saw_close(store) == False)
    s2.drop(store)
    assert(wasm.wasm_state2_saw_close(store) == True)

    arg1 = wasm.wasm_state_create(store)
    arg2 = wasm.wasm_state2_create(store)
    c, d = wasm.two_wasm_states(store, arg1, arg2)
    arg1.drop(store)
    arg2.drop(store)

    wasm.wasm_state2_param_record(store, e.WasmStateParamRecord(d))
    wasm.wasm_state2_param_tuple(store, (d,))
    wasm.wasm_state2_param_option(store, d)
    wasm.wasm_state2_param_option(store, None)
    wasm.wasm_state2_param_result(store, e.Ok(d))
    wasm.wasm_state2_param_result(store, e.Err(2))
    wasm.wasm_state2_param_variant(store, d)
    wasm.wasm_state2_param_variant(store, 2)
    wasm.wasm_state2_param_list(store, [])
    wasm.wasm_state2_param_list(store, [d])
    wasm.wasm_state2_param_list(store, [d, d])

    c.drop(store)
    d.drop(store)

    wasm.wasm_state2_result_record(store).a.drop(store)
    wasm.wasm_state2_result_tuple(store)[0].drop(store)
    opt = wasm.wasm_state2_result_option(store)
    assert(opt is not None)
    opt.drop(store)
    result = wasm.wasm_state2_result_result(store)
    assert(isinstance(result, e.Ok))
    result.value.drop(store)
    variant = wasm.wasm_state2_result_variant(store)
    print(variant)
    assert(isinstance(variant, e.WasmState2))
    variant.drop(store)
    for val in wasm.wasm_state2_result_list(store):
        val.drop(store)

    s.drop(store)

    md = e.Markdown.create(store, wasm)
    if md:
        md.append(store, "red is the best color")
        assert(md.render(store) == "green is the best color")
        md.drop(store)

if __name__ == '__main__':
    run(sys.argv[1])
