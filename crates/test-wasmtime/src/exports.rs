use anyhow::Result;
use wasmtime::Instance;

witx_bindgen_wasmtime::export!("tests/wasm.witx");

pub fn test(wasm: &Wasm) -> Result<()> {
    wasm.run_import_tests()?;
    scalars(wasm)?;

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
    Ok(())
}

fn run(i: &Instance, name: &str) -> Result<()> {
    let run = i.get_func(name).unwrap();
    let run = run.get0::<()>()?;
    run()?;
    Ok(())
}

fn run_err(i: &Instance, name: &str, err: &str) -> Result<()> {
    match run(i, name) {
        Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
        Err(e) if e.to_string().contains(err) => Ok(()),
        Err(e) => Err(e),
    }
}
