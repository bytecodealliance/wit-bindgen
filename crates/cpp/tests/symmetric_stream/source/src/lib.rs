use wit_bindgen_symmetric_rt::async_support::Stream;

#[allow(non_snake_case)]

#[no_mangle]
pub fn testX3AtestX2Fstream_sourceX00X5BasyncX5Dcreate(_args:*mut u8, results:*mut u8) -> *mut u8 {
    let obj = Box::new(Stream::new());
    *unsafe{&mut *results.cast::<*mut Stream>()} = Box::into_raw(obj);
    std::ptr::null_mut()
}
