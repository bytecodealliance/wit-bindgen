include!(env!("BINDINGS"));

use exports::test::results::test as imports;

pub struct Component;

export!(Component);

impl exports::test::results::test::Guest for Component {
    fn string_error(a: f32) -> Result<f32, String> {
        if a == 0.0 {
            Err("zero".to_owned())
        } else {
            Ok(a)
        }
    }

    fn enum_error(a: f32) -> Result<f32, imports::E> {
        if a == 0.0 {
            Err(imports::E::A)
        } else {
            Ok(a)
        }
    }

    fn record_error(a: f32) -> Result<f32, imports::E2> {
        if a == 0.0 {
            Err(imports::E2 {
                line: 420,
                column: 0,
            })
        } else if a == 1.0 {
            Err(imports::E2 {
                line: 77,
                column: 2,
            })
        } else {
            Ok(a)
        }
    }

    fn variant_error(a: f32) -> Result<f32, imports::E3> {
        if a == 0.0 {
            Err(imports::E3::E2(imports::E2 {
                line: 420,
                column: 0,
            }))
        } else if a == 1.0 {
            Err(imports::E3::E1(imports::E::B))
        } else if a == 2.0 {
            Err(imports::E3::E1(imports::E::C))
        } else {
            Ok(a)
        }
    }

    fn empty_error(a: u32) -> Result<u32, ()> {
        if a == 0 {
            Err(())
        } else if a == 1 {
            Ok(42)
        } else {
            Ok(a)
        }
    }

    fn double_error(a: u32) -> Result<Result<(), String>, String> {
        if a == 0 {
            Ok(Ok(()))
        } else if a == 1 {
            Ok(Err("one".into()))
        } else {
            Err("two".into())
        }
    }
}
