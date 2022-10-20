use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/lists/imports.wit",
    default: "../../tests/runtime/lists/exports.wit",
    name: "exports",
});

use imports::*;

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
    fn empty_list_param(&mut self, a: Vec<u8>) -> Result<()> {
        assert_eq!(a, []);
        Ok(())
    }

    fn empty_string_param(&mut self, a: String) -> Result<()> {
        assert_eq!(a, "");
        Ok(())
    }

    fn empty_list_result(&mut self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn empty_string_result(&mut self) -> Result<String> {
        Ok(String::new())
    }

    fn list_param(&mut self, list: Vec<u8>) -> Result<()> {
        assert_eq!(list, [1, 2, 3, 4]);
        Ok(())
    }

    fn list_param2(&mut self, ptr: String) -> Result<()> {
        assert_eq!(ptr, "foo");
        Ok(())
    }

    fn list_param3(&mut self, ptr: Vec<String>) -> Result<()> {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
        Ok(())
    }

    fn list_param4(&mut self, ptr: Vec<Vec<String>>) -> Result<()> {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
        Ok(())
    }

    fn list_result(&mut self) -> Result<Vec<u8>> {
        Ok(vec![1, 2, 3, 4, 5])
    }

    fn list_result2(&mut self) -> Result<String> {
        Ok("hello!".to_string())
    }

    fn list_result3(&mut self) -> Result<Vec<String>> {
        Ok(vec!["hello,".to_string(), "world!".to_string()])
    }

    fn list_roundtrip(&mut self, list: Vec<u8>) -> Result<Vec<u8>> {
        Ok(list.to_vec())
    }

    fn string_roundtrip(&mut self, s: String) -> Result<String> {
        Ok(s.to_string())
    }

    fn list_minmax8(&mut self, u: Vec<u8>, s: Vec<i8>) -> Result<(Vec<u8>, Vec<i8>)> {
        assert_eq!(u, [u8::MIN, u8::MAX]);
        assert_eq!(s, [i8::MIN, i8::MAX]);
        Ok((u, s))
    }

    fn list_minmax16(&mut self, u: Vec<u16>, s: Vec<i16>) -> Result<(Vec<u16>, Vec<i16>)> {
        assert_eq!(u, [u16::MIN, u16::MAX]);
        assert_eq!(s, [i16::MIN, i16::MAX]);
        Ok((u, s))
    }

    fn list_minmax32(&mut self, u: Vec<u32>, s: Vec<i32>) -> Result<(Vec<u32>, Vec<i32>)> {
        assert_eq!(u, [u32::MIN, u32::MAX]);
        assert_eq!(s, [i32::MIN, i32::MAX]);
        Ok((u, s))
    }

    fn list_minmax64(&mut self, u: Vec<u64>, s: Vec<i64>) -> Result<(Vec<u64>, Vec<i64>)> {
        assert_eq!(u, [u64::MIN, u64::MAX]);
        assert_eq!(s, [i64::MIN, i64::MAX]);
        Ok((u, s))
    }

    fn list_minmax_float(&mut self, u: Vec<f32>, s: Vec<f64>) -> Result<(Vec<f32>, Vec<f64>)> {
        assert_eq!(u, [f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY]);
        assert_eq!(s, [f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]);
        Ok((u, s))
    }
}

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| Exports::instantiate(store, module, linker),
    )?;

    let bytes = exports.allocated_bytes(&mut store)?;
    exports.test_imports(&mut store)?;
    exports.empty_list_param(&mut store, &[])?;
    exports.empty_string_param(&mut store, "")?;
    assert_eq!(exports.empty_list_result(&mut store)?, []);
    assert_eq!(exports.empty_string_result(&mut store)?, "");
    exports.list_param(&mut store, &[1, 2, 3, 4])?;
    exports.list_param2(&mut store, "foo")?;
    exports.list_param3(&mut store, &["foo", "bar", "baz"])?;
    exports.list_param4(&mut store, &[&["foo", "bar"], &["baz"]])?;
    assert_eq!(exports.list_result(&mut store)?, [1, 2, 3, 4, 5]);
    assert_eq!(exports.list_result2(&mut store)?, "hello!");
    assert_eq!(exports.list_result3(&mut store)?, ["hello,", "world!"]);
    assert_eq!(exports.string_roundtrip(&mut store, "x")?, "x");
    assert_eq!(exports.string_roundtrip(&mut store, "")?, "");
    assert_eq!(
        exports.string_roundtrip(&mut store, "hello ⚑ world")?,
        "hello ⚑ world"
    );
    // Ensure that we properly called `free` everywhere in all the glue that we
    // needed to.
    assert_eq!(bytes, exports.allocated_bytes(&mut store)?);
    Ok(())
}
