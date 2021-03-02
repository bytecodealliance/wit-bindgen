#![cfg(target_arch = "wasm32")]

witx_bindgen_rust::import!("tests/host.witx");

#[no_mangle]
pub extern "C" fn run_host_tests() {
    host_scalars();
}

fn host_scalars() {
    assert_eq!(roundtrip_u8(1u8), 1);
    assert_eq!(roundtrip_u8(u8::min_value()), u8::min_value());
    assert_eq!(roundtrip_u8(u8::max_value()), u8::max_value());

    assert_eq!(roundtrip_s8(1i8), 1);
    assert_eq!(roundtrip_s8(i8::min_value()), i8::min_value());
    assert_eq!(roundtrip_s8(i8::max_value()), i8::max_value());
}
