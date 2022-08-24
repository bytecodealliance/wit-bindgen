wit_bindgen_guest_rust::export!("../../tests/runtime/invalid/exports.wit");

#[link(wasm_import_module = "imports")]
extern "C" {
    #[link_name = "roundtrip-bool"]
    fn roundtrip_bool(a: i32) -> i32;
    #[link_name = "roundtrip-u16"]
    fn roundtrip_u16(a: i32) -> i32;
    #[link_name = "roundtrip-u8"]
    fn roundtrip_u8(a: i32) -> i32;
    #[link_name = "roundtrip-s16"]
    fn roundtrip_s16(a: i32) -> i32;
    #[link_name = "roundtrip-s8"]
    fn roundtrip_s8(a: i32) -> i32;
    #[link_name = "roundtrip-char"]
    fn roundtrip_char(a: i32) -> i32;
    #[link_name = "roundtrip-enum"]
    fn roundtrip_enum(a: i32) -> i32;
    #[link_name = "get-internal"]
    fn get_internal(a: i32) -> i32;
}

#[link(wasm_import_module = "canonical_abi")]
extern "C" {
    #[link_name = "resource_drop_host-state"]
    fn resource_drop_host_state(a: i32);
}

struct Exports;

impl exports::Exports for Exports {
    fn invalid_u8() {
        unsafe {
            roundtrip_u8(i32::MAX);
        }
        unreachable!();
    }
    fn invalid_s8() {
        unsafe {
            roundtrip_s8(i32::MAX);
        }
        unreachable!();
    }
    fn invalid_u16() {
        unsafe {
            roundtrip_u16(i32::MAX);
        }
        unreachable!();
    }
    fn invalid_s16() {
        unsafe {
            roundtrip_s16(i32::MAX);
        }
        unreachable!();
    }
    fn invalid_char() {
        unsafe {
            roundtrip_char(0xd800);
        }
        unreachable!();
    }
    fn invalid_bool() {
        unsafe {
            roundtrip_bool(2);
        }
        unreachable!();
    }
    fn invalid_enum() {
        unsafe {
            roundtrip_enum(400);
        }
        unreachable!();
    }

    fn invalid_handle() {
        unsafe {
            get_internal(100);
        }
        unreachable!();
    }

    fn invalid_handle_close() {
        unsafe {
            resource_drop_host_state(100);
        }
        unreachable!();
    }
}
