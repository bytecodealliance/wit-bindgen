use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_gen_core::witx2::abi::{
    Bindgen, CallMode, Instruction, WasmType, WitxInstruction,
};
use witx_bindgen_gen_core::{witx2::*, Files, Generator, TypeInfo, Types};
use witx_bindgen_gen_rust::{int_repr, wasm_type, TypeMode, TypePrint, Visibility};

#[derive(Default)]
pub struct Wasmtime {
    tmp: usize,
    src: String,
    opts: Opts,
    needs_memory: bool,
    needs_borrow_checker: bool,
    needs_get_memory: bool,
    needs_get_func: bool,
    needs_char_from_i32: bool,
    needs_invalid_variant: bool,
    needs_validate_flags: bool,
    needs_store: bool,
    needs_load: bool,
    needs_bad_int: bool,
    needs_slice_as_bytes: bool,
    needs_copy_slice: bool,
    needs_functions: HashMap<String, NeededFunction>,
    needs_buffer_transaction: bool,
    needs_buffer_glue: bool,
    all_needed_handles: BTreeSet<String>,
    types: Types,
    imports: HashMap<String, Vec<Import>>,
    exports: HashMap<String, Exports>,
    params: Vec<String>,
    block_storage: Vec<String>,
    blocks: Vec<String>,
    is_dtor: bool,
    in_import: bool,
    in_trait: bool,
    cleanup: Option<String>,
    trait_name: String,
    closures: String,
    after_call: bool,
    // Whether or not the `caller_memory` variable has been defined and is
    // available for use.
    caller_memory_available: bool,
    sizes: SizeAlign,
}

enum NeededFunction {
    Realloc,
    Free,
}

struct Import {
    name: String,
    trait_signature: String,
    closure: String,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, (String, String)>,
    funcs: Vec<String>,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "structopt", structopt(long))]
    rustfmt: bool,
}

impl Opts {
    pub fn build(self) -> Wasmtime {
        let mut r = Wasmtime::new();
        r.opts = self;
        r
    }
}

impl Wasmtime {
    pub fn new() -> Wasmtime {
        Wasmtime::default()
    }

    fn print_intrinsics(&mut self) {
        if self.needs_store || self.needs_load {
            self.push_str("use witx_bindgen_wasmtime::rt::RawMem;");
        }
        if self.needs_char_from_i32 {
            self.push_str("use witx_bindgen_wasmtime::rt::char_from_i32;");
        }
        if self.needs_invalid_variant {
            self.push_str("use witx_bindgen_wasmtime::rt::invalid_variant;");
        }
        if self.needs_bad_int {
            self.push_str("use core::convert::TryFrom;\n");
            self.push_str("use witx_bindgen_wasmtime::rt::bad_int;");
        }
        if self.needs_validate_flags {
            self.push_str("use witx_bindgen_wasmtime::rt::validate_flags;");
        }
        if self.needs_slice_as_bytes {
            self.push_str(
                "
                    unsafe fn slice_as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
                        core::slice::from_raw_parts(
                            slice.as_ptr() as *const u8,
                            core::mem::size_of_val(slice),
                        )
                    }
                ",
            );
        }
        if self.needs_copy_slice {
            self.push_str("use witx_bindgen_wasmtime::rt::copy_slice;");
        }
    }

    fn memory_src(&mut self) -> String {
        if self.in_import {
            if !self.after_call {
                // Before calls we use `_bc` which is a borrow checker used for
                // getting long-lasting borrows into memory.
                self.needs_borrow_checker = true;
                return format!("_bc");
            }

            if !self.caller_memory_available {
                self.needs_memory = true;
                self.caller_memory_available = true;
                // get separate borrows of `caller_memory` and `_tables` if we
                // might need handle tables later. If we don't end up using
                // `_tables` that's ok, it'll almost always be optimized away.
                if self.all_needed_handles.len() > 0 {
                    self.push_str("let (caller_memory, data) = witx_bindgen_wasmtime::rt::data_and_memory(&mut caller, memory);");
                    self.push_str("let (_, _tables) = get(data);");
                } else {
                    self.push_str("let caller_memory = memory.data_mut(&mut caller);");
                }
            }
            format!("caller_memory")
        } else {
            self.needs_memory = true;
            format!("memory.data_mut(&mut caller)")
        }
    }
}

impl TypePrint for Wasmtime {
    fn krate(&self) -> &'static str {
        "witx_bindgen_wasmtime"
    }

    fn call_mode(&self) -> CallMode {
        if self.in_import {
            CallMode::NativeImport
        } else {
            CallMode::WasmExport
        }
    }

    fn default_param_mode(&self) -> TypeMode {
        if self.in_import {
            // The default here is that only leaf values can be borrowed because
            // otherwise lists and such need to be copied into our own memory.
            TypeMode::LeafBorrowed("'a")
        } else {
            // When we're calling wasm exports, however, there's no need to take
            // any ownership of anything from the host so everything is borrowed
            // in the parameter position.
            TypeMode::AllBorrowed("'a")
        }
    }

    fn handle_projection(&self) -> Option<(&'static str, String)> {
        if self.in_import {
            if self.in_trait {
                Some(("Self", self.trait_name.clone()))
            } else {
                Some(("T", self.trait_name.clone()))
            }
        } else {
            None
        }
    }

    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.types.get(ty)
    }

    fn types_mut(&mut self) -> &mut Types {
        &mut self.types
    }

    fn print_usize(&mut self) {
        self.src.push_str("u32");
    }

    fn print_pointer(&mut self, _iface: &Interface, _const_: bool, _ty: &Type) {
        self.push_str("u32");
    }

    fn print_borrowed_slice(
        &mut self,
        iface: &Interface,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
    ) {
        self.print_rust_slice(iface, mutbl, ty, lifetime);
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str(" str");
    }
}

impl Generator for Wasmtime {
    fn preprocess(&mut self, iface: &Interface, import: bool) {
        self.types.analyze(iface);
        self.in_import = import;
        self.trait_name = iface.name.to_camel_case();
        self.src
            .push_str(&format!("mod {} {{", iface.name.to_snake_case()));
        self.src
            .push_str("#[allow(unused_imports)] use witx_bindgen_wasmtime::{wasmtime, anyhow};\n");
        let mode = self.call_mode();
        self.sizes.fill(mode, iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        if record.is_flags() {
            self.src
                .push_str("witx_bindgen_wasmtime::bitflags::bitflags! {\n");
            self.rustdoc(docs);
            self.src
                .push_str(&format!("pub struct {}: ", name.to_camel_case()));
            let repr = iface
                .flags_repr(record)
                .expect("unsupported number of flags");
            self.int_repr(repr);
            self.src.push_str(" {\n");
            for (i, field) in record.fields.iter().enumerate() {
                self.rustdoc(&field.docs);
                self.src.push_str(&format!(
                    "const {} = 1 << {};\n",
                    field.name.to_camel_case(),
                    i
                ));
            }
            self.src.push_str("}\n");
            self.src.push_str("}\n\n");

            self.src.push_str("impl core::fmt::Display for ");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str(
                "{\nfn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
            );

            self.src.push_str("f.write_str(\"");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str("(\")?;\n");
            self.src.push_str("core::fmt::Debug::fmt(self, f)?;\n");
            self.src.push_str("f.write_str(\" (0x\")?;\n");
            self.src
                .push_str("core::fmt::LowerHex::fmt(&self.bits, f)?;\n");
            self.src.push_str("f.write_str(\"))\")?;\n");
            self.src.push_str("Ok(())");

            self.src.push_str("}\n");
            self.src.push_str("}\n\n");
            return;
        }

        self.print_typedef_record(iface, id, record, docs);
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.print_typedef_variant(iface, id, name, variant, docs);
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        let name = &iface.resources[ty].name;
        self.all_needed_handles.insert(name.to_string());

        // If we're binding imports then all handles are associated types so
        // there's nothing that we need to do about that.
        if self.in_import {
            return;
        }

        // ... otherwise for exports we generate a newtype wrapper around an
        // `i32` to manage the resultt.
        let tyname = name.to_camel_case();
        self.rustdoc(&iface.resources[ty].docs);
        self.src.push_str(&format!("pub struct {}(i32", tyname));
        self.src
            .push_str(", std::mem::ManuallyDrop<wasmtime::TypedFunc<(i32,), ()>>");
        self.src.push_str(");\n");

        self.src.push_str("impl ");
        self.src.push_str(&tyname);
        self.src.push_str(
            "{
                pub fn close(mut self) -> Result<(), wasmtime::Trap> {
                    let res = self.1.call((self.0,));
                    unsafe {
                        std::mem::ManuallyDrop::drop(&mut self.1);
                        std::mem::forget(self);
                    }
                    res
                }
            }",
        );

        self.src.push_str("impl std::fmt::Debug for ");
        self.src.push_str(&tyname);
        self.src.push_str(&format!(
            "{{
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                    f.debug_struct(\"{}\")
                        .field(\"handle\", &self.0)
                        .finish()
                }}
            }}",
            tyname,
        ));

        self.src.push_str("impl Drop for ");
        self.src.push_str(&tyname);
        self.src.push_str(
            "{
                fn drop(&mut self) {
                    drop(self.1.call((self.0,)));
                    unsafe {
                        std::mem::ManuallyDrop::drop(&mut self.1);
                    }
                }
            }",
        );
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(iface, id, ty, docs);
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_type_list(iface, id, ty, docs);
    }

    fn type_pointer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        const_: bool,
        ty: &Type,
        docs: &Docs,
    ) {
        self.rustdoc(docs);
        let mutbl = if const_ { "const" } else { "mut" };
        self.src
            .push_str(&format!("pub type {} = *{} ", name.to_camel_case(), mutbl,));
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push(';');
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push(';');
    }

    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        self.print_typedef_buffer(iface, id, true, ty, docs);
    }

    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
        self.print_typedef_buffer(iface, id, false, ty, docs);
    }

    // fn const_(&mut self, name: &Id, ty: &Id, val: u64, docs: &str) {
    //     self.rustdoc(docs);
    //     self.src.push_str(&format!(
    //         "pub const {}_{}: {} = {};\n",
    //         ty.to_shouty_snake_case(),
    //         name.to_shouty_snake_case(),
    //         ty.to_camel_case(),
    //         val
    //     ));
    // }

    fn import(&mut self, iface: &Interface, func: &Function) {
        self.tmp = 0;
        self.after_call = false;
        self.caller_memory_available = false;
        let prev = mem::take(&mut self.src);
        self.is_dtor = self.types.is_preview1_dtor_func(func);

        self.in_trait = true;
        self.print_signature(
            iface,
            func,
            Visibility::Private,
            false,
            Some("&mut self"),
            if self.is_dtor {
                TypeMode::Owned
            } else {
                TypeMode::LeafBorrowed("'_")
            },
        );
        self.in_trait = false;
        let trait_signature = mem::take(&mut self.src);

        self.params.truncate(0);
        let sig = iface.wasm_signature(CallMode::NativeImport, func);
        self.src
            .push_str("move |mut caller: wasmtime::Caller<'_, T>");
        for (i, param) in sig.params.iter().enumerate() {
            let arg = format!("arg{}", i);
            self.src.push_str(",");
            self.src.push_str(&arg);
            self.src.push_str(":");
            self.wasm_type(*param);
            self.params.push(arg);
        }
        self.src.push_str("| -> Result<_, wasmtime::Trap> {\n");
        let pos = self.src.len();
        iface.call(self.call_mode(), func, self);
        self.src.push_str("}");
        self.src.insert_str(pos, &mem::take(&mut self.closures));

        if self.all_needed_handles.len() > 0 {
            self.src.insert_str(pos, "let (host, _tables) = host;");
        }

        if self.needs_borrow_checker {
            self.src.insert_str(
                pos,
                "let (mem, data) = witx_bindgen_wasmtime::rt::data_and_memory(&mut caller, memory);
                let mut _bc = witx_bindgen_wasmtime::BorrowChecker::new(mem);
                let host = get(data);",
            );
            self.needs_memory = true;
        } else {
            self.src
                .insert_str(pos, "let host = get(caller.data_mut());\n");
        }

        if self.needs_memory {
            self.src
                .insert_str(pos, "let memory = &get_memory(&mut caller, \"memory\")?;\n");
            self.needs_get_memory = true;
        }

        self.needs_memory = false;
        self.needs_borrow_checker = false;
        assert!(!self.needs_buffer_transaction);

        for (name, func) in self.needs_functions.drain() {
            self.src.insert_str(
                pos,
                &format!(
                    "
                        let func = get_func(&mut caller, \"{name}\")?;
                        let func_{name} = func.typed::<{cvt}, _>(&caller)?;
                    ",
                    name = name,
                    cvt = func.cvt(),
                ),
            );
            self.needs_get_func = true;
        }

        let closure = mem::replace(&mut self.src, prev);
        self.imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.to_string(),
                closure,
                trait_signature,
            });
        assert!(self.cleanup.is_none());
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        self.tmp = 0;
        self.after_call = false;
        let prev = mem::take(&mut self.src);
        self.is_dtor = self.types.is_preview1_dtor_func(func);
        if self.is_dtor {
            assert_eq!(func.results.len(), 0, "destructors cannot have results");
        }
        self.params = self.print_docs_and_params(
            iface,
            func,
            Visibility::Pub,
            false,
            Some("&self, mut caller: impl wasmtime::AsContextMut"),
            TypeMode::AllBorrowed("'_"),
        );
        self.push_str("-> Result<");
        self.print_results(iface, func);
        self.push_str(", wasmtime::Trap> {\n");
        let pos = self.src.len();
        iface.call(self.call_mode(), func, self);
        self.src.push_str("}");

        if mem::take(&mut self.needs_buffer_transaction) {
            self.needs_buffer_glue = true;
            self.src.insert_str(
                pos,
                "let mut buffer_transaction = self.buffer_glue.transaction();\n",
            );
        }

        self.src.insert_str(pos, &mem::take(&mut self.closures));

        let exports = self
            .exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);

        assert!(!self.needs_borrow_checker);
        if self.needs_memory {
            self.needs_memory = false;
            self.src.insert_str(pos, "let memory = &self.memory;\n");
            exports.fields.insert(
                "memory".to_string(),
                (
                    "wasmtime::Memory".to_string(),
                    "instance
                        .get_memory(&mut store, \"memory\")
                         .ok_or_else(|| {
                             anyhow::anyhow!(\"`memory` export not a memory\")
                         })?
                    "
                    .to_string(),
                ),
            );
        }

        for (name, func) in self.needs_functions.drain() {
            self.src
                .insert_str(pos, &format!("let func_{0} = &self.{0};\n", name));
            let get = format!(
                "instance.get_typed_func::<{}, _>(&mut store, \"{}\")?",
                func.cvt(),
                name
            );
            exports.fields.insert(name, (func.ty(), get));
        }
        let func_body = mem::replace(&mut self.src, prev);
        if !self.is_dtor {
            exports.funcs.push(func_body);
        }

        // Create the code snippet which will define the type of this field in
        // the struct that we're exporting and additionally extracts the
        // function from an instantiated instance.
        let sig = iface.wasm_signature(CallMode::WasmExport, func);
        let mut cvt = "(".to_string();
        for param in sig.params.iter() {
            cvt.push_str(wasm_type(*param));
            cvt.push_str(",");
        }
        cvt.push_str("), (");
        for result in sig.results.iter() {
            cvt.push_str(wasm_type(*result));
            cvt.push_str(",");
        }
        cvt.push_str(")");
        exports.fields.insert(
            func.name.to_string(),
            (
                format!("wasmtime::TypedFunc<{}>", cvt),
                format!(
                    "instance.get_typed_func::<{}, _>(&mut store, \"{}\")?",
                    cvt, func.name,
                ),
            ),
        );
    }

    fn finish(&mut self, files: &mut Files) {
        for (module, funcs) in sorted_iter(&self.imports) {
            let module_camel = module.to_camel_case();
            self.src.push_str("\npub trait ");
            self.src.push_str(&module_camel);
            self.src.push_str(": Sized {\n");
            if self.all_needed_handles.len() > 0 {
                for handle in self.all_needed_handles.iter() {
                    self.src.push_str("type ");
                    self.src.push_str(&handle.to_camel_case());
                    self.src.push_str(";\n");
                }
            }
            for f in funcs {
                self.src.push_str(&f.trait_signature);
                self.src.push_str(";\n\n");
            }
            for handle in self.all_needed_handles.iter() {
                self.src.push_str(&format!(
                    "fn drop_{}(&mut self, state: Self::{}) {{
                        drop(state);
                    }}",
                    handle,
                    handle.to_camel_case(),
                ));
            }
            self.src.push_str("}\n");

            if self.all_needed_handles.len() > 0 {
                self.src.push_str("\npub struct ");
                self.src.push_str(&module_camel);
                self.src.push_str("Tables<T: ");
                self.src.push_str(&module_camel);
                self.src.push_str("> {\n");
                for handle in self.all_needed_handles.iter() {
                    self.src.push_str(&handle.to_snake_case());
                    self.src
                        .push_str("_table: witx_bindgen_wasmtime::Table<T::");
                    self.src.push_str(&handle.to_camel_case());
                    self.src.push_str(">,\n");
                }
                self.src.push_str("}\n");
                self.src.push_str("impl<T: ");
                self.src.push_str(&module_camel);
                self.src.push_str("> Default for ");
                self.src.push_str(&module_camel);
                self.src.push_str("Tables<T> {\n");
                self.src.push_str("fn default() -> Self { Self {");
                for handle in self.all_needed_handles.iter() {
                    self.src.push_str(&handle.to_snake_case());
                    self.src.push_str("_table: Default::default(),");
                }
                self.src.push_str("}}}");
            }
        }

        for (module, funcs) in mem::take(&mut self.imports) {
            let module_camel = module.to_camel_case();
            self.push_str("\npub fn add_");
            self.push_str(&module);
            self.push_str("_to_linker<T, U>(linker: &mut wasmtime::Linker<T>");
            self.push_str(", get: impl Fn(&mut T) -> ");
            if self.all_needed_handles.is_empty() {
                self.push_str("&mut U");
            } else {
                self.push_str(&format!("(&mut U, &mut {}Tables<U>)", module_camel));
            }
            self.push_str("+ Send + Sync + Copy + 'static) -> anyhow::Result<()> \n");
            self.push_str("where U: ");
            self.push_str(&module_camel);
            self.push_str("{\n");
            if self.needs_get_memory {
                self.push_str("use witx_bindgen_wasmtime::rt::get_memory;");
            }
            if self.needs_get_func {
                self.push_str("use witx_bindgen_wasmtime::rt::get_func;");
            }
            for f in funcs {
                self.push_str(&format!(
                    "linker.func_wrap(\"{}\", \"{}\", {})?;\n",
                    module, f.name, f.closure,
                ));
            }
            for handle in self.all_needed_handles.iter() {
                self.src.push_str(&format!(
                    "linker.func_wrap(
                        \"canonical_abi\",
                        \"resource_drop_{}\",
                        move |mut caller: wasmtime::Caller<'_, T>, handle: u32| {{
                            let (host, tables) = get(caller.data_mut());
                            let handle = tables
                                .{0}_table
                                .remove(handle)
                                .map_err(|e| {{
                                    wasmtime::Trap::new(format!(\"failed to remove handle: {{}}\", e))
                                }})?;
                            host.drop_{0}(handle);
                            Ok(())
                        }}
                    )?;\n",
                    handle
                ));
            }
            self.push_str("Ok(())\n}\n");
        }

        for (module, exports) in sorted_iter(&mem::take(&mut self.exports)) {
            let name = module.to_camel_case();
            self.push_str("pub struct ");
            self.push_str(&name);
            self.push_str("{\n");
            // Use `pub(super)` so that crates/test-wasmtime/src/exports.rs can access it.
            self.push_str("pub(super) instance: wasmtime::Instance,\n");
            for (name, (ty, _)) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(": ");
                self.push_str(ty);
                self.push_str(",\n");
            }
            if self.needs_buffer_glue {
                self.push_str("buffer_glue: witx_bindgen_wasmtime::exports::BufferGlue,");
            }
            self.push_str("}\n");
            self.push_str("impl ");
            self.push_str(&name);
            self.push_str(" {\n");

            self.push_str(
                "pub fn new<T>(
                    mut store: impl wasmtime::AsContextMut<Data = T>,
                    module: &wasmtime::Module,
                    linker: &mut wasmtime::Linker<T>,
                ) -> anyhow::Result<Self> {\n",
            );
            if self.needs_buffer_glue {
                self.push_str(
                    "
                        use witx_bindgen_wasmtime::rt::get_memory;

                        let buffer_glue = witx_bindgen_wasmtime::exports::BufferGlue::default();
                        let g = buffer_glue.clone();
                        linker.func(
                            \"witx_canonical_buffer_abi\",
                            \"in_len\",
                            move |handle: u32| g.in_len(handle),
                        )?;
                        let g = buffer_glue.clone();
                        linker.func(
                            \"witx_canonical_buffer_abi\",
                            \"in_read\",
                            move |caller: wasmtime::Caller<'_>, handle: u32, len: u32, offset: u32| {
                                let memory = get_memory(&mut caller, \"memory\")?;
                                g.in_read(handle, &memory, offset, len)
                            },
                        )?;
                        let g = buffer_glue.clone();
                        linker.func(
                            \"witx_canonical_buffer_abi\",
                            \"out_len\",
                            move |handle: u32| g.out_len(handle),
                        )?;
                        let g = buffer_glue.clone();
                        linker.func(
                            \"witx_canonical_buffer_abi\",
                            \"out_write\",
                            move |caller: wasmtime::Caller<'_>, handle: u32, len: u32, offset: u32| {
                                let memory = get_memory(&mut caller, \"memory\")?;
                                g.out_write(handle, &memory, offset, len)
                            },
                        )?;
                    ",
                );
            }
            self.push_str("let instance = linker.instantiate(&mut store, module)?;\n");
            assert!(!self.needs_get_func);
            for (name, (_, get)) in exports.fields.iter() {
                self.push_str("let ");
                self.push_str(&name);
                self.push_str("= ");
                self.push_str(&get);
                self.push_str(";\n");
            }
            self.push_str("Ok(");
            self.push_str(&name);
            self.push_str("{ instance,");
            for (name, _) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(",");
            }
            if self.needs_buffer_glue {
                self.push_str("buffer_glue,");
            }
            self.push_str("})");
            self.push_str("}\n");

            for func in exports.funcs.iter() {
                self.push_str(func);
            }

            self.push_str("}\n");
        }
        self.print_intrinsics();

        // Close the opening `mod`.
        self.push_str("}");

        let mut src = mem::take(&mut self.src);
        if self.opts.rustfmt {
            let mut child = Command::new("rustfmt")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn `rustfmt`");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(src.as_bytes())
                .unwrap();
            src.truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut src)
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }

        files.push("bindings.rs", &src);
    }
}

impl Bindgen for Wasmtime {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, to_restore);
        let expr = match operands.len() {
            0 => "()".to_string(),
            1 => operands[0].clone(),
            _ => format!("({})", operands.join(", ")),
        };
        if src.is_empty() {
            self.blocks.push(expr);
        } else if operands.is_empty() {
            self.blocks.push(format!("{{ {}; }}", src));
        } else {
            self.blocks.push(format!("{{ {}; {} }}", src, expr));
        }
        self.caller_memory_available = false;
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> String {
        unimplemented!()
    }

    fn i64_return_pointer_area(&mut self, _amt: usize) -> String {
        unimplemented!()
        // TODO: this should be a stack allocation, not one that goes through
        // malloc/free. Using malloc/free is too heavyweight for this purpose.
        // It's not clear how we can get access to the wasm module's stack,
        // however...
        // assert!(self.cleanup.is_none());
        // let tmp = self.tmp();
        // self.needs_functions
        //     .insert("witx_malloc".to_string(), NeededFunction::Malloc);
        // self.needs_functions
        //     .insert("witx_free".to_string(), NeededFunction::Free);
        // let ptr = format!("ptr{}", tmp);
        // self.src.push_str(&format!(
        //     "let {} = func_witx_malloc.call(&mut caller, ({} * 8, 8))?;\n",
        //     ptr, amt
        // ));
        // self.cleanup = Some(format!(
        //     "func_witx_free.call(&mut caller, ({}, {} * 8, 8))?;\n",
        //     ptr, amt
        // ));
        // return ptr;
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
        };

        let mut try_from = |cvt: &str, operands: &[String], results: &mut Vec<String>| {
            self.needs_bad_int = true;
            let result = format!("{}::try_from({}).map_err(bad_int)?", cvt, operands[0]);
            results.push(result);
        };

        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(format!("{}i32", val)),
            Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("0i32".to_string()),
                        WasmType::I64 => results.push("0i64".to_string()),
                        WasmType::F32 => results.push("0.0f32".to_string()),
                        WasmType::F64 => results.push("0.0f64".to_string()),
                    }
                }
            }

            Instruction::I64FromU64 | Instruction::I64FromS64 => {
                let s = operands.pop().unwrap();
                results.push(format!("witx_bindgen_wasmtime::rt::as_i64({})", s));
            }
            Instruction::I32FromUsize
            | Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromChar8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!("witx_bindgen_wasmtime::rt::as_i32({})", s));
            }

            Instruction::F32FromIf32
            | Instruction::F64FromIf64
            | Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }

            // Downcasts from `i32` into smaller integers are checked to ensure
            // that they fit within the valid range. While not strictly
            // necessary since we could chop bits off this should be more
            // forward-compatible with any future changes.
            Instruction::S8FromI32 => try_from("i8", operands, results),
            Instruction::Char8FromI32 | Instruction::U8FromI32 => try_from("u8", operands, results),
            Instruction::S16FromI32 => try_from("i16", operands, results),
            Instruction::U16FromI32 => try_from("u16", operands, results),

            // Casts of the same bit width simply use `as` since we're just
            // reinterpreting the bits already there.
            Instruction::U32FromI32 | Instruction::UsizeFromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),

            Instruction::CharFromI32 => {
                self.needs_char_from_i32 = true;
                results.push(format!("char_from_i32({})?", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                witx_bindgen_gen_rust::bitcast(casts, operands, results)
            }

            Instruction::I32FromOwnedHandle { ty } => {
                let name = &iface.resources[*ty].name;
                results.push(format!(
                    "_tables.{}_table.insert({}) as i32",
                    name.to_snake_case(),
                    operands[0]
                ));
            }
            Instruction::HandleBorrowedFromI32 { ty } => {
                let name = &iface.resources[*ty].name;
                if self.is_dtor {
                    results.push(format!(
                        "_tables.{}_table.remove(({}) as u32).map_err(|e| {{
                            wasmtime::Trap::new(format!(\"failed to remove handle: {{}}\", e))
                        }})?",
                        name.to_snake_case(),
                        operands[0]
                    ));
                } else {
                    results.push(format!(
                        "_tables.{}_table.get(({}) as u32).ok_or_else(|| {{
                            wasmtime::Trap::new(\"invalid handle index\")
                        }})?",
                        name.to_snake_case(),
                        operands[0]
                    ));
                }
            }
            Instruction::I32FromBorrowedHandle { .. } => {
                results.push(format!("{}.0", operands[0]));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                let name = &iface.resources[*ty].name;
                results.push(format!(
                    "{}({}, std::mem::ManuallyDrop::new(self.{}_close.clone()))",
                    name.to_camel_case(),
                    operands[0],
                    name.to_snake_case(),
                ));
            }

            Instruction::RecordLower { ty, record, .. } => {
                self.record_lower(iface, *ty, record, &operands[0], results);
            }
            Instruction::RecordLift { ty, record, .. } => {
                self.record_lift(iface, *ty, record, operands, results);
            }

            Instruction::FlagsLower { record, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
                for i in 0..record.num_i32s() {
                    results.push(format!("(flags{}.bits >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLower64 { .. } => {
                results.push(format!("({}).bits as i64", operands[0]));
            }
            Instruction::FlagsLift { record, name, .. }
            | Instruction::FlagsLift64 { record, name, .. } => {
                self.needs_validate_flags = true;
                let repr = iface
                    .flags_repr(record)
                    .expect("unsupported number of flags");
                let mut flags = String::from("0");
                for (i, op) in operands.iter().enumerate() {
                    flags.push_str(&format!("| (i64::from({}) << {})", op, i * 32));
                }
                results.push(format!(
                    "validate_flags(
                        {},
                        {name}::all().bits() as i64,
                        \"{name}\",
                        |b| {name} {{ bits: b as {ty} }}
                    )?",
                    flags,
                    name = name.to_camel_case(),
                    ty = int_repr(repr),
                ));
            }

            Instruction::VariantPayload => results.push("e".to_string()),

            Instruction::VariantLower {
                variant,
                name,
                nresults,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                self.variant_lower(variant, *name, *nresults, &operands[0], results, blocks);
            }

            Instruction::VariantLift { variant, name, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let mut result = format!("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    result.push_str(&i.to_string());
                    result.push_str(" => ");
                    self.variant_lift_case(variant, *name, case, &block, &mut result);
                    result.push_str(",\n");
                }
                let variant_name = name.map(|s| s.to_camel_case());
                let variant_name = variant_name.as_deref().unwrap_or_else(|| {
                    if variant.is_bool() {
                        "bool"
                    } else if variant.as_expected().is_some() {
                        "Result"
                    } else if variant.as_option().is_some() {
                        "Option"
                    } else {
                        unimplemented!()
                    }
                });
                result.push_str("_ => return Err(invalid_variant(\"");
                result.push_str(&variant_name);
                result.push_str("\")),\n");
                result.push_str("}");
                results.push(result);
                self.needs_invalid_variant = true;
            }

            Instruction::ListCanonLower { element, realloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                //
                // Note that the size of a list of `char` is 1 because it's
                // encoded as utf-8, otherwise it's just normal contiguous array
                // elements.
                let realloc = realloc.unwrap();
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);
                let (size, align) = match element {
                    Type::Char => (1, 1),
                    _ => (self.sizes.size(element), self.sizes.align(element)),
                };

                // Store the operand into a temporary...
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then realloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!(
                    "let {} = func_{}.call(&mut caller, (0, 0, ({}.len() as i32) * {}, {}))?;\n",
                    ptr, realloc, val, size, align
                ));
                self.caller_memory_available = false; // invalidated from above

                // ... and then copy over the result.
                //
                // Note the unsafety here, in general it's not safe to copy
                // from arbitrary types on the host as a slice of bytes, but in
                // this case we should be able to get away with it since
                // canonical lowerings have the same memory representation on
                // the host as in the guest.
                let mem = self.memory_src();
                self.push_str(&format!(
                    "{}.store({}, unsafe {{ slice_as_bytes({}.as_ref()) }})?;\n",
                    mem, ptr, val
                ));
                self.needs_store = true;
                self.needs_memory = true;
                self.needs_slice_as_bytes = true;
                results.push(ptr);
                results.push(format!("{}.len() as i32", val));
            }

            Instruction::ListCanonLift { element, free } => {
                // Note the unsafety here. This is possibly an unsafe operation
                // because the representation of the target must match the
                // representation on the host, but `ListCanonLift` is only
                // generated for types where that's true, so this should be
                // safe.
                match free {
                    Some(free) => {
                        self.needs_memory = true;
                        self.needs_copy_slice = true;
                        self.needs_functions
                            .insert(free.to_string(), NeededFunction::Free);
                        let (stringify, align) = match element {
                            Type::Char => (true, 1),
                            _ => (false, self.sizes.align(element)),
                        };
                        let tmp = self.tmp();
                        self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                        self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                        let result = format!(
                            "
                                unsafe {{
                                    copy_slice(
                                        &mut caller,
                                        memory,
                                        func_{},
                                        ptr{tmp}, len{tmp}, {}
                                    )?
                                }}
                            ",
                            free,
                            align,
                            tmp = tmp
                        );
                        if stringify {
                            results.push(format!(
                                "String::from_utf8({})
                                    .map_err(|_| wasmtime::Trap::new(\"invalid utf-8\"))?",
                                result
                            ));
                        } else {
                            results.push(result);
                        }
                    }
                    None => {
                        self.needs_borrow_checker = true;
                        let method = match element {
                            Type::Char => "slice_str",
                            _ => "slice",
                        };
                        let tmp = self.tmp();
                        self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                        self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                        let mut slice = format!("_bc.{}(ptr{1}, len{1})?", method, tmp);
                        if method == "slice" {
                            slice = format!("unsafe {{ {} }}", slice);
                        }
                        results.push(slice);
                    }
                }
            }

            Instruction::ListLower { element, realloc } => {
                let realloc = realloc.unwrap();
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);
                let size = self.sizes.size(element);
                let align = self.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));

                // ... then realloc space for the result in the guest module
                self.push_str(&format!(
                    "let {} = func_{}.call(&mut caller, (0, 0, {} * {}, {}))?;\n",
                    result, realloc, len, size, align,
                ));
                self.caller_memory_available = false; // invalidated by call

                // ... then consume the vector and use the block to lower the
                // result.
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!("let base = {} + (i as i32) * {};\n", result, size));
                self.push_str(&body);
                self.push_str("}");

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.sizes.size(element);
                let align = self.sizes.align(element);
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {};\n", len, operands[1]));
                let base = format!("base{}", tmp);
                self.push_str(&format!("let {} = {};\n", base, operands[0]));
                let result = format!("result{}", tmp);
                self.push_str(&format!(
                    "let mut {} = Vec::with_capacity({} as usize);\n",
                    result, len,
                ));

                self.push_str("for i in 0..");
                self.push_str(&len);
                self.push_str(" {\n");
                self.push_str("let base = ");
                self.push_str(&base);
                self.push_str(" + i *");
                self.push_str(&size.to_string());
                self.push_str(";\n");
                self.push_str(&result);
                self.push_str(".push(");
                self.push_str(&body);
                self.push_str(");\n");
                self.push_str("}\n");
                results.push(result);

                if let Some(free) = free {
                    self.push_str(&format!(
                        "func_{}.call(&mut caller, ({}, {} * {}, {}))?;\n",
                        free, base, len, size, align,
                    ));
                    self.needs_functions
                        .insert(free.to_string(), NeededFunction::Free);
                }
            }

            Instruction::IterElem => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            // Never used due to the call modes that this binding generator
            // uses
            Instruction::BufferLowerPtrLen { .. } => unreachable!(),
            Instruction::BufferLiftHandle { .. } => unimplemented!(),

            Instruction::BufferLiftPtrLen { push, ty } => {
                let block = self.blocks.pop().unwrap();
                self.needs_borrow_checker = true;
                let tmp = self.tmp();
                self.push_str(&format!("let _ = {};\n", operands[0]));
                self.push_str(&format!("let ptr{} = {};\n", tmp, operands[1]));
                self.push_str(&format!("let len{} = {};\n", tmp, operands[2]));
                if iface.all_bits_valid(ty) {
                    let method = if *push { "slice_mut" } else { "slice" };
                    results.push(format!("unsafe {{ _bc.{}(ptr{1}, len{1})? }}", method, tmp));
                } else {
                    let size = self.sizes.size(ty);
                    let closure = format!("closure{}", tmp);
                    self.closures.push_str(&format!("let {} = ", closure));
                    if *push {
                        self.closures.push_str("|_bc: &mut [u8], e:");
                        mem::swap(&mut self.closures, &mut self.src);
                        self.print_ty(iface, ty, TypeMode::Owned);
                        mem::swap(&mut self.closures, &mut self.src);
                        self.closures.push_str("| {let base = 0;\n");
                        self.closures.push_str(&block);
                        self.closures.push_str("; Ok(()) };\n");
                        results.push(format!(
                            "witx_bindgen_wasmtime::imports::PushBuffer::new(
                                &mut _bc, ptr{}, len{}, {}, &{})?",
                            tmp, tmp, size, closure
                        ));
                    } else {
                        self.closures.push_str("|_bc: &[u8]| { let base = 0;Ok(");
                        self.closures.push_str(&block);
                        self.closures.push_str(") };\n");
                        results.push(format!(
                            "witx_bindgen_wasmtime::imports::PullBuffer::new(
                                &mut _bc, ptr{}, len{}, {}, &{})?",
                            tmp, tmp, size, closure
                        ));
                    }
                }
            }

            Instruction::BufferLowerHandle { push, ty } => {
                let block = self.blocks.pop().unwrap();
                let size = self.sizes.size(ty);
                let tmp = self.tmp();
                let handle = format!("handle{}", tmp);
                let closure = format!("closure{}", tmp);
                self.needs_buffer_transaction = true;
                if iface.all_bits_valid(ty) {
                    let method = if *push { "push_out_raw" } else { "push_in_raw" };
                    self.push_str(&format!(
                        "let {} = unsafe {{ buffer_transaction.{}({}) }};\n",
                        handle, method, operands[0],
                    ));
                } else if *push {
                    self.closures.push_str(&format!(
                        "let {} = |memory: &wasmtime::Memory, base: i32| {{
                            Ok(({}, {}))
                        }};\n",
                        closure, block, size,
                    ));
                    self.push_str(&format!(
                        "let {} = unsafe {{ buffer_transaction.push_out({}, &{}) }};\n",
                        handle, operands[0], closure,
                    ));
                } else {
                    let start = self.src.len();
                    self.print_ty(iface, ty, TypeMode::AllBorrowed("'_"));
                    let ty = self.src[start..].to_string();
                    self.src.truncate(start);
                    self.closures.push_str(&format!(
                        "let {} = |memory: &wasmtime::Memory, base: i32, e: {}| {{
                            {};
                            Ok({})
                        }};\n",
                        closure, ty, block, size,
                    ));
                    self.push_str(&format!(
                        "let {} = unsafe {{ buffer_transaction.push_in({}, &{}) }};\n",
                        handle, operands[0], closure,
                    ));
                }
                results.push(format!("{}", handle));
            }

            Instruction::CallWasm {
                module: _,
                name,
                sig,
            } => {
                if sig.results.len() > 0 {
                    let tmp = self.tmp();
                    self.push_str("let (");
                    for i in 0..sig.results.len() {
                        let arg = format!("result{}_{}", tmp, i);
                        self.push_str(&arg);
                        self.push_str(",");
                        results.push(arg);
                    }
                    self.push_str(") = ");
                }
                self.push_str("self.");
                self.push_str(name);
                self.push_str(".call(&mut caller, (");
                for operand in operands {
                    self.push_str(operand);
                    self.push_str(", ");
                }
                self.push_str("))?;");
                self.after_call = true;
                self.caller_memory_available = false; // invalidated by call
            }

            Instruction::CallInterface { module: _, func } => {
                for (i, operand) in operands.iter().enumerate() {
                    self.push_str(&format!("let param{} = {};\n", i, operand));
                }
                self.let_results(func.results.len(), results);
                self.push_str("host.");
                self.push_str(&func.name);
                self.push_str("(");
                for i in 0..operands.len() {
                    self.push_str(&format!("param{}, ", i));
                }
                self.push_str(");");
                self.after_call = true;
            }

            Instruction::Return { amt } => {
                let result = match amt {
                    0 => format!("Ok(())"),
                    1 => format!("Ok({})", operands[0]),
                    _ => format!("Ok(({}))", operands.join(", ")),
                };
                match self.cleanup.take() {
                    Some(cleanup) => {
                        self.push_str("let ret = ");
                        self.push_str(&result);
                        self.push_str(";\n");
                        self.push_str(&cleanup);
                        self.push_str("ret");
                    }
                    None => self.push_str(&result),
                }
            }

            Instruction::I32Load { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "{}.load({} + {}, [0u8; 4], i32::from_le_bytes)?",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I32Load8U { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "i32::from({}.load({} + {}, [0u8; 1], u8::from_le_bytes)?)",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I32Load8S { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "i32::from({}.load({} + {}, [0u8; 1], i8::from_le_bytes)?)",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I32Load16U { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "i32::from({}.load({} + {}, [0u8; 2], u16::from_le_bytes)?)",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I32Load16S { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "i32::from({}.load({} + {}, [0u8; 2], i16::from_le_bytes)?)",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I64Load { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "{}.load({} + {}, [0u8; 8], i64::from_le_bytes)?",
                    mem, operands[0], offset,
                ));
            }
            Instruction::F32Load { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "{}.load({} + {}, [0u8; 4], f32::from_le_bytes)?",
                    mem, operands[0], offset,
                ));
            }
            Instruction::F64Load { offset } => {
                let mem = self.memory_src();
                self.needs_load = true;
                results.push(format!(
                    "{}.load({} + {}, [0u8; 8], f64::from_le_bytes)?",
                    mem, operands[0], offset,
                ));
            }
            Instruction::I32Store { offset }
            | Instruction::I64Store { offset }
            | Instruction::F32Store { offset }
            | Instruction::F64Store { offset } => {
                let mem = self.memory_src();
                self.needs_store = true;
                self.push_str(&format!(
                    "{}.store({} + {}, &({}).to_le_bytes())?;\n",
                    mem, operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                let mem = self.memory_src();
                self.needs_store = true;
                self.push_str(&format!(
                    "{}.store({} + {}, &(({}) as u8).to_le_bytes())?;\n",
                    mem, operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                let mem = self.memory_src();
                self.needs_store = true;
                self.push_str(&format!(
                    "{}.store({} + {}, &(({}) as u16).to_le_bytes())?;\n",
                    mem, operands[1], offset, operands[0]
                ));
            }

            Instruction::Witx { instr } => match instr {
                WitxInstruction::PointerFromI32 { .. }
                | WitxInstruction::ConstPointerFromI32 { .. } => {
                    for _ in 0..instr.results_len() {
                        results.push("XXX".to_string());
                    }
                }
                i => unimplemented!("{:?}", i),
            },
        }
    }
}

impl NeededFunction {
    fn cvt(&self) -> &'static str {
        match self {
            NeededFunction::Realloc => "(i32, i32, i32, i32), i32",
            NeededFunction::Free => "(i32, i32, i32), ()",
        }
    }

    fn ty(&self) -> String {
        format!("wasmtime::TypedFunc<{}>", self.cvt())
    }
}

fn sorted_iter<K: Ord, V>(map: &HashMap<K, V>) -> impl Iterator<Item = (&K, &V)> {
    let mut list = map.into_iter().collect::<Vec<_>>();
    list.sort_by_key(|p| p.0);
    list.into_iter()
}
