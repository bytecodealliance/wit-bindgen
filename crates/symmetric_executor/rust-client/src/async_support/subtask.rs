use std::alloc::Layout;
use std::future::Future;

// dummy to just make the generated code compile, for now
pub unsafe trait Subtask {
    const ABI_LAYOUT: Layout;
    const RESULTS_OFFSET: usize;
    type Params;
    type Results;
    type ParamsLower: Copy;
    unsafe fn call_import(params: Self::ParamsLower, results: *mut u8) -> u32;
    unsafe fn params_lower(params: Self::Params, dst: *mut u8) -> Self::ParamsLower;
    unsafe fn params_dealloc_lists(lower: Self::ParamsLower);
    unsafe fn params_dealloc_lists_and_own(lower: Self::ParamsLower);
    unsafe fn results_lift(src: *mut u8) -> Self::Results;
    fn call(_params: Self::Params) -> impl Future<Output = Self::Results>
    where
        Self: Sized,
    {
        async { todo!() }
    }
}
