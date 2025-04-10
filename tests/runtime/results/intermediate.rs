include!(env!("BINDINGS"));

struct Exports;

export!(Exports);

use exports::test::results::test as test_exports;
use test::results::test as test_imports;

impl test_exports::Guest for Exports {
    fn string_error(a: f32) -> Result<f32, String> {
        test_imports::string_error(a)
    }

    fn enum_error(a: f32) -> Result<f32, test_exports::E> {
        match test_imports::enum_error(a) {
            Ok(b) => Ok(b),
            Err(test_imports::E::A) => Err(test_exports::E::A),
            Err(test_imports::E::B) => Err(test_exports::E::B),
            Err(test_imports::E::C) => Err(test_exports::E::C),
        }
    }

    fn record_error(a: f32) -> Result<f32, test_exports::E2> {
        match test_imports::record_error(a) {
            Ok(b) => Ok(b),
            Err(test_imports::E2 { line, column }) => Err(test_exports::E2 { line, column }),
        }
    }

    fn variant_error(a: f32) -> Result<f32, test_exports::E3> {
        match test_imports::variant_error(a) {
            Ok(b) => Ok(b),
            Err(test_imports::E3::E1(test_imports::E::A)) => {
                Err(test_exports::E3::E1(test_exports::E::A))
            }
            Err(test_imports::E3::E1(test_imports::E::B)) => {
                Err(test_exports::E3::E1(test_exports::E::B))
            }
            Err(test_imports::E3::E1(test_imports::E::C)) => {
                Err(test_exports::E3::E1(test_exports::E::C))
            }
            Err(test_imports::E3::E2(test_imports::E2 { line, column })) => {
                Err(test_exports::E3::E2(test_exports::E2 { line, column }))
            }
        }
    }

    fn empty_error(a: u32) -> Result<u32, ()> {
        test_imports::empty_error(a)
    }

    fn double_error(a: u32) -> Result<Result<(), String>, String> {
        test_imports::double_error(a)
    }
}
