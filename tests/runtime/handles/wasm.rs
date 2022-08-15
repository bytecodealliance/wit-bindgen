wit_bindgen_guest_rust::import!("../../tests/runtime/handles/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/handles/exports.wit");

use exports::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use wit_bindgen_guest_rust::Handle;

struct Exports;

static CLOSED: AtomicU32 = AtomicU32::new(0);

pub struct WasmState(u32);

pub struct WasmState2(u32);

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;

        let s: HostState = host_state_create();
        assert_eq!(host_state_get(&s), 100);
        assert_eq!(host_state2_saw_close(), false);
        let s: HostState2 = host_state2_create();
        assert_eq!(host_state2_saw_close(), false);
        drop(s);
        assert_eq!(host_state2_saw_close(), true);

        let (_a, s2) = two_host_states(&host_state_create(), &host_state2_create());

        host_state2_param_record(HostStateParamRecord { a: &s2 });
        host_state2_param_tuple((&s2,));
        host_state2_param_option(Some(&s2));
        host_state2_param_option(None);
        host_state2_param_result(Ok(&s2));
        host_state2_param_result(Err(2));
        host_state2_param_variant(HostStateParamVariant::HostState2(&s2));
        host_state2_param_variant(HostStateParamVariant::U32(2));
        host_state2_param_list(&[]);
        host_state2_param_list(&[&s2]);
        host_state2_param_list(&[&s2, &s2]);

        drop(host_state2_result_record().a);
        drop(host_state2_result_tuple().0);
        drop(host_state2_result_option().unwrap());
        drop(host_state2_result_result().unwrap());
        drop(host_state2_result_variant());
        drop(host_state2_result_list());

        let md = Markdown2::create();
        md.append("red is the best color");
        assert_eq!(md.render(), "green is the best color");

        let odd = OddName::create();
        odd.frob_the_odd();
    }

    fn wasm_state_create() -> Handle<WasmState> {
        WasmState(100).into()
    }

    fn wasm_state_get_val(state: Handle<WasmState>) -> u32 {
        state.0
    }

    fn wasm_state2_create() -> Handle<WasmState2> {
        WasmState2(33).into()
    }

    fn wasm_state2_saw_close() -> bool {
        CLOSED.load(SeqCst) != 0
    }

    fn drop_wasm_state2(_state: WasmState2) {
        CLOSED.store(1, SeqCst);
    }

    fn two_wasm_states(
        _a: Handle<WasmState>,
        _b: Handle<WasmState2>,
    ) -> (Handle<WasmState>, Handle<WasmState2>) {
        (WasmState(101).into(), WasmState2(102).into())
    }

    fn wasm_state2_param_record(_a: WasmStateParamRecord) {}
    fn wasm_state2_param_tuple(_a: (Handle<WasmState2>,)) {}
    fn wasm_state2_param_option(_a: Option<Handle<WasmState2>>) {}
    fn wasm_state2_param_result(_a: Result<Handle<WasmState2>, u32>) {}
    fn wasm_state2_param_variant(_a: WasmStateParamVariant) {}
    fn wasm_state2_param_list(_a: Vec<Handle<WasmState2>>) {}

    fn wasm_state2_result_record() -> WasmStateResultRecord {
        WasmStateResultRecord {
            a: WasmState2(222).into(),
        }
    }
    fn wasm_state2_result_tuple() -> (Handle<WasmState2>,) {
        (WasmState2(333).into(),)
    }
    fn wasm_state2_result_option() -> Option<Handle<WasmState2>> {
        Some(WasmState2(444).into())
    }
    fn wasm_state2_result_result() -> Result<Handle<WasmState2>, u32> {
        Ok(WasmState2(555).into())
    }
    fn wasm_state2_result_variant() -> WasmStateResultVariant {
        WasmStateResultVariant::WasmState2(Handle::new(WasmState2(666)))
    }
    fn wasm_state2_result_list() -> Vec<Handle<WasmState2>> {
        vec![WasmState2(777).into(), WasmState2(888).into()]
    }
}

#[derive(Default)]
pub struct Markdown {
    buf: RefCell<String>,
}

impl exports::Markdown for Markdown {
    fn create() -> Option<Handle<Markdown>> {
        Some(Markdown::default().into())
    }

    fn append(&self, input: String) {
        self.buf.borrow_mut().push_str(&input);
    }

    fn render(&self) -> String {
        self.buf.borrow().replace("red", "green")
    }
}
