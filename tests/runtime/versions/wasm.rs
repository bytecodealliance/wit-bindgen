wit_bindgen::generate!({
    path: "../../tests/runtime/versions",
    exports: {
        world: Component,
        "test:dep/test@0.1.0": Component1,
        "test:dep/test@0.2.0": Component2,
    }
});

use exports::test::dep0_1_0::test::Guest as v1;
use exports::test::dep0_2_0::test::Guest as v2;

struct Component;
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

struct Component1;
impl v1 for Component1 {
    fn x() -> f32 {
        1.0
    }

    fn y(a: f32) -> f32 {
        1.0 + a
    }
}

struct Component2;
impl v2 for Component2 {
    fn x() -> f32 {
        2.0
    }

    fn z(a: f32, b: f32) -> f32 {
        2.0 + a + b
    }
}
