from exports.bindings import Exports
from imports.bindings import add_imports_to_imports, Imports
from typing import Tuple, List, Any
import exports.bindings as e
import imports.bindings as i
import sys
import wasmer # type: ignore

class MyImports:
    def list_in_record1(self, a: i.ListInRecord1) -> None:
        pass

    def list_in_record2(self) -> i.ListInRecord2:
        return i.ListInRecord2('list_in_record2')

    def list_in_record3(self, a: i.ListInRecord3) -> i.ListInRecord3:
        assert(a.a == 'list_in_record3 input')
        return i.ListInRecord3('list_in_record3 output')

    def list_in_record4(self, a: i.ListInAlias) -> i.ListInAlias:
        assert(a.a == 'input4')
        return i.ListInRecord4('result4')

    def list_in_variant1(self, a: i.ListInVariant11, b: i.ListInVariant12, c: i.ListInVariant13) -> None:
        assert(a == 'foo')
        assert(b == i.Err('bar'))
        assert(c == i.ListInVariant130('baz'))

    def list_in_variant2(self) -> i.ListInVariant2:
        return 'list_in_variant2'

    def list_in_variant3(self, a: i.ListInVariant3) -> i.ListInVariant3:
        assert(a == 'input3')
        return 'output3'

    def errno_result(self) -> i.Expected[None, i.MyErrno]:
        return i.Err(i.MyErrno.B)

    def list_typedefs(self, a: i.ListTypedef, c: i.ListTypedef3) -> Tuple[i.ListTypedef2, i.ListTypedef3]:
        assert(a == 'typedef1')
        assert(c == ['typedef2'])
        return (b'typedef3', ['typedef4'])

    def list_of_variants(self, a: List[bool], b: List[i.Expected[None, None]], c: List[i.MyErrno]) -> Tuple[List[bool], List[i.Expected[None, None]], List[i.MyErrno]]:
          assert(a == [True, False])
          assert(b == [i.Ok(None), i.Err(None)])
          assert(c == [i.MyErrno.SUCCESS, i.MyErrno.A])
          return (
                [False, True],
                [i.Err(None), i.Ok(None)],
                [i.MyErrno.A, i.MyErrno.B],
          )

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
    wasm.list_in_record1(e.ListInRecord1("list_in_record1"))
    assert(wasm.list_in_record2() == e.ListInRecord2(a="list_in_record2"))

    assert(wasm.list_in_record3(e.ListInRecord3("list_in_record3 input")).a == "list_in_record3 output")
    assert(wasm.list_in_record4(e.ListInRecord4("input4")).a == "result4")

    wasm.list_in_variant1("foo", e.Err("bar"), e.ListInVariant130('baz'))
    assert(wasm.list_in_variant2() == "list_in_variant2")
    assert(wasm.list_in_variant3("input3") == "output3")

    assert(isinstance(wasm.errno_result(), e.Err))

    r1, r2 = wasm.list_typedefs("typedef1", ["typedef2"])
    assert(r1 == b'typedef3')
    assert(r2 == ['typedef4'])

if __name__ == '__main__':
    run(sys.argv[1])
