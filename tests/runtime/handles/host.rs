wit_bindgen_wasmtime::export!("../../tests/runtime/handles/imports.wit");

use anyhow::Result;
use imports::*;
use std::cell::RefCell;

#[derive(Default)]
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
        HostStateResultVariant::HostState2(())
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

wit_bindgen_wasmtime::import!("../../tests/runtime/handles/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| {
            imports::add_to_linker(
                linker,
                |cx: &mut crate::Context<(MyImports, imports::ImportsTables<MyImports>), _>| {
                    (&mut cx.imports.0, &mut cx.imports.1)
                },
            )
        },
        |store, module, linker| Exports::instantiate(store, module, linker, |cx| &mut cx.exports),
    )?;

    exports.test_imports(&mut store)?;

    let s: WasmState = exports.wasm_state_create(&mut store)?;
    assert_eq!(exports.wasm_state_get_val(&mut store, &s)?, 100);
    exports.drop_wasm_state(&mut store, s)?;

    assert_eq!(exports.wasm_state2_saw_close(&mut store)?, false);
    let s: WasmState2 = exports.wasm_state2_create(&mut store)?;
    assert_eq!(exports.wasm_state2_saw_close(&mut store)?, false);
    exports.drop_wasm_state2(&mut store, s)?;
    assert_eq!(exports.wasm_state2_saw_close(&mut store)?, true);

    let a = exports.wasm_state_create(&mut store)?;
    let b = exports.wasm_state2_create(&mut store)?;
    let (s1, s2) = exports.two_wasm_states(&mut store, &a, &b)?;
    exports.drop_wasm_state(&mut store, a)?;
    exports.drop_wasm_state(&mut store, s1)?;
    exports.drop_wasm_state2(&mut store, b)?;

    exports.wasm_state2_param_record(&mut store, WasmStateParamRecord { a: &s2 })?;
    exports.wasm_state2_param_tuple(&mut store, (&s2,))?;
    exports.wasm_state2_param_option(&mut store, Some(&s2))?;
    exports.wasm_state2_param_option(&mut store, None)?;
    exports.wasm_state2_param_result(&mut store, Ok(&s2))?;
    exports.wasm_state2_param_result(&mut store, Err(2))?;
    exports.wasm_state2_param_variant(&mut store, WasmStateParamVariant::WasmState2(&s2))?;
    exports.wasm_state2_param_variant(&mut store, WasmStateParamVariant::U32(2))?;
    exports.wasm_state2_param_list(&mut store, &[])?;
    exports.wasm_state2_param_list(&mut store, &[&s2])?;
    exports.wasm_state2_param_list(&mut store, &[&s2, &s2])?;
    exports.drop_wasm_state2(&mut store, s2)?;

    let s = exports.wasm_state2_result_record(&mut store)?.a;
    exports.drop_wasm_state2(&mut store, s)?;
    let s = exports.wasm_state2_result_tuple(&mut store)?.0;
    exports.drop_wasm_state2(&mut store, s)?;
    let s = exports.wasm_state2_result_option(&mut store)?.unwrap();
    exports.drop_wasm_state2(&mut store, s)?;
    let s = exports.wasm_state2_result_result(&mut store)?.unwrap();
    match exports.wasm_state2_result_variant(&mut store)? {
        WasmStateResultVariant::WasmState2(s) => exports.drop_wasm_state2(&mut store, s)?,
        WasmStateResultVariant::U32(_) => panic!(),
    }
    exports.drop_wasm_state2(&mut store, s)?;
    for s in exports.wasm_state2_result_list(&mut store)? {
        exports.drop_wasm_state2(&mut store, s)?;
    }
    Ok(())
}
