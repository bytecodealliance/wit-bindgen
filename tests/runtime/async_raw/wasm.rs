use std::arch::wasm32;

#[link(wasm_import_module = "canonical_abi")]
extern "C" {
    pub fn async_export_done(ctx: i32, ptr: i32);
}

#[link(wasm_import_module = "imports")]
extern "C" {
    pub fn thunk(cb: i32, ptr: i32);
}

#[no_mangle]
pub extern "C" fn complete_immediately(ctx: i32) {
    unsafe {
        async_export_done(ctx, 0);
    }
}

#[no_mangle]
pub extern "C" fn completion_not_called(_ctx: i32) {}

#[no_mangle]
pub extern "C" fn complete_twice(ctx: i32) {
    unsafe {
        async_export_done(ctx, 0);
        async_export_done(ctx, 0);
    }
}

#[no_mangle]
pub extern "C" fn complete_then_trap(ctx: i32) {
    unsafe {
        async_export_done(ctx, 0);
        wasm32::unreachable();
    }
}

#[no_mangle]
pub extern "C" fn assert_coroutine_id_zero(ctx: i32) {
    unsafe {
        assert_eq!(ctx, 0);
        async_export_done(ctx, 0);
    }
}

#[no_mangle]
pub extern "C" fn not_async_export_done() {
    unsafe {
        async_export_done(0, 0);
    }
}

#[no_mangle]
pub extern "C" fn not_async_calls_async() {
    extern "C" fn callback(_x: i32) {}
    unsafe {
        thunk(callback as i32, 0);
    }
}

#[no_mangle]
pub extern "C" fn import_callback_null(_cx: i32) {
    unsafe {
        thunk(0, 0);
    }
}

#[no_mangle]
pub extern "C" fn import_callback_wrong_type(_cx: i32) {
    extern "C" fn callback() {}
    unsafe {
        thunk(callback as i32, 0);
    }
}

#[no_mangle]
pub extern "C" fn import_callback_bad_index(_cx: i32) {
    unsafe {
        thunk(i32::MAX, 0);
    }
}
