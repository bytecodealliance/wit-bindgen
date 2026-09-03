//! Native (non-wasm) import resolution.
//!
//! On native targets imports aren't resolved by the linker. Each generated
//! import shim instead asks a host-installed resolver for its implementation
//! the first time it's called, identifying the import by its core module and
//! function name as plain strings. The host installs the resolver once per
//! loaded library through `__wit_bindgen_set_import_resolver`.
//!
//! Because each shim caches the pointer the resolver handed it, installing a
//! resolver a second time has no effect on imports that have already been
//! called. Hosts are expected to install one before calling any export.

use core::ffi::{CStr, c_char};
use core::sync::atomic::{AtomicPtr, Ordering};

/// A host-provided callback returning the implementation of the import
/// named by `module` and `name`, or null if the host doesn't implement
/// it. The returned pointer must be a function with the import's core
/// signature. `ctx` is the value passed alongside the resolver, returned
/// to the host on every call.
///
/// `module` and `name` are the import's canonical ABI core module and
/// function name as two NUL-terminated strings. For example
/// `my:pkg/iface@1.0.0` and `[method]res.frob`, with module `$root` for a
/// function imported at the top level of a world.
pub type ImportResolver =
    unsafe extern "C" fn(ctx: *mut (), module: *const c_char, name: *const c_char) -> *mut ();

static RESOLVER: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());
static RESOLVER_CTX: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

/// Installs the import resolver for this linkage unit. Hosts call this
/// after loading the library and before calling any export.
///
/// Passing `None` for `resolver` uninstalls the current one. That only
/// affects imports which haven't been resolved yet. Imports already
/// called keep the pointer they cached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __wit_bindgen_set_import_resolver(
    resolver: Option<ImportResolver>,
    ctx: *mut (),
) {
    let resolver = match resolver {
        Some(resolver) => resolver as *mut (),
        None => core::ptr::null_mut(),
    };
    RESOLVER_CTX.store(ctx, Ordering::Relaxed);
    RESOLVER.store(resolver, Ordering::Release);
}

/// Builds the `&CStr` for an import name from a `concat!(name, "\0")`
/// literal, for the runtime's own intrinsic shims.
pub(crate) const fn cstr(with_nul: &'static str) -> &'static CStr {
    match CStr::from_bytes_with_nul(with_nul.as_bytes()) {
        Ok(s) => s,
        Err(_) => panic!("import name contains an interior NUL"),
    }
}

/// Called by generated import shims on their first invocation.
///
/// `module` and `name` are the strings described on [`ImportResolver`].
pub fn resolve_import(module: &CStr, name: &CStr) -> *mut () {
    let module_display = module.to_string_lossy();
    let name_display = name.to_string_lossy();
    let resolver = RESOLVER.load(Ordering::Acquire);
    assert!(
        !resolver.is_null(),
        "import `{module_display}#{name_display}` was called before the host installed an \
         import resolver via `__wit_bindgen_set_import_resolver`"
    );
    let ctx = RESOLVER_CTX.load(Ordering::Relaxed);
    let resolver: ImportResolver = unsafe { core::mem::transmute(resolver) };
    let ptr = unsafe { resolver(ctx, module.as_ptr(), name.as_ptr()) };
    assert!(
        !ptr.is_null(),
        "the host's import resolver provided no implementation for \
         import `{module_display}#{name_display}`"
    );
    ptr
}

/// The guest allocator, exported so hosts can allocate guest-owned
/// memory when lowering data, as the canonical ABI requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __wit_bindgen_cabi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    align: usize,
    new_len: usize,
) -> *mut u8 {
    unsafe { crate::rt::cabi_realloc(old_ptr, old_len, align, new_len) }
}
