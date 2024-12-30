use module::symmetric::runtime::symmetric_executor::{self, CallbackData, CallbackFunction};
pub use module::symmetric::runtime::symmetric_executor::{run, CallbackState, EventSubscription};

pub mod async_support;
mod module;

pub fn register(
    event: EventSubscription,
    f: extern "C" fn(*mut ()) -> CallbackState,
    data: *mut (),
) {
    let callback = unsafe { CallbackFunction::from_handle(f as *const () as usize) };
    let cb_data = unsafe { CallbackData::from_handle(data as usize) };
    symmetric_executor::register(event, callback, cb_data);
}

#[no_mangle]
fn cabi_realloc_wit_bindgen_0_36_0(
    _old_ptr: *mut u8,
    _old_len: usize,
    _align: usize,
    _new_len: usize,
) -> *mut u8 {
    todo!()
}
