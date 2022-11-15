wit_bindgen_guest_rust::generate!("../../tests/runtime/results/world.wit");

struct Exports;

export_the_world!(Exports);

impl the_world::TheWorld for Exports {
    fn string_error(a: f32) -> Result<f32, String> {
        imports::string_error(a)
    }
    fn enum_error(a: f64) -> Result<f64, the_world::E> {
        match imports::enum_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E::A) => Err(the_world::E::A),
            Err(imports::E::B) => Err(the_world::E::B),
            Err(imports::E::C) => Err(the_world::E::C),
        }
    }
    fn record_error(a: f64) -> Result<f64, the_world::E2> {
        match imports::record_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E2 { line, column }) => Err(the_world::E2 { line, column }),
        }
    }

    fn variant_error(a: f64) -> Result<f64, the_world::E3> {
        match imports::variant_error(a) {
            Ok(b) => Ok(b),
            Err(imports::E3::E1(imports::E::A)) => Err(the_world::E3::E1(the_world::E::A)),
            Err(imports::E3::E1(imports::E::B)) => Err(the_world::E3::E1(the_world::E::B)),
            Err(imports::E3::E1(imports::E::C)) => Err(the_world::E3::E1(the_world::E::C)),
            Err(imports::E3::E2(imports::E2 { line, column })) => {
                Err(the_world::E3::E2(the_world::E2 { line, column }))
            }
        }
    }
    fn empty_error(a: u32) -> Result<u32, ()> {
        imports::empty_error(a)
    }
}
