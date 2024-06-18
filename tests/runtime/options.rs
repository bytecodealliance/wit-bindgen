use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/options");

#[derive(Default)]
pub struct MyImports;

impl test::options::test::Host for MyImports {
    fn option_none_param(&mut self, a: Option<String>) -> Result<()> {
        assert!(a.is_none());
        Ok(())
    }

    fn option_none_result(&mut self) -> Result<Option<String>> {
        Ok(None)
    }

    fn option_some_param(&mut self, a: Option<String>) -> Result<()> {
        assert_eq!(a, Some("foo".to_string()));
        Ok(())
    }

    fn option_some_result(&mut self) -> Result<Option<String>> {
        Ok(Some("foo".to_string()))
    }

    fn option_roundtrip(&mut self, a: Option<String>) -> Result<Option<String>> {
        Ok(a)
    }

    fn double_option_roundtrip(&mut self, a: Option<Option<u32>>) -> Result<Option<Option<u32>>> {
        Ok(a)
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "options",
        |linker| Options::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Options::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: Options, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_test_imports(&mut *store)?;
    let exports = exports.test_options_test();
    assert!(exports.call_option_none_result(&mut *store)?.is_none());
    assert_eq!(
        exports.call_option_some_result(&mut *store)?,
        Some("foo".to_string())
    );
    exports.call_option_none_param(&mut *store, None)?;
    exports.call_option_some_param(&mut *store, Some("foo"))?;
    assert_eq!(
        exports.call_option_roundtrip(&mut *store, Some("foo"))?,
        Some("foo".to_string())
    );
    assert_eq!(
        exports.call_double_option_roundtrip(&mut *store, Some(Some(42)))?,
        Some(Some(42))
    );
    assert_eq!(
        exports.call_double_option_roundtrip(&mut *store, Some(None))?,
        Some(None)
    );
    assert_eq!(
        exports.call_double_option_roundtrip(&mut *store, None)?,
        None
    );
    Ok(())
}
