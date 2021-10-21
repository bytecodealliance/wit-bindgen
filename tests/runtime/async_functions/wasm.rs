witx_bindgen_rust::import!("./tests/runtime/async_functions/imports.witx");
witx_bindgen_rust::export!("./tests/runtime/async_functions/exports.witx");

struct Exports;

#[witx_bindgen_rust::async_trait(?Send)]
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

    async fn concurrent_export(idx: u32) {
        imports::concurrent_export_helper(idx).await
    }

    async fn infinite_loop_async() {
        imports::iloop_entered();
        loop {}
    }

    fn infinite_loop() {
        imports::iloop_entered();
        loop {}
    }

    async fn call_import_then_trap() {
        let _f = imports::import_to_cancel();
        std::arch::wasm32::unreachable();
    }

    async fn call_infinite_import() {
        imports::import_to_cancel().await;
    }
}
