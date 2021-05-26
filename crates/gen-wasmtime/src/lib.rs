use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_gen_core::{witx::*, Files, Generator, TypeInfo, Types};
use witx_bindgen_gen_rust::{int_repr, wasm_type, TypeMode, TypePrint, Visibility};

#[derive(Default)]
pub struct Wasmtime {
    tmp: usize,
    src: String,
    opts: Opts,
    needs_memory: bool,
    needs_guest_memory: bool,
    needs_get_memory: bool,
    needs_get_func: bool,
    needs_char_from_i32: bool,
    needs_invalid_variant: bool,
    needs_validate_flags: bool,
    needs_store: bool,
    needs_load: bool,
    needs_bad_int: bool,
    needs_borrow_checker: bool,
    needs_slice_as_bytes: bool,
    needs_copy_slice: bool,
    needs_functions: HashMap<String, NeededFunction>,
    needs_buffer_transaction: bool,
    needs_buffer_glue: bool,
    all_needed_handles: BTreeSet<String>,
    handles_for_func: BTreeSet<String>,
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
}

enum NeededFunction {
    Malloc,
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
        if self.needs_store {
            self.push_str(
                "
                    fn store(
                        mem: &wasmtime::Memory,
                        offset: i32,
                        bytes: &[u8],
                    ) -> Result<(), wasmtime::Trap> {
                        mem.write(offset as usize, bytes)
                            .map_err(|_| wasmtime::Trap::new(\"out of bounds write\"))?;
                        Ok(())
                    }
                ",
            );
        }
        if self.needs_load {
            self.push_str(
                "
                    fn load<T: AsMut<[u8]>, U>(
                        mem: &wasmtime::Memory,
                        offset: i32,
                        mut bytes: T,
                        cvt: impl FnOnce(T) -> U,
                    ) -> Result<U, wasmtime::Trap> {
                        mem.read(offset as usize, bytes.as_mut())
                            .map_err(|_| wasmtime::Trap::new(\"out of bounds read\"))?;
                        Ok(cvt(bytes))
                    }
                ",
            );
        }
        if self.needs_char_from_i32 {
            self.push_str(
                "
                    fn char_from_i32(
                        val: i32,
                    ) -> Result<char, wasmtime::Trap> {
                        core::char::from_u32(val as u32)
                            .ok_or_else(|| {
                                wasmtime::Trap::new(\"char value out of valid range\")
                            })
                    }
                ",
            );
        }
        if self.needs_invalid_variant {
            self.push_str(
                "
                    fn invalid_variant(name: &str) -> wasmtime::Trap {
                        let msg = format!(\"invalid discriminant for `{}`\", name);
                        wasmtime::Trap::new(msg)
                    }
                ",
            );
        }
        if self.needs_bad_int {
            self.push_str("use core::convert::TryFrom;\n");
            self.push_str(
                "
                    fn bad_int(_: core::num::TryFromIntError) -> wasmtime::Trap {
                        let msg = \"out-of-bounds integer conversion\";
                        wasmtime::Trap::new(msg)
                    }
                ",
            );
        }
        if self.needs_validate_flags {
            self.push_str(
                "
                    fn validate_flags<U>(
                        bits: i64,
                        all: i64,
                        name: &str,
                        mk: impl FnOnce(i64) -> U,
                    ) -> Result<U, wasmtime::Trap> {
                        if bits & !all != 0 {
                            let msg = format!(\"invalid flags specified for `{}`\", name);
                            Err(wasmtime::Trap::new(msg))
                        } else {
                            Ok(mk(bits))
                        }
                    }
                ",
            );
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
            self.push_str(
                "
                    unsafe fn copy_slice<T: Copy>(
                        memory: &wasmtime::Memory,
                        free: &wasmtime::TypedFunc<(i32, i32, i32), ()>,
                        base: i32,
                        len: i32,
                        align: i32,
                    ) -> Result<Vec<T>, wasmtime::Trap> {
                        let mut result = Vec::with_capacity(len as usize);
                        let size = len * (std::mem::size_of::<T>() as i32);
                        let slice = memory.data_unchecked()
                            .get(base as usize..)
                            .and_then(|s| s.get(..size as usize))
                            .ok_or_else(|| wasmtime::Trap::new(\"out of bounds read\"))?;
                        std::slice::from_raw_parts_mut(
                            result.as_mut_ptr() as *mut u8,
                            size as usize,
                        ).copy_from_slice(slice);
                        result.set_len(size as usize);
                        free.call((base, size, align))?;
                        Ok(result)
                    }
                ",
            );
        }
    }
}

impl TypePrint for Wasmtime {
    fn krate(&self) -> &'static str {
        "witx_bindgen_wasmtime"
    }

    fn call_mode(&self) -> CallMode {
        if self.in_import {
            CallMode::DefinedImport
        } else {
            CallMode::DeclaredExport
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

    fn info(&self, ty: &Id) -> TypeInfo {
        self.types.get(ty)
    }

    fn types_mut(&mut self) -> &mut Types {
        &mut self.types
    }

    fn print_usize(&mut self) {
        self.src.push_str("u32");
    }

    fn print_pointer(&mut self, const_: bool, ty: &TypeRef) {
        self.push_str("*");
        if const_ {
            self.push_str("const ");
        } else {
            self.push_str("mut ");
        }
        match &**ty.type_() {
            Type::Builtin(_) | Type::Pointer(_) | Type::ConstPointer(_) => {
                self.print_tref(ty, TypeMode::Owned);
            }
            Type::List(_) | Type::Variant(_) | Type::Buffer(_) => panic!("unsupported type"),
            Type::Handle(_) | Type::Record(_) => {
                self.push_str("core::mem::ManuallyDrop<");
                self.print_tref(ty, TypeMode::Owned);
                self.push_str(">");
            }
        }
    }

    fn print_borrowed_slice(&mut self, mutbl: bool, ty: &TypeRef, lifetime: &'static str) {
        if self.in_import {
            self.push_str("witx_bindgen_wasmtime::GuestPtr<");
            self.push_str(lifetime);
            self.push_str(",[");
            // This should only ever be used on types without lifetimes, so use
            // invalid syntax here to catch bugs where that's not the case.
            self.print_tref(ty, TypeMode::AllBorrowed("INVALID"));
            self.push_str("]>");
        } else {
            self.print_rust_slice(mutbl, ty, lifetime);
        }
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        if self.in_import {
            self.push_str("witx_bindgen_wasmtime::GuestPtr<");
            self.push_str(lifetime);
            self.push_str(",str>");
        } else {
            self.push_str("&");
            if lifetime != "'_" {
                self.push_str(lifetime);
                self.push_str(" ");
            }
            self.push_str(" str");
        }
    }
}

impl Generator for Wasmtime {
    fn preprocess(&mut self, module: &Module, import: bool) {
        self.types.analyze(module);
        self.in_import = import;
        self.trait_name = module.name().as_str().to_camel_case();
        self.src
            .push_str(&format!("mod {} {{", module.name().as_str().to_snake_case()));
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
        if let Some(repr) = record.bitflags_repr() {
            let name = name.as_str();
            self.src.push_str("bitflags::bitflags! {\n");
            self.rustdoc(docs);
            self.src
                .push_str(&format!("pub struct {}: ", name.to_camel_case()));
            self.int_repr(repr);
            self.src.push_str(" {\n");
            for (i, member) in record.members.iter().enumerate() {
                self.rustdoc(&member.docs);
                self.src.push_str(&format!(
                    "const {} = 1 << {};\n",
                    member.name.as_str().to_camel_case(),
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

        self.print_typedef_record(name, record, docs);
    }

    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str) {
        self.print_typedef_variant(name, variant, docs);
    }

    fn type_handle(&mut self, name: &Id, _ty: &HandleDatatype, docs: &str) {
        // If we're binding imports then all handles are associated types so
        // there's nothing that we need to do about that.
        if self.in_import {
            return;
        }

        // ... otherwise for exports we generate a newtype wrapper around an
        // `i32` to manage the resultt.
        let info = self.info(name);
        let tyname = name.as_str().to_camel_case();
        self.rustdoc(docs);
        if !info.handle_with_dtor {
            self.src.push_str("#[derive(Debug)]\n");
        }
        self.src.push_str(&format!("pub struct {}(i32", tyname));
        if info.handle_with_dtor {
            self.src
                .push_str(", std::mem::ManuallyDrop<wasmtime::TypedFunc<(i32,), ()>>");
        }
        self.src.push_str(");\n");

        if info.handle_with_dtor {
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
    }

    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str) {
        self.print_typedef_alias(name, ty, docs);
    }

    fn type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str) {
        self.print_type_list(name, ty, docs);
    }

    fn type_pointer(&mut self, name: &Id, const_: bool, ty: &TypeRef, docs: &str) {
        self.rustdoc(docs);
        let mutbl = if const_ { "const" } else { "mut" };
        self.src.push_str(&format!(
            "pub type {} = *{} ",
            name.as_str().to_camel_case(),
            mutbl,
        ));
        self.print_tref(ty, TypeMode::Owned);
        self.src.push(';');
    }

    fn type_builtin(&mut self, name: &Id, ty: BuiltinType, docs: &str) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.as_str().to_camel_case()));
        self.src.push_str(" = ");
        self.print_builtin(ty);
        self.src.push(';');
    }

    fn type_buffer(&mut self, name: &Id, ty: &Buffer, docs: &str) {
        self.print_typedef_buffer(name, ty, docs);
    }

    fn const_(&mut self, name: &Id, ty: &Id, val: u64, docs: &str) {
        self.rustdoc(docs);
        self.src.push_str(&format!(
            "pub const {}_{}: {} = {};\n",
            ty.as_str().to_shouty_snake_case(),
            name.as_str().to_shouty_snake_case(),
            ty.as_str().to_camel_case(),
            val
        ));
    }

    fn import(&mut self, module: &Id, func: &Function) {
        self.tmp = 0;
        let prev = mem::take(&mut self.src);
        self.is_dtor = self.types.is_dtor_func(&func.name);

        self.in_trait = true;
        self.print_signature(
            func,
            Visibility::Private,
            false,
            true,
            if self.is_dtor {
                TypeMode::Owned
            } else {
                TypeMode::LeafBorrowed("'_")
            },
        );
        self.in_trait = false;
        let trait_signature = mem::take(&mut self.src);

        self.params.truncate(0);
        let sig = func.wasm_signature(CallMode::DefinedImport);
        self.src.push_str("move |_caller: wasmtime::Caller<'_>");
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
        func.call(module, CallMode::DefinedImport, self);
        self.src.push_str("}");
        self.src.insert_str(pos, &mem::take(&mut self.closures));

        if self.needs_guest_memory {
            // TODO: this unsafe isn't justified and it's actually unsafe, we
            // need a better solution for where to store this.
            self.src.insert_str(
                pos,
                "let guest_memory = &unsafe { witx_bindgen_wasmtime::WasmtimeGuestMemory::new(
                    memory,
                    m.borrow_checker(),
                ) };\n",
            );
            self.needs_borrow_checker = true;
        }
        if self.needs_memory || self.needs_guest_memory {
            self.src
                .insert_str(pos, "let memory = &get_memory(&_caller, \"memory\")?;\n");
            self.needs_get_memory = true;
        }

        self.needs_memory = false;
        self.needs_guest_memory = false;
        assert!(!self.needs_buffer_transaction);

        if self.handles_for_func.len() > 0 {
            for handle in self.handles_for_func.iter() {
                self.src.insert_str(
                    pos,
                    &format!(
                        "let {0}_table_access = m.{0}_table().access();\n",
                        handle.as_str().to_snake_case()
                    ),
                );
                self.all_needed_handles.insert(handle.clone());
            }
            self.handles_for_func.clear();
        }

        for (name, func) in sorted_iter(&self.needs_functions) {
            self.src.insert_str(
                pos,
                &format!(
                    "
                        let func = get_func(&_caller, \"{name}\")?;
                        let func_{name} = func.typed::<{cvt}>()?;
                    ",
                    name = name,
                    cvt = func.cvt(),
                ),
            );
            self.needs_get_func = true;
        }

        let closure = mem::replace(&mut self.src, prev);
        self.imports
            .entry(module.as_str().to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.as_str().to_string(),
                closure,
                trait_signature,
            });
        assert!(self.cleanup.is_none());
    }

    fn export(&mut self, module: &Id, func: &Function) {
        self.tmp = 0;
        let prev = mem::take(&mut self.src);
        self.is_dtor = self.types.is_dtor_func(&func.name);
        if self.is_dtor {
            assert_eq!(func.results.len(), 0, "destructors cannot have results");
        }
        self.params = self.print_docs_and_params(
            func,
            Visibility::Pub,
            false,
            true,
            TypeMode::AllBorrowed("'_"),
        );
        self.push_str("-> Result<");
        self.print_results(func);
        self.push_str(", wasmtime::Trap> {\n");
        let pos = self.src.len();
        func.call(module, CallMode::DeclaredExport, self);
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
            .entry(module.as_str().to_string())
            .or_insert_with(Exports::default);

        assert!(!self.needs_guest_memory);
        if self.needs_memory {
            self.needs_memory = false;
            self.src.insert_str(pos, "let memory = &self.memory;\n");
            exports.fields.insert(
                "memory".to_string(),
                (
                    "wasmtime::Memory".to_string(),
                    "get_memory(\"memory\")?".to_string(),
                ),
            );
            self.needs_get_memory = true;
        }
        assert!(self.handles_for_func.len() == 0);

        for (name, func) in self.needs_functions.drain() {
            self.src
                .insert_str(pos, &format!("let func_{0} = &self.{0};\n", name));
            let get = format!("instance.get_typed_func::<{}>(\"{}\")?", func.cvt(), name);
            exports.fields.insert(name, (func.ty(), get));
        }
        let func_body = mem::replace(&mut self.src, prev);
        if !self.is_dtor {
            exports.funcs.push(func_body);
        }

        // Create the code snippet which will define the type of this field in
        // the struct that we're exporting and additionally extracts the
        // function from an instantiated instance.
        let sig = func.wasm_signature(CallMode::DeclaredExport);
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
            func.name.as_str().to_string(),
            (
                format!("wasmtime::TypedFunc<{}>", cvt),
                format!(
                    "instance.get_typed_func::<{}>(\"{}\")?",
                    cvt,
                    func.name.as_str(),
                ),
            ),
        );
    }

    fn finish(&mut self, files: &mut Files) {
        for (module, funcs) in sorted_iter(&self.imports) {
            self.src.push_str("\npub trait ");
            self.src.push_str(&module.as_str().to_camel_case());
            self.src.push_str(": Sized {\n");
            if self.needs_borrow_checker || self.all_needed_handles.len() > 0 {
                if self.needs_borrow_checker {
                    self.src.push_str(
                        "fn borrow_checker(&self) -> &witx_bindgen_wasmtime::BorrowChecker;\n",
                    );
                }
                for handle in mem::take(&mut self.all_needed_handles) {
                    self.src.push_str("type ");
                    self.src.push_str(&handle.to_camel_case());
                    self.src.push_str(";\n");

                    self.src.push_str("fn ");
                    self.src.push_str(&handle.to_snake_case());
                    self.src
                        .push_str("_table(&self) -> &witx_bindgen_wasmtime::Table<Self::");
                    self.src.push_str(&handle.to_camel_case());
                    self.src.push_str(">;\n");
                }
            }
            for f in funcs {
                self.src.push_str(&f.trait_signature);
                self.src.push_str(";\n\n");
            }
            self.src.push_str("}\n");
        }

        let get_memory_from_caller = "
            fn get_memory(
                caller: &wasmtime::Caller<'_>,
                mem: &str,
            ) -> Result<wasmtime::Memory, wasmtime::Trap> {
                let mem = caller.get_export(mem)
                    .ok_or_else(|| {
                        let msg = format!(\"`{}` export not available\", mem);
                        wasmtime::Trap::new(msg)
                    })?
                    .into_memory()
                    .ok_or_else(|| {
                        let msg = format!(\"`{}` export not a memory\", mem);
                        wasmtime::Trap::new(msg)
                    })?;
                Ok(mem)
            }
        ";

        for (module, funcs) in mem::take(&mut self.imports) {
            self.push_str("\npub fn add_");
            self.push_str(module.as_str());
            self.push_str("_to_linker<T: ");
            self.push_str(&module.as_str().to_camel_case());
            self.push_str(" + 'static>(module: T, ");
            self.push_str("linker: &mut wasmtime::Linker) -> anyhow::Result<()> {\n");
            self.push_str("let module = std::rc::Rc::new(module);\n");
            if self.needs_get_memory {
                self.push_str(get_memory_from_caller);
            }
            if self.needs_get_func {
                self.push_str(
                    "
                        fn get_func(
                            caller: &wasmtime::Caller<'_>,
                            func: &str,
                        ) -> Result<wasmtime::Func, wasmtime::Trap> {
                            let func = caller.get_export(func)
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not available\", func);
                                    wasmtime::Trap::new(msg)
                                })?
                                .into_func()
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not a function\", func);
                                    wasmtime::Trap::new(msg)
                                })?;
                            Ok(func)
                        }
                    ",
                );
            }
            for f in funcs {
                self.push_str("let m = module.clone();\n");
                self.push_str(&format!(
                    "linker.func(\"{}\", \"{}\", {})?;\n",
                    module.as_str(),
                    f.name,
                    f.closure,
                ));
            }
            self.push_str("Ok(())\n}\n");
        }

        for (module, exports) in sorted_iter(&mem::take(&mut self.exports)) {
            let name = module.as_str().to_camel_case();
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
                "pub fn new(
                    module: &wasmtime::Module,
                    linker: &mut wasmtime::Linker,
                ) -> anyhow::Result<Self> {\n",
            );
            if self.needs_buffer_glue {
                self.push_str(get_memory_from_caller);
                self.push_str(
                    "
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
                                let memory = get_memory(&caller, \"memory\")?;
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
                                let memory = get_memory(&caller, \"memory\")?;
                                g.out_write(handle, &memory, offset, len)
                            },
                        )?;
                    ",
                );
            }
            self.push_str("let instance = linker.instantiate(module)?;\n");
            if self.needs_get_memory {
                self.push_str(
                    "
                        let get_memory = |mem: &str| -> anyhow::Result<_> {
                            let mem = instance.get_memory(mem)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(\"`{}` export not a memory\", mem)
                                })?;
                            Ok(mem)
                        };
                    ",
                );
            }
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
    }

    fn allocate_typed_space(&mut self, _ty: &NamedType) -> String {
        unimplemented!()
    }

    fn allocate_i64_array(&mut self, amt: usize) -> String {
        // TODO: this should be a stack allocation, not one that goes through
        // malloc/free. Using malloc/free is too heavyweight for this purpose.
        // It's not clear how we can get access to the wasm module's stack,
        // however...
        assert!(self.cleanup.is_none());
        let tmp = self.tmp();
        self.needs_functions
            .insert("witx_malloc".to_string(), NeededFunction::Malloc);
        self.needs_functions
            .insert("witx_free".to_string(), NeededFunction::Free);
        let ptr = format!("ptr{}", tmp);
        self.src.push_str(&format!(
            "let {} = self.witx_malloc.call(({} * 8, 8))?;\n",
            ptr, amt
        ));
        self.cleanup = Some(format!(
            "self.witx_free.call(({}, {} * 8, 8))?;\n",
            ptr, amt
        ));
        return ptr;
    }

    fn emit(
        &mut self,
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

            Instruction::I64FromU64 => top_as("i64"),
            Instruction::I32FromUsize
            | Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromChar8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32 => top_as("i32"),

            Instruction::F32FromIf32
            | Instruction::F64FromIf64
            | Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::I64FromS64
            | Instruction::I32FromS32
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
                self.all_needed_handles.insert(ty.name.as_str().to_string());
                results.push(format!(
                    "m.{}_table().insert({}) as i32",
                    ty.name.as_str().to_snake_case(),
                    operands[0]
                ));
            }
            Instruction::HandleBorrowedFromI32 { ty } => {
                if self.is_dtor {
                    self.all_needed_handles.insert(ty.name.as_str().to_string());
                    results.push(format!(
                        "m.{}_table().remove(({}) as u32).map_err(|e| {{
                            wasmtime::Trap::new(format!(\"failed to remove handle: {{}}\", e))
                        }})?",
                        ty.name.as_str().to_snake_case(),
                        operands[0]
                    ));
                } else {
                    self.handles_for_func.insert(ty.name.as_str().to_string());
                    results.push(format!(
                        "{}_table_access.get(({}) as u32).ok_or_else(|| {{
                            wasmtime::Trap::new(\"invalid handle index\")
                        }})?",
                        ty.name.as_str().to_snake_case(),
                        operands[0]
                    ));
                }
            }
            Instruction::I32FromBorrowedHandle { .. } => {
                results.push(format!("{}.0", operands[0]));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}({}{})",
                    ty.name.as_str().to_camel_case(),
                    operands[0],
                    if self.info(&ty.name).handle_with_dtor {
                        format!(
                            ", std::mem::ManuallyDrop::new(self.{}_close.clone())",
                            ty.name.as_str().to_snake_case()
                        )
                    } else {
                        String::new()
                    }
                ));
            }

            Instruction::I32FromBitflags { .. } => {
                results.push(format!("({}).bits as i32", operands[0]));
            }
            Instruction::I64FromBitflags { .. } => {
                results.push(format!("({}).bits as i64", operands[0]));
            }
            Instruction::BitflagsFromI32 { repr, name, .. }
            | Instruction::BitflagsFromI64 { repr, name, .. } => {
                self.needs_validate_flags = true;
                results.push(format!(
                    "validate_flags(
                        i64::from({}),
                        {name}::all().bits() as i64,
                        \"{name}\",
                        |b| {name} {{ bits: b as {ty} }}
                    )?",
                    operands[0],
                    name = name.name.as_str().to_camel_case(),
                    ty = int_repr(*repr),
                ));
            }

            Instruction::RecordLower { ty, name } => {
                self.record_lower(ty, *name, &operands[0], results);
            }
            Instruction::RecordLift { ty, name } => {
                self.record_lift(ty, *name, operands, results);
            }

            Instruction::VariantPayload => results.push("e".to_string()),

            Instruction::VariantLower { ty, name, nresults } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                self.variant_lower(ty, *name, *nresults, &operands[0], results, blocks);
            }

            Instruction::VariantLift { ty, name } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                let mut result = format!("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                for (i, (case, block)) in ty.cases.iter().zip(blocks).enumerate() {
                    result.push_str(&i.to_string());
                    result.push_str(" => ");
                    self.variant_lift_case(ty, *name, case, &block, &mut result);
                    result.push_str(",\n");
                }
                let variant_name = name.map(|s| s.name.as_str().to_camel_case());
                let variant_name = variant_name.as_deref().unwrap_or_else(|| {
                    if ty.is_bool() {
                        "bool"
                    } else if ty.as_expected().is_some() {
                        "Result"
                    } else if ty.as_option().is_some() {
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

            Instruction::ListCanonLower { element, malloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                //
                // Note that the size of a list of `char` is 1 because it's
                // encoded as utf-8, otherwise it's just normal contiguous array
                // elements.
                let malloc = malloc.unwrap();
                self.needs_functions
                    .insert(malloc.to_string(), NeededFunction::Malloc);
                let (size, align) = match &**element.type_() {
                    Type::Builtin(BuiltinType::Char) => (1, 1),
                    _ => {
                        let size = element.mem_size_align(!self.in_import);
                        (size.size, size.align)
                    }
                };

                // Store the operand into a temporary...
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then malloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!(
                    "let {} = func_{}.call((({}.len() as i32) * {}, {}))?;\n",
                    ptr, malloc, val, size, align
                ));

                // ... and then copy over the result.
                //
                // Note the unsafety here, in general it's not safe to copy
                // from arbitrary types on the host as a slice of bytes, but in
                // this case we should be able to get away with it since
                // canonical lowerings have the same memory representation on
                // the host as in the guest.
                self.push_str(&format!(
                    "store(memory, {}, unsafe {{ slice_as_bytes({}.as_ref()) }})?;\n",
                    ptr, val
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
                        let (stringify, align) = match &**element.type_() {
                            Type::Builtin(BuiltinType::Char) => (true, 1),
                            _ => (false, element.mem_size_align(!self.in_import).align),
                        };
                        let result = format!(
                            "
                                unsafe {{
                                    copy_slice(
                                        memory,
                                        func_{},
                                        {}, {}, {}
                                    )?
                                }}
                            ",
                            free, operands[0], operands[1], align,
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
                        self.needs_guest_memory = true;
                        results.push(format!(
                            "
                                unsafe {{
                                    witx_bindgen_wasmtime::GuestPtr::new(
                                        guest_memory,
                                        (({}) as u32, ({}) as u32),
                                    )
                                }}
                            ",
                            operands[0], operands[1]
                        ));
                    }
                }
            }

            Instruction::ListLower { element, malloc } => {
                let malloc = malloc.unwrap();
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                self.needs_functions
                    .insert(malloc.to_string(), NeededFunction::Malloc);
                let size_align = element.mem_size_align(!self.in_import);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));

                // ... then malloc space for the result in the guest module
                self.push_str(&format!(
                    "let {} = func_{}.call(({} * {}, {}))?;\n",
                    result, malloc, len, size_align.size, size_align.align,
                ));

                // ... then consume the vector and use the block to lower the
                // result.
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!(
                    "let base = {} + (i as i32) * {};\n",
                    result, size_align.size,
                ));
                self.push_str(&body);
                self.push_str("}");

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size_align = element.mem_size_align(!self.in_import);
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
                self.push_str(&size_align.size.to_string());
                self.push_str(";\n");
                self.push_str(&result);
                self.push_str(".push(");
                self.push_str(&body);
                self.push_str(");\n");
                self.push_str("}\n");
                results.push(result);

                if let Some(free) = free {
                    self.push_str(&format!(
                        "func_{}.call(({}, {} * {}, {}))?;\n",
                        free, base, len, size_align.size, size_align.align,
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

            Instruction::BufferLiftPtrLen { buffer } => {
                let block = self.blocks.pop().unwrap();
                self.needs_guest_memory = true;
                if buffer.tref.type_().all_bits_valid() {
                    results.push(format!(
                        "
                            unsafe {{
                                witx_bindgen_wasmtime::GuestPtr::new(
                                    guest_memory,
                                    (({}) as u32, ({}) as u32),
                                )
                            }}
                        ",
                        operands[0], operands[1]
                    ));
                } else {
                    let size = buffer.tref.mem_size_align(!self.in_import).size;
                    let tmp = self.tmp();
                    let closure = format!("closure{}", tmp);
                    self.closures.push_str(&format!("let {} = ", closure));
                    if buffer.out {
                        self.closures.push_str("|base: i32, e:");
                        mem::swap(&mut self.closures, &mut self.src);
                        self.print_tref(&buffer.tref, TypeMode::Owned);
                        mem::swap(&mut self.closures, &mut self.src);
                        self.closures.push_str("| {\n");
                        self.closures.push_str(&block);
                        self.closures.push_str("; Ok(()) };\n");
                        results.push(format!(
                            "witx_bindgen_wasmtime::imports::OutBuffer::new(guest_memory, {}, {}, {}, &{})",
                            operands[0], operands[1], size, closure
                        ));
                    } else {
                        self.closures.push_str("|base: i32| { Ok(");
                        self.closures.push_str(&block);
                        self.closures.push_str(") };\n");
                        results.push(format!(
                            "witx_bindgen_wasmtime::imports::InBuffer::new(guest_memory, {}, {}, {}, &{})",
                            operands[0], operands[1], size, closure
                        ));
                    }
                }
            }

            Instruction::BufferLowerHandle { buffer } => {
                let block = self.blocks.pop().unwrap();
                let size = buffer.tref.mem_size_align(!self.in_import).size;
                let tmp = self.tmp();
                let handle = format!("handle{}", tmp);
                let closure = format!("closure{}", tmp);
                self.needs_buffer_transaction = true;
                if buffer.tref.type_().all_bits_valid() {
                    let method = if buffer.out {
                        "push_out_raw"
                    } else {
                        "push_in_raw"
                    };
                    self.push_str(&format!(
                        "let {} = unsafe {{ buffer_transaction.{}({}) }};\n",
                        handle, method, operands[0],
                    ));
                } else if buffer.out {
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
                    self.print_tref(&buffer.tref, TypeMode::AllBorrowed("'_"));
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
                params: _,
                results: func_results,
            } => {
                if func_results.len() > 0 {
                    let tmp = self.tmp();
                    self.push_str("let (");
                    for i in 0..func_results.len() {
                        let arg = format!("result{}_{}", tmp, i);
                        self.push_str(&arg);
                        self.push_str(",");
                        results.push(arg);
                    }
                    self.push_str(") = ");
                }
                self.push_str("self.");
                self.push_str(name);
                self.push_str(".call((");
                for operand in operands {
                    self.push_str(operand);
                    self.push_str(", ");
                }
                self.push_str("))?;");
            }

            Instruction::CallInterface { module: _, func } => {
                self.let_results(func.results.len(), results);
                self.push_str("m.");
                self.push_str(func.name.as_str());
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
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
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "load(memory, {} + {}, [0u8; 4], i32::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load8U { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "i32::from(load(memory, {} + {}, [0u8; 1], u8::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load8S { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "i32::from(load(memory, {} + {}, [0u8; 1], i8::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load16U { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "i32::from(load(memory, {} + {}, [0u8; 2], u16::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load16S { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "i32::from(load(memory, {} + {}, [0u8; 2], i16::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I64Load { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "load(memory, {} + {}, [0u8; 8], i64::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::F32Load { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "load(memory, {} + {}, [0u8; 4], f32::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::F64Load { offset } => {
                self.needs_memory = true;
                self.needs_load = true;
                results.push(format!(
                    "load(memory, {} + {}, [0u8; 8], f64::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::I32Store { offset }
            | Instruction::I64Store { offset }
            | Instruction::F32Store { offset }
            | Instruction::F64Store { offset } => {
                self.needs_memory = true;
                self.needs_store = true;
                self.push_str(&format!(
                    "store(memory, {} + {}, &({}).to_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.needs_memory = true;
                self.needs_store = true;
                self.push_str(&format!(
                    "store(memory, {} + {}, &(({}) as u8).to_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.needs_memory = true;
                self.needs_store = true;
                self.push_str(&format!(
                    "store(memory, {} + {}, &(({}) as u16).to_le_bytes())?;\n",
                    operands[1], offset, operands[0]
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
            NeededFunction::Malloc => "(i32, i32), i32",
            NeededFunction::Free => "(i32, i32, i32), ()",
        }
    }

    fn ty(&self) -> String {
        match self {
            NeededFunction::Malloc => "wasmtime::TypedFunc<(i32, i32), i32>".to_string(),
            NeededFunction::Free => "wasmtime::TypedFunc<(i32, i32, i32), ()>".to_string(),
        }
    }
}

fn sorted_iter<K: Ord, V>(map: &HashMap<K, V>) -> impl Iterator<Item = (&K, &V)> {
    let mut list = map.into_iter().collect::<Vec<_>>();
    list.sort_by_key(|p| p.0);
    list.into_iter()
}
