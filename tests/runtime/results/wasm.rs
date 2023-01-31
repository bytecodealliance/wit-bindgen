wit_bindgen_guest_rust::generate!("world" in "../../tests/runtime/results");

struct Exports;

export_results!(Exports);

impl exports::Exports for Exports {
    fn string_error(a: f32) -> Result<f32, String> {
        imports::string_error(a)
    }
    fn enum_error(a: f64) -> Result<f64, exports::E> {
        match imports::enum_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E::A) => Err(exports::E::A),
            Err(imports::E::B) => Err(exports::E::B),
            Err(imports::E::C) => Err(exports::E::C),
        }
    }
    fn record_error(a: f64) -> Result<f64, exports::E2> {
        match imports::record_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E2 { line, column }) => Err(exports::E2 { line, column }),
        }
    }

    fn variant_error(a: f64) -> Result<f64, exports::E3> {
        match imports::variant_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E3::E1(imports::E::A)) => Err(exports::E3::E1(exports::E::A)),
            Err(imports::E3::E1(imports::E::B)) => Err(exports::E3::E1(exports::E::B)),
            Err(imports::E3::E1(imports::E::C)) => Err(exports::E3::E1(exports::E::C)),
            Err(imports::E3::E2(imports::E2 { line, column })) => {
                Err(exports::E3::E2(exports::E2 { line, column }))
            }
        }
    }
    fn empty_error(a: u32) -> Result<u32, ()> {
        imports::empty_error(a)
    }
}
