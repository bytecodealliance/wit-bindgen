use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!({
    inline: r#"package test:deps;

    world test {
        export other:test/test;
    }"#,
    path: "tests/runtime/other-dependencies/other"
});

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "other-dependencies",
        |_linker| Ok(()),
        |store, component, linker| Test::instantiate(store, component, linker),
        |_exports, _store: &mut Store<crate::Wasi<()>>| Ok(()),
    )
}
