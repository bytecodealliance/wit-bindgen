#[link(name = "symmetric_executor")]
extern "C" {
    fn symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(nanoseconds: u64) -> *mut ();
}

#[no_mangle]
unsafe extern "C" fn async_sleep(
    args: *const (),
    _results: *mut (),
) -> *mut () {
    let nanoseconds = *args.cast::<u64>();
    symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(nanoseconds)
}
