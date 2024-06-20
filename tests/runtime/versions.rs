use anyhow::{Ok, Result};
use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/versions");
use crate::versions::test::dep0_1_0::test::Host as v1;
use crate::versions::test::dep0_2_0::test::Host as v2;

#[derive(Default)]
pub struct MyFoo;

impl v1 for MyFoo {
    fn x(&mut self) -> f32 {
        1.0
    }

    fn y(&mut self, a: f32) -> f32 {
        1.0 + a
    }
}

impl v2 for MyFoo {
    fn x(&mut self) -> f32 {
        2.0
    }

    fn z(&mut self, a: f32, b: f32) -> f32 {
        2.0 + a + b
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "versions",
        |linker| Foo::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Foo::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: Foo, store: &mut Store<crate::Wasi<MyFoo>>) -> Result<()> {
    // test version 1
    assert_eq!(exports.test_dep0_1_0_test().call_x(&mut *store)?, 1.0);
    assert_eq!(exports.test_dep0_1_0_test().call_y(&mut *store, 1.0)?, 2.0);

    // test version 2
    assert_eq!(exports.test_dep0_2_0_test().call_x(&mut *store)?, 2.0);
    assert_eq!(
        exports.test_dep0_2_0_test().call_z(&mut *store, 1.0, 1.0)?,
        4.0
    );

    // test imports
    exports.call_test_imports(&mut *store)?;
    Ok(())
}
