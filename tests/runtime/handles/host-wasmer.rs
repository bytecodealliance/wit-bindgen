wit_bindgen_wasmer::export!("./tests/runtime/handles/imports.wit");

use anyhow::Result;
use imports::*;
use std::cell::RefCell;
use wasmer::WasmerEnv;

#[derive(Default, WasmerEnv, Clone)]
pub struct MyImports {
    host_state2_closed: bool,
}

#[derive(Debug)]
pub struct SuchState(u32);

#[derive(Default, Debug)]
pub struct Markdown {
    buf: RefCell<String>,
}

impl Imports for MyImports {
    type HostState = SuchState;
    type HostState2 = ();
    type Markdown2 = Markdown;
    type OddName = ();

    fn host_state_create(&mut self) -> SuchState {
        SuchState(100)
    }

    fn host_state_get(&mut self, state: &SuchState) -> u32 {
        state.0
    }

    fn host_state2_create(&mut self) {}

    fn host_state2_saw_close(&mut self) -> bool {
        self.host_state2_closed
    }

    fn drop_host_state2(&mut self, _state: ()) {
        self.host_state2_closed = true;
    }

    fn two_host_states(&mut self, _a: &SuchState, _b: &()) -> (SuchState, ()) {
        (SuchState(2), ())
    }

    fn host_state2_param_record(&mut self, _a: HostStateParamRecord<'_, Self>) {}
    fn host_state2_param_tuple(&mut self, _a: (&'_ (),)) {}
    fn host_state2_param_option(&mut self, _a: Option<&'_ ()>) {}
    fn host_state2_param_result(&mut self, _a: Result<&'_ (), u32>) {}
    fn host_state2_param_variant(&mut self, _a: HostStateParamVariant<'_, Self>) {}
    fn host_state2_param_list(&mut self, _a: Vec<&()>) {}

    fn host_state2_result_record(&mut self) -> HostStateResultRecord<Self> {
        HostStateResultRecord { a: () }
    }
    fn host_state2_result_tuple(&mut self) -> ((),) {
        ((),)
    }
    fn host_state2_result_option(&mut self) -> Option<()> {
        Some(())
    }
    fn host_state2_result_result(&mut self) -> Result<(), u32> {
        Ok(())
    }
    fn host_state2_result_variant(&mut self) -> HostStateResultVariant<Self> {
        HostStateResultVariant::V0(())
    }
    fn host_state2_result_list(&mut self) -> Vec<()> {
        vec![(), ()]
    }

    fn markdown2_create(&mut self) -> Markdown {
        Markdown::default()
    }

    fn markdown2_append(&mut self, md: &Markdown, buf: &str) {
        md.buf.borrow_mut().push_str(buf);
    }

    fn markdown2_render(&mut self, md: &Markdown) -> String {
        md.buf.borrow().replace("red", "green")
    }

    fn odd_name_create(&mut self) {}
    fn odd_name_frob_the_odd(&mut self, _: &()) {}
}

wit_bindgen_wasmer::import!("./tests/runtime/handles/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let exports = crate::instantiate(
        wasm,
        |store, import_object| imports::add_to_imports(store, import_object, MyImports::default()),
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    exports.test_imports()?;

    let s: WasmState = exports.wasm_state_create()?;
    assert_eq!(exports.wasm_state_get_val(&s)?, 100);
    exports.drop_wasm_state(s)?;

    assert_eq!(exports.wasm_state2_saw_close()?, false);
    let s: WasmState2 = exports.wasm_state2_create()?;
    assert_eq!(exports.wasm_state2_saw_close()?, false);
    exports.drop_wasm_state2(s)?;
    assert_eq!(exports.wasm_state2_saw_close()?, true);

    let a = exports.wasm_state_create()?;
    let b = exports.wasm_state2_create()?;
    let (s1, s2) = exports.two_wasm_states(&a, &b)?;
    exports.drop_wasm_state(a)?;
    exports.drop_wasm_state(s1)?;
    exports.drop_wasm_state2(b)?;

    exports.wasm_state2_param_record(WasmStateParamRecord { a: &s2 })?;
    exports.wasm_state2_param_tuple((&s2,))?;
    exports.wasm_state2_param_option(Some(&s2))?;
    exports.wasm_state2_param_option(None)?;
    exports.wasm_state2_param_result(Ok(&s2))?;
    exports.wasm_state2_param_result(Err(2))?;
    exports.wasm_state2_param_variant(WasmStateParamVariant::V0(&s2))?;
    exports.wasm_state2_param_variant(WasmStateParamVariant::V1(2))?;
    exports.wasm_state2_param_list(&[])?;
    exports.wasm_state2_param_list(&[&s2])?;
    exports.wasm_state2_param_list(&[&s2, &s2])?;
    exports.drop_wasm_state2(s2)?;

    let s = exports.wasm_state2_result_record()?.a;
    exports.drop_wasm_state2(s)?;
    let s = exports.wasm_state2_result_tuple()?.0;
    exports.drop_wasm_state2(s)?;
    let s = exports.wasm_state2_result_option()?.unwrap();
    exports.drop_wasm_state2(s)?;
    let s = exports.wasm_state2_result_result()?.unwrap();
    match exports.wasm_state2_result_variant()? {
        WasmStateResultVariant::V0(s) => exports.drop_wasm_state2(s)?,
        WasmStateResultVariant::V1(_) => panic!(),
    }
    exports.drop_wasm_state2(s)?;
    for s in exports.wasm_state2_result_list()? {
        exports.drop_wasm_state2(s)?;
    }
    Ok(())
}
