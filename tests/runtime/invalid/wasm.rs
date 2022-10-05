wit_bindgen_guest_rust::export!("../../tests/runtime/invalid/exports.wit");
wit_bindgen_guest_rust::import!("../../tests/runtime/invalid/imports.wit");

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
}
