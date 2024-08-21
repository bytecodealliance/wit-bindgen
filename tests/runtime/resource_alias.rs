use wasmtime::Store;

use exports::test::resource_alias::e1::Foo as Foo1;
use exports::test::resource_alias::e2::Foo as Foo2;

wasmtime::component::bindgen!(in "tests/runtime/resource_alias");

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_alias",
        |_| Ok(()),
        |store, component, linker| ResourceAlias::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(instance: ResourceAlias, store: &mut Store<crate::Wasi<()>>) -> anyhow::Result<()> {
    let foo_e1 = Foo1 {
        x: instance
            .test_resource_alias_e1()
            .x()
            .call_constructor(&mut *store, 42)?,
    };
    let _ = instance
        .test_resource_alias_e1()
        .call_a(&mut *store, foo_e1)?;

    // TODO: how do I test deep equal of ResourceAny type?
    // assert_eq!(
    //     res,
    //     vec![instance
    //         .test_resource_alias_e1()
    //         .x()
    //         .call_constructor(&mut *store, 42)?]
    // );

    let foo_e2 = Foo2 {
        x: instance
            .test_resource_alias_e1()
            .x()
            .call_constructor(&mut *store, 7)?,
    };
    let bar_e2 = Foo1 {
        x: instance
            .test_resource_alias_e1()
            .x()
            .call_constructor(&mut *store, 8)?,
    };
    let y = instance
        .test_resource_alias_e1()
        .x()
        .call_constructor(&mut *store, 8)?;
    let _ = instance
        .test_resource_alias_e2()
        .call_a(&mut *store, foo_e2, bar_e2, y)?;

    // TODO: how do I test deep equal of ResourceAny type?
    // assert_eq!(
    //     res,
    //     vec![
    //         instance
    //             .test_resource_alias_e1()
    //             .x()
    //             .call_constructor(&mut *store, 7)?,
    //         instance
    //             .test_resource_alias_e1()
    //             .x()
    //             .call_constructor(&mut *store, 8)?
    //     ]
    // );
    Ok(())
}
