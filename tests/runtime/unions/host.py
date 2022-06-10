from exports.bindings import Exports
from imports.bindings import add_imports_to_linker, Imports
from typing import Union
import exports.bindings as e
import imports.bindings as i
import sys
import wasmtime

class MyImports:
    # Simple uses of unions whose inner values all have the same Python representation
    def add_one_integer(self, num: i.AllIntegers) -> i.AllIntegers:
        # Bool
        if isinstance(num, i.AllIntegers0):
            assert num.value in (True, False)
            return i.AllIntegers0(not num.value)
        # The unsigned numbers
        elif isinstance(num, i.AllIntegers1):
            lower_limit = 0
            upper_limit = 2**8
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers1(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers2):
            lower_limit = 0
            upper_limit = 2**16
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers2(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers3):
            lower_limit = 0
            upper_limit = 2**32
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers3(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers4):
            lower_limit = 0
            upper_limit = 2**64
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers4(num.value + 1 % upper_limit)
        # The signed numbers
        elif isinstance(num, i.AllIntegers5):
            lower_limit = -2**7
            upper_limit = 2**7
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers5(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers6):
            lower_limit = -2**15
            upper_limit = 2**15
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers6(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers7):
            lower_limit = -2**31
            upper_limit = 2**31
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers7(num.value + 1 % upper_limit)
        elif isinstance(num, i.AllIntegers8):
            lower_limit = -2**63
            upper_limit = 2**63
            assert lower_limit <= num.value < upper_limit
            return i.AllIntegers8(num.value + 1 % upper_limit)
        else:
            raise ValueError("Invalid input value!")

    def add_one_float(self, num: i.AllFloats) -> i.AllFloats:
        if isinstance(num, i.AllFloats0):
            return i.AllFloats0(num.value + 1)
        if isinstance(num, i.AllFloats1):
            return i.AllFloats1(num.value + 1)
        else:
            raise ValueError("Invalid input value!")

    def replace_first_char(self, text: i.AllText, letter: str) -> i.AllText:
        if isinstance(text, i.AllText0):
            return i.AllText0(letter)
        if isinstance(text, i.AllFloats1):
            return i.AllText1(letter + text.value[1:])
        else:
            raise ValueError("Invalid input value!")

    # Identify each case of unions whose inner values all have the same Python representation
    def identify_integer(self, num: i.AllIntegers) -> int:
        # Bool
        if isinstance(num, i.AllIntegers0):
            return 0
        # The unsigned numbers
        elif isinstance(num, i.AllIntegers1):
            return 1
        elif isinstance(num, i.AllIntegers2):
            return 2
        elif isinstance(num, i.AllIntegers3):
            return 3
        elif isinstance(num, i.AllIntegers4):
            return 4
        # The signed numbers
        elif isinstance(num, i.AllIntegers5):
            return 5
        elif isinstance(num, i.AllIntegers6):
            return 6
        elif isinstance(num, i.AllIntegers7):
            return 7
        elif isinstance(num, i.AllIntegers8):
            return 8
        else:
            raise ValueError("Invalid input value!")

    def identify_float(self, num: i.AllFloats) -> int:
        if isinstance(num, i.AllFloats0):
            return 0
        if isinstance(num, i.AllFloats1):
            return 1
        else:
            raise ValueError("Invalid input value!")

    def identify_text(self, text: i.AllText) -> int:
        if isinstance(text, i.AllText0):
            return 0
        if isinstance(text, i.AllFloats1):
            return 1
        else:
            raise ValueError("Invalid input value!")

    # A simple use of a union which contains multiple entries of the same type
    def add_one_duplicated(self, num: i.DuplicatedS32) ->  i.DuplicatedS32:
        if isinstance(num, i.DuplicatedS320):
            return i.DuplicatedS320(num.value + 1)
        if isinstance(num, i.DuplicatedS321):
            return i.DuplicatedS321(num.value + 1)
        if isinstance(num, i.DuplicatedS322):
            return i.DuplicatedS322(num.value + 1)
        else:
            raise ValueError("Invalid input value!")

    # Identify each case of unions which contains multiple entries of the same type
    def identify_duplicated(self, num: i.DuplicatedS32) -> int:
        if isinstance(num, i.DuplicatedS320):
            return 0
        if isinstance(num, i.DuplicatedS321):
            return 1
        if isinstance(num, i.DuplicatedS322):
            return 2
        else:
            raise ValueError("Invalid input value!")

    # A simple use of a union whose cases have distinct Python representations
    def add_one_distinguishable_num(self, num: Union[float, int]) -> Union[float, int]:
        return num + 1

    # Identify each case of unions whose cases have distinct Python representations
    def identify_distinguishable_num(self, num: i.DistinguishableNum) -> int:
        if isinstance(num, float):
            return 0
        elif isinstance(num, int):
            return 1
        else:
            raise ValueError("Invalid input value!")

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

    # wasm.test_imports(store)

    # All-Integers
    # Booleans
    assert wasm.add_one_integer(store, e.AllIntegers0(False)) == e.AllIntegers0(True)
    assert wasm.add_one_integer(store, e.AllIntegers0(True)) == e.AllIntegers0(False)
    # Unsigned integers
    assert wasm.add_one_integer(store, e.AllIntegers1(0)) == e.AllIntegers1(1)
    assert wasm.add_one_integer(store, e.AllIntegers1(2**8-1)) == e.AllIntegers1(0)
    assert wasm.add_one_integer(store, e.AllIntegers2(0)) == e.AllIntegers2(1)
    assert wasm.add_one_integer(store, e.AllIntegers2(2**16-1)) == e.AllIntegers2(0)
    assert wasm.add_one_integer(store, e.AllIntegers3(0)) == e.AllIntegers3(1)
    assert wasm.add_one_integer(store, e.AllIntegers3(2**32-1)) == e.AllIntegers3(0)
    assert wasm.add_one_integer(store, e.AllIntegers4(0)) == e.AllIntegers4(1)
    assert wasm.add_one_integer(store, e.AllIntegers4(2**64-1)) == e.AllIntegers4(0)
    # Signed integers
    assert wasm.add_one_integer(store, e.AllIntegers5(0)) == e.AllIntegers5(1)
    assert wasm.add_one_integer(store, e.AllIntegers5(2**7-1)) == e.AllIntegers5(-2**7)
    assert wasm.add_one_integer(store, e.AllIntegers6(0)) == e.AllIntegers6(1)
    assert wasm.add_one_integer(store, e.AllIntegers6(2**15-1)) == e.AllIntegers6(-2**15)
    assert wasm.add_one_integer(store, e.AllIntegers7(0)) == e.AllIntegers7(1)
    assert wasm.add_one_integer(store, e.AllIntegers7(2**31-1)) == e.AllIntegers7(-2**31)
    assert wasm.add_one_integer(store, e.AllIntegers8(0)) == e.AllIntegers8(1)
    assert wasm.add_one_integer(store, e.AllIntegers8(2**63-1)) == e.AllIntegers8(-2**63)

    # All-Floats
    assert wasm.add_one_float(store, e.AllFloats0(0.0)) == e.AllFloats0(1.0)
    assert wasm.add_one_float(store, e.AllFloats1(0.0)) == e.AllFloats1(1.0)

    # All-Text
    assert wasm.replace_first_char(store, e.AllText0('a'), 'z') == e.AllText0('z')
    assert wasm.replace_first_char(store, e.AllText1('abc'), 'z') == e.AllText1('zbc')

    # All-Integers
    assert wasm.identify_integer(store, e.AllIntegers0(True)) == 0
    assert wasm.identify_integer(store, e.AllIntegers1(0)) == 1
    assert wasm.identify_integer(store, e.AllIntegers2(0)) == 2
    assert wasm.identify_integer(store, e.AllIntegers3(0)) == 3
    assert wasm.identify_integer(store, e.AllIntegers4(0)) == 4
    assert wasm.identify_integer(store, e.AllIntegers5(0)) == 5
    assert wasm.identify_integer(store, e.AllIntegers6(0)) == 6
    assert wasm.identify_integer(store, e.AllIntegers7(0)) == 7
    assert wasm.identify_integer(store, e.AllIntegers8(0)) == 8

    # All-Floats
    assert wasm.identify_float(store, e.AllFloats0(0.0)) == 0
    assert wasm.identify_float(store, e.AllFloats1(0.0)) == 1

    # All-Text
    assert wasm.identify_text(store, e.AllText0('a')) == 0
    assert wasm.identify_text(store, e.AllText1('abc')) == 1

    # Duplicated
    assert wasm.add_one_duplicated(store, e.DuplicatedS320(0)) == e.DuplicatedS320(1)
    assert wasm.add_one_duplicated(store, e.DuplicatedS321(1)) == e.DuplicatedS321(2)
    assert wasm.add_one_duplicated(store, e.DuplicatedS322(2)) == e.DuplicatedS322(3)

    assert wasm.identify_duplicated(store, e.DuplicatedS320(0)) == 0
    assert wasm.identify_duplicated(store, e.DuplicatedS321(0)) == 1
    assert wasm.identify_duplicated(store, e.DuplicatedS322(0)) == 2

    # Distinguishable
    assert wasm.add_one_distinguishable_num(store, 0.0) == 1.0
    assert wasm.add_one_distinguishable_num(store, 0) == 1

    assert wasm.identify_distinguishable_num(store, 0.0) == 0
    assert wasm.identify_distinguishable_num(store, 1) == 1

if __name__ == '__main__':
    run(sys.argv[1])
