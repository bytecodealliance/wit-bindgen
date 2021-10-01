use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use std::str::FromStr;
use wit_bindgen_gen_core::wit_parser::abi::{
    Abi, AbiVariant, Bindgen, Instruction, LiftLower, WasmType, WitxInstruction,
};
use wit_bindgen_gen_core::{wit_parser::*, Direction, Files, Generator, Source, TypeInfo, Types};
use wit_bindgen_gen_rust::{
    int_repr, to_rust_ident, wasm_type, FnSig, RustFunctionGenerator, RustGenerator, TypeMode,
};

#[derive(Default)]
pub struct Wasmer {
    src: Source,
    opts: Opts,
    needs_memory: bool,
    needs_functions: BTreeMap<String, NeededFunction>,
    needs_char_from_i32: bool,
    needs_invalid_variant: bool,
    needs_validate_flags: bool,
    needs_raw_mem: bool,
    needs_bad_int: bool,
    needs_copy_slice: bool,
    needs_buffer_glue: bool,
    needs_le: bool,
    needs_custom_error_to_trap: bool,
    needs_custom_error_to_types: BTreeSet<String>,
    all_needed_handles: BTreeSet<String>,
    exported_resources: BTreeSet<ResourceId>,
    types: Types,
    guest_imports: HashMap<String, Vec<Import>>,
    guest_exports: HashMap<String, Exports>,
    in_import: bool,
    in_trait: bool,
    trait_name: String,
    has_preview1_dtor: bool,
    sizes: SizeAlign,
}

enum NeededFunction {
    Realloc,
    Free,
}

struct Import {
    is_async: bool,
    name: String,
    trait_signature: String,
    closure: String,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, (String, String)>,
    funcs: Vec<String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub rustfmt: bool,

    /// Whether or not to emit `tracing` macro calls on function entry/exit.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub tracing: bool,

    /// Indicates which functions should be `async`: `all`, `none`, or a
    /// comma-separated list.
    #[cfg_attr(
        feature = "structopt",
        structopt(long = "async", default_value = "none")
    )]
    pub async_: Async,

    /// A flag to indicate that all trait methods in imports should return a
    /// custom trait-defined error. Applicable for import bindings.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub custom_error: bool,
}

#[derive(Debug, Clone)]
pub enum Async {
    None,
    All,
    Only(HashSet<String>),
}

impl Async {
    fn includes(&self, name: &str) -> bool {
        match self {
            Async::None => false,
            Async::All => true,
            Async::Only(list) => list.contains(name),
        }
    }

    fn is_none(&self) -> bool {
        match self {
            Async::None => true,
            _ => false,
        }
    }
}

impl Default for Async {
    fn default() -> Async {
        Async::None
    }
}

impl FromStr for Async {
    type Err = String;
    fn from_str(s: &str) -> Result<Async, String> {
        Ok(if s == "all" {
            Async::All
        } else if s == "none" {
            Async::None
        } else {
            Async::Only(s.split(',').map(|s| s.trim().to_string()).collect())
        })
    }
}

impl Opts {
    pub fn build(self) -> Wasmer {
        let mut r = Wasmer::new();
        r.opts = self;
        r
    }
}

enum FunctionRet {
    /// The function return is normal and needs to extra handling.
    Normal,
    /// The function return was wrapped in a `Result` in Rust. The `Ok` variant
    /// is the actual value that will be lowered, and the `Err`, if present,
    /// means that a trap has occurred.
    CustomToTrap,
    /// The function returns a `Result` in both wasm and in Rust, but the
    /// Rust error type is a custom error and must be converted to `err`. The
    /// `ok` variant payload is provided here too.
    CustomToError { ok: Option<Type>, err: String },
}

impl Wasmer {
    pub fn new() -> Wasmer {
        Wasmer::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the Wasmer host-side
        // bindings, we don't use any extra adapter layer between guest wasm
        // modules and the host. When the guest imports functions using the
        // `GuestImport` ABI, the host directly implements the `GuestImport`
        // ABI, even though the host is *exporting* functions. Similarly, when
        // the guest exports functions using the `GuestExport` ABI, the host
        // directly imports them with the `GuestExport` ABI, even though the
        // host is *importing* functions.
        match dir {
            Direction::Import => AbiVariant::GuestExport,
            Direction::Export => AbiVariant::GuestImport,
        }
    }

    fn print_intrinsics(&mut self) {
        if self.needs_raw_mem {
            self.push_str("use wit_bindgen_wasmer::rt::RawMem;\n");
        }
        if self.needs_char_from_i32 {
            self.push_str("use wit_bindgen_wasmer::rt::char_from_i32;\n");
        }
        if self.needs_invalid_variant {
            self.push_str("use wit_bindgen_wasmer::rt::invalid_variant;\n");
        }
        if self.needs_bad_int {
            self.push_str("use core::convert::TryFrom;\n");
            self.push_str("use wit_bindgen_wasmer::rt::bad_int;\n");
        }
        if self.needs_validate_flags {
            self.push_str("use wit_bindgen_wasmer::rt::validate_flags;\n");
        }
        if self.needs_le {
            self.push_str("use wit_bindgen_wasmer::Le;\n");
        }
        if self.needs_copy_slice {
            self.push_str("use wit_bindgen_wasmer::rt::copy_slice;\n");
        }
    }

    /// Classifies the return value of a function to see if it needs handling
    /// with respect to the `custom_error` configuration option.
    fn classify_fn_ret(&mut self, iface: &Interface, f: &Function) -> FunctionRet {
        if !self.opts.custom_error {
            return FunctionRet::Normal;
        }

        if f.results.len() != 1 {
            self.needs_custom_error_to_trap = true;
            return FunctionRet::CustomToTrap;
        }
        if let Type::Id(id) = &f.results[0].1 {
            if let TypeDefKind::Variant(v) = &iface.types[*id].kind {
                if let Some((ok, Some(err))) = v.as_expected() {
                    if let Type::Id(err) = err {
                        if let Some(name) = &iface.types[*err].name {
                            self.needs_custom_error_to_types.insert(name.clone());
                            return FunctionRet::CustomToError {
                                ok: ok.cloned(),
                                err: name.to_string(),
                            };
                        }
                    }
                }
            }
        }

        self.needs_custom_error_to_trap = true;
        FunctionRet::CustomToTrap
    }
}

impl RustGenerator for Wasmer {
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

    fn handle_wrapper(&self) -> Option<&'static str> {
        None
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
        if self.sizes.align(ty) > 1 && self.in_import {
            // If we're generating bindings for an import we ideally want to
            // hand out raw pointers into memory. We can't guarantee anything
            // about alignment in memory, though, so if the alignment
            // requirement is bigger than one then we have to use slices where
            // the type has a `Le<...>` wrapper.
            //
            // For exports we're generating functions that take values from
            // Rust, so we can assume alignment and use raw slices. For types
            // with an align of 1, then raw pointers are fine since Rust will
            // have the same alignment requirement.
            self.needs_le = true;
            self.push_str("&");
            if lifetime != "'_" {
                self.push_str(lifetime);
                self.push_str(" ");
            }
            if mutbl {
                self.push_str(" mut ");
            }
            self.push_str("[Le<");
            self.print_ty(iface, ty, TypeMode::AllBorrowed(lifetime));
            self.push_str(">]");
        } else {
            self.print_rust_slice(iface, mutbl, ty, lifetime);
        }
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str(" str");
    }

    fn print_lib_buffer(
        &mut self,
        iface: &Interface,
        push: bool,
        ty: &Type,
        mode: TypeMode,
        lt: &'static str,
    ) {
        if self.in_import {
            if let TypeMode::AllBorrowed(_) = mode {
                self.push_str("&");
                if lt != "'_" {
                    self.push_str(lt);
                }
                self.push_str(" mut ");
            }
            self.push_str(&format!(
                "wit_bindgen_wasmer::exports::{}Buffer<{}, ",
                if push { "Push" } else { "Pull" },
                lt,
            ));
            self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
            self.push_str(">");
        } else {
            if push {
                // Push buffers, where wasm pushes, are a `Vec` which is pushed onto
                self.push_str("&");
                if lt != "'_" {
                    self.push_str(lt);
                }
                self.push_str(" mut Vec<");
                self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                self.push_str(">");
            } else {
                // Pull buffers, which wasm pulls from, are modeled as iterators
                // in Rust.
                self.push_str("&");
                if lt != "'_" {
                    self.push_str(lt);
                }
                self.push_str(" mut (dyn ExactSizeIterator<Item = ");
                self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                self.push_str(">");
                if lt != "'_" {
                    self.push_str(" + ");
                    self.push_str(lt);
                }
                self.push_str(")");
            }
        }
    }
}

impl Generator for Wasmer {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.types.analyze(iface);
        self.in_import = variant == AbiVariant::GuestImport;
        self.trait_name = iface.name.to_camel_case();
        self.src
            .push_str(&format!("pub mod {} {{\n", iface.name.to_snake_case()));
        self.src
            .push_str("#[allow(unused_imports)]\nuse wit_bindgen_wasmer::{anyhow, wasmer};\n");
        self.sizes.fill(variant, iface);
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
                .push_str("wit_bindgen_wasmer::bitflags::bitflags! {\n");
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
                    field.name.to_shouty_snake_case(),
                    i,
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

        // If this record might be used as a slice type in various places then
        // we synthesize an `Endian` implementation for it so `&[Le<ThisType>]`
        // is usable.
        if self.modes_of(iface, id).len() > 0
            && record.fields.iter().all(|f| iface.all_bits_valid(&f.ty))
            && !record.is_tuple()
        {
            self.src.push_str("impl wit_bindgen_wasmer::Endian for ");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str(" {\n");

            self.src.push_str("fn into_le(self) -> Self {\n");
            self.src.push_str("Self {\n");
            for field in record.fields.iter() {
                self.src.push_str(&field.name.to_snake_case());
                self.src.push_str(": self.");
                self.src.push_str(&field.name.to_snake_case());
                self.src.push_str(".into_le(),\n");
            }
            self.src.push_str("}\n");
            self.src.push_str("}\n");

            self.src.push_str("fn from_le(self) -> Self {\n");
            self.src.push_str("Self {\n");
            for field in record.fields.iter() {
                self.src.push_str(&field.name.to_snake_case());
                self.src.push_str(": self.");
                self.src.push_str(&field.name.to_snake_case());
                self.src.push_str(".from_le(),\n");
            }
            self.src.push_str("}\n");
            self.src.push_str("}\n");

            self.src.push_str("}\n");

            // Also add an `AllBytesValid` valid impl since this structure's
            // byte representations are valid (guarded by the `all_bits_valid`
            // predicate).
            self.src
                .push_str("unsafe impl wit_bindgen_wasmer::AllBytesValid for ");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str(" {}\n");
        }
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

        self.exported_resources.insert(ty);

        // ... otherwise for exports we generate a newtype wrapper around an
        // `i32` to manage the resultt.
        let tyname = name.to_camel_case();
        self.rustdoc(&iface.resources[ty].docs);
        self.src.push_str("#[derive(Debug)]\n");
        self.src.push_str(&format!(
            "pub struct {}(wit_bindgen_wasmer::rt::ResourceIndex);\n",
            tyname
        ));
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
        self.src.push_str(";\n");
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push_str(";\n");
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

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let prev = mem::take(&mut self.src);

        let is_dtor = self.types.is_preview1_dtor_func(func);
        self.has_preview1_dtor = self.has_preview1_dtor || is_dtor;

        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        let params = (0..sig.params.len())
            .map(|i| format!("arg{}", i))
            .collect::<Vec<_>>();
        let mut f = FunctionBindgen::new(self, is_dtor, params);
        f.func_takes_all_memory = func.abi == Abi::Preview1
            && func
                .params
                .iter()
                .any(|(_, t)| iface.has_preview1_pointer(t));
        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            src,
            cleanup,
            needs_borrow_checker,
            needs_memory,
            needs_buffer_transaction,
            needs_functions,
            closures,
            async_intrinsic_called,
            func_takes_all_memory,
            ..
        } = f;
        assert!(cleanup.is_none());
        assert!(!needs_buffer_transaction);

        // Generate the signature this function will have in the final trait
        let mut self_arg = "&mut self".to_string();
        if func_takes_all_memory {
            self_arg.push_str(", mem: wit_bindgen_wasmer::RawMemory");
        }
        self.in_trait = true;

        let mut fnsig = FnSig::default();
        fnsig.private = true;
        fnsig.self_arg = Some(self_arg);
        self.print_docs_and_params(
            iface,
            func,
            if is_dtor {
                TypeMode::Owned
            } else {
                TypeMode::LeafBorrowed("'_")
            },
            &fnsig,
        );
        // The Rust return type may differ from the wasm return type based on
        // the `custom_error` configuration of this code generator.
        match self.classify_fn_ret(iface, func) {
            FunctionRet::Normal => {
                if func.results.len() > 0 {
                    self.push_str(" -> ");
                    self.print_results(iface, func);
                }
            }
            FunctionRet::CustomToTrap => {
                self.push_str(" -> Result<");
                self.print_results(iface, func);
                self.push_str(", Self::Error>");
            }
            FunctionRet::CustomToError { ok, .. } => {
                self.push_str(" -> Result<");
                match ok {
                    Some(ty) => self.print_ty(iface, &ty, TypeMode::Owned),
                    None => self.push_str("()"),
                }
                self.push_str(", Self::Error>");
            }
        }
        self.in_trait = false;
        let trait_signature = mem::take(&mut self.src).into();

        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        let result_ty = match &sig.results[..] {
            &[] => format!("()"),
            &[ty] => format!("{}", wasm_type(ty)),
            tys => format!(
                "({})",
                tys.iter()
                    .map(|&ty| wasm_type(ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        self.src
            .push_str("move |env: &std::sync::Arc<std::sync::Mutex<EnvWrapper<T>>>");
        for (i, param) in sig.params.iter().enumerate() {
            let arg = format!("arg{}", i);
            self.src.push_str(",");
            self.src.push_str(&arg);
            self.src.push_str(":");
            self.wasm_type(*param);
        }
        self.src.push_str(&format!(
            "| -> Result<{}, wasmer::RuntimeError> {{\n",
            result_ty
        ));

        // If an intrinsic was called asynchronously, which happens if anything
        // in the module could be asynchronous, then we must wrap this host
        // import with an async block. Otherwise if the function is itself
        // explicitly async then we must also wrap it in an async block.
        //
        // If none of that happens, then this is fine to be sync because
        // everything is sync.
        let is_async = if async_intrinsic_called || self.opts.async_.includes(&func.name) {
            self.src.push_str("Box::new(async move {\n");
            true
        } else {
            false
        };

        self.src.push_str("let env = &mut *env.lock().unwrap();\n");

        if self.opts.tracing {
            self.src.push_str(&format!(
                "
                    let span = wit_bindgen_wasmer::tracing::span!(
                        wit_bindgen_wasmer::tracing::Level::TRACE,
                        \"wit-bindgen abi\",
                        module = \"{}\",
                        function = \"{}\",
                    );
                    let _enter = span.enter();
                ",
                iface.name, func.name,
            ));
        }
        self.src.push_str(&closures);

        for name in needs_functions.keys() {
            self.src.push_str(&format!(
                "let func_{0} = env.func_{0}.get_ref().unwrap();\n",
                name
            ));
        }
        self.needs_functions.extend(needs_functions);
        self.needs_memory |= needs_memory || needs_borrow_checker;

        if needs_borrow_checker {
            // TODO: This isn't actually sound and should be replaced with use
            // of WasmPtr/WasmCell.
            self.src.push_str(
                "let mut _bc = unsafe { wit_bindgen_wasmer::BorrowChecker::new(env.memory.get_ref().unwrap().data_unchecked_mut()) };\n",
            );
        }

        self.src.push_str("let host = &mut env.data;\n");

        if self.all_needed_handles.len() > 0 {
            self.src.push_str("let _tables = &mut env.tables;\n");
        }

        self.src.push_str(&String::from(src));

        if is_async {
            self.src.push_str("})\n");
        }
        self.src.push_str("}");
        let closure = mem::replace(&mut self.src, prev).into();

        self.guest_imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                is_async,
                name: func.name.to_string(),
                closure,
                trait_signature,
            });
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "import" uses the "export" ABI variant on the inside of
    // this `Generator` implementation.
    fn import(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let prev = mem::take(&mut self.src);

        // If anything is asynchronous on exports then everything must be
        // asynchronous, we can't intermix async and sync calls because
        // it's unknown whether the wasm module will make an async host call.
        let is_async = !self.opts.async_.is_none();
        let mut sig = FnSig::default();
        sig.async_ = is_async;
        sig.self_arg = Some("&self".to_string());
        self.print_docs_and_params(iface, func, TypeMode::AllBorrowed("'_"), &sig);
        self.push_str("-> Result<");
        self.print_results(iface, func);
        self.push_str(", wasmer::RuntimeError> {\n");

        let is_dtor = self.types.is_preview1_dtor_func(func);
        if is_dtor {
            assert_eq!(func.results.len(), 0, "destructors cannot have results");
        }
        let params = func
            .params
            .iter()
            .map(|(name, _)| to_rust_ident(name).to_string())
            .collect();
        let mut f = FunctionBindgen::new(self, is_dtor, params);
        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_memory,
            src,
            needs_borrow_checker,
            needs_buffer_transaction,
            closures,
            needs_functions,
            ..
        } = f;

        let exports = self
            .guest_exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);

        for (name, func) in needs_functions {
            self.src
                .push_str(&format!("let func_{0} = &self.func_{0};\n", name));
            let get = format!(
                "instance.exports.get_native_function::<{}>(\"{}\")?",
                func.cvt(),
                name
            );
            exports
                .fields
                .insert(format!("func_{}", name), (func.ty(), get));
        }

        self.src.push_str(&closures);

        assert!(!needs_borrow_checker);
        if needs_memory {
            self.src.push_str("let memory = &self.memory;\n");
            exports.fields.insert(
                "memory".to_string(),
                (
                    "wasmer::Memory".to_string(),
                    "instance.exports.get_memory(\"memory\")?.clone()".to_string(),
                ),
            );
        }

        if needs_buffer_transaction {
            self.needs_buffer_glue = true;
            self.src
                .push_str("let mut buffer_transaction = self.buffer_glue.transaction();\n");
        }

        self.src.push_str(&String::from(src));
        self.src.push_str("}\n");
        let func_body = mem::replace(&mut self.src, prev);
        if !is_dtor {
            exports.funcs.push(func_body.into());
        }

        // Create the code snippet which will define the type of this field in
        // the struct that we're exporting and additionally extracts the
        // function from an instantiated instance.
        let sig = iface.wasm_signature(AbiVariant::GuestExport, func);
        let mut cvt = String::new();
        if sig.params.len() == 1 {
            cvt.push_str(wasm_type(sig.params[0]));
        } else {
            cvt.push_str("(");
            for param in sig.params.iter() {
                cvt.push_str(wasm_type(*param));
                cvt.push_str(",");
            }
            cvt.push_str(")");
        }
        cvt.push_str(", ");
        if sig.results.len() == 1 {
            cvt.push_str(wasm_type(sig.results[0]));
        } else {
            cvt.push_str("(");
            for result in sig.results.iter() {
                cvt.push_str(wasm_type(*result));
                cvt.push_str(",");
            }
            cvt.push_str(")");
        }
        exports.fields.insert(
            format!("func_{}", to_rust_ident(&func.name)),
            (
                format!("wasmer::NativeFunc<{}>", cvt),
                format!(
                    "instance.exports.get_native_function::<{}>(\"{}\")?",
                    cvt, func.name,
                ),
            ),
        );
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        for (module, funcs) in sorted_iter(&self.guest_imports) {
            let module_camel = module.to_camel_case();
            let is_async = !self.opts.async_.is_none();
            if is_async {
                self.src.push_str("#[wit_bindgen_wasmer::async_trait]\n");
            }
            self.src.push_str("pub trait ");
            self.src.push_str(&module_camel);
            self.src.push_str(": Sized + wasmer::WasmerEnv + 'static");
            if is_async {
                self.src.push_str(" + Send");
            }
            self.src.push_str("{\n");
            if self.all_needed_handles.len() > 0 {
                for handle in self.all_needed_handles.iter() {
                    self.src.push_str("type ");
                    self.src.push_str(&handle.to_camel_case());
                    self.src.push_str(": std::fmt::Debug");
                    if is_async {
                        self.src.push_str(" + Send + Sync");
                    }
                    self.src.push_str(";\n");
                }
            }
            if self.opts.custom_error {
                self.src.push_str("type Error;\n");
                if self.needs_custom_error_to_trap {
                    self.src.push_str(
                        "fn error_to_trap(&mut self, err: Self::Error) -> wasmer::RuntimeError;\n",
                    );
                }
                for ty in self.needs_custom_error_to_types.iter() {
                    self.src.push_str(&format!(
                        "fn error_to_{}(&mut self, err: Self::Error) -> Result<{}, wasmer::RuntimeError>;\n",
                        ty.to_snake_case(),
                        ty.to_camel_case(),
                    ));
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
                    }}\n",
                    handle.to_snake_case(),
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
                    self.src.push_str("pub(crate) ");
                    self.src.push_str(&handle.to_snake_case());
                    self.src.push_str("_table: wit_bindgen_wasmer::Table<T::");
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
                self.src.push_str("impl<T: ");
                self.src.push_str(&module_camel);
                self.src.push_str("> Clone for ");
                self.src.push_str(&module_camel);
                self.src.push_str("Tables<T> {\n");
                self.src.push_str("fn clone(&self) -> Self {\n");
                self.src.push_str("Self::default()\n");
                self.src.push_str("}}\n");
            }
        }

        for (module, funcs) in mem::take(&mut self.guest_imports) {
            let module_camel = module.to_camel_case();
            self.push_str("\npub fn add_to_imports<T>(store: &wasmer::Store, imports: &mut wasmer::ImportObject, data: T)\n");
            self.push_str("where T: ");
            self.push_str(&module_camel);
            self.push_str("\n{\n");

            self.push_str("#[derive(Clone)]");
            self.push_str("struct EnvWrapper<T: ");
            self.push_str(&module_camel);
            self.push_str("> {\n");
            self.push_str("data: T,\n");
            if self.all_needed_handles.len() > 0 {
                self.push_str("tables: ");
                self.push_str(&module_camel);
                self.push_str("Tables<T>,\n");
            }
            if self.needs_memory {
                self.push_str("memory: wasmer::LazyInit<wasmer::Memory>,\n");
            }
            for (name, func) in &self.needs_functions {
                self.src.push_str(&format!(
                    "func_{name}: wasmer::LazyInit<wasmer::NativeFunc<{cvt}>>,\n",
                    name = name,
                    cvt = func.cvt(),
                ));
            }
            self.push_str("}\n");
            self.push_str("unsafe impl<T: ");
            self.push_str(&module_camel);
            self.push_str("> Send for EnvWrapper<T> {}\n");
            self.push_str("unsafe impl<T: ");
            self.push_str(&module_camel);
            self.push_str("> Sync for EnvWrapper<T> {}\n");
            self.push_str("impl<T: ");
            self.push_str(&module_camel);
            self.push_str("> wasmer::WasmerEnv for EnvWrapper<T> {\n");
            self.push_str("fn init_with_instance(&mut self, instance: &wasmer::Instance) -> Result<(), wasmer::HostEnvInitError>{\n");
            self.push_str("self.data.init_with_instance(instance)?;");
            if self.needs_memory {
                self.push_str("self.memory.initialize(instance.exports.get_with_generics_weak(\"memory\")?);\n");
            }
            for name in self.needs_functions.keys() {
                self.src.push_str(&format!(
                    "self.func_{name}.initialize(instance.exports.get_with_generics_weak(\"{name}\")?);\n",
                    name = name
                ));
            }
            self.push_str("Ok(())");
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("let env = std::sync::Arc::new(std::sync::Mutex::new(EnvWrapper {\n");
            self.push_str("data,\n");
            if self.all_needed_handles.len() > 0 {
                self.push_str("tables: ");
                self.push_str(&module_camel);
                self.push_str("Tables::default(),\n");
            }
            if self.needs_memory {
                self.push_str("memory: wasmer::LazyInit::new(),\n");
            }
            for name in self.needs_functions.keys() {
                self.src
                    .push_str(&format!("func_{}: wasmer::LazyInit::new(),\n", name,));
            }
            self.push_str("}));\n");

            self.push_str("let mut exports = wasmer::Exports::new();\n");
            for f in funcs {
                if f.is_async {
                    unimplemented!();
                }
                self.push_str(&format!(
                    "exports.insert(\"{}\", wasmer::Function::new_native_with_env(store, env.clone(), {}));\n",
                    f.name, f.closure,
                ));
            }
            self.push_str(&format!("imports.register(\"{}\", exports);\n", module));

            if !self.has_preview1_dtor && !self.all_needed_handles.is_empty() {
                self.push_str("let mut canonical_abi = imports.get_namespace_exports(\"canonical_abi\").unwrap_or_else(wasmer::Exports::new);\n");
                for handle in self.all_needed_handles.iter() {
                    self.src.push_str(&format!(
                        "canonical_abi.insert(
                            \"resource_drop_{name}\",
                            wasmer::Function::new_native_with_env(store, env.clone(),
                                move |env: &std::sync::Arc<std::sync::Mutex<EnvWrapper<T>>>, handle: u32| -> Result<(), wasmer::RuntimeError> {{
                                    let env = &mut *env.lock().unwrap();
                                    let handle = env
                                        .tables
                                        .{snake}_table
                                        .remove(handle)
                                        .map_err(|e| {{
                                            wasmer::RuntimeError::new(format!(\"failed to remove handle: {{}}\", e))
                                        }})?;
                                    env.data.drop_{snake}(handle);
                                    Ok(())
                                }}
                            )
                        );\n",
                        name = handle,
                        snake = handle.to_snake_case(),
                    ));
                }
                self.push_str("imports.register(\"canonical_abi\", canonical_abi);\n");
            }
            self.push_str("}\n");
        }

        for (module, exports) in sorted_iter(&mem::take(&mut self.guest_exports)) {
            let name = module.to_camel_case();

            // Generate a struct that is the "state" of this exported module
            // which is held internally.
            self.push_str(
                "
                /// Auxiliary data associated with the wasm exports.
                ",
            );
            self.push_str("#[derive(Default)]\n");
            self.push_str("pub struct ");
            self.push_str(&name);
            self.push_str("Data {\n");
            for r in self.exported_resources.iter() {
                self.src.push_str(&format!(
                    "
                        index_slab{idx}: wit_bindgen_wasmer::rt::IndexSlab,
                        resource_slab{idx}: wit_bindgen_wasmer::rt::ResourceSlab,
                        dtor{idx}: wasmer::LazyInit<wasmer::NativeFunc<i32, ()>>,
                    ",
                    idx = r.index(),
                ));
            }
            self.push_str("}\n");
            self.push_str("impl wasmer::WasmerEnv for ");
            self.push_str(&name);
            self.push_str("Data {\n");
            self.push_str("fn init_with_instance(&mut self, instance: &wasmer::Instance) -> Result<(), wasmer::HostEnvInitError>{\n");
            self.push_str("let _ = instance;\n");
            for r in self.exported_resources.iter() {
                self.src.push_str(&format!(
                    "self.dtor{idx}.initialize(instance.exports.get_with_generics_weak(\"canonical_abi_drop_{name}\")?);\n",
                    idx = r.index(),
                    name = iface.resources[*r].name,
                ));
            }
            self.push_str("Ok(())");
            self.push_str("}\n");
            self.push_str("}\n");
            self.src.push_str("impl Clone for ");
            self.src.push_str(&name);
            self.src.push_str("Data {\n");
            self.src.push_str("fn clone(&self) -> Self {\n");
            self.src.push_str("Self::default()\n");
            self.src.push_str("}}\n");

            self.push_str("pub struct ");
            self.push_str(&name);
            self.push_str(" {\n");
            self.push_str(&format!(
                "state: std::sync::Arc<std::sync::Mutex<{}Data>>,\n",
                name
            ));
            for (name, (ty, _)) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(": ");
                self.push_str(ty);
                self.push_str(",\n");
            }
            // if self.needs_buffer_glue {
            //     self.push_str("buffer_glue: wit_bindgen_wasmer::imports::BufferGlue,");
            // }
            self.push_str("}\n");
            self.push_str(&format!("impl {} {{\n", name));

            if self.exported_resources.len() == 0 {
                self.push_str("#[allow(unused_variables)]\n");
            }
            self.push_str(&format!(
                "
                    /// Adds any intrinsics, if necessary for this exported wasm
                    /// functionality to the `ImportObject` provided.
                    ///
                    /// This function returns the `{0}Data` which needs to be
                    /// passed through to `{0}::new`.
                    fn add_to_imports(
                        store: &wasmer::Store,
                        imports: &mut wasmer::ImportObject,
                    ) -> std::sync::Arc<std::sync::Mutex<{0}Data>> {{
                ",
                name,
            ));
            self.push_str(
                "let state = std::sync::Arc::new(std::sync::Mutex::new(Default::default()));\n",
            );
            if !self.all_needed_handles.is_empty() {
                self.push_str("let mut canonical_abi = imports.get_namespace_exports(\"canonical_abi\").unwrap_or_else(wasmer::Exports::new);\n");
                for r in self.exported_resources.iter() {
                    if !self.opts.async_.is_none() {
                        unimplemented!();
                    }
                    self.src.push_str(&format!(
                        "
                        canonical_abi.insert(
                            \"resource_drop_{resource}\",
                            wasmer::Function::new_native_with_env(store, state.clone(),
                                move |env: &std::sync::Arc<std::sync::Mutex<{name}Data>>, idx: u32| -> Result<(), wasmer::RuntimeError> {{
                                    let state = &mut *env.lock().unwrap();
                                    let resource_idx = state.index_slab{idx}.remove(idx)?;
                                    let wasm = match state.resource_slab{idx}.drop(resource_idx) {{
                                        Some(wasm) => wasm,
                                        None => return Ok(()),
                                    }};
                                    let dtor = state.dtor{idx}.get_ref().unwrap();
                                    dtor.call(wasm)?;
                                    Ok(())
                                }},
                            )
                        );
                        canonical_abi.insert(
                            \"resource_clone_{resource}\",
                            wasmer::Function::new_native_with_env(store, state.clone(),
                                move |env: &std::sync::Arc<std::sync::Mutex<{name}Data>>, idx: u32| -> Result<u32, wasmer::RuntimeError>  {{
                                    let state = &mut *env.lock().unwrap();
                                    let resource_idx = state.index_slab{idx}.get(idx)?;
                                    state.resource_slab{idx}.clone(resource_idx)?;
                                    Ok(state.index_slab{idx}.insert(resource_idx))
                                }},
                            )
                        );
                        canonical_abi.insert(
                            \"resource_get_{resource}\",
                            wasmer::Function::new_native_with_env(store, state.clone(),
                                move |env: &std::sync::Arc<std::sync::Mutex<{name}Data>>, idx: u32| -> Result<i32, wasmer::RuntimeError>  {{
                                    let state = &mut *env.lock().unwrap();
                                    let resource_idx = state.index_slab{idx}.get(idx)?;
                                    Ok(state.resource_slab{idx}.get(resource_idx))
                                }},
                            )
                        );
                        canonical_abi.insert(
                            \"resource_new_{resource}\",
                            wasmer::Function::new_native_with_env(store, state.clone(),
                                move |env: &std::sync::Arc<std::sync::Mutex<{name}Data>>, val: i32| -> Result<u32, wasmer::RuntimeError>  {{
                                    let state = &mut *env.lock().unwrap();
                                    let resource_idx = state.resource_slab{idx}.insert(val);
                                    Ok(state.index_slab{idx}.insert(resource_idx))
                                }},
                            )
                        );
                    ",
                        name = name,
                        resource = iface.resources[*r].name,
                        idx = r.index(),
                    ));
                }
                self.push_str("imports.register(\"canonical_abi\", canonical_abi);\n");
            }
            // if self.needs_buffer_glue {
            //     self.push_str(
            //         "
            //             use wit_bindgen_wasmer::rt::get_memory;

            //             let buffer_glue = wit_bindgen_wasmer::imports::BufferGlue::default();
            //             let g = buffer_glue.clone();
            //             linker.func(
            //                 \"wit_canonical_buffer_abi\",
            //                 \"in_len\",
            //                 move |handle: u32| g.in_len(handle),
            //             )?;
            //             let g = buffer_glue.clone();
            //             linker.func(
            //                 \"wit_canonical_buffer_abi\",
            //                 \"in_read\",
            //                 move |caller: wasmtime::Caller<'_>, handle: u32, len: u32, offset: u32| {
            //                     let memory = get_memory(&mut caller, \"memory\")?;
            //                     g.in_read(handle, &memory, offset, len)
            //                 },
            //             )?;
            //             let g = buffer_glue.clone();
            //             linker.func(
            //                 \"wit_canonical_buffer_abi\",
            //                 \"out_len\",
            //                 move |handle: u32| g.out_len(handle),
            //             )?;
            //             let g = buffer_glue.clone();
            //             linker.func(
            //                 \"wit_canonical_buffer_abi\",
            //                 \"out_write\",
            //                 move |caller: wasmtime::Caller<'_>, handle: u32, len: u32, offset: u32| {
            //                     let memory = get_memory(&mut caller, \"memory\")?;
            //                     g.out_write(handle, &memory, offset, len)
            //                 },
            //             )?;
            //         ",
            //     );
            // }
            self.push_str("state\n");
            self.push_str("}\n");

            if !self.opts.async_.is_none() {
                unimplemented!();
            }
            self.push_str(&format!(
                "
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    ///
                    /// The `imports` provided will have intrinsics added to it
                    /// automatically, so it's not necessary to call
                    /// `add_to_imports` beforehand. This function will
                    /// instantiate the `module` otherwise using `imports`, and
                    /// both an instance of this structure and the underlying
                    /// `wasmer::Instance` will be returned.
                    pub fn instantiate(
                        store: &wasmer::Store,
                        module: &wasmer::Module,
                        imports: &mut wasmer::ImportObject,
                    ) -> anyhow::Result<(Self, wasmer::Instance)> {{
                        let state = Self::add_to_imports(store, imports);
                        let instance = wasmer::Instance::new(module, &*imports)?;
                        Ok((Self::new(&instance, state)?, instance))
                    }}
                ",
            ));

            self.push_str(&format!(
                "
                    /// Low-level creation wrapper for wrapping up the exports
                    /// of the `instance` provided in this structure of wasm
                    /// exports.
                    ///
                    /// This function will extract exports from the `instance`
                    /// and wrap them all up in the returned structure which can
                    /// be used to interact with the wasm module.
                    pub fn new(
                        instance: &wasmer::Instance,
                        state: std::sync::Arc<std::sync::Mutex<{}Data>>,
                    ) -> Result<Self, wasmer::ExportError> {{
                ",
                name,
            ));
            //assert!(!self.needs_get_func);
            for (name, (_, get)) in exports.fields.iter() {
                self.push_str("let ");
                self.push_str(&name);
                self.push_str("= ");
                self.push_str(&get);
                self.push_str(";\n");
            }
            self.push_str("Ok(");
            self.push_str(&name);
            self.push_str("{\n");
            for (name, _) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(",\n");
            }
            self.push_str("state,\n");
            self.push_str("\n})\n");
            self.push_str("}\n");

            for func in exports.funcs.iter() {
                self.push_str(func);
            }

            for r in self.exported_resources.iter() {
                if !self.opts.async_.is_none() {
                    unimplemented!();
                }
                self.src.push_str(&format!(
                    "
                        /// Drops the host-owned handle to the resource
                        /// specified.
                        ///
                        /// Note that this may execute the WebAssembly-defined
                        /// destructor for this type. This also may not run
                        /// the destructor if there are still other references
                        /// to this type.
                        pub fn drop_{name_snake}(
                            &self,
                            val: {name_camel},
                        ) -> Result<(), wasmer::RuntimeError> {{
                            let data = &mut *self.state.lock().unwrap();
                            let wasm = match data.resource_slab{idx}.drop(val.0) {{
                                Some(val) => val,
                                None => return Ok(()),
                            }};
                            data.dtor{idx}.get_ref().unwrap().call(wasm)?;
                            Ok(())
                        }}
                    ",
                    name_snake = iface.resources[*r].name.to_snake_case(),
                    name_camel = iface.resources[*r].name.to_camel_case(),
                    idx = r.index(),
                ));
            }

            self.push_str("}\n");
        }
        self.print_intrinsics();

        // Close the opening `mod`.
        self.push_str("}\n");

        let mut src = mem::take(&mut self.src);
        if self.opts.rustfmt {
            let mut child = Command::new("rustfmt")
                .arg("--edition=2018")
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
            src.as_mut_string().truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(src.as_mut_string())
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }

        files.push("bindings.rs", src.as_bytes());
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut Wasmer,

    // Number used to assign unique names to temporary variables.
    tmp: usize,

    // Destination where source code is pushed onto for this function
    src: Source,

    // Whether or not this function is a preview1 dtor
    is_dtor: bool,

    // The named parameters that are available to this function
    params: Vec<String>,

    // Management of block scopes used by `Bindgen`.
    block_storage: Vec<Source>,
    blocks: Vec<String>,

    // Whether or not the code generator is after the invocation of wasm or the
    // host, used for knowing where to acquire memory from.
    after_call: bool,
    // Whether or not the `caller_memory` variable has been defined and is
    // available for use.
    caller_memory_available: bool,
    // Whether or not a helper function was called in an async fashion. If so
    // and this is an import, then the import must be defined asynchronously as
    // well.
    async_intrinsic_called: bool,
    // Code that must be executed before a return, generated during instruction
    // lowering.
    cleanup: Option<String>,
    // Only present for preview1 ABIs where some arguments might be a `pointer`
    // type.
    func_takes_all_memory: bool,

    // Rust clousures for buffers that must be placed at the front of the
    // function.
    closures: Source,

    // Various intrinsic properties this function's codegen required, must be
    // satisfied in the function header if any are set.
    needs_buffer_transaction: bool,
    needs_borrow_checker: bool,
    needs_memory: bool,
    needs_functions: HashMap<String, NeededFunction>,
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut Wasmer, is_dtor: bool, params: Vec<String>) -> FunctionBindgen<'_> {
        FunctionBindgen {
            gen,
            block_storage: Vec::new(),
            blocks: Vec::new(),
            src: Source::default(),
            after_call: false,
            caller_memory_available: false,
            async_intrinsic_called: false,
            tmp: 0,
            cleanup: None,
            func_takes_all_memory: false,
            closures: Source::default(),
            needs_buffer_transaction: false,
            needs_borrow_checker: false,
            needs_memory: false,
            needs_functions: HashMap::new(),
            is_dtor,
            params,
        }
    }

    fn memory_src(&mut self) -> String {
        if self.gen.in_import {
            if !self.after_call {
                // Before calls we use `_bc` which is a borrow checker used for
                // getting long-lasting borrows into memory.
                self.needs_borrow_checker = true;
                return format!("_bc");
            }

            if !self.caller_memory_available {
                self.needs_memory = true;
                self.caller_memory_available = true;
                self.push_str("let caller_memory = unsafe { env.memory.get_ref().unwrap().data_unchecked_mut() };\n");
            }
            format!("caller_memory")
        } else {
            self.needs_memory = true;
            format!("unsafe {{ memory.data_unchecked_mut() }}")
        }
    }

    fn call_intrinsic(&mut self, name: &str, args: String) {
        if !self.gen.opts.async_.is_none() {
            self.async_intrinsic_called = true;
            unimplemented!();
        };
        self.push_str(&format!("func_{}.call({})?;\n", name, args));
        self.caller_memory_available = false; // invalidated by call
    }

    fn type_string(&mut self, iface: &Interface, ty: &Type, mode: TypeMode) -> String {
        let start = self.gen.src.len();
        self.gen.print_ty(iface, ty, mode);
        let ty = self.gen.src[start..].to_string();
        self.gen.src.as_mut_string().truncate(start);
        ty
    }

    fn load(&mut self, offset: i32, ty: &str, operands: &[String]) -> String {
        let mem = self.memory_src();
        self.gen.needs_raw_mem = true;
        let tmp = self.tmp();
        self.push_str(&format!(
            "let load{} = {}.load::<{}>({} + {})?;\n",
            tmp, mem, ty, operands[0], offset
        ));
        format!("load{}", tmp)
    }

    fn store(&mut self, offset: i32, method: &str, extra: &str, operands: &[String]) {
        let mem = self.memory_src();
        self.gen.needs_raw_mem = true;
        self.push_str(&format!(
            "{}.store({} + {}, wit_bindgen_wasmer::rt::{}({}){})?;\n",
            mem, operands[1], offset, method, operands[0], extra
        ));
    }
}

impl RustFunctionGenerator for FunctionBindgen<'_> {
    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn rust_gen(&self) -> &dyn RustGenerator {
        self.gen
    }

    fn lift_lower(&self) -> LiftLower {
        if self.gen.in_import {
            LiftLower::LiftArgsLowerResults
        } else {
            LiftLower::LowerArgsLiftResults
        }
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
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
            self.blocks.push(format!("{{\n{}}}", &src[..]));
        } else {
            self.blocks.push(format!("{{\n{}{}\n}}", &src[..], expr));
        }
        self.caller_memory_available = false;
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> String {
        unimplemented!()
    }

    fn i64_return_pointer_area(&mut self, _amt: usize) -> String {
        unimplemented!()
    }

    fn is_list_canonical(&self, iface: &Interface, ty: &Type) -> bool {
        iface.all_bits_valid(ty)
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
            self.gen.needs_bad_int = true;
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
                results.push(format!("wit_bindgen_wasmer::rt::as_i64({})", s));
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
                results.push(format!("wit_bindgen_wasmer::rt::as_i32({})", s));
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
                self.gen.needs_char_from_i32 = true;
                results.push(format!("char_from_i32({})?", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                wit_bindgen_gen_rust::bitcast(casts, operands, results)
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
                            wasmer::RuntimeError::new(format!(\"failed to remove handle: {{}}\", e))
                        }})?",
                        name.to_snake_case(),
                        operands[0]
                    ));
                } else {
                    results.push(format!(
                        "_tables.{}_table.get(({}) as u32).ok_or_else(|| {{
                            wasmer::RuntimeError::new(\"invalid handle index\")
                        }})?",
                        name.to_snake_case(),
                        operands[0]
                    ));
                }
            }
            Instruction::I32FromBorrowedHandle { ty } => {
                let tmp = self.tmp();
                self.push_str(&format!(
                    "
                        let obj{tmp} = {op};
                        let handle{tmp} = {{
                            let state = &mut *self.state.lock().unwrap();
                            state.resource_slab{idx}.clone(obj{tmp}.0)?;
                            state.index_slab{idx}.insert(obj{tmp}.0)
                        }};
                    ",
                    tmp = tmp,
                    idx = ty.index(),
                    op = operands[0],
                ));

                results.push(format!("handle{} as i32", tmp,));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                let tmp = self.tmp();
                self.push_str(&format!(
                    "let handle{} = self.state.lock().unwrap().index_slab{}.remove({} as u32)?;\n",
                    tmp,
                    ty.index(),
                    operands[0],
                ));

                let name = iface.resources[*ty].name.to_camel_case();
                results.push(format!("{}(handle{})", name, tmp));
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
                self.gen.needs_validate_flags = true;
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

            Instruction::VariantPayloadName => results.push("e".to_string()),
            Instruction::BufferPayloadName => results.push("e".to_string()),

            Instruction::VariantLower {
                variant,
                results: result_types,
                ty,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                self.variant_lower(
                    iface,
                    *ty,
                    variant,
                    result_types.len(),
                    &operands[0],
                    results,
                    blocks,
                );
            }

            Instruction::VariantLift { variant, name, ty } => {
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
                    self.variant_lift_case(iface, *ty, variant, case, &block, &mut result);
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
                self.gen.needs_invalid_variant = true;
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
                    _ => (self.gen.sizes.size(element), self.gen.sizes.align(element)),
                };

                // Store the operand into a temporary...
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then realloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!("let {} = ", ptr));
                self.call_intrinsic(
                    realloc,
                    format!("0, 0, {}, ({}.len() as i32) * {}", align, val, size),
                );

                // ... and then copy over the result.
                let mem = self.memory_src();
                self.push_str(&format!(
                    "{}.store_many({}, {}.as_ref())?;\n",
                    mem, ptr, val
                ));
                self.gen.needs_raw_mem = true;
                self.needs_memory = true;
                results.push(ptr);
                results.push(format!("{}.len() as i32", val));
            }

            Instruction::ListCanonLift { element, free, .. } => match free {
                Some(free) => {
                    self.needs_memory = true;
                    self.gen.needs_copy_slice = true;
                    self.needs_functions
                        .insert(free.to_string(), NeededFunction::Free);
                    let (stringify, align) = match element {
                        Type::Char => (true, 1),
                        _ => (false, self.gen.sizes.align(element)),
                    };
                    let tmp = self.tmp();
                    self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                    self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                    let result = format!(
                        "
                                copy_slice(
                                    memory,
                                    func_{},
                                    ptr{tmp}, len{tmp}, {}
                                )?
                            ",
                        free,
                        align,
                        tmp = tmp
                    );
                    if stringify {
                        results.push(format!(
                            "String::from_utf8({})
                                    .map_err(|_| wasmer::RuntimeError::new(\"invalid utf-8\"))?",
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
                    let slice = format!("_bc.{}(ptr{1}, len{1})?", method, tmp);
                    results.push(slice);
                }
            },

            Instruction::ListLower { element, realloc } => {
                let realloc = realloc.unwrap();
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));

                // ... then realloc space for the result in the guest module
                self.push_str(&format!("let {} = ", result));
                self.call_intrinsic(realloc, format!("0, 0, {}, {} * {}", align, len, size));

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

            Instruction::ListLift { element, free, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
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
                    self.call_intrinsic(free, format!("{}, {} * {}, {}", base, len, size, align));
                    self.needs_functions
                        .insert(free.to_string(), NeededFunction::Free);
                }
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

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
                    results.push(format!("_bc.{}(ptr{1}, len{1})?", method, tmp));
                } else {
                    let size = self.gen.sizes.size(ty);
                    let closure = format!("closure{}", tmp);
                    self.closures.push_str(&format!("let {} = ", closure));
                    if *push {
                        self.closures.push_str("|_bc: &mut [u8], e:");
                        let ty = self.type_string(iface, ty, TypeMode::Owned);
                        self.closures.push_str(&ty);
                        self.closures.push_str("| {let base = 0;\n");
                        self.closures.push_str(&block);
                        self.closures.push_str("; Ok(()) };\n");
                        results.push(format!(
                            "wit_bindgen_wasmer::exports::PushBuffer::new(
                                &mut _bc, ptr{}, len{}, {}, &{})?",
                            tmp, tmp, size, closure
                        ));
                    } else {
                        self.closures.push_str("|_bc: &[u8]| { let base = 0;Ok(");
                        self.closures.push_str(&block);
                        self.closures.push_str(") };\n");
                        results.push(format!(
                            "wit_bindgen_wasmer::exports::PullBuffer::new(
                                &mut _bc, ptr{}, len{}, {}, &{})?",
                            tmp, tmp, size, closure
                        ));
                    }
                }
            }

            Instruction::BufferLowerHandle { push, ty } => {
                let block = self.blocks.pop().unwrap();
                let size = self.gen.sizes.size(ty);
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
                        "let {} = |memory: &wasmer::Memory, base: i32| {{
                            Ok(({}, {}))
                        }};\n",
                        closure, block, size,
                    ));
                    self.push_str(&format!(
                        "let {} = unsafe {{ buffer_transaction.push_out({}, &{}) }};\n",
                        handle, operands[0], closure,
                    ));
                } else {
                    let ty = self.type_string(iface, ty, TypeMode::AllBorrowed("'_"));
                    self.closures.push_str(&format!(
                        "let {} = |memory: &wasmer::Memory, base: i32, e: {}| {{
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
                    if sig.results.len() == 1 {
                        self.push_str("let ");
                        let arg = format!("result{}", tmp);
                        self.push_str(&arg);
                        results.push(arg);
                        self.push_str(" = ");
                    } else {
                        self.push_str("let (");
                        for i in 0..sig.results.len() {
                            let arg = format!("result{}_{}", tmp, i);
                            self.push_str(&arg);
                            self.push_str(",");
                            results.push(arg);
                        }
                        self.push_str(") = ");
                    }
                }
                self.push_str("self.func_");
                self.push_str(&to_rust_ident(name));
                if self.gen.opts.async_.includes(name) {
                    self.push_str(".call_async(");
                } else {
                    self.push_str(".call(");
                }
                for operand in operands {
                    self.push_str(operand);
                    self.push_str(", ");
                }
                self.push_str(")");
                if self.gen.opts.async_.includes(name) {
                    self.push_str(".await");
                }
                self.push_str("?;\n");
                self.after_call = true;
                self.caller_memory_available = false; // invalidated by call
            }

            Instruction::CallWasmAsyncImport { .. } => unimplemented!(),
            Instruction::CallWasmAsyncExport { .. } => unimplemented!(),

            Instruction::CallInterface { module: _, func } => {
                for (i, operand) in operands.iter().enumerate() {
                    self.push_str(&format!("let param{} = {};\n", i, operand));
                }
                if self.gen.opts.tracing && func.params.len() > 0 {
                    self.push_str("wit_bindgen_wasmer::tracing::event!(\n");
                    self.push_str("wit_bindgen_wasmer::tracing::Level::TRACE,\n");
                    for (i, (name, _ty)) in func.params.iter().enumerate() {
                        self.push_str(&format!(
                            "{} = wit_bindgen_wasmer::tracing::field::debug(&param{}),\n",
                            to_rust_ident(name),
                            i
                        ));
                    }
                    self.push_str(");\n");
                }

                if self.func_takes_all_memory {
                    let mem = self.memory_src();
                    self.push_str("let raw_memory = wit_bindgen_wasmer::RawMemory { slice: ");
                    self.push_str(&mem);
                    self.push_str(".raw() };\n");
                }

                let mut call = format!("host.{}(", func.name.to_snake_case());
                if self.func_takes_all_memory {
                    call.push_str("raw_memory, ");
                }
                for i in 0..operands.len() {
                    call.push_str(&format!("param{}, ", i));
                }
                call.push_str(")");
                if self.gen.opts.async_.includes(&func.name) {
                    call.push_str(".await");
                }

                self.let_results(func.results.len(), results);
                match self.gen.classify_fn_ret(iface, func) {
                    FunctionRet::Normal => self.push_str(&call),
                    // Unwrap the result, translating errors to unconditional
                    // traps
                    FunctionRet::CustomToTrap => {
                        self.push_str("match ");
                        self.push_str(&call);
                        self.push_str("{\n");
                        self.push_str("Ok(val) => val,\n");
                        self.push_str("Err(e) => return Err(host.error_to_trap(e)),\n");
                        self.push_str("}");
                    }
                    // Keep the `Result` as a `Result`, but convert the error
                    // to either the expected destination value or a trap,
                    // propagating a trap outwards.
                    FunctionRet::CustomToError { err, .. } => {
                        self.push_str("match ");
                        self.push_str(&call);
                        self.push_str("{\n");
                        self.push_str("Ok(val) => Ok(val),\n");
                        self.push_str(&format!("Err(e) => Err(host.error_to_{}(e)?),\n", err));
                        self.push_str("}");
                    }
                }
                self.push_str(";\n");
                self.after_call = true;
                if self.gen.opts.tracing && func.results.len() > 0 {
                    self.push_str("wit_bindgen_wasmer::tracing::event!(\n");
                    self.push_str("wit_bindgen_wasmer::tracing::Level::TRACE,\n");
                    for name in results.iter() {
                        self.push_str(&format!(
                            "{} = wit_bindgen_wasmer::tracing::field::debug(&{0}),\n",
                            name,
                        ));
                    }
                    self.push_str(");\n");
                }
            }

            Instruction::Return { amt, .. } => {
                let result = match amt {
                    0 => format!("Ok(())\n"),
                    1 => format!("Ok({})\n", operands[0]),
                    _ => format!("Ok(({}))\n", operands.join(", ")),
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

            Instruction::ReturnAsyncExport { .. } => unimplemented!(),
            Instruction::ReturnAsyncImport { .. } => unimplemented!(),

            Instruction::I32Load { offset } => results.push(self.load(*offset, "i32", operands)),
            Instruction::I32Load8U { offset } => {
                results.push(format!("i32::from({})", self.load(*offset, "u8", operands)));
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!("i32::from({})", self.load(*offset, "i8", operands)));
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!(
                    "i32::from({})",
                    self.load(*offset, "u16", operands)
                ));
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!(
                    "i32::from({})",
                    self.load(*offset, "i16", operands)
                ));
            }
            Instruction::I64Load { offset } => results.push(self.load(*offset, "i64", operands)),
            Instruction::F32Load { offset } => results.push(self.load(*offset, "f32", operands)),
            Instruction::F64Load { offset } => results.push(self.load(*offset, "f64", operands)),

            Instruction::I32Store { offset } => self.store(*offset, "as_i32", "", operands),
            Instruction::I64Store { offset } => self.store(*offset, "as_i64", "", operands),
            Instruction::F32Store { offset } => self.store(*offset, "as_f32", "", operands),
            Instruction::F64Store { offset } => self.store(*offset, "as_f64", "", operands),
            Instruction::I32Store8 { offset } => self.store(*offset, "as_i32", " as u8", operands),
            Instruction::I32Store16 { offset } => {
                self.store(*offset, "as_i32", " as u16", operands)
            }

            Instruction::Witx { instr } => match instr {
                WitxInstruction::PointerFromI32 { .. }
                | WitxInstruction::ConstPointerFromI32 { .. } => top_as("u32"),
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
        format!("wasmer::NativeFunc<{}>", self.cvt())
    }
}

fn sorted_iter<K: Ord, V>(map: &HashMap<K, V>) -> impl Iterator<Item = (&K, &V)> {
    let mut list = map.into_iter().collect::<Vec<_>>();
    list.sort_by_key(|p| p.0);
    list.into_iter()
}
