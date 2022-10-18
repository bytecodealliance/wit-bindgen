wit_bindgen_guest_rust::generate!({
    import: "../../tests/runtime/invalid/imports.wit",
    default: "../../tests/runtime/invalid/exports.wit",
    name: "exports",
});

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
    #[link_name = "unaligned1"]
    fn unaligned1(ptr: i32, len: i32);
    #[link_name = "unaligned2"]
    fn unaligned2(ptr: i32, len: i32);
    #[link_name = "unaligned3"]
    fn unaligned3(ptr: i32, len: i32);
    #[link_name = "unaligned4"]
    fn unaligned4(ptr: i32, len: i32);
    #[link_name = "unaligned5"]
    fn unaligned5(ptr: i32, len: i32);
    #[link_name = "unaligned6"]
    fn unaligned6(ptr: i32, len: i32);
    #[link_name = "unaligned7"]
    fn unaligned7(ptr: i32, len: i32);
    #[link_name = "unaligned8"]
    fn unaligned8(ptr: i32, len: i32);
    #[link_name = "unaligned9"]
    fn unaligned9(ptr: i32, len: i32);
    #[link_name = "unaligned10"]
    fn unaligned10(ptr: i32, len: i32);
}

struct Exports;

export_exports!(Exports);

impl exports::Exports for Exports {
    fn invalid_bool() {
        unsafe {
            let b = roundtrip_bool(2);
            assert_eq!(b, 1);
        }
    }
    fn invalid_u8() {
        unsafe {
            let u = roundtrip_u8(i32::MAX);
            assert!(u <= (u8::MAX as i32));
            assert!(u >= (u8::MIN as i32));
        }
    }
    fn invalid_s8() {
        unsafe {
            let s = roundtrip_s8(i32::MAX);
            assert!(s <= (i8::MAX as i32));
            assert!(s >= (i8::MIN as i32));
        }
    }
    fn invalid_u16() {
        unsafe {
            let u = roundtrip_u16(i32::MAX);
            assert!(u <= (u16::MAX as i32));
            assert!(u >= (u16::MIN as i32));
        }
    }
    fn invalid_s16() {
        unsafe {
            let s = roundtrip_s16(i32::MAX);
            assert!(s <= (i16::MAX as i32));
            assert!(s >= (i16::MIN as i32));
        }
    }
    fn invalid_char() {
        unsafe {
            roundtrip_char(0xd800);
        }
        unreachable!();
    }
    fn invalid_enum() {
        unsafe {
            roundtrip_enum(400);
        }
        unreachable!();
    }

    fn unaligned1() {
        unsafe {
            unaligned1(1, 1);
        }
    }
    fn unaligned2() {
        unsafe {
            unaligned2(1, 1);
        }
    }
    fn unaligned3() {
        unsafe {
            unaligned3(1, 1);
        }
    }
    fn unaligned4() {
        unsafe {
            unaligned4(1, 1);
        }
    }
    fn unaligned5() {
        unsafe {
            unaligned5(1, 1);
        }
    }
    fn unaligned6() {
        unsafe {
            unaligned6(1, 1);
        }
    }
    fn unaligned7() {
        unsafe {
            unaligned7(1, 1);
        }
    }
    fn unaligned8() {
        unsafe {
            unaligned8(1, 1);
        }
    }
    fn unaligned9() {
        unsafe {
            unaligned9(1, 1);
        }
    }
    fn unaligned10() {
        unsafe {
            unaligned10(1, 1);
        }
    }
}
