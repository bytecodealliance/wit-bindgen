wit_bindgen_guest_rust::generate!("../../tests/runtime/results/world.wit");

struct Exports;

export_results!(Exports);

impl results::Results for Exports {
    fn string_error(a: f32) -> Result<f32, String> {
        imports::string_error(a)
    }
    fn enum_error(a: f64) -> Result<f64, results::E> {
        match imports::enum_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E::A) => Err(results::E::A),
            Err(imports::E::B) => Err(results::E::B),
            Err(imports::E::C) => Err(results::E::C),
        }
    }
    fn record_error(a: f64) -> Result<f64, results::E2> {
        match imports::record_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E2 { line, column }) => Err(results::E2 { line, column }),
        }
    }

    fn variant_error(a: f64) -> Result<f64, results::E3> {
        match imports::variant_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E3::E1(imports::E::A)) => Err(results::E3::E1(results::E::A)),
            Err(imports::E3::E1(imports::E::B)) => Err(results::E3::E1(results::E::B)),
            Err(imports::E3::E1(imports::E::C)) => Err(results::E3::E1(results::E::C)),
            Err(imports::E3::E2(imports::E2 { line, column })) => {
                Err(results::E3::E2(results::E2 { line, column }))
            }
        }
    }
    fn empty_error(a: u32) -> Result<u32, ()> {
        imports::empty_error(a)
    }
}
