use heck::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wai_bindgen_gen_core::wai_parser::abi::{
    AbiVariant, Bindgen, Instruction, LiftLower, WasmType, WitxInstruction,
};
use wai_bindgen_gen_core::{wai_parser::*, Direction, Files, Generator, Source, TypeInfo, Types};
use wai_bindgen_gen_rust::{
    int_repr, wasm_type, FnSig, RustFunctionGenerator, RustGenerator, TypeMode,
};

#[derive(Default)]
pub struct RustWasm {
    src: Source,
    opts: Opts,
    types: Types,
    in_import: bool,
    traits: BTreeMap<String, Trait>,
    in_trait: bool,
    trait_name: String,
    i64_return_pointer_area_size: usize,
    sizes: SizeAlign,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub rustfmt: bool,

    /// Adds the wai module name into import binding names when enabled.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub multi_module: bool,

    /// Whether or not the bindings assume interface values are always
    /// well-formed or whether checks are performed.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub unchecked: bool,

    /// A prefix to prepend to all exported symbols. Note that this is only
    /// intended for testing because it breaks the general form of the ABI.
    #[cfg_attr(feature = "structopt", structopt(skip))]
    pub symbol_namespace: String,

    /// The alias to use for the `wai_bindgen_rust` crate.
    ///
    /// This allows code generators to alias the `wai_bindgen_rust` crate
    /// to a re-export in another crate.
    #[cfg_attr(feature = "structopt", structopt(skip))]
    pub crate_alias: Option<String>,
}

#[derive(Default)]
struct Trait {
    methods: Vec<String>,
    resource_methods: BTreeMap<ResourceId, Vec<String>>,
}

impl Opts {
    pub fn build(self) -> RustWasm {
        let mut r = RustWasm::new();
        r.opts = self;
        r
    }
}

impl RustWasm {
    pub fn new() -> RustWasm {
        RustWasm::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses the obvious direction to ABI variant mapping.
        match dir {
            Direction::Export => AbiVariant::GuestExport,
            Direction::Import => AbiVariant::GuestImport,
        }
    }
}

impl RustGenerator for RustWasm {
    fn default_param_mode(&self) -> TypeMode {
        if self.in_import {
            // We default to borrowing as much as possible to maximize the ability
            // for host to take views into our memory without forcing wasm modules
            // to allocate anything.
            TypeMode::AllBorrowed("'a")
        } else {
            // In exports everythig is always owned, slices and handles and all.
            // Nothing is borrowed.
            TypeMode::Owned
        }
    }

    fn handle_projection(&self) -> Option<(&'static str, String)> {
        None
    }

    fn handle_in_super(&self) -> bool {
        !self.in_import
    }

    fn handle_wrapper(&self) -> Option<&'static str> {
        if self.in_import {
            None
        } else {
            Some("wai_bindgen_rust::Handle")
        }
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
        self.src.push_str("usize");
    }

    fn print_pointer(&mut self, iface: &Interface, const_: bool, ty: &Type) {
        self.push_str("*");
        if const_ {
            self.push_str("const ");
        } else {
            self.push_str("mut ");
        }
        let manually_drop = match ty {
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Record(_) => true,
                TypeDefKind::List(_)
                | TypeDefKind::Variant(_)
                | TypeDefKind::PushBuffer(_)
                | TypeDefKind::PullBuffer(_)
                | TypeDefKind::Type(_) => panic!("unsupported pointer type"),
                TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => true,
            },
            Type::Handle(_) => true,
            _ => false,
        };
        if manually_drop {
            self.push_str("core::mem::ManuallyDrop<");
        }
        self.print_ty(iface, ty, TypeMode::Owned);
        if manually_drop {
            self.push_str(">");
        }
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

    fn print_lib_buffer(
        &mut self,
        iface: &Interface,
        push: bool,
        ty: &Type,
        mode: TypeMode,
        lt: &'static str,
    ) {
        let prefix = if push { "Push" } else { "Pull" };
        if self.in_import {
            if let TypeMode::AllBorrowed(_) = mode {
                self.push_str("&");
                if lt != "'_" {
                    self.push_str(lt);
                }
                self.push_str(" mut ");
            }
            self.push_str(&format!(
                "wai_bindgen_rust::imports::{}Buffer<{}, ",
                prefix, lt,
            ));
            self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
            self.push_str(">");
        } else {
            // Buffers in exports are represented with special types from the
            // library support crate since they're wrappers around
            // externally-provided handles.
            self.push_str("wai_bindgen_rust::exports::");
            self.push_str(prefix);
            self.push_str("Buffer");
            self.push_str("<");
            self.push_str(lt);
            self.push_str(", ");
            self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
            self.push_str(">");
        }
    }
}

impl Generator for RustWasm {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.in_import = variant == AbiVariant::GuestImport;
        self.types.analyze(iface);
        self.trait_name = iface.name.to_camel_case();
        self.src
            .push_str(&format!("mod {} {{\n", iface.name.to_snake_case()));

        if let Some(alias) = &self.opts.crate_alias {
            self.src
                .push_str(&format!("use {} as wai_bindgen_rust;\n", alias));
        }

        for func in iface.functions.iter() {
            let sig = iface.wasm_signature(variant, func);
            if let Some(results) = sig.retptr {
                self.i64_return_pointer_area_size =
                    self.i64_return_pointer_area_size.max(results.len());
            }
        }
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
            self.rustdoc(docs);
            self.src
                .push_str(&format!("pub type {} = ", name.to_camel_case()));
            let repr = iface
                .flags_repr(record)
                .expect("unsupported number of flags");
            self.src.push_str(int_repr(repr));
            self.src.push_str(";\n");
            for (i, field) in record.fields.iter().enumerate() {
                self.rustdoc(&field.docs);
                self.src.push_str(&format!(
                    "pub const {}_{}: {} = 1 << {};\n",
                    name.to_shouty_snake_case(),
                    field.name.to_shouty_snake_case(),
                    name.to_camel_case(),
                    i,
                ));
            }
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
        // For exported handles we synthesize some trait implementations
        // automatically for runtime-required traits.
        if !self.in_import {
            self.src.push_str(&format!(
                "   #[cfg(any(target_arch = \"wasm32\", target_arch = \"wasm64\"))]
                    unsafe impl wai_bindgen_rust::handle::HandleType for super::{ty} {{
                        #[inline]
                        fn clone(_val: i32) -> i32 {{
                            {{
                                #[link(wasm_import_module = \"canonical_abi\")]
                                extern \"C\" {{
                                    #[link_name = \"resource_clone_{name}\"]
                                    fn clone(val: i32) -> i32;
                                }}
                                unsafe {{ clone(_val) }}
                            }}
                        }}

                        #[inline]
                        fn drop(_val: i32) {{
                            {{
                                #[link(wasm_import_module = \"canonical_abi\")]
                                extern \"C\" {{
                                    #[link_name = \"resource_drop_{name}\"]
                                    fn drop(val: i32);
                                }}
                                unsafe {{ drop(_val) }}
                            }}
                        }}
                    }}

                    #[cfg(any(target_arch = \"wasm32\", target_arch = \"wasm64\"))]
                    unsafe impl wai_bindgen_rust::handle::LocalHandle for super::{ty} {{
                        #[inline]
                        fn new(_val: i32) -> i32 {{
                            {{
                                #[link(wasm_import_module = \"canonical_abi\")]
                                extern \"C\" {{
                                    #[link_name = \"resource_new_{name}\"]
                                    fn new(val: i32) -> i32;
                                }}
                                unsafe {{ new(_val) }}
                            }}
                        }}

                        #[inline]
                        fn get(_val: i32) -> i32 {{
                            {{
                                #[link(wasm_import_module = \"canonical_abi\")]
                                extern \"C\" {{
                                    #[link_name = \"resource_get_{name}\"]
                                    fn get(val: i32) -> i32;
                                }}
                                unsafe {{ get(_val) }}
                            }}
                        }}
                    }}

                    #[cfg(any(target_arch = \"wasm32\", target_arch = \"wasm64\"))]
                    const _: () = {{
                        #[export_name = \"{ns}canonical_abi_drop_{name}\"]
                        extern \"C\" fn drop(ty: Box<super::{ty}>) {{
                            <super::{iface} as {iface}>::drop_{name_snake}(*ty)
                        }}
                    }};
                    #[cfg(not(any(target_arch = \"wasm32\", target_arch = \"wasm64\")))]
                    const _: () = {{
                        #[export_name = \"resource_drop_{name}\"]
                        extern \"C\" fn drop(ty: Box<super::{ty}>) {{
                            <super::{iface} as {iface}>::drop_{name_snake}(*ty)
                        }}
                    }};
                ",
                ty = iface.resources[ty].name.to_camel_case(),
                name = iface.resources[ty].name,
                name_snake = iface.resources[ty].name.to_snake_case(),
                iface = iface.name.to_camel_case(),
                ns = self.opts.symbol_namespace,
            ));
            let trait_ = self
                .traits
                .entry(iface.name.to_camel_case())
                .or_insert(Trait::default());
            trait_.methods.push(format!(
                "
                    /// An optional callback invoked when a handle is finalized
                    /// and destroyed.
                    fn drop_{}(val: super::{}) {{
                        drop(val);
                    }}
                ",
                iface.resources[ty].name.to_snake_case(),
                iface.resources[ty].name.to_camel_case(),
            ));
            return;
        }

        let resource = &iface.resources[ty];
        let name = &resource.name;

        self.rustdoc(&resource.docs);
        self.src.push_str("#[derive(Debug)]\n");
        self.src.push_str("#[repr(transparent)]\n");
        self.src
            .push_str(&format!("pub struct {}(i32);\n", name.to_camel_case()));
        self.src.push_str("impl ");
        self.src.push_str(&name.to_camel_case());
        self.src.push_str(
            " {
                pub unsafe fn from_raw(raw: i32) -> Self {
                    Self(raw)
                }

                pub fn into_raw(self) -> i32 {
                    let ret = self.0;
                    core::mem::forget(self);
                    return ret;
                }

                pub fn as_raw(&self) -> i32 {
                    self.0
                }
            }\n",
        );

        self.src.push_str("impl Drop for ");
        self.src.push_str(&name.to_camel_case());
        if self.types.has_preview1_dtor(ty) {
            self.src.push_str(&format!(
                "{{
                    fn drop(&mut self) {{
                        unsafe {{
                            drop({}_close({}(self.0)));
                        }}
                    }}
                }}\n",
                name,
                name.to_camel_case(),
            ));
        } else {
            self.src.push_str(&format!(
                "{{
                    fn drop(&mut self) {{
                        #[link(wasm_import_module = \"canonical_abi\")]
                        extern \"C\" {{
                            #[link_name = \"resource_drop_{}\"]
                            fn close(fd: i32);
                        }}
                        unsafe {{
                            close(self.0);
                        }}
                    }}
                }}\n",
                name,
            ));
        }
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

    fn import(&mut self, iface: &Interface, func: &Function) {
        let is_dtor = self.types.is_preview1_dtor_func(func);
        let mut sig = FnSig::default();
        let param_mode = if is_dtor {
            sig.unsafe_ = true;
            TypeMode::Owned
        } else {
            TypeMode::AllBorrowed("'_")
        };
        sig.async_ = func.is_async;
        match &func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => {
                sig.use_item_name = true;
                self.src.push_str(&format!(
                    "impl {} {{\n",
                    iface.resources[*resource].name.to_camel_case()
                ));
            }
        }
        if let FunctionKind::Method { .. } = func.kind {
            sig.self_arg = Some("&self".to_string());
            sig.self_is_first_param = true;
        }
        let params = self.print_signature(iface, func, param_mode, &sig);
        self.src.push_str("{\n");
        if !is_dtor {
            self.src.push_str("unsafe {\n");
        }

        let mut f = FunctionBindgen::new(self, is_dtor, params);
        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_cleanup_list,
            src,
            ..
        } = f;

        if needs_cleanup_list {
            self.src.push_str("let mut cleanup_list = Vec::new();\n");
        }
        self.src.push_str(&String::from(src));

        if !is_dtor {
            self.src.push_str("}\n");
        }
        self.src.push_str("}\n");

        match &func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Static { .. } | FunctionKind::Method { .. } => {
                self.src.push_str("}\n");
            }
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        let is_dtor = self.types.is_preview1_dtor_func(func);
        let rust_name = func.name.to_snake_case();

        self.src.push_str("#[cfg_attr(target_arch = \"wasm32\", export_name = \"");
        self.src.push_str(&self.opts.symbol_namespace);
        self.src.push_str(&func.name);
        self.src.push_str("\")]\n");
        self.src.push_str("#[cfg_attr(not(target_arch = \"wasm32\"), export_name = \"");
        self.src.push_str(&self.opts.symbol_namespace);
        self.src.push_str(&iface.name);
        self.src.push_str("_");
        self.src.push_str(&func.name);
        self.src.push_str("\")]\n");
        self.src.push_str("unsafe extern \"C\" fn __wai_bindgen_");
        self.src.push_str(&rust_name);
        self.src.push_str("(");
        let sig = iface.wasm_signature(AbiVariant::GuestExport, func);
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            self.src.push_str(&name);
            self.src.push_str(": ");
            self.wasm_type(*param);
            self.src.push_str(", ");
            params.push(name);
        }
        self.src.push_str("#[cfg(not(target_arch = \"wasm32\"))] ret: *mut i64");
        self.src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                self.src.push_str(" -> ");
                self.wasm_type(sig.results[0]);
            }
            _ => unimplemented!(),
        }
        self.src.push_str("{\n");

        if func.is_async {
            self.src.push_str("let future = async move {\n");
        }

        let mut f = FunctionBindgen::new(self, is_dtor, params);
        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_cleanup_list,
            src,
            ..
        } = f;
        assert!(!needs_cleanup_list);
        self.src.push_str(&String::from(src));
        if func.is_async {
            self.src.push_str("};\n");
            self.src
                .push_str("wai_bindgen_rust::rt::execute(Box::pin(future));\n");
        }
        self.src.push_str("}\n");

        let prev = mem::take(&mut self.src);
        self.in_trait = true;
        let mut sig = FnSig::default();
        sig.private = true;
        sig.async_ = func.is_async;
        match &func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Static { .. } => sig.use_item_name = true,
            FunctionKind::Method { .. } => {
                sig.use_item_name = true;
                sig.self_is_first_param = true;
                sig.self_arg = Some("&self".to_string());
            }
        }
        self.print_signature(iface, func, TypeMode::Owned, &sig);
        self.src.push_str(";");
        self.in_trait = false;
        let trait_ = self
            .traits
            .entry(iface.name.to_camel_case())
            .or_insert(Trait::default());
        let dst = match &func.kind {
            FunctionKind::Freestanding => &mut trait_.methods,
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => trait_
                .resource_methods
                .entry(*resource)
                .or_insert(Vec::new()),
        };
        dst.push(mem::replace(&mut self.src, prev).into());
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        let mut src = mem::take(&mut self.src);

        let any_async = iface.functions.iter().any(|f| f.is_async);
        for (name, trait_) in self.traits.iter() {
            if any_async {
                src.push_str("#[wai_bindgen_rust::async_trait(?Send)]\n");
            }
            src.push_str("pub trait ");
            src.push_str(&name);
            src.push_str(" {\n");
            for f in trait_.methods.iter() {
                src.push_str(&f);
                src.push_str("\n");
            }
            src.push_str("}\n");

            for (id, methods) in trait_.resource_methods.iter() {
                if any_async {
                    src.push_str("#[wai_bindgen_rust::async_trait(?Send)]\n");
                }
                src.push_str(&format!(
                    "pub trait {} {{\n",
                    iface.resources[*id].name.to_camel_case()
                ));
                for f in methods {
                    src.push_str(&f);
                    src.push_str("\n");
                }
                src.push_str("}\n");
            }
        }

        if self.i64_return_pointer_area_size > 0 {
            src.push_str(&format!(
                "#[cfg(any(target_arch = \"wasm32\", target_arch = \"wasm64\"))]
                static mut RET_AREA: [i64; {0}] = [0; {0}];\n",
                self.i64_return_pointer_area_size,
            ));
        }

        // Close the opening `mod`.
        src.push_str("}\n");

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
    gen: &'a mut RustWasm,
    params: Vec<String>,
    src: Source,
    blocks: Vec<String>,
    block_storage: Vec<(Source, Vec<(String, String)>)>,
    tmp: usize,
    needs_cleanup_list: bool,
    cleanup: Vec<(String, String)>,
    is_dtor: bool,
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut RustWasm, is_dtor: bool, params: Vec<String>) -> FunctionBindgen<'_> {
        FunctionBindgen {
            gen,
            params,
            is_dtor,
            src: Default::default(),
            blocks: Vec::new(),
            block_storage: Vec::new(),
            tmp: 0,
            needs_cleanup_list: false,
            cleanup: Vec::new(),
        }
    }

    fn emit_cleanup(&mut self) {
        for (ptr, layout) in mem::take(&mut self.cleanup) {
            self.push_str(&format!("std::alloc::dealloc({}, {});\n", ptr, layout));
        }
        if self.needs_cleanup_list {
            self.push_str(
                "for (ptr, layout) in cleanup_list {
                    std::alloc::dealloc(ptr, layout);
                }\n",
            );
        }
    }

    fn declare_import(
        &mut self,
        module: &str,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        // Define the actual function we're calling inline
        self.push_str("#[link(wasm_import_module = \"");
        self.push_str(module);
        self.push_str("\")]\n");
        self.push_str("extern \"C\" {\n");
        self.push_str("#[cfg_attr(target_arch = \"wasm32\", link_name = \"");
        self.push_str(name);
        self.push_str("\")]\n");
        self.push_str("#[cfg_attr(not(target_arch = \"wasm32\"), link_name = \"");
        self.push_str(module);
        self.push_str("_");
        self.push_str(name);
        self.push_str("\")]\n");
        self.push_str("fn wai_import(");
        for param in params.iter() {
            self.push_str("_: ");
            self.push_str(wasm_type(*param));
            self.push_str(", ");
        }
        self.push_str(")");
        assert!(results.len() < 2);
        for result in results.iter() {
            self.push_str(" -> ");
            self.push_str(wasm_type(*result));
        }
        self.push_str(";\n}\n");
        "wai_import".to_string()
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
            LiftLower::LowerArgsLiftResults
        } else {
            LiftLower::LiftArgsLowerResults
        }
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn push_block(&mut self) {
        let prev_src = mem::take(&mut self.src);
        let prev_cleanup = mem::take(&mut self.cleanup);
        self.block_storage.push((prev_src, prev_cleanup));
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        if self.cleanup.len() > 0 {
            self.needs_cleanup_list = true;
            self.push_str("cleanup_list.extend_from_slice(&[");
            for (ptr, layout) in mem::take(&mut self.cleanup) {
                self.push_str("(");
                self.push_str(&ptr);
                self.push_str(", ");
                self.push_str(&layout);
                self.push_str("),");
            }
            self.push_str("]);\n");
        }
        let (prev_src, prev_cleanup) = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, prev_src);
        self.cleanup = prev_cleanup;
        let expr = match operands.len() {
            0 => "()".to_string(),
            1 => operands[0].clone(),
            _ => format!("({})", operands.join(", ")),
        };
        if src.is_empty() {
            self.blocks.push(expr);
        } else if operands.is_empty() {
            self.blocks.push(format!("{{\n{}\n}}", &src[..]));
        } else {
            self.blocks.push(format!("{{\n{}\n{}\n}}", &src[..], expr));
        }
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, ty: TypeId) -> String {
        let tmp = self.tmp();
        self.push_str(&format!(
            "let mut rp{} = core::mem::MaybeUninit::<[u8;",
            tmp
        ));
        let size = self.gen.sizes.size(&Type::Id(ty));
        self.push_str(&size.to_string());
        self.push_str("]>::uninit();\n");
        self.push_str(&format!("let ptr{} = rp{0}.as_mut_ptr() as i32;\n", tmp));
        format!("ptr{}", tmp)
    }

    fn i64_return_pointer_area(&mut self, amt: usize) -> String {
        assert!(amt <= self.gen.i64_return_pointer_area_size);
        let tmp = self.tmp();

        self.push_str("#[cfg(target_arch = \"wasm32\")]");
        self.push_str(&format!("let ptr{} = RET_AREA.as_mut_ptr() as i32;\n", tmp));
        if self.gen.in_import {
            self.push_str("#[cfg(not(target_arch = \"wasm32\"))]");
            self.push_str(&format!("let ptr{} = &mut [0i64; {}] as *mut i64 as i32;\n", tmp, amt));
        } else {
            self.push_str("#[cfg(not(target_arch = \"wasm32\"))]");
            self.push_str(&format!("let ptr{} = ret as i32;\n", tmp));
        }
        format!("ptr{}", tmp)
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
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
        let unchecked = self.gen.opts.unchecked;
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
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
                results.push(format!("wai_bindgen_rust::rt::as_i64({})", s));
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
                results.push(format!("wai_bindgen_rust::rt::as_i32({})", s));
            }

            Instruction::F32FromIf32 => {
                let s = operands.pop().unwrap();
                results.push(format!("wai_bindgen_rust::rt::as_f32({})", s));
            }
            Instruction::F64FromIf64 => {
                let s = operands.pop().unwrap();
                results.push(format!("wai_bindgen_rust::rt::as_f64({})", s));
            }
            Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }
            Instruction::S8FromI32 => top_as("i8"),
            Instruction::Char8FromI32 | Instruction::U8FromI32 => top_as("u8"),
            Instruction::S16FromI32 => top_as("i16"),
            Instruction::U16FromI32 => top_as("u16"),
            Instruction::U32FromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),
            Instruction::UsizeFromI32 => top_as("usize"),
            Instruction::CharFromI32 => {
                if unchecked {
                    results.push(format!(
                        "core::char::from_u32_unchecked({} as u32)",
                        operands[0]
                    ));
                } else {
                    results.push(format!(
                        "core::char::from_u32({} as u32).unwrap()",
                        operands[0]
                    ));
                }
            }

            Instruction::Bitcasts { casts } => {
                wai_bindgen_gen_rust::bitcast(casts, operands, results)
            }

            // handles in exports
            Instruction::I32FromOwnedHandle { .. } => {
                results.push(format!(
                    "wai_bindgen_rust::Handle::into_raw({})",
                    operands[0]
                ));
            }
            Instruction::HandleBorrowedFromI32 { .. } => {
                assert!(!self.is_dtor);
                results.push(format!(
                    "wai_bindgen_rust::Handle::from_raw({})",
                    operands[0],
                ));
            }

            // handles in imports
            Instruction::I32FromBorrowedHandle { .. } => {
                if self.is_dtor {
                    results.push(format!("{}.into_raw()", operands[0]));
                } else {
                    results.push(format!("{}.0", operands[0]));
                }
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}({})",
                    iface.resources[*ty].name.to_camel_case(),
                    operands[0]
                ));
            }

            Instruction::FlagsLower { record, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
                for i in 0..record.num_i32s() {
                    results.push(format!("(flags{} >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLower64 { .. } => {
                let s = operands.pop().unwrap();
                results.push(format!("wai_bindgen_rust::rt::as_i64({})", s));
            }
            Instruction::FlagsLift { name, .. } | Instruction::FlagsLift64 { name, .. } => {
                let name = name.to_camel_case();
                let mut result = String::from("0");
                for (i, op) in operands.iter().enumerate() {
                    result.push_str(&format!("| (({} as {}) << {})", op, name, i * 32));
                }
                results.push(result);
            }

            Instruction::RecordLower { ty, record, .. } => {
                self.record_lower(iface, *ty, record, &operands[0], results);
            }
            Instruction::RecordLift { ty, record, .. } => {
                self.record_lift(iface, *ty, record, operands, results);
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

            // In unchecked mode when this type is a named enum then we know we
            // defined the type so we can transmute directly into it.
            Instruction::VariantLift {
                name: Some(name),
                variant,
                ..
            } if variant.cases.iter().all(|c| c.ty.is_none()) && unchecked => {
                self.blocks.drain(self.blocks.len() - variant.cases.len()..);
                let mut result = format!("core::mem::transmute::<_, ");
                result.push_str(&name.to_camel_case());
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push_str(" as ");
                result.push_str(int_repr(variant.tag));
                result.push_str(")");
                results.push(result);
            }

            Instruction::VariantLift { variant, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let mut result = format!("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    if i == variant.cases.len() - 1 && unchecked {
                        result.push_str("_");
                    } else {
                        result.push_str(&i.to_string());
                    }
                    result.push_str(" => ");
                    self.variant_lift_case(iface, *ty, variant, case, &block, &mut result);
                    result.push_str(",\n");
                }
                if !unchecked {
                    result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::ListCanonLower { element, realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = match element {
                        Type::Char => {
                            format!("{}.into_bytes()", operands[0])
                        }
                        _ => operands.pop().unwrap(),
                    };
                    self.push_str(&format!("let {} = ({}).into_boxed_slice();\n", val, op0));
                }
                self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, val));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, val));
                if realloc.is_some() {
                    self.push_str(&format!("core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListCanonLift { element, free, .. } => {
                // This only happens when we're receiving a list from the
                // outside world, so `free` should always be `Some`.
                assert!(free.is_some());
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                match element {
                    Type::Char => {
                        if unchecked {
                            results.push(format!("String::from_utf8_unchecked({})", result));
                        } else {
                            results.push(format!("String::from_utf8({}).unwrap()", result));
                        }
                    }
                    _ => results.push(result),
                }
            }

            Instruction::ListLower { element, realloc } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let layout = format!("layout{}", tmp);
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                self.push_str(&format!(
                    "let {} = core::alloc::Layout::from_size_align_unchecked({}.len() * {}, {});\n",
                    layout, vec, size, align,
                ));
                self.push_str(&format!(
                    "let {} = std::alloc::alloc({});\n",
                    result, layout,
                ));
                self.push_str(&format!(
                    "if {}.is_null() {{ std::alloc::handle_alloc_error({}); }}\n",
                    result, layout,
                ));
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!(
                    "let base = {} as i32 + (i as i32) * {};\n",
                    result, size,
                ));
                self.push_str(&body);
                self.push_str("}\n");
                results.push(format!("{} as i32", result));
                results.push(len);

                if realloc.is_none() {
                    // If an allocator isn't requested then we must clean up the
                    // allocation ourselves since our callee isn't taking
                    // ownership.
                    self.cleanup.push((result, layout));
                }
            }

            Instruction::ListLift { element, free, .. } => {
                // This only happens when we're receiving a list from the
                // outside world, so `free` should always be `Some`.
                assert!(free.is_some());
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{}", tmp);
                let base = format!("base{}", tmp);
                let result = format!("result{}", tmp);
                self.push_str(&format!("let {} = {};\n", base, operands[0]));
                self.push_str(&format!("let {} = {};\n", len, operands[1],));
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
                // To keep the api implementation identical for native and wasm
                // the api is still owned. But the caller deallocates the resources.
                self.push_str(&format!(
                    "#[cfg(target_arch = \"wasm32\")]
                    std::alloc::dealloc(
                        {} as *mut _,
                        std::alloc::Layout::from_size_align_unchecked(
                            ({} as usize) * {},
                            {},
                        ),
                    );\n",
                    base, len, size, align
                ));
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            // Never used due to the call modes that this binding generator
            // uses
            Instruction::BufferLowerHandle { .. } => unimplemented!(),
            Instruction::BufferLiftPtrLen { .. } => unimplemented!(),

            Instruction::BufferLowerPtrLen { push, ty } => {
                let block = self.blocks.pop().unwrap();
                let size = self.gen.sizes.size(ty);
                let tmp = self.tmp();
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if iface.all_bits_valid(ty) {
                    let vec = format!("vec{}", tmp);
                    self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                    self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, vec));
                    self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));
                } else {
                    if *push {
                        self.push_str("let (");
                        self.push_str(&ptr);
                        self.push_str(", ");
                        self.push_str(&len);
                        self.push_str(") = ");
                        self.push_str(&operands[0]);
                        self.push_str(".ptr_len::<");
                        self.push_str(&size.to_string());
                        self.push_str(">(|base| {\n");
                        self.push_str(&block);
                        self.push_str("});\n");
                    } else {
                        self.push_str("let (");
                        self.push_str(&ptr);
                        self.push_str(", ");
                        self.push_str(&len);
                        self.push_str(") = ");
                        self.push_str(&operands[0]);
                        self.push_str(".serialize::<_, ");
                        self.push_str(&size.to_string());
                        self.push_str(">(|e, base| {\n");
                        self.push_str(&block);
                        self.push_str("});\n");
                    }
                }
                results.push("0".to_string());
                results.push(ptr);
                results.push(len);
            }

            Instruction::BufferLiftHandle { push, ty } => {
                let block = self.blocks.pop().unwrap();
                let size = self.gen.sizes.size(ty);
                let mut result = String::from("wai_bindgen_rust::exports::");
                if *push {
                    result.push_str("Push");
                } else {
                    result.push_str("Pull");
                }
                result.push_str("Buffer");
                if iface.all_bits_valid(ty) {
                    result.push_str("Raw::new(");
                    result.push_str(&operands[0]);
                    result.push_str(")");
                } else {
                    result.push_str("::new(");
                    result.push_str(&operands[0]);
                    result.push_str(", ");
                    result.push_str(&size.to_string());
                    result.push_str(", ");
                    if *push {
                        result.push_str("|base, e|");
                        result.push_str(&block);
                    } else {
                        result.push_str("|base|");
                        result.push_str(&block);
                    }
                    result.push_str(")");
                }
                results.push(result);
            }

            Instruction::CallWasm { module, name, sig } => {
                let func = self.declare_import(module, name, &sig.params, &sig.results);

                // ... then call the function with all our operands
                if sig.results.len() > 0 {
                    self.push_str("let ret = ");
                    results.push("ret".to_string());
                }
                self.push_str(&func);
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");\n");
            }

            Instruction::CallWasmAsyncImport {
                module,
                name,
                params: wasm_params,
                results: wasm_results,
            } => {
                // The first thing we do here is define the completion callback
                // which the host will invoke when the asynchronous call
                // actually finishes. This receives our own custom state
                // parameter as the first parameter which is the `Sender`
                // converted to a `usize`. Afterwards it receives all the
                // results which we'll transfer ove the `sender`, the canonical
                // ABI of the results.
                self.push_str("unsafe extern \"C\" fn completion_callback(sender: usize");
                for (i, result) in wasm_results.iter().enumerate() {
                    self.push_str(", ");
                    self.push_str(&format!("ret{}: ", i));
                    self.push_str(wasm_type(*result));
                }
                self.push_str(") {\n");
                self.push_str("wai_bindgen_rust::rt::Sender::from_usize(sender).send((");
                for i in 0..wasm_results.len() {
                    self.push_str(&format!("ret{},", i));
                }
                self.push_str("));\n");
                self.push_str("}\n");

                // Next we create the future channel which will be used to track
                // the state of this import. The "oneshot" here means that the
                // sender (`tx`) will send something once over `rx`. The type of
                // the `Oneshot` is the type of the `wasm_results` which is the
                // canonical ABI of the results that this function produces.
                self.push_str("let (rx, tx) = wai_bindgen_rust::rt::Oneshot::<(");
                for ty in *wasm_results {
                    self.push_str(wasm_type(*ty));
                    self.push_str(", ");
                }
                self.push_str(")>::new();\n");

                // Then we can actually call the function now that we have
                // all the parameters. The first parameters to the import are
                // the canonical ABI `operands` we were provided, and the last
                // two arguments are our completion callback and the context for
                // the callback, our `tx` sender.
                let func = self.declare_import(module, name, wasm_params, &[]);
                self.push_str(&func);
                self.push_str("(");
                for op in operands {
                    self.push_str(op);
                    self.push_str(", ");
                }
                self.push_str("completion_callback as i32, ");
                self.push_str("tx.into_usize() as i32");
                self.push_str(");\n");

                // And finally we want to "appear synchronous" with an async
                // function, so we immediately `.await` the results of the
                // oneshot. This binds all the canonical ABI results to then get
                // translated in further instructions to the result of this
                // function call.
                let tmp = self.tmp();
                self.push_str("let (");
                for i in 0..wasm_results.len() {
                    let name = format!("ret{}_{}", tmp, i);
                    self.push_str(&name);
                    self.push_str(",");
                    results.push(name);
                }
                self.push_str(") = rx.await;\n");
            }

            Instruction::CallWasmAsyncExport { .. } => unreachable!(),

            Instruction::CallInterface { module, func } => {
                self.let_results(func.results.len(), results);
                match &func.kind {
                    FunctionKind::Freestanding => {
                        self.push_str(&format!(
                            "<super::{m} as {m}>::{}",
                            func.name.to_snake_case(),
                            m = module.to_camel_case()
                        ));
                    }
                    FunctionKind::Static { resource, name }
                    | FunctionKind::Method { resource, name } => {
                        self.push_str(&format!(
                            "<super::{r} as {r}>::{}",
                            name.to_snake_case(),
                            r = iface.resources[*resource].name.to_camel_case(),
                        ));
                    }
                }
                self.push_str("(");
                if let FunctionKind::Method { .. } = func.kind {
                    self.push_str("&");
                }
                self.push_str(&operands.join(", "));
                self.push_str(")");
                if func.is_async {
                    self.push_str(".await");
                }
                self.push_str(";\n");
            }

            Instruction::Return { amt, .. } => {
                self.emit_cleanup();
                match amt {
                    0 => {}
                    1 => {
                        self.push_str(&operands[0]);
                        self.push_str("\n");
                    }
                    _ => {
                        self.push_str("(");
                        self.push_str(&operands.join(", "));
                        self.push_str(")\n");
                    }
                }
            }

            Instruction::ReturnAsyncExport { .. } => {
                self.emit_cleanup();
                self.push_str(&format!(
                    "unsafe {{ wai_bindgen_rust::rt::async_export_done({}, {}); }}\n",
                    operands[0], operands[1]
                ));
            }
            Instruction::ReturnAsyncImport { .. } => unreachable!(),

            Instruction::I32Load { offset } => {
                results.push(format!("*(({} + {}) as *const i32)", operands[0], offset));
            }
            Instruction::I32Load8U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u16))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i16))",
                    operands[0], offset
                ));
            }
            Instruction::I64Load { offset } => {
                results.push(format!("*(({} + {}) as *const i64)", operands[0], offset));
            }
            Instruction::F32Load { offset } => {
                results.push(format!("*(({} + {}) as *const f32)", operands[0], offset));
            }
            Instruction::F64Load { offset } => {
                results.push(format!("*(({} + {}) as *const f64)", operands[0], offset));
            }
            Instruction::I32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u8) = ({}) as u8;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u16) = ({}) as u16;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }

            Instruction::Witx { instr } => match instr {
                WitxInstruction::I32FromPointer => top_as("i32"),
                WitxInstruction::I32FromConstPointer => top_as("i32"),
                WitxInstruction::ReuseReturn => results.push("ret".to_string()),
                WitxInstruction::AddrOf => {
                    let i = self.tmp();
                    self.push_str(&format!("let t{} = {};\n", i, operands[0]));
                    results.push(format!("&t{} as *const _ as i32", i));
                }
                i => unimplemented!("{:?}", i),
            },
        }
    }
}
