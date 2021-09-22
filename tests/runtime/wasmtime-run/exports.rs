use anyhow::Result;
use wasmtime::{Instance, Store};

witx_bindgen_wasmtime::export!("tests/wasm.witx");

use crate::Context;
use wasm::*;

pub(crate) use wasm::{Wasm, WasmData};

pub fn test(wasm: &Wasm<Context>, instance: Instance, store: &mut Store<Context>) -> Result<()> {
    let bytes = wasm.allocated_bytes(&mut *store)?;
    wasm.run_import_tests(&mut *store)?;
    scalars(wasm, store)?;
    records(wasm, store)?;
    variants(wasm, store)?;
    lists(wasm, store)?;
    flavorful(wasm, store)?;
    invalid(&instance, store)?;
    // buffers(wasm)?;
    handles(wasm, store)?;

    // Ensure that we properly called `free` everywhere in all the glue that we
    // needed to.
    assert_eq!(bytes, wasm.allocated_bytes(&mut *store)?);
    Ok(())
}

fn flavorful(wasm: &Wasm<Context>, store: &mut Store<Context>) -> Result<()> {
    wasm.list_in_record1(
        &mut *store,
        ListInRecord1 {
            a: "list_in_record1",
        },
    )?;
    assert_eq!(wasm.list_in_record2(&mut *store)?.a, "list_in_record2");

    assert_eq!(
        wasm.list_in_record3(
            &mut *store,
            ListInRecord3Param {
                a: "list_in_record3 input"
            }
        )?
        .a,
        "list_in_record3 output"
    );

    assert_eq!(
        wasm.list_in_record4(&mut *store, ListInAliasParam { a: "input4" })?
            .a,
        "result4"
    );

    wasm.list_in_variant1(
        &mut *store,
        Some("foo"),
        Err("bar"),
        ListInVariant13::V0("baz"),
    )?;
    assert_eq!(
        wasm.list_in_variant2(&mut *store)?,
        Some("list_in_variant2".to_string())
    );
    assert_eq!(
        wasm.list_in_variant3(&mut *store, Some("input3"))?,
        Some("output3".to_string())
    );

    assert!(wasm.errno_result(&mut *store)?.is_err());
    MyErrno::A.to_string();
    format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    let (a, b) = wasm.list_typedefs(&mut *store, "typedef1", &["typedef2"])?;
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");
    Ok(())
}

// fn buffers(wasm: &Wasm<Context>) -> Result<()> {
//     let mut out = [0; 10];
//     let n = wasm.buffer_u8(&[0u8], &mut out)? as usize;
//     assert_eq!(n, 3);
//     assert_eq!(&out[..n], [1, 2, 3]);
//     assert!(out[n..].iter().all(|x| *x == 0));

//     let mut out = [0; 10];
//     let n = wasm.buffer_u32(&[0], &mut out)? as usize;
//     assert_eq!(n, 3);
//     assert_eq!(&out[..n], [1, 2, 3]);
//     assert!(out[n..].iter().all(|x| *x == 0));

//     assert_eq!(wasm.buffer_bool(&mut iter::empty(), &mut Vec::new())?, 0);
//     assert_eq!(wasm.buffer_string(&mut iter::empty(), &mut Vec::new())?, 0);
//     assert_eq!(
//         wasm.buffer_list_bool(&mut iter::empty(), &mut Vec::new())?,
//         0
//     );

//     let mut bools = [true, false, true].iter().copied();
//     let mut out = Vec::with_capacity(4);
//     let n = wasm.buffer_bool(&mut bools, &mut out)?;
//     assert_eq!(n, 3);
//     assert_eq!(out, [false, true, false]);

//     let mut strings = ["foo", "bar", "baz"].iter().copied();
//     let mut out = Vec::with_capacity(3);
//     let n = wasm.buffer_string(&mut strings, &mut out)?;
//     assert_eq!(n, 3);
//     assert_eq!(out, ["FOO", "BAR", "BAZ"]);

//     let a = &[true, false, true][..];
//     let b = &[false, false][..];
//     let list = [a, b];
//     let mut lists = list.iter().copied();
//     let mut out = Vec::with_capacity(4);
//     let n = wasm.buffer_list_bool(&mut lists, &mut out)?;
//     assert_eq!(n, 2);
//     assert_eq!(out, [vec![false, true, false], vec![true, true]]);

//     let a = [true, false, true, true, false];
//     // let mut bools = a.iter().copied();
//     // let mut list = [&mut bools as &mut dyn ExactSizeIterator<Item = _>];
//     // let mut buffers = list.iter_mut().map(|b| &mut **b);
//     // wasm.buffer_buffer_bool(&mut buffers)?;

//     let mut bools = a.iter().copied();
//     wasm.buffer_mutable1(&mut [&mut bools])?;

//     let mut dst = [0; 10];
//     let n = wasm.buffer_mutable2(&mut [&mut dst])? as usize;
//     assert_eq!(n, 4);
//     assert_eq!(&dst[..n], [1, 2, 3, 4]);

//     let mut out = Vec::with_capacity(10);
//     let n = wasm.buffer_mutable3(&mut [&mut out])?;
//     assert_eq!(n, 3);
//     assert_eq!(out, [false, true, false]);

//     Ok(())
// }

fn invalid(i: &Instance, store: &mut Store<Context>) -> Result<()> {
    run_err(i, store, "invalid_bool", "invalid discriminant for `bool`")?;
    run_err(i, store, "invalid_u8", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_s8", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_u16", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_s16", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_char", "char value out of valid range")?;
    run_err(i, store, "invalid_e1", "invalid discriminant for `E1`")?;
    run_err(i, store, "invalid_handle", "invalid handle index")?;
    run_err(i, store, "invalid_handle_close", "invalid handle index")?;
    return Ok(());

    fn run_err(i: &Instance, store: &mut Store<Context>, name: &str, err: &str) -> Result<()> {
        match run(i, store, name) {
            Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
            Err(e) if e.to_string().contains(err) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn run(i: &Instance, store: &mut Store<Context>, name: &str) -> Result<()> {
        let run = i.get_typed_func::<(), (), _>(&mut *store, name)?;
        run.call(store, ())?;
        Ok(())
    }
}
