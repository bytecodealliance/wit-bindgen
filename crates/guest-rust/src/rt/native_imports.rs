//! Native (non-WebAssembly) import resolution.
//!
//! On native targets imports are not resolved by the linker. Instead, each
//! generated import shim (via [`crate::rt::extern_wasm!`]) asks a
//! host-registered resolver for its implementation on first call and caches the
//! returned function pointer in a per-import `AtomicPtr`.

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

type ResolverFn = dyn Fn(&str, &str) -> Option<NonNull<()>> + Send + Sync;

static RESOLVERS: Mutex<Vec<Arc<ResolverFn>>> = Mutex::new(Vec::new());

/// Registers an import resolver for the current linkage unit.
///
/// When a generated import or runtime intrinsic is called on a native target
/// for the first time, registered resolvers are queried (most recently
/// registered first) with the import's canonical ABI core `module` and `name`
/// (for example `"my:pkg/iface@1.0.0"` and `"[method]res.frob"`, or `"$root"`
/// for world-level imports and canonical intrinsics). The first non-`None`
/// pointer returned is cached for all subsequent calls to that import.
///
/// Multiple resolvers may be registered (for example when independent worlds
/// coexist in the same process or test binary); each resolver should return
/// `None` for imports it does not recognize.
///
/// Note that `wit-bindgen` intentionally does not export a `#[no_mangle]` C
/// symbol for this function so that multiple versions of `wit-bindgen` can link
/// into the same binary without duplicate symbol errors. Dynamic library guests
/// loaded via `dlopen` should expose their own entry point that delegates here.
///
/// # Safety
///
/// Every pointer returned by `resolver` must be a valid function pointer
/// matching the exact `unsafe extern "C" fn(...)` Canonical ABI signature
/// expected by the requested `(module, name)` import, must abide by the
/// Canonical ABI memory ownership rules, and must remain valid for `'static`
/// because import shims cache the resolved pointer permanently.
pub unsafe fn register_import_resolver<F>(resolver: F)
where
    F: Fn(&str, &str) -> Option<NonNull<()>> + Send + Sync + 'static,
{
    RESOLVERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Arc::new(resolver));
}

/// Fast-path lookup called by [`crate::rt::extern_wasm!`] shims.
#[doc(hidden)]
#[inline]
pub fn resolve_import(module: &str, name: &str, cache: &AtomicPtr<()>) -> *mut () {
    let ptr = cache.load(Ordering::Acquire);
    if !ptr.is_null() {
        return ptr;
    }
    resolve_import_cold(module, name, cache)
}

#[cold]
#[inline(never)]
fn resolve_import_cold(module: &str, name: &str, cache: &AtomicPtr<()>) -> *mut () {
    let len = RESOLVERS.lock().unwrap_or_else(|e| e.into_inner()).len();
    assert!(
        len > 0,
        "import `{module}#{name}` was called on a native target without an import resolver \
         registered via `wit_bindgen::rt::register_import_resolver`"
    );
    for i in (0..len).rev() {
        let resolver = RESOLVERS.lock().unwrap_or_else(|e| e.into_inner())[i].clone();
        if let Some(ptr) = resolver(module, name) {
            let ptr = ptr.as_ptr();
            cache.store(ptr, Ordering::Release);
            return ptr;
        }
    }
    panic!("no registered import resolver provided an implementation for import `{module}#{name}`");
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicUsize;

    crate::rt::extern_wasm! {
        #[link(wasm_import_module = "test:native-unit/math")]
        unsafe extern "C" {
            #[link_name = "add"]
            fn imported_add(a: u32, b: u32) -> u32;
            #[link_name = "mul-ptr"]
            fn imported_mul_with_ptr_arg(ptr: u32, factor: u32) -> u32;
        }
    }

    unsafe extern "C" fn host_add(a: u32, b: u32) -> u32 {
        a + b
    }

    unsafe extern "C" fn host_mul(val: u32, factor: u32) -> u32 {
        val * factor
    }

    #[test]
    fn resolve_and_cache_imports() {
        static CALLS: AtomicUsize = AtomicUsize::new(0);

        unsafe {
            register_import_resolver(|module, name| {
                if module != "test:native-unit/math" {
                    return None;
                }
                CALLS.fetch_add(1, Ordering::Relaxed);
                match name {
                    "add" => NonNull::new(host_add as *mut ()),
                    "mul-ptr" => NonNull::new(host_mul as *mut ()),
                    _ => None,
                }
            });
        }

        assert_eq!(unsafe { imported_add(19, 23) }, 42);
        // Subsequent calls hit the per-import AtomicPtr cache without calling the resolver.
        assert_eq!(unsafe { imported_add(1, 2) }, 3);
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);

        // Parameter named `ptr` does not collide with anything inside `extern_wasm!`.
        assert_eq!(unsafe { imported_mul_with_ptr_arg(6, 7) }, 42);
        assert_eq!(CALLS.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn concurrent_registration_and_resolution() {
        crate::rt::extern_wasm! {
            #[link(wasm_import_module = "test:native-unit/concurrent")]
            unsafe extern "C" {
                #[link_name = "inc"]
                fn imported_inc(x: u32) -> u32;
            }
        }

        unsafe extern "C" fn host_inc(x: u32) -> u32 {
            x + 1
        }

        unsafe {
            register_import_resolver(|module, name| {
                if module == "test:native-unit/concurrent" && name == "inc" {
                    NonNull::new(host_inc as *mut ())
                } else {
                    None
                }
            });
        }

        std::thread::scope(|s| {
            for i in 0..4 {
                s.spawn(move || {
                    unsafe {
                        register_import_resolver(move |module, _name| {
                            if module == "test:native-unit/unused" {
                                NonNull::new(host_inc as *mut ())
                            } else {
                                let _ = i;
                                None
                            }
                        });
                    }
                    assert_eq!(unsafe { imported_inc(i) }, i + 1);
                });
            }
        });
    }
}
