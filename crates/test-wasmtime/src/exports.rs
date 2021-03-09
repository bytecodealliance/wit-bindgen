use anyhow::Result;
use wasmtime::Instance;

witx_bindgen_wasmtime::export!("tests/wasm.witx");

pub fn test(wasm: &Wasm) -> Result<()> {
    let bytes = wasm.allocated_bytes()?;
    wasm.run_import_tests()?;
    scalars(wasm)?;
    records(wasm)?;
    variants(wasm)?;
    lists(wasm)?;
    flavorful(wasm)?;
    invalid(wasm)?;

    // Ensure that we properly called `free` everywhere in all the glue that we
    // needed to.
    assert_eq!(bytes, wasm.allocated_bytes()?);
    Ok(())
}

fn scalars(wasm: &Wasm) -> Result<()> {
    assert_eq!(wasm.roundtrip_u8(1)?, 1);
    assert_eq!(wasm.roundtrip_u8(u8::min_value())?, u8::min_value());
    assert_eq!(wasm.roundtrip_u8(u8::max_value())?, u8::max_value());

    assert_eq!(wasm.roundtrip_s8(1)?, 1);
    assert_eq!(wasm.roundtrip_s8(i8::min_value())?, i8::min_value());
    assert_eq!(wasm.roundtrip_s8(i8::max_value())?, i8::max_value());

    assert_eq!(wasm.roundtrip_u16(1)?, 1);
    assert_eq!(wasm.roundtrip_u16(u16::min_value())?, u16::min_value());
    assert_eq!(wasm.roundtrip_u16(u16::max_value())?, u16::max_value());

    assert_eq!(wasm.roundtrip_s16(1)?, 1);
    assert_eq!(wasm.roundtrip_s16(i16::min_value())?, i16::min_value());
    assert_eq!(wasm.roundtrip_s16(i16::max_value())?, i16::max_value());

    assert_eq!(wasm.roundtrip_u32(1)?, 1);
    assert_eq!(wasm.roundtrip_u32(u32::min_value())?, u32::min_value());
    assert_eq!(wasm.roundtrip_u32(u32::max_value())?, u32::max_value());

    assert_eq!(wasm.roundtrip_s32(1)?, 1);
    assert_eq!(wasm.roundtrip_s32(i32::min_value())?, i32::min_value());
    assert_eq!(wasm.roundtrip_s32(i32::max_value())?, i32::max_value());

    assert_eq!(wasm.roundtrip_u64(1)?, 1);
    assert_eq!(wasm.roundtrip_u64(u64::min_value())?, u64::min_value());
    assert_eq!(wasm.roundtrip_u64(u64::max_value())?, u64::max_value());

    assert_eq!(wasm.roundtrip_s64(1)?, 1);
    assert_eq!(wasm.roundtrip_s64(i64::min_value())?, i64::min_value());
    assert_eq!(wasm.roundtrip_s64(i64::max_value())?, i64::max_value());

    assert_eq!(wasm.multiple_results()?, (100, 200));

    assert_eq!(wasm.roundtrip_f32(1.0)?, 1.0);
    assert_eq!(wasm.roundtrip_f32(f32::INFINITY)?, f32::INFINITY);
    assert_eq!(wasm.roundtrip_f32(f32::NEG_INFINITY)?, f32::NEG_INFINITY);
    assert!(wasm.roundtrip_f32(f32::NAN)?.is_nan());

    assert_eq!(wasm.roundtrip_f64(1.0)?, 1.0);
    assert_eq!(wasm.roundtrip_f64(f64::INFINITY)?, f64::INFINITY);
    assert_eq!(wasm.roundtrip_f64(f64::NEG_INFINITY)?, f64::NEG_INFINITY);
    assert!(wasm.roundtrip_f64(f64::NAN)?.is_nan());

    assert_eq!(wasm.roundtrip_char('a')?, 'a');
    assert_eq!(wasm.roundtrip_char(' ')?, ' ');
    assert_eq!(wasm.roundtrip_char('🚩')?, '🚩');

    wasm.set_scalar(2)?;
    assert_eq!(wasm.get_scalar()?, 2);
    wasm.set_scalar(4)?;
    assert_eq!(wasm.get_scalar()?, 4);
    Ok(())
}

fn records(wasm: &Wasm) -> Result<()> {
    assert_eq!(wasm.swap_tuple((1u8, 2u32))?, (2u32, 1u8));
    assert_eq!(wasm.roundtrip_flags1(F1::A)?, F1::A);
    assert_eq!(wasm.roundtrip_flags1(F1::empty())?, F1::empty());
    assert_eq!(wasm.roundtrip_flags1(F1::B)?, F1::B);
    assert_eq!(wasm.roundtrip_flags1(F1::A | F1::B)?, F1::A | F1::B);

    assert_eq!(wasm.roundtrip_flags2(F2::C)?, F2::C);
    assert_eq!(wasm.roundtrip_flags2(F2::empty())?, F2::empty());
    assert_eq!(wasm.roundtrip_flags2(F2::D)?, F2::D);
    assert_eq!(wasm.roundtrip_flags2(F2::C | F2::E)?, F2::C | F2::E);

    let r = wasm.roundtrip_record1(R1 {
        a: 8,
        b: F1::empty(),
    })?;
    assert_eq!(r.a, 8);
    assert_eq!(r.b, F1::empty());

    let r = wasm.roundtrip_record1(R1 {
        a: 0,
        b: F1::A | F1::B,
    })?;
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1::A | F1::B);

    assert_eq!(wasm.tuple0(())?, ());
    assert_eq!(wasm.tuple1((1,))?, (1,));
    Ok(())
}

fn variants(wasm: &Wasm) -> Result<()> {
    assert_eq!(wasm.roundtrip_option(Some(1.0))?, Some(1));
    assert_eq!(wasm.roundtrip_option(None)?, None);
    assert_eq!(wasm.roundtrip_option(Some(2.0))?, Some(2));
    assert_eq!(wasm.roundtrip_result(Ok(2))?, Ok(2.0));
    assert_eq!(wasm.roundtrip_result(Ok(4))?, Ok(4.0));
    assert_eq!(wasm.roundtrip_result(Err(5.3))?, Err(5));

    assert_eq!(wasm.roundtrip_enum(E1::A)?, E1::A);
    assert_eq!(wasm.roundtrip_enum(E1::B)?, E1::B);

    assert_eq!(wasm.invert_bool(true)?, false);
    assert_eq!(wasm.invert_bool(false)?, true);

    let (a1, a2, a3, a4, a5, a6) =
        wasm.variant_casts((C1::A(1), C2::A(2), C3::A(3), C4::A(4), C5::A(5), C6::A(6.0)))?;
    assert!(matches!(a1, C1::A(1)));
    assert!(matches!(a2, C2::A(2)));
    assert!(matches!(a3, C3::A(3)));
    assert!(matches!(a4, C4::A(4)));
    assert!(matches!(a5, C5::A(5)));
    assert!(matches!(a6, C6::A(b) if b == 6.0));

    let (a1, a2, a3, a4, a5, a6) = wasm.variant_casts((
        C1::B(1),
        C2::B(2.0),
        C3::B(3.0),
        C4::B(4.0),
        C5::B(5.0),
        C6::B(6.0),
    ))?;
    assert!(matches!(a1, C1::B(1)));
    assert!(matches!(a2, C2::B(b) if b == 2.0));
    assert!(matches!(a3, C3::B(b) if b == 3.0));
    assert!(matches!(a4, C4::B(b) if b == 4.0));
    assert!(matches!(a5, C5::B(b) if b == 5.0));
    assert!(matches!(a6, C6::B(b) if b == 6.0));

    let (a1, a2, a3, a4) = wasm.variant_zeros((Z1::A(1), Z2::A(2), Z3::A(3.0), Z4::A(4.0)))?;
    assert!(matches!(a1, Z1::A(1)));
    assert!(matches!(a2, Z2::A(2)));
    assert!(matches!(a3, Z3::A(b) if b == 3.0));
    assert!(matches!(a4, Z4::A(b) if b == 4.0));

    wasm.variant_typedefs(None, false, Err(()))?;
    Ok(())
}

fn lists(wasm: &Wasm) -> Result<()> {
    wasm.list_param(&[1, 2, 3, 4])?;
    wasm.list_param2("foo")?;
    wasm.list_param3(&["foo", "bar", "baz"])?;
    wasm.list_param4(&[&["foo", "bar"], &["baz"]])?;
    assert_eq!(wasm.list_result()?, [1, 2, 3, 4, 5]);
    assert_eq!(wasm.list_result2()?, "hello!");
    assert_eq!(wasm.list_result3()?, ["hello,", "world!"]);
    Ok(())
}

fn flavorful(wasm: &Wasm) -> Result<()> {
    wasm.list_in_record1(ListInRecord1 {
        a: "list_in_record1",
    })?;
    assert_eq!(wasm.list_in_record2()?.a, "list_in_record2");

    assert_eq!(
        wasm.list_in_record3(ListInRecord3Param {
            a: "list_in_record3 input"
        })?
        .a,
        "list_in_record3 output"
    );

    assert_eq!(
        wasm.list_in_record4(ListInAliasParam { a: "input4" })?.a,
        "result4"
    );

    wasm.list_in_variant1(Some("foo"), Err("bar"), ListInVariant13::V0("baz"))?;
    assert_eq!(
        wasm.list_in_variant2()?,
        Some("list_in_variant2".to_string())
    );
    assert_eq!(
        wasm.list_in_variant3(Some("input3"))?,
        Some("output3".to_string())
    );

    assert!(wasm.errno_result()?.is_err());
    MyErrno::A.to_string();
    format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    let (a, b) = wasm.list_typedefs("typedef1", &["typedef2"])?;
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");
    Ok(())
}

fn invalid(wasm: &Wasm) -> Result<()> {
    let i = &wasm.instance;
    run_err(i, "invalid_bool", "invalid discriminant for `bool`")?;
    run_err(i, "invalid_u8", "out-of-bounds integer conversion")?;
    run_err(i, "invalid_s8", "out-of-bounds integer conversion")?;
    run_err(i, "invalid_u16", "out-of-bounds integer conversion")?;
    run_err(i, "invalid_s16", "out-of-bounds integer conversion")?;
    run_err(i, "invalid_char", "char value out of valid range")?;
    run_err(i, "invalid_e1", "invalid discriminant for `E1`")?;
    run_err(i, "invalid_handle", "invalid handle index")?;
    run_err(i, "invalid_handle_close", "invalid handle index")?;
    return Ok(());

    fn run_err(i: &Instance, name: &str, err: &str) -> Result<()> {
        match run(i, name) {
            Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
            Err(e) if e.to_string().contains(err) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn run(i: &Instance, name: &str) -> Result<()> {
        let run = i.get_func(name).unwrap();
        let run = run.get0::<()>()?;
        run()?;
        Ok(())
    }
}
