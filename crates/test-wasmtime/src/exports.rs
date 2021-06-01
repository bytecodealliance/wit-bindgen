use anyhow::Result;
use std::iter;
use wasmtime::{Instance, Store};

witx_bindgen_wasmtime::export!("tests/wasm.witx");

use crate::Context;
use wasm::*;

pub(crate) use wasm::Wasm;

pub fn test(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
    let bytes = wasm.allocated_bytes(&mut *store)?;
    wasm.run_import_tests(&mut *store)?;
    scalars(wasm, store)?;
    records(wasm, store)?;
    variants(wasm, store)?;
    lists(wasm, store)?;
    flavorful(wasm, store)?;
    // invalid(wasm)?;
    // buffers(wasm)?;

    // Ensure that we properly called `free` everywhere in all the glue that we
    // needed to.
    assert_eq!(bytes, wasm.allocated_bytes(&mut *store)?);

    // // It's known that we have a handle type that leaks here, so it's expected
    // // we leak a few bytes.
    // handles(wasm)?;
    Ok(())
}

fn scalars(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
    assert_eq!(wasm.roundtrip_u8(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_u8(&mut *store, u8::min_value())?,
        u8::min_value()
    );
    assert_eq!(
        wasm.roundtrip_u8(&mut *store, u8::max_value())?,
        u8::max_value()
    );

    assert_eq!(wasm.roundtrip_s8(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_s8(&mut *store, i8::min_value())?,
        i8::min_value()
    );
    assert_eq!(
        wasm.roundtrip_s8(&mut *store, i8::max_value())?,
        i8::max_value()
    );

    assert_eq!(wasm.roundtrip_u16(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_u16(&mut *store, u16::min_value())?,
        u16::min_value()
    );
    assert_eq!(
        wasm.roundtrip_u16(&mut *store, u16::max_value())?,
        u16::max_value()
    );

    assert_eq!(wasm.roundtrip_s16(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_s16(&mut *store, i16::min_value())?,
        i16::min_value()
    );
    assert_eq!(
        wasm.roundtrip_s16(&mut *store, i16::max_value())?,
        i16::max_value()
    );

    assert_eq!(wasm.roundtrip_u32(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_u32(&mut *store, u32::min_value())?,
        u32::min_value()
    );
    assert_eq!(
        wasm.roundtrip_u32(&mut *store, u32::max_value())?,
        u32::max_value()
    );

    assert_eq!(wasm.roundtrip_s32(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_s32(&mut *store, i32::min_value())?,
        i32::min_value()
    );
    assert_eq!(
        wasm.roundtrip_s32(&mut *store, i32::max_value())?,
        i32::max_value()
    );

    assert_eq!(wasm.roundtrip_u64(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_u64(&mut *store, u64::min_value())?,
        u64::min_value()
    );
    assert_eq!(
        wasm.roundtrip_u64(&mut *store, u64::max_value())?,
        u64::max_value()
    );

    assert_eq!(wasm.roundtrip_s64(&mut *store, 1)?, 1);
    assert_eq!(
        wasm.roundtrip_s64(&mut *store, i64::min_value())?,
        i64::min_value()
    );
    assert_eq!(
        wasm.roundtrip_s64(&mut *store, i64::max_value())?,
        i64::max_value()
    );

    assert_eq!(wasm.multiple_results(&mut *store,)?, (100, 200));

    assert_eq!(wasm.roundtrip_f32(&mut *store, 1.0)?, 1.0);
    assert_eq!(
        wasm.roundtrip_f32(&mut *store, f32::INFINITY)?,
        f32::INFINITY
    );
    assert_eq!(
        wasm.roundtrip_f32(&mut *store, f32::NEG_INFINITY)?,
        f32::NEG_INFINITY
    );
    assert!(wasm.roundtrip_f32(&mut *store, f32::NAN)?.is_nan());

    assert_eq!(wasm.roundtrip_f64(&mut *store, 1.0)?, 1.0);
    assert_eq!(
        wasm.roundtrip_f64(&mut *store, f64::INFINITY)?,
        f64::INFINITY
    );
    assert_eq!(
        wasm.roundtrip_f64(&mut *store, f64::NEG_INFINITY)?,
        f64::NEG_INFINITY
    );
    assert!(wasm.roundtrip_f64(&mut *store, f64::NAN)?.is_nan());

    assert_eq!(wasm.roundtrip_char(&mut *store, 'a')?, 'a');
    assert_eq!(wasm.roundtrip_char(&mut *store, ' ')?, ' ');
    assert_eq!(wasm.roundtrip_char(&mut *store, '🚩')?, '🚩');

    wasm.set_scalar(&mut *store, 2)?;
    assert_eq!(wasm.get_scalar(&mut *store)?, 2);
    wasm.set_scalar(&mut *store, 4)?;
    assert_eq!(wasm.get_scalar(&mut *store)?, 4);
    Ok(())
}

fn records(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
    assert_eq!(wasm.swap_tuple(&mut *store, (1u8, 2u32))?, (2u32, 1u8));
    assert_eq!(wasm.roundtrip_flags1(&mut *store, F1::A)?, F1::A);
    assert_eq!(
        wasm.roundtrip_flags1(&mut *store, F1::empty())?,
        F1::empty()
    );
    assert_eq!(wasm.roundtrip_flags1(&mut *store, F1::B)?, F1::B);
    assert_eq!(
        wasm.roundtrip_flags1(&mut *store, F1::A | F1::B)?,
        F1::A | F1::B
    );

    assert_eq!(wasm.roundtrip_flags2(&mut *store, F2::C)?, F2::C);
    assert_eq!(
        wasm.roundtrip_flags2(&mut *store, F2::empty())?,
        F2::empty()
    );
    assert_eq!(wasm.roundtrip_flags2(&mut *store, F2::D)?, F2::D);
    assert_eq!(
        wasm.roundtrip_flags2(&mut *store, F2::C | F2::E)?,
        F2::C | F2::E
    );

    let r = wasm.roundtrip_record1(
        &mut *store,
        R1 {
            a: 8,
            b: F1::empty(),
        },
    )?;
    assert_eq!(r.a, 8);
    assert_eq!(r.b, F1::empty());

    let r = wasm.roundtrip_record1(
        &mut *store,
        R1 {
            a: 0,
            b: F1::A | F1::B,
        },
    )?;
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1::A | F1::B);

    assert_eq!(wasm.tuple0(&mut *store, ())?, ());
    assert_eq!(wasm.tuple1(&mut *store, (1,))?, (1,));
    Ok(())
}

fn variants(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
    assert_eq!(wasm.roundtrip_option(&mut *store, Some(1.0))?, Some(1));
    assert_eq!(wasm.roundtrip_option(&mut *store, None)?, None);
    assert_eq!(wasm.roundtrip_option(&mut *store, Some(2.0))?, Some(2));
    assert_eq!(wasm.roundtrip_result(&mut *store, Ok(2))?, Ok(2.0));
    assert_eq!(wasm.roundtrip_result(&mut *store, Ok(4))?, Ok(4.0));
    assert_eq!(wasm.roundtrip_result(&mut *store, Err(5.3))?, Err(5));

    assert_eq!(wasm.roundtrip_enum(&mut *store, E1::A)?, E1::A);
    assert_eq!(wasm.roundtrip_enum(&mut *store, E1::B)?, E1::B);

    assert_eq!(wasm.invert_bool(&mut *store, true)?, false);
    assert_eq!(wasm.invert_bool(&mut *store, false)?, true);

    let (a1, a2, a3, a4, a5, a6) = wasm.variant_casts(
        &mut *store,
        (C1::A(1), C2::A(2), C3::A(3), C4::A(4), C5::A(5), C6::A(6.0)),
    )?;
    assert!(matches!(a1, C1::A(1)));
    assert!(matches!(a2, C2::A(2)));
    assert!(matches!(a3, C3::A(3)));
    assert!(matches!(a4, C4::A(4)));
    assert!(matches!(a5, C5::A(5)));
    assert!(matches!(a6, C6::A(b) if b == 6.0));

    let (a1, a2, a3, a4, a5, a6) = wasm.variant_casts(
        &mut *store,
        (
            C1::B(1),
            C2::B(2.0),
            C3::B(3.0),
            C4::B(4.0),
            C5::B(5.0),
            C6::B(6.0),
        ),
    )?;
    assert!(matches!(a1, C1::B(1)));
    assert!(matches!(a2, C2::B(b) if b == 2.0));
    assert!(matches!(a3, C3::B(b) if b == 3.0));
    assert!(matches!(a4, C4::B(b) if b == 4.0));
    assert!(matches!(a5, C5::B(b) if b == 5.0));
    assert!(matches!(a6, C6::B(b) if b == 6.0));

    let (a1, a2, a3, a4) =
        wasm.variant_zeros(&mut *store, (Z1::A(1), Z2::A(2), Z3::A(3.0), Z4::A(4.0)))?;
    assert!(matches!(a1, Z1::A(1)));
    assert!(matches!(a2, Z2::A(2)));
    assert!(matches!(a3, Z3::A(b) if b == 3.0));
    assert!(matches!(a4, Z4::A(b) if b == 4.0));

    wasm.variant_typedefs(&mut *store, None, false, Err(()))?;
    Ok(())
}

fn lists(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
    wasm.list_param(&mut *store, &[1, 2, 3, 4])?;
    wasm.list_param2(&mut *store, "foo")?;
    wasm.list_param3(&mut *store, &["foo", "bar", "baz"])?;
    wasm.list_param4(&mut *store, &[&["foo", "bar"], &["baz"]])?;
    assert_eq!(wasm.list_result(&mut *store)?, [1, 2, 3, 4, 5]);
    assert_eq!(wasm.list_result2(&mut *store)?, "hello!");
    assert_eq!(wasm.list_result3(&mut *store)?, ["hello,", "world!"]);
    Ok(())
}

fn flavorful(wasm: &Wasm, store: &mut Store<Context>) -> Result<()> {
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

// fn handles(wasm: &Wasm) -> Result<()> {
//     let bytes = wasm.allocated_bytes()?;
//     {
//         let s: WasmState = wasm.wasm_state_create()?;
//         assert_eq!(wasm.wasm_state_get(&s)?, 100);
//         assert_eq!(wasm.wasm_state2_saw_close()?, false);
//         let s: WasmState2 = wasm.wasm_state2_create()?;
//         assert_eq!(wasm.wasm_state2_saw_close()?, false);
//         drop(s);
//         assert_eq!(wasm.wasm_state2_saw_close()?, true);

//         let (_a, s2) =
//             wasm.two_wasm_states(&wasm.wasm_state_create()?, &wasm.wasm_state2_create()?)?;

//         wasm.wasm_state2_param_record(WasmStateParamRecord { a: &s2 })?;
//         wasm.wasm_state2_param_tuple((&s2,))?;
//         wasm.wasm_state2_param_option(Some(&s2))?;
//         wasm.wasm_state2_param_option(None)?;
//         wasm.wasm_state2_param_result(Ok(&s2))?;
//         wasm.wasm_state2_param_result(Err(2))?;
//         wasm.wasm_state2_param_variant(WasmStateParamVariant::V0(&s2))?;
//         wasm.wasm_state2_param_variant(WasmStateParamVariant::V1(2))?;
//         wasm.wasm_state2_param_list(&[])?;
//         wasm.wasm_state2_param_list(&[&s2])?;
//         wasm.wasm_state2_param_list(&[&s2, &s2])?;

//         drop(wasm.wasm_state2_result_record()?.a);
//         drop(wasm.wasm_state2_result_tuple()?.0);
//         drop(wasm.wasm_state2_result_option()?.unwrap());
//         drop(wasm.wasm_state2_result_result()?.unwrap());
//         drop(wasm.wasm_state2_result_variant()?);
//         drop(wasm.wasm_state2_result_list()?);
//     }
//     assert_eq!(bytes + 12, wasm.allocated_bytes()?);
//     Ok(())
// }

// fn buffers(wasm: &Wasm) -> Result<()> {
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

// fn invalid(wasm: &Wasm) -> Result<()> {
//     let i = &wasm.instance;
//     run_err(i, "invalid_bool", "invalid discriminant for `bool`")?;
//     run_err(i, "invalid_u8", "out-of-bounds integer conversion")?;
//     run_err(i, "invalid_s8", "out-of-bounds integer conversion")?;
//     run_err(i, "invalid_u16", "out-of-bounds integer conversion")?;
//     run_err(i, "invalid_s16", "out-of-bounds integer conversion")?;
//     run_err(i, "invalid_char", "char value out of valid range")?;
//     run_err(i, "invalid_e1", "invalid discriminant for `E1`")?;
//     run_err(i, "invalid_handle", "invalid handle index")?;
//     run_err(i, "invalid_handle_close", "invalid handle index")?;
//     return Ok(());

//     fn run_err(i: &Instance, name: &str, err: &str) -> Result<()> {
//         match run(i, name) {
//             Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
//             Err(e) if e.to_string().contains(err) => Ok(()),
//             Err(e) => Err(e),
//         }
//     }

//     fn run(i: &Instance, name: &str) -> Result<()> {
//         let run = i.get_typed_func::<(), ()>(name)?;
//         run.call(())?;
//         Ok(())
//     }
// }
