from typing import Tuple, List
from helpers import TestWasi
from flavorful import Flavorful, FlavorfulImports
import flavorful as e
from flavorful.imports import imports as i
from flavorful.types import Result, Ok, Err
import wasmtime

class MyImports:
    def f_list_in_record1(self, a: i.ListInRecord1) -> None:
        pass

    def f_list_in_record2(self) -> i.ListInRecord2:
        return i.ListInRecord2('list_in_record2')

    def f_list_in_record3(self, a: i.ListInRecord3) -> i.ListInRecord3:
        assert(a.a == 'list_in_record3 input')
        return i.ListInRecord3('list_in_record3 output')

    def f_list_in_record4(self, a: i.ListInAlias) -> i.ListInAlias:
        assert(a.a == 'input4')
        return i.ListInRecord4('result4')

    def f_list_in_variant1(self, a: i.ListInVariant1V1, b: i.ListInVariant1V2, c: i.ListInVariant1V3) -> None:
        assert(a == 'foo')
        assert(b == Err('bar'))
        assert(c == 'baz')

    def f_list_in_variant2(self) -> i.ListInVariant2:
        return 'list_in_variant2'

    def f_list_in_variant3(self, a: i.ListInVariant3) -> i.ListInVariant3:
        assert(a == 'input3')
        return 'output3'

    def errno_result(self) -> Result[None, i.MyErrno]:
        return Err(i.MyErrno.B)

    def list_typedefs(self, a: i.ListTypedef, c: i.ListTypedef3) -> Tuple[i.ListTypedef2, i.ListTypedef3]:
        assert(a == 'typedef1')
        assert(c == ['typedef2'])
        return (b'typedef3', ['typedef4'])

    def list_of_variants(self, a: List[bool], b: List[Result[None, None]], c: List[i.MyErrno]) -> Tuple[List[bool], List[Result[None, None]], List[i.MyErrno]]:
          assert(a == [True, False])
          assert(b == [Ok(None), Err(None)])
          assert(c == [i.MyErrno.SUCCESS, i.MyErrno.A])
          return (
                [False, True],
                [Err(None), Ok(None)],
                [i.MyErrno.A, i.MyErrno.B],
          )

def run() -> None:
    store = wasmtime.Store()
    wasm = Flavorful(store, FlavorfulImports(MyImports(), TestWasi()))

    wasm.test_imports(store)
    wasm.f_list_in_record1(store, e.ListInRecord1("list_in_record1"))
    assert(wasm.f_list_in_record2(store) == e.ListInRecord2(a="list_in_record2"))

    assert(wasm.f_list_in_record3(store, e.ListInRecord3("list_in_record3 input")).a == "list_in_record3 output")
    assert(wasm.f_list_in_record4(store, e.ListInRecord4("input4")).a == "result4")

    wasm.f_list_in_variant1(store, "foo", e.Err("bar"), 'baz')
    assert(wasm.f_list_in_variant2(store) == "list_in_variant2")
    assert(wasm.f_list_in_variant3(store, "input3") == "output3")

    assert(isinstance(wasm.errno_result(store), e.Err))

    r1, r2 = wasm.list_typedefs(store, "typedef1", ["typedef2"])
    assert(r1 == b'typedef3')
    assert(r2 == ['typedef4'])

if __name__ == '__main__':
    run()
