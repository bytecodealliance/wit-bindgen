wit_bindgen::generate!({
    path: "../../tests/runtime/versions",
    with: {
        "test:dep/test@0.1.0": generate,
        "test:dep/test@0.2.0": generate,
    }
});

use exports::test::dep0_1_0::test::Guest as v1;
use exports::test::dep0_2_0::test::Guest as v2;

struct Component;

export!(Component);

impl Guest for Component {
    fn test_imports() {
        use test::dep0_1_0::test as v1;
        assert_eq!(v1::x(), 1.0);
        assert_eq!(v1::y(1.0), 2.0);

        use test::dep0_2_0::test as v2;
        assert_eq!(v2::x(), 2.0);
        assert_eq!(v2::z(1.0, 1.0), 4.0);
    }
}

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
