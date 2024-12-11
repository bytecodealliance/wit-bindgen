// use wit_bindgen_symmetric_rt::{CallbackState, EventSubscription};

#[link(name = "stream")]
extern "C" {
    pub fn testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
        args: *const (),
        results: *mut (),
    ) -> *mut ();
}

fn main() {
    let mut result_stream: *mut () = core::ptr::null_mut();
    let handle = unsafe {
        testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
            core::ptr::null_mut(),
            (&mut result_stream as *mut *mut ()).cast(),
        )
    };
    assert!(handle.is_null());
}
