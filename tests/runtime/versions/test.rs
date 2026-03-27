include!(env!("BINDINGS"));

use exports::test::dep0_1_0::test::Guest as v1;
use exports::test::dep0_2_0::test::Guest as v2;

struct Component;

export!(Component);

impl v1 for Component {
    fn x() -> f32 {
        1.0
    }

    fn y(a: f32) -> f32 {
        1.0 + a
    }
}

impl v2 for Component {
    fn x() -> f32 {
        2.0
    }

    fn z(a: f32, b: f32) -> f32 {
        2.0 + a + b
    }
}
