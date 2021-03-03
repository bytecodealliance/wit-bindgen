use std::cell::Cell;
use wasmtime::*;

const WASM: &[u8] = include_bytes!(env!("WASM"));

witx_bindgen_wasmtime::import!("tests/host.witx");

#[derive(Default)]
struct MyHost {
    scalar: Cell<u32>,
}

impl Host for MyHost {
    fn roundtrip_u8(&self, val: u8) -> u8 {
        val
    }

    fn roundtrip_s8(&self, val: i8) -> i8 {
        val
    }

    fn roundtrip_u16(&self, val: u16) -> u16 {
        val
    }

    fn roundtrip_s16(&self, val: i16) -> i16 {
        val
    }

    fn roundtrip_u32(&self, val: u32) -> u32 {
        val
    }

    fn roundtrip_s32(&self, val: i32) -> i32 {
        val
    }

    fn roundtrip_u64(&self, val: u64) -> u64 {
        val
    }

    fn roundtrip_s64(&self, val: i64) -> i64 {
        val
    }

    fn roundtrip_usize(&self, val: u32) -> u32 {
        val
    }

    fn roundtrip_f32(&self, val: f32) -> f32 {
        val
    }

    fn roundtrip_f64(&self, val: f64) -> f64 {
        val
    }

    fn roundtrip_char(&self, val: char) -> char {
        val
    }

    fn multiple_results(&self) -> (u8, u16) {
        (4, 5)
    }

    fn set_scalar(&self, val: u32) {
        self.scalar.set(val);
    }

    fn get_scalar(&self) -> u32 {
        self.scalar.get()
    }

    fn swap_tuple(&self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&self, a: F1) -> F1 {
        drop(a.to_string());
        drop(format!("{:?}", a));
        drop(a & F1::all());
        a
    }

    fn roundtrip_flags2(&self, a: F2) -> F2 {
        a
    }

    fn roundtrip_record1(&self, a: R1) -> R1 {
        drop(format!("{:?}", a));
        a
    }

    fn tuple0(&self, _: ()) {}

    fn tuple1(&self, a: (u8,)) -> (u8,) {
        (a.0,)
    }

    fn roundtrip_option(&self, a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(&self, a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(&self, a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(&self, a: bool) -> bool {
        !a
    }

    fn variant_casts(&self, a: Casts) -> Casts {
        a
    }

    fn variant_zeros(&self, a: Zeros) -> Zeros {
        a
    }

    fn legacy_params(
        &self,
        a: (u32, u32),
        _: R1,
        _: (u8, i8, u16, i16, u32, i32, u64, i64, f32, f64),
    ) {
        assert_eq!(a, (1, 2));
    }

    fn legacy_result(&self, succeed: bool) -> Result<LegacyResult, E1> {
        if succeed {
            Ok((
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9.,
                10.,
                R1 {
                    a: 0,
                    b: F1::empty(),
                },
            ))
        } else {
            Err(E1::B)
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.cache_config_load_default()?;
    let engine = Engine::new(&config);
    let module = Module::new(&engine, WASM)?;
    let store = Store::new(&engine);
    let mut linker = Linker::new(&store);

    add_host_to_linker(MyHost::default(), &mut linker)?;

    wasmtime_wasi::Wasi::new(
        &store,
        wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .build()?,
    )
    .add_to_linker(&mut linker)?;

    let instance = linker.instantiate(&module)?;
    run(&instance, "run_host_tests")?;
    run_err(&instance, "invalid_bool", "invalid discriminant for `bool`")?;
    run_err(&instance, "invalid_u8", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_s8", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_u16", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_s16", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_char", "char value out of valid range")?;
    run_err(&instance, "invalid_e1", "invalid discriminant for `E1`")?;
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
