use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use std::str::FromStr;
use wit_bindgen_core::wit_parser::abi::{AbiVariant, Bindgen, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{wit_parser::*, Direction, Files, Generator, Source, TypeInfo, Types};
use wit_bindgen_gen_rust_lib::{
    to_rust_ident, wasm_type, FnSig, RustFlagsRepr, RustFunctionGenerator, RustGenerator, TypeMode,
};

#[derive(Default)]
pub struct Wasmtime {
    src: Source,
    opts: Opts,
    needs_get_memory: bool,
    needs_get_func: bool,
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
    num_wasm_params: usize,
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
    pub fn build(self) -> Wasmtime {
        let mut r = Wasmtime::new();
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
    CustomToError { ok: Type, err: String },
}

impl Wasmtime {
    pub fn new() -> Wasmtime {
        Wasmtime::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the Wasmtime host-side
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
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::RawMem;\n");
        }
        if self.needs_char_from_i32 {
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::char_from_i32;\n");
        }
        if self.needs_invalid_variant {
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::invalid_variant;\n");
        }
        if self.needs_bad_int {
            self.push_str("use core::convert::TryFrom;\n");
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::bad_int;\n");
        }
        if self.needs_validate_flags {
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::validate_flags;\n");
        }
        if self.needs_le {
            self.push_str("use wit_bindgen_host_wasmtime_rust::Le;\n");
        }
        if self.needs_copy_slice {
            self.push_str("use wit_bindgen_host_wasmtime_rust::rt::copy_slice;\n");
        }
    }

    /// Classifies the return value of a function to see if it needs handling
    /// with respect to the `custom_error` configuration option.
    fn classify_fn_ret(&mut self, iface: &Interface, f: &Function) -> FunctionRet {
        if !self.opts.custom_error {
            return FunctionRet::Normal;
        }

        if let Type::Id(id) = &f.result {
            if let TypeDefKind::Expected(e) = &iface.types[*id].kind {
                if let Type::Id(err) = e.err {
                    if let Some(name) = &iface.types[err].name {
                        self.needs_custom_error_to_types.insert(name.clone());
                        return FunctionRet::CustomToError {
                            ok: e.ok,
                            err: name.to_string(),
                        };
                    }
                }
            }
        }

        self.needs_custom_error_to_trap = true;
        FunctionRet::CustomToTrap
    }
}

impl RustGenerator for Wasmtime {
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
}

impl Generator for Wasmtime {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.types.analyze(iface);
        self.in_import = variant == AbiVariant::GuestImport;
        self.trait_name = iface.name.to_camel_case();
        self.src.push_str(&format!(
            "#[allow(clippy::all)]\npub mod {} {{\n",
            iface.name.to_snake_case(),
        ));
        self.src.push_str(
            "#[allow(unused_imports)]\nuse wit_bindgen_host_wasmtime_rust::{wasmtime, anyhow};\n",
        );
        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.print_typedef_record(iface, id, record, docs);

        // If this record might be used as a slice type in various places then
        // we synthesize an `Endian` implementation for it so `&[Le<ThisType>]`
        // is usable.
        if self.modes_of(iface, id).len() > 0
            && record.fields.iter().all(|f| iface.all_bits_valid(&f.ty))
        {
            self.src
                .push_str("impl wit_bindgen_host_wasmtime_rust::Endian for ");
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
                .push_str("unsafe impl wit_bindgen_host_wasmtime_rust::AllBytesValid for ");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str(" {}\n");
        }
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        self.print_typedef_tuple(iface, id, tuple, docs);
    }

    fn type_flags(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        self.src
            .push_str("wit_bindgen_host_wasmtime_rust::bitflags::bitflags! {\n");
        self.rustdoc(docs);
        let repr = RustFlagsRepr::new(flags);
        self.src
            .push_str(&format!("pub struct {}: {repr} {{\n", name.to_camel_case()));
        for (i, flag) in flags.flags.iter().enumerate() {
            self.rustdoc(&flag.docs);
            self.src.push_str(&format!(
                "const {} = 1 << {};\n",
                flag.name.to_shouty_snake_case(),
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
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.print_typedef_variant(iface, id, variant, docs);
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        self.print_typedef_union(iface, id, union, docs);
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        self.print_typedef_option(iface, id, payload, docs);
    }

    fn type_expected(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        expected: &Expected,
        docs: &Docs,
    ) {
        self.print_typedef_expected(iface, id, expected, docs);
    }

    fn type_enum(&mut self, _iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs);
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
            "pub struct {}(wit_bindgen_host_wasmtime_rust::rt::ResourceIndex);\n",
            tyname
        ));
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(iface, id, ty, docs);
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_type_list(iface, id, ty, docs);
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push_str(";\n");
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let prev = mem::take(&mut self.src);

        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        let params = (0..sig.params.len())
            .map(|i| format!("arg{}", i))
            .collect::<Vec<_>>();
        let mut f = FunctionBindgen::new(self, params);
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
            ..
        } = f;
        assert!(cleanup.is_none());
        assert!(!needs_buffer_transaction);

        // Generate the signature this function will have in the final trait
        let self_arg = "&mut self".to_string();
        self.in_trait = true;

        let mut fnsig = FnSig::default();
        fnsig.private = true;
        fnsig.async_ = self.opts.async_.includes(&func.name);
        fnsig.self_arg = Some(self_arg);
        self.print_docs_and_params(iface, func, TypeMode::LeafBorrowed("'_"), &fnsig);
        // The Rust return type may differ from the wasm return type based on
        // the `custom_error` configuration of this code generator.
        match self.classify_fn_ret(iface, func) {
            FunctionRet::Normal => {
                self.push_str(" -> ");
                self.print_ty(iface, &func.result, TypeMode::Owned);
            }
            FunctionRet::CustomToTrap => {
                self.push_str(" -> Result<");
                self.print_ty(iface, &func.result, TypeMode::Owned);
                self.push_str(", Self::Error>");
            }
            FunctionRet::CustomToError { ok, .. } => {
                self.push_str(" -> Result<");
                self.print_ty(iface, &ok, TypeMode::Owned);
                self.push_str(", Self::Error>");
            }
        }
        self.in_trait = false;
        let trait_signature = mem::take(&mut self.src).into();

        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        self.src
            .push_str("move |mut caller: wasmtime::Caller<'_, T>");
        for (i, param) in sig.params.iter().enumerate() {
            let arg = format!("arg{}", i);
            self.src.push_str(",");
            self.src.push_str(&arg);
            self.src.push_str(":");
            self.wasm_type(*param);
        }
        self.src.push_str("| {\n");

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

        if self.opts.tracing {
            self.src.push_str(&format!(
                "
                    let span = wit_bindgen_host_wasmtime_rust::tracing::span!(
                        wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,
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

        for (name, func) in needs_functions {
            self.src.push_str(&format!(
                "
                    let func = get_func(&mut caller, \"{name}\")?;
                    let func_{name} = func.typed::<{cvt}, _>(&caller)?;
                ",
                name = name,
                cvt = func.cvt(),
            ));
            self.needs_get_func = true;
        }

        if needs_memory || needs_borrow_checker {
            self.src
                .push_str("let memory = &get_memory(&mut caller, \"memory\")?;\n");
            self.needs_get_memory = true;
        }

        if needs_borrow_checker {
            self.src.push_str(
                "let (mem, data) = memory.data_and_store_mut(&mut caller);
                let mut _bc = wit_bindgen_host_wasmtime_rust::BorrowChecker::new(mem);
                let host = get(data);\n",
            );
        } else {
            self.src.push_str("let host = get(caller.data_mut());\n");
        }

        if self.all_needed_handles.len() > 0 {
            self.src.push_str("let (host, _tables) = host;\n");
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
                num_wasm_params: sig.params.len(),
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
        // asynchronous, Wasmtime can't intermix async and sync calls because
        // it's unknown whether the wasm module will make an async host call.
        let is_async = !self.opts.async_.is_none();
        let mut sig = FnSig::default();
        sig.async_ = is_async;
        sig.self_arg = Some("&self, mut caller: impl wasmtime::AsContextMut<Data = T>".to_string());
        self.print_docs_and_params(iface, func, TypeMode::AllBorrowed("'_"), &sig);
        self.push_str("-> Result<");
        self.print_ty(iface, &func.result, TypeMode::Owned);
        self.push_str(", wasmtime::Trap> {\n");

        let params = func
            .params
            .iter()
            .map(|(name, _)| to_rust_ident(name).to_string())
            .collect();
        let mut f = FunctionBindgen::new(self, params);
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
                .push_str(&format!("let func_{0} = &self.{0};\n", name));
            let get = format!(
                "instance.get_typed_func::<{}, _>(&mut store, \"{}\")?",
                func.cvt(),
                name
            );
            exports.fields.insert(name, (func.ty(), get));
        }

        self.src.push_str(&closures);

        assert!(!needs_borrow_checker);
        if needs_memory {
            self.src.push_str("let memory = &self.memory;\n");
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

        if needs_buffer_transaction {
            self.needs_buffer_glue = true;
            self.src
                .push_str("let mut buffer_transaction = self.buffer_glue.transaction();\n");
        }

        self.src.push_str(&String::from(src));
        self.src.push_str("}\n");
        let func_body = mem::replace(&mut self.src, prev);
        exports.funcs.push(func_body.into());

        // Create the code snippet which will define the type of this field in
        // the struct that we're exporting and additionally extracts the
        // function from an instantiated instance.
        let sig = iface.wasm_signature(AbiVariant::GuestExport, func);
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
            to_rust_ident(&func.name),
            (
                format!("wasmtime::TypedFunc<{}>", cvt),
                format!(
                    "instance.get_typed_func::<{}, _>(&mut store, \"{}\")?",
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
                self.src
                    .push_str("#[wit_bindgen_host_wasmtime_rust::async_trait]\n");
            }
            self.src.push_str("pub trait ");
            self.src.push_str(&module_camel);
            self.src.push_str(": Sized ");
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
                        "fn error_to_trap(&mut self, err: Self::Error) -> wasmtime::Trap;\n",
                    );
                }
                for ty in self.needs_custom_error_to_types.iter() {
                    self.src.push_str(&format!(
                        "fn error_to_{}(&mut self, err: Self::Error) -> Result<{}, wasmtime::Trap>;\n",
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
                    self.src
                        .push_str("_table: wit_bindgen_host_wasmtime_rust::Table<T::");
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

        for (module, funcs) in mem::take(&mut self.guest_imports) {
            let module_camel = module.to_camel_case();
            let is_async = !self.opts.async_.is_none();
            self.push_str("\npub fn add_to_linker<T, U>(linker: &mut wasmtime::Linker<T>");
            self.push_str(", get: impl Fn(&mut T) -> ");
            if self.all_needed_handles.is_empty() {
                self.push_str("&mut U");
            } else {
                self.push_str(&format!("(&mut U, &mut {}Tables<U>)", module_camel));
            }
            self.push_str("+ Send + Sync + Copy + 'static) -> anyhow::Result<()> \n");
            self.push_str("where U: ");
            self.push_str(&module_camel);
            if is_async {
                self.push_str(", T: Send,");
            }
            self.push_str("\n{\n");
            if self.needs_get_memory {
                self.push_str("use wit_bindgen_host_wasmtime_rust::rt::get_memory;\n");
            }
            if self.needs_get_func {
                self.push_str("use wit_bindgen_host_wasmtime_rust::rt::get_func;\n");
            }
            for f in funcs {
                let method = if f.is_async {
                    format!("func_wrap{}_async", f.num_wasm_params)
                } else {
                    String::from("func_wrap")
                };
                self.push_str(&format!(
                    "linker.{}(\"{}\", \"{}\", {})?;\n",
                    method, module, f.name, f.closure,
                ));
            }
            for handle in self.all_needed_handles.iter() {
                self.src.push_str(&format!(
                    "linker.func_wrap(
                        \"canonical_abi\",
                        \"resource_drop_{name}\",
                        move |mut caller: wasmtime::Caller<'_, T>, handle: u32| {{
                            let (host, tables) = get(caller.data_mut());
                            let handle = tables
                                .{snake}_table
                                .remove(handle)
                                .map_err(|e| {{
                                    wasmtime::Trap::new(format!(\"failed to remove handle: {{}}\", e))
                                }})?;
                            host.drop_{snake}(handle);
                            Ok(())
                        }}
                    )?;\n",
                    name = handle,
                    snake = handle.to_snake_case(),
                ));
            }
            self.push_str("Ok(())\n}\n");
        }

        for (module, exports) in sorted_iter(&mem::take(&mut self.guest_exports)) {
            let name = module.to_camel_case();

            // Generate a struct that is the "state" of this exported module
            // which is required to be included in the host state `T` of the
            // store.
            self.push_str(
                "
                /// Auxiliary data associated with the wasm exports.
                ///
                /// This is required to be stored within the data of a
                /// `Store<T>` itself so lifting/lowering state can be managed
                /// when translating between the host and wasm.
                ",
            );
            self.push_str("#[derive(Default)]\n");
            self.push_str("pub struct ");
            self.push_str(&name);
            self.push_str("Data {\n");
            for r in self.exported_resources.iter() {
                self.src.push_str(&format!(
                    "
                        index_slab{}: wit_bindgen_host_wasmtime_rust::rt::IndexSlab,
                        resource_slab{0}: wit_bindgen_host_wasmtime_rust::rt::ResourceSlab,
                        dtor{0}: Option<wasmtime::TypedFunc<i32, ()>>,
                    ",
                    r.index()
                ));
            }
            self.push_str("}\n");

            self.push_str("pub struct ");
            self.push_str(&name);
            self.push_str("<T> {\n");
            self.push_str(&format!(
                "get_state: Box<dyn Fn(&mut T) -> &mut {}Data + Send + Sync>,\n",
                name
            ));
            for (name, (ty, _)) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(": ");
                self.push_str(ty);
                self.push_str(",\n");
            }
            self.push_str("}\n");
            let bound = if self.opts.async_.is_none() {
                ""
            } else {
                ": Send"
            };
            self.push_str(&format!("impl<T{}> {}<T> {{\n", bound, name));

            if self.exported_resources.len() == 0 {
                self.push_str("#[allow(unused_variables)]\n");
            }
            self.push_str(&format!(
                "
                    /// Adds any intrinsics, if necessary for this exported wasm
                    /// functionality to the `linker` provided.
                    ///
                    /// The `get_state` closure is required to access the
                    /// auxiliary data necessary for these wasm exports from
                    /// the general store's state.
                    pub fn add_to_linker(
                        linker: &mut wasmtime::Linker<T>,
                        get_state: impl Fn(&mut T) -> &mut {}Data + Send + Sync + Copy + 'static,
                    ) -> anyhow::Result<()> {{
                ",
                name,
            ));
            for r in self.exported_resources.iter() {
                let (func_wrap, call, wait, prefix, suffix) = if self.opts.async_.is_none() {
                    ("func_wrap", "call", "", "", "")
                } else {
                    (
                        "func_wrap1_async",
                        "call_async",
                        ".await",
                        "Box::new(async move {",
                        "})",
                    )
                };
                self.src.push_str(&format!(
                    "
                        linker.{func_wrap}(
                            \"canonical_abi\",
                            \"resource_drop_{name}\",
                            move |mut caller: wasmtime::Caller<'_, T>, idx: u32| {prefix}{{
                                let state = get_state(caller.data_mut());
                                let resource_idx = state.index_slab{idx}.remove(idx)?;
                                let wasm = match state.resource_slab{idx}.drop(resource_idx) {{
                                    Some(wasm) => wasm,
                                    None => return Ok(()),
                                }};
                                let dtor = state.dtor{idx}.expect(\"destructor not set yet\");
                                dtor.{call}(&mut caller, wasm){wait}?;
                                Ok(())
                            }}{suffix},
                        )?;
                        linker.func_wrap(
                            \"canonical_abi\",
                            \"resource_clone_{name}\",
                            move |mut caller: wasmtime::Caller<'_, T>, idx: u32| {{
                                let state = get_state(caller.data_mut());
                                let resource_idx = state.index_slab{idx}.get(idx)?;
                                state.resource_slab{idx}.clone(resource_idx)?;
                                Ok(state.index_slab{idx}.insert(resource_idx))
                            }},
                        )?;
                        linker.func_wrap(
                            \"canonical_abi\",
                            \"resource_get_{name}\",
                            move |mut caller: wasmtime::Caller<'_, T>, idx: u32| {{
                                let state = get_state(caller.data_mut());
                                let resource_idx = state.index_slab{idx}.get(idx)?;
                                Ok(state.resource_slab{idx}.get(resource_idx))
                            }},
                        )?;
                        linker.func_wrap(
                            \"canonical_abi\",
                            \"resource_new_{name}\",
                            move |mut caller: wasmtime::Caller<'_, T>, val: i32| {{
                                let state = get_state(caller.data_mut());
                                let resource_idx = state.resource_slab{idx}.insert(val);
                                Ok(state.index_slab{idx}.insert(resource_idx))
                            }},
                        )?;
                    ",
                    name = iface.resources[*r].name,
                    idx = r.index(),
                    func_wrap = func_wrap,
                    call = call,
                    wait = wait,
                    prefix = prefix,
                    suffix = suffix,
                ));
            }
            self.push_str("Ok(())\n");
            self.push_str("}\n");

            let (async_fn, instantiate, wait) = if self.opts.async_.is_none() {
                ("", "", "")
            } else {
                ("async ", "_async", ".await")
            };
            self.push_str(&format!(
                "
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    ///
                    /// The `linker` provided will have intrinsics added to it
                    /// automatically, so it's not necessary to call
                    /// `add_to_linker` beforehand. This function will
                    /// instantiate the `module` otherwise using `linker`, and
                    /// both an instance of this structure and the underlying
                    /// `wasmtime::Instance` will be returned.
                    ///
                    /// The `get_state` parameter is used to access the
                    /// auxiliary state necessary for these wasm exports from
                    /// the general store state `T`.
                    pub {}fn instantiate(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        module: &wasmtime::Module,
                        linker: &mut wasmtime::Linker<T>,
                        get_state: impl Fn(&mut T) -> &mut {}Data + Send + Sync + Copy + 'static,
                    ) -> anyhow::Result<(Self, wasmtime::Instance)> {{
                        Self::add_to_linker(linker, get_state)?;
                        let instance = linker.instantiate{}(&mut store, module){}?;
                        Ok((Self::new(store, &instance,get_state)?, instance))
                    }}
                ",
                async_fn, name, instantiate, wait,
            ));

            self.push_str(&format!(
                "
                    /// Low-level creation wrapper for wrapping up the exports
                    /// of the `instance` provided in this structure of wasm
                    /// exports.
                    ///
                    /// This function will extract exports from the `instance`
                    /// defined within `store` and wrap them all up in the
                    /// returned structure which can be used to interact with
                    /// the wasm module.
                    pub fn new(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        instance: &wasmtime::Instance,
                        get_state: impl Fn(&mut T) -> &mut {}Data + Send + Sync + Copy + 'static,
                    ) -> anyhow::Result<Self> {{
                ",
                name,
            ));
            self.push_str("let mut store = store.as_context_mut();\n");
            assert!(!self.needs_get_func);
            for (name, (_, get)) in exports.fields.iter() {
                self.push_str("let ");
                self.push_str(&name);
                self.push_str("= ");
                self.push_str(&get);
                self.push_str(";\n");
            }
            for r in self.exported_resources.iter() {
                self.src.push_str(&format!(
                    "
                        get_state(store.data_mut()).dtor{} = \
                            Some(instance.get_typed_func::<i32, (), _>(\
                                &mut store, \
                                \"canonical_abi_drop_{}\", \
                            )?);\n
                    ",
                    r.index(),
                    iface.resources[*r].name,
                ));
            }
            self.push_str("Ok(");
            self.push_str(&name);
            self.push_str("{\n");
            for (name, _) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(",\n");
            }
            self.push_str("get_state: Box::new(get_state),\n");
            self.push_str("\n})\n");
            self.push_str("}\n");

            for func in exports.funcs.iter() {
                self.push_str(func);
            }

            for r in self.exported_resources.iter() {
                let (async_fn, call, wait) = if self.opts.async_.is_none() {
                    ("", "call", "")
                } else {
                    ("async ", "call_async", ".await")
                };
                self.src.push_str(&format!(
                    "
                        /// Drops the host-owned handle to the resource
                        /// specified.
                        ///
                        /// Note that this may execute the WebAssembly-defined
                        /// destructor for this type. This also may not run
                        /// the destructor if there are still other references
                        /// to this type.
                        pub {async}fn drop_{name_snake}(
                            &self,
                            mut store: impl wasmtime::AsContextMut<Data = T>,
                            val: {name_camel},
                        ) -> Result<(), wasmtime::Trap> {{
                            let mut store = store.as_context_mut();
                            let data = (self.get_state)(store.data_mut());
                            let wasm = match data.resource_slab{idx}.drop(val.0) {{
                                Some(val) => val,
                                None => return Ok(()),
                            }};
                            data.dtor{idx}.unwrap().{call}(&mut store, wasm){wait}?;
                            Ok(())
                        }}
                    ",
                    name_snake = iface.resources[*r].name.to_snake_case(),
                    name_camel = iface.resources[*r].name.to_camel_case(),
                    idx = r.index(),
                    async = async_fn,
                    call = call,
                    wait = wait,
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
    gen: &'a mut Wasmtime,

    // Number used to assign unique names to temporary variables.
    tmp: usize,

    // Destination where source code is pushed onto for this function
    src: Source,

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
    fn new(gen: &mut Wasmtime, params: Vec<String>) -> FunctionBindgen<'_> {
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
            closures: Source::default(),
            needs_buffer_transaction: false,
            needs_borrow_checker: false,
            needs_memory: false,
            needs_functions: HashMap::new(),
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
                // get separate borrows of `caller_memory` and `_tables` if we
                // might need handle tables later. If we don't end up using
                // `_tables` that's ok, it'll almost always be optimized away.
                if self.gen.all_needed_handles.len() > 0 {
                    self.push_str(
                        "let (caller_memory, data) = memory.data_and_store_mut(&mut caller);\n",
                    );
                    self.push_str("let (_, _tables) = get(data);\n");
                } else {
                    self.push_str("let caller_memory = memory.data_mut(&mut caller);\n");
                }
            }
            format!("caller_memory")
        } else {
            self.needs_memory = true;
            format!("memory.data_mut(&mut caller)")
        }
    }

    fn call_intrinsic(&mut self, name: &str, args: String) {
        let (method, suffix) = if self.gen.opts.async_.is_none() {
            ("call", "")
        } else {
            self.async_intrinsic_called = true;
            ("call_async", ".await")
        };
        self.push_str(&format!(
            "func_{}.{}(&mut caller, {}){}?;\n",
            name, method, args, suffix
        ));
        self.caller_memory_available = false; // invalidated by call
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
            "{}.store({} + {}, wit_bindgen_host_wasmtime_rust::rt::{}({}){})?;\n",
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

    fn return_pointer(&mut self, _iface: &Interface, _size: usize, _align: usize) -> String {
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
                results.push(format!("wit_bindgen_host_wasmtime_rust::rt::as_i64({})", s));
            }
            Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!("wit_bindgen_host_wasmtime_rust::rt::as_i32({})", s));
            }

            Instruction::F32FromFloat32
            | Instruction::F64FromFloat64
            | Instruction::Float32FromF32
            | Instruction::Float64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }

            // Downcasts from `i32` into smaller integers are checked to ensure
            // that they fit within the valid range. While not strictly
            // necessary since we could chop bits off this should be more
            // forward-compatible with any future changes.
            Instruction::S8FromI32 => try_from("i8", operands, results),
            Instruction::U8FromI32 => try_from("u8", operands, results),
            Instruction::S16FromI32 => try_from("i16", operands, results),
            Instruction::U16FromI32 => try_from("u16", operands, results),

            // Casts of the same bit width simply use `as` since we're just
            // reinterpreting the bits already there.
            Instruction::U32FromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),

            Instruction::CharFromI32 => {
                self.gen.needs_char_from_i32 = true;
                results.push(format!("char_from_i32({})?", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                wit_bindgen_gen_rust_lib::bitcast(casts, operands, results)
            }

            Instruction::UnitLower => {
                self.push_str(&format!("let () = {};\n", operands[0]));
            }
            Instruction::UnitLift => {
                results.push("()".to_string());
            }

            Instruction::I32FromBool => {
                results.push(format!("match {} {{ true => 1, false => 0 }}", operands[0]));
            }
            Instruction::BoolFromI32 => {
                self.gen.needs_invalid_variant = true;
                results.push(format!(
                    "match {} {{
                        0 => false,
                        1 => true,
                        _ => return Err(invalid_variant(\"bool\")),
                    }}",
                    operands[0],
                ));
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
                results.push(format!(
                    "_tables.{}_table.get(({}) as u32).ok_or_else(|| {{
                            wasmtime::Trap::new(\"invalid handle index\")
                        }})?",
                    name.to_snake_case(),
                    operands[0]
                ));
            }
            Instruction::I32FromBorrowedHandle { ty } => {
                let tmp = self.tmp();
                self.push_str(&format!(
                    "
                        let obj{tmp} = {op};
                        (self.get_state)(caller.as_context_mut().data_mut()).resource_slab{idx}.clone(obj{tmp}.0)?;
                        let handle{tmp} = (self.get_state)(caller.as_context_mut().data_mut()).index_slab{idx}.insert(obj{tmp}.0);
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
                    "let handle{} = (self.get_state)(caller.as_context_mut().data_mut()).index_slab{}.remove({} as u32)?;\n",
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

            Instruction::TupleLower { tuple, .. } => {
                self.tuple_lower(tuple, &operands[0], results);
            }
            Instruction::TupleLift { .. } => {
                self.tuple_lift(operands, results);
            }

            Instruction::FlagsLower { flags, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
                for i in 0..flags.repr().count() {
                    results.push(format!("(flags{}.bits >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLift { flags, name, .. } => {
                self.gen.needs_validate_flags = true;
                let repr = RustFlagsRepr::new(flags);
                let mut flags = String::from("0");
                for (i, op) in operands.iter().enumerate() {
                    flags.push_str(&format!("| (({} as {repr}) << {})", op, i * 32));
                }
                results.push(format!(
                    "validate_flags(
                        {},
                        {name}::all().bits(),
                        \"{name}\",
                        |bits| {name} {{ bits }}
                    )?",
                    flags,
                    name = name.to_camel_case(),
                ));
            }

            Instruction::VariantPayloadName => results.push("e".to_string()),

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
                self.let_results(result_types.len(), results);
                let op0 = &operands[0];
                self.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lower(iface, *ty);
                for (case, block) in variant.cases.iter().zip(blocks) {
                    let case_name = case.name.to_camel_case();
                    self.push_str(&format!("{name}::{case_name}"));
                    if case.ty == Type::Unit {
                        self.push_str(&format!(" => {{\nlet e = ();\n{block}\n}}\n"));
                    } else {
                        self.push_str(&format!("(e) => {block},\n"));
                    }
                }
                self.push_str("};\n");
            }

            Instruction::VariantLift { variant, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                let mut result = format!("match {op0} {{\n");
                let name = self.typename_lift(iface, *ty);
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    let block = if case.ty != Type::Unit {
                        format!("({block})")
                    } else {
                        String::new()
                    };
                    let case = case.name.to_camel_case();
                    result.push_str(&format!("{i} => {name}::{case}{block},\n"));
                }
                result.push_str(&format!("_ => return Err(invalid_variant(\"{name}\")),\n"));
                result.push_str("}");
                results.push(result);
                self.gen.needs_invalid_variant = true;
            }

            Instruction::UnionLower {
                union,
                results: result_types,
                ty,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                self.let_results(result_types.len(), results);
                let op0 = &operands[0];
                self.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lower(iface, *ty);
                for (case_name, block) in self
                    .gen
                    .union_case_names(iface, union)
                    .into_iter()
                    .zip(blocks)
                {
                    self.push_str(&format!("{name}::{case_name}(e) => {block},\n"));
                }
                self.push_str("};\n");
            }

            Instruction::UnionLift { union, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                let mut result = format!("match {op0} {{\n");
                let name = self.typename_lift(iface, *ty);
                for (i, (case_name, block)) in self
                    .gen
                    .union_case_names(iface, union)
                    .into_iter()
                    .zip(blocks)
                    .enumerate()
                {
                    result.push_str(&format!("{i} => {name}::{case_name}({block}),\n"));
                }
                result.push_str(&format!("_ => return Err(invalid_variant(\"{name}\")),\n"));
                result.push_str("}");
                results.push(result);
            }

            Instruction::OptionLower {
                results: result_types,
                ..
            } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                self.let_results(result_types.len(), results);
                let operand = &operands[0];
                self.push_str(&format!(
                    "match {operand} {{
                        Some(e) => {some},
                        None => {{\nlet e = ();\n{none}\n}},
                    }};"
                ));
            }

            Instruction::OptionLift { .. } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                assert_eq!(none, "()");
                let operand = &operands[0];
                results.push(format!(
                    "match {operand} {{
                        0 => None,
                        1 => Some({some}),
                        _ => return Err(invalid_variant(\"option\")),
                    }}"
                ));
                self.gen.needs_invalid_variant = true;
            }

            Instruction::ExpectedLower {
                results: result_types,
                ..
            } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                self.let_results(result_types.len(), results);
                let operand = &operands[0];
                self.push_str(&format!(
                    "match {operand} {{
                        Ok(e) => {{ {ok} }},
                        Err(e) => {{ {err} }},
                    }};"
                ));
            }

            Instruction::ExpectedLift { .. } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                let operand = &operands[0];
                results.push(format!(
                    "match {operand} {{
                        0 => Ok({ok}),
                        1 => Err({err}),
                        _ => return Err(invalid_variant(\"expected\")),
                    }}"
                ));
                self.gen.needs_invalid_variant = true;
            }

            Instruction::EnumLower { .. } => {
                results.push(format!("{} as i32", operands[0]));
            }

            Instruction::EnumLift { name, enum_, .. } => {
                let op0 = &operands[0];
                let mut result = format!("match {op0} {{\n");
                let name = name.to_camel_case();
                for (i, case) in enum_.cases.iter().enumerate() {
                    let case = case.name.to_camel_case();
                    result.push_str(&format!("{i} => {name}::{case},\n"));
                }
                result.push_str(&format!("_ => return Err(invalid_variant(\"{name}\")),\n"));
                result.push_str("}");
                results.push(result);
                self.gen.needs_invalid_variant = true;
            }

            Instruction::ListCanonLower { element, realloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);
                let (size, align) = (self.gen.sizes.size(element), self.gen.sizes.align(element));

                // Store the operand into a temporary...
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then realloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!("let {} = ", ptr));
                self.call_intrinsic(
                    realloc,
                    format!("(0, 0, {}, ({}.len() as i32) * {})", align, val, size),
                );

                // ... and then copy over the result.
                let mem = self.memory_src();
                self.push_str(&format!("{}.store_many({}, &{})?;\n", mem, ptr, val));
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
                    let (align, el_size) =
                        (self.sizes().align(element), self.sizes().size(element));
                    let tmp = self.tmp();
                    self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                    self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                    self.push_str(&format!(
                        "
                            let data{tmp} = copy_slice(
                                &mut caller,
                                memory,
                                ptr{tmp}, len{tmp}, {}
                            )?;
                        ",
                        align,
                        tmp = tmp,
                    ));
                    self.call_intrinsic(
                        free,
                        // we use normal multiplication here as copy_slice has
                        // already verified that multiplied size fits i32
                        format!("(ptr{tmp}, len{tmp} * {}, {})", el_size, align, tmp = tmp),
                    );
                    results.push(format!("data{}", tmp));
                }
                None => {
                    self.needs_borrow_checker = true;
                    let tmp = self.tmp();
                    self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                    self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                    let slice = format!("_bc.slice(ptr{0}, len{0})?", tmp);
                    results.push(slice);
                }
            },

            Instruction::StringLower { realloc } => {
                // see above for this unwrap
                let realloc = realloc.unwrap();
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);

                // Store the operand into a temporary...
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then realloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!("let {} = ", ptr));
                self.call_intrinsic(realloc, format!("(0, 0, 1, {}.len() as i32)", val));

                // ... and then copy over the result.
                let mem = self.memory_src();
                self.push_str(&format!(
                    "{}.store_many({}, {}.as_bytes())?;\n",
                    mem, ptr, val
                ));
                self.gen.needs_raw_mem = true;
                self.needs_memory = true;
                results.push(ptr);
                results.push(format!("{}.len() as i32", val));
            }

            Instruction::StringLift { free } => match free {
                Some(free) => {
                    self.needs_memory = true;
                    self.gen.needs_copy_slice = true;
                    self.needs_functions
                        .insert(free.to_string(), NeededFunction::Free);
                    let tmp = self.tmp();
                    self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                    self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                    self.push_str(&format!(
                        "
                            let data{tmp} = copy_slice(
                                &mut caller,
                                memory,
                                ptr{tmp}, len{tmp}, 1,
                            )?;
                        ",
                        tmp = tmp,
                    ));
                    self.call_intrinsic(
                        free,
                        // we use normal multiplication here as copy_slice has
                        // already verified that multiplied size fits i32
                        format!("(ptr{tmp}, len{tmp}, 1)", tmp = tmp),
                    );
                    results.push(format!(
                        "String::from_utf8(data{})
                            .map_err(|_| wasmtime::Trap::new(\"invalid utf-8\"))?",
                        tmp,
                    ));
                }
                None => {
                    self.needs_borrow_checker = true;
                    let tmp = self.tmp();
                    self.push_str(&format!("let ptr{} = {};\n", tmp, operands[0]));
                    self.push_str(&format!("let len{} = {};\n", tmp, operands[1]));
                    let slice = format!("_bc.slice_str(ptr{0}, len{0})?", tmp);
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
                self.call_intrinsic(realloc, format!("(0, 0, {}, {} * {})", align, len, size));

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
                    self.call_intrinsic(free, format!("({}, {} * {}, {})", base, len, size, align));
                    self.needs_functions
                        .insert(free.to_string(), NeededFunction::Free);
                }
            }

            Instruction::IterElem { .. } => {
                self.caller_memory_available = false; // invalidated by for loop
                results.push("e".to_string())
            }

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm {
                iface: _,
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
                self.push_str(&to_rust_ident(name));
                if self.gen.opts.async_.includes(name) {
                    self.push_str(".call_async(");
                } else {
                    self.push_str(".call(");
                }
                self.push_str("&mut caller, (");
                for operand in operands {
                    self.push_str(operand);
                    self.push_str(", ");
                }
                self.push_str("))");
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
                    self.push_str("wit_bindgen_host_wasmtime_rust::tracing::event!(\n");
                    self.push_str("wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,\n");
                    for (i, (name, _ty)) in func.params.iter().enumerate() {
                        self.push_str(&format!(
                            "{} = wit_bindgen_host_wasmtime_rust::tracing::field::debug(&param{}),\n",
                            to_rust_ident(name),
                            i
                        ));
                    }
                    self.push_str(");\n");
                }

                let mut call = format!("host.{}(", func.name.to_snake_case());
                for i in 0..operands.len() {
                    call.push_str(&format!("param{}, ", i));
                }
                call.push_str(")");
                if self.gen.opts.async_.includes(&func.name) {
                    call.push_str(".await");
                }

                self.push_str("let result = ");
                results.push("result".to_string());
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
                        self.push_str(&format!(
                            "Err(e) => Err(host.error_to_{}(e)?),\n",
                            err.to_snake_case()
                        ));
                        self.push_str("}");
                    }
                }
                self.push_str(";\n");
                self.after_call = true;
                match &func.result {
                    Type::Unit => {}
                    _ if self.gen.opts.tracing => {
                        self.push_str("wit_bindgen_host_wasmtime_rust::tracing::event!(\n");
                        self.push_str("wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,\n");
                        self.push_str(&format!(
                            "{} = wit_bindgen_host_wasmtime_rust::tracing::field::debug(&{0}),\n",
                            results[0],
                        ));
                        self.push_str(");\n");
                    }
                    _ => {}
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

            Instruction::Malloc {
                realloc,
                size,
                align,
            } => {
                self.needs_functions
                    .insert(realloc.to_string(), NeededFunction::Realloc);
                let tmp = self.tmp();
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!("let {} = ", ptr));
                self.call_intrinsic(realloc, format!("(0, 0, {}, {})", align, size));
                results.push(ptr);
            }

            Instruction::Free { .. } => unimplemented!(),
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
