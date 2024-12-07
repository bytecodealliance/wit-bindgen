#[link(name = "async_module")]
extern "C" {
    pub fn X5BasyncX5DtestX3AtestX2Fstring_delayX00forward(args: *const (), results: *mut (),) -> *mut ();
}

fn main() {
    let argument1: [usize;2] = ["A".as_ptr()as usize, 1];
    let mut result1: [usize;2] = [0,0];
    let handle1 = unsafe {
        X5BasyncX5DtestX3AtestX2Fstring_delayX00forward((&argument1 as *const usize).cast(), result1.as_mut_ptr().cast())
    };
    assert_eq!(handle1, core::ptr::null_mut());
    let vec = unsafe { Vec::from_raw_parts(result1[0] as *mut u8, result1[1], result1[1])};
    let string = std::str::from_utf8(&vec).unwrap();
    println!("Result {string}");

    let argument2: [usize;2] = ["B".as_ptr()as usize, 1];
    let mut result2: [usize;2] = [0,0];
    let handle2 = unsafe {
        X5BasyncX5DtestX3AtestX2Fstring_delayX00forward((&argument2 as *const usize).cast(), result2.as_mut_ptr().cast())
    };
    assert_ne!(handle2, core::ptr::null_mut());
    // register cb

    let argument3: [usize;2] = ["C".as_ptr()as usize, 1];
    let mut result3: [usize;2] = [0,0];
    let handle3 = unsafe {
        X5BasyncX5DtestX3AtestX2Fstring_delayX00forward((&argument3 as *const usize).cast(), result3.as_mut_ptr().cast())
    };
    assert_ne!(handle3, core::ptr::null_mut());
    // register cb

    // run
}
