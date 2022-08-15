wit_bindgen_guest_rust::import!("../../tests/runtime/async_functions/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/async_functions/exports.wit");

struct Exports;

#[wit_bindgen_guest_rust::async_trait(?Send)]
impl exports::Exports for Exports {
    fn allocated_bytes() -> u32 {
        test_rust_wasm::get() as u32
    }

    async fn thunk() {
        imports::thunk().await;
    }

    async fn test_concurrent() {
        let a1 = imports::concurrent1(1);
        let a2 = imports::concurrent2(2);
        let a3 = imports::concurrent3(3);

        assert_eq!(futures_util::join!(a2, a3, a1), (12, 13, 11));
    }
}
