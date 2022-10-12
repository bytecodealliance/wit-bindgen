use heck::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_bindgen_core::wit_parser::abi::{AbiVariant, Bindgen, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{wit_parser::*, Direction, Files, Generator, Source, TypeInfo, Types};
use wit_bindgen_gen_rust_lib::{
    int_repr, wasm_type, FnSig, RustFlagsRepr, RustFunctionGenerator, RustGenerator, TypeMode,
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
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
    sizes: SizeAlign,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rustfmt: bool,

    /// Adds the wit module name into import binding names when enabled.
    #[cfg_attr(feature = "clap", arg(long))]
    pub multi_module: bool,

    /// Whether or not the bindings assume interface values are always
    /// well-formed or whether checks are performed.
    #[cfg_attr(feature = "clap", arg(long))]
    pub unchecked: bool,

    /// A prefix to prepend to all exported symbols. Note that this is only
    /// intended for testing because it breaks the general form of the ABI.
    #[cfg_attr(feature = "clap", arg(skip))]
    pub symbol_namespace: String,

    /// If true, the code generation is intended for standalone crates.
    ///
    /// Standalone mode generates bindings without a wrapping module.
    ///
    /// For exported interfaces, an `export!` macro is also generated
    /// that can be used to export an implementation from a different
    /// crate.
    #[cfg_attr(feature = "clap", arg(skip))]
    pub standalone: bool,

    /// If true, code generation should avoid any features that depend on `std`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub no_std: bool,

    /// If true, code generation should pass borrowed string arguments as
    /// `&[u8]` instead of `&str`. Strings are still required to be valid
    /// UTF-8, but this avoids the need for Rust code to do its own UTF-8
    /// validation if it doesn't already have a `&str`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub raw_strings: bool,
}

#[derive(Default)]
struct Trait {
    methods: Vec<String>,
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

    fn ret_area_type_name(iface: &Interface) -> String {
        format!("__{}RetArea", iface.name.to_upper_camel_case())
    }

    fn ret_area_name(iface: &Interface) -> String {
        format!("__{}_RET_AREA", iface.name.to_shouty_snake_case())
    }
}

impl RustGenerator for RustWasm {
    fn use_std(&self) -> bool {
        !self.opts.no_std
    }

    fn use_raw_strings(&self) -> bool {
        self.opts.raw_strings
    }

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
        self.print_rust_slice(iface, mutbl, ty, lifetime);
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if self.opts.raw_strings {
            self.push_str("[u8]");
        } else {
            self.push_str("str");
        }
    }
}

impl Generator for RustWasm {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.in_import = variant == AbiVariant::GuestImport;
        self.types.analyze(iface);
        self.trait_name = iface.name.to_upper_camel_case();

        if !self.opts.standalone {
            self.src.push_str(&format!(
                "#[allow(clippy::all)]\nmod {} {{\n",
                iface.name.to_snake_case(),
            ));
        }

        self.src.push_str("#[allow(unused_imports)]");
        self.src
            .push_str("use wit_bindgen_guest_rust::rt::{alloc, vec::Vec, string::String};");

        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.print_typedef_record(iface, id, record, docs);
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
            .push_str("wit_bindgen_guest_rust::bitflags::bitflags! {\n");
        self.rustdoc(docs);
        let repr = RustFlagsRepr::new(flags);
        self.src.push_str(&format!(
            "pub struct {}: {repr} {{\n",
            name.to_upper_camel_case(),
        ));
        for (i, flag) in flags.flags.iter().enumerate() {
            self.rustdoc(&flag.docs);
            self.src.push_str(&format!(
                "const {} = 1 << {};\n",
                flag.name.to_shouty_snake_case(),
                i,
            ));
        }
        self.src.push_str("}\n");
        self.src.push_str("}\n");

        // Add a `from_bits_preserve` method.
        self.src
            .push_str(&format!("impl {} {{\n", name.to_upper_camel_case()));
        self.src.push_str(&format!(
            "    /// Convert from a raw integer, preserving any unknown bits. See\n"
        ));
        self.src.push_str(&format!(
            "    /// <https://github.com/bitflags/bitflags/issues/263#issuecomment-957088321>\n"
        ));
        self.src.push_str(&format!(
            "    pub fn from_bits_preserve(bits: {repr}) -> Self {{\n",
        ));
        self.src.push_str(&format!("        Self {{ bits }}\n"));
        self.src.push_str(&format!("    }}\n"));
        self.src.push_str(&format!("}}\n"));
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

    fn type_result(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        result: &Result_,
        docs: &Docs,
    ) {
        self.print_typedef_result(iface, id, result, docs);
    }

    fn type_enum(&mut self, _iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs);
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
            .push_str(&format!("pub type {}", name.to_upper_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push_str(";\n");
    }

    fn preprocess_functions(&mut self, _iface: &Interface, dir: Direction) {
        if self.opts.standalone && dir == Direction::Export {
            self.src.push_str(
                "/// Declares the export of the interface for the given type.\n\
                 #[macro_export]\n\
                 macro_rules! export(($t:ident) => {\n",
            );
        }
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        let sig = FnSig::default();
        let param_mode = TypeMode::AllBorrowed("'_");
        match &func.kind {
            FunctionKind::Freestanding => {}
        }
        let params = self.print_signature(iface, func, param_mode, &sig);
        self.src.push_str("{\n");
        self.src.push_str("unsafe {\n");

        let mut f = FunctionBindgen::new(self, params);
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

        self.src.push_str("}\n");
        self.src.push_str("}\n");

        match &func.kind {
            FunctionKind::Freestanding => {}
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        let iface_name = iface.name.to_snake_case();

        let name_snake = func.name.to_snake_case();
        let name = match &iface.module {
            Some(module) => {
                format!("{module}#{}", func.name)
            }
            None => format!("{}{}", self.opts.symbol_namespace, func.name),
        };

        self.src.push_str(&format!("#[export_name = \"{name}\"]\n"));
        self.src.push_str("unsafe extern \"C\" fn __wit_bindgen_");
        self.src.push_str(&iface_name);
        self.src.push_str("_");
        self.src.push_str(&name_snake);
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
        self.src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                self.src.push_str(" -> ");
                self.wasm_type(sig.results[0]);
            }
            _ => unimplemented!(),
        }

        self.push_str("{\n");

        if self.opts.standalone {
            // Force the macro code to reference wit_bindgen_guest_rust for standalone crates.
            // Also ensure any referenced types are also used from the external crate.
            self.src
                .push_str("#[allow(unused_imports)]\nuse wit_bindgen_guest_rust;\nuse ");
            self.src.push_str(&iface_name);
            self.src.push_str("::*;\n");
        }

        let mut f = FunctionBindgen::new(self, params);
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
        self.src.push_str("}\n");

        if iface.guest_export_needs_post_return(func) {
            self.src.push_str(&format!(
                "#[export_name = \"{}cabi_post_{}\"]\n",
                self.opts.symbol_namespace, func.name,
            ));
            self.src.push_str(&format!(
                "unsafe extern \"C\" fn __wit_bindgen_{iface_name}_{name_snake}_post_return("
            ));
            let mut params = Vec::new();
            for (i, result) in sig.results.iter().enumerate() {
                let name = format!("arg{}", i);
                self.src.push_str(&name);
                self.src.push_str(": ");
                self.wasm_type(*result);
                self.src.push_str(", ");
                params.push(name);
            }
            self.src.push_str(") {\n");

            let mut f = FunctionBindgen::new(self, params);
            iface.post_return(func, &mut f);
            let FunctionBindgen {
                needs_cleanup_list,
                src,
                ..
            } = f;
            assert!(!needs_cleanup_list);
            self.src.push_str(&String::from(src));
            self.src.push_str("}\n");
        }

        let prev = mem::take(&mut self.src);
        self.in_trait = true;
        let mut sig = FnSig::default();
        sig.private = true;
        self.print_signature(iface, func, TypeMode::Owned, &sig);
        self.src.push_str(";");
        self.in_trait = false;
        let trait_ = self
            .traits
            .entry(iface.name.to_upper_camel_case())
            .or_insert(Trait::default());
        let dst = match &func.kind {
            FunctionKind::Freestanding => &mut trait_.methods,
        };
        dst.push(mem::replace(&mut self.src, prev).into());
    }

    fn finish_functions(&mut self, iface: &Interface, dir: Direction) {
        if !self.in_import && self.return_pointer_area_align > 0 {
            self.src.push_str(&format!(
                "
                    #[repr(align({align}))]
                    struct {ty}([u8; {size}]);
                    static mut {name}: {ty} = {ty}([0; {size}]);
                ",
                ty = Self::ret_area_type_name(iface),
                name = Self::ret_area_name(iface),
                align = self.return_pointer_area_align,
                size = self.return_pointer_area_size,
            ));
        }

        self.src.push_str("#[cfg(target_arch = \"wasm32\")]\n");

        // The custom section name here must start with "component-type" but
        // otherwise is attempted to be unique here to ensure that this doesn't get
        // concatenated to other custom sections by LLD by accident since LLD will
        // concatenate custom sections of the same name.
        let direction = match dir {
            Direction::Import => "import",
            Direction::Export => "export",
        };
        let iface_name = &iface.name;
        self.src.push_str(&format!(
            "#[link_section = \"component-type:{direction}:{iface_name}\"]\n"
        ));

        let mut encoder = wit_component::ComponentEncoder::default();
        encoder = match dir {
            Direction::Import => encoder.imports([iface.clone()]).unwrap(),
            Direction::Export => encoder.interface(iface.clone()).unwrap(),
        };
        let component_type = encoder.types_only(true).encode().expect(&format!(
            "encoding interface {} as a component type",
            iface.name
        ));
        self.src.push_str(&format!(
            "pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; {}] = ",
            component_type.len()
        ));
        self.src.push_str(&format!("{:?};\n", component_type));

        // For standalone generation, close the export! macro
        if self.opts.standalone && dir == Direction::Export {
            self.src.push_str("});\n");
        }
    }

    fn finish_one(&mut self, _iface: &Interface, files: &mut Files) {
        let mut src = mem::take(&mut self.src);

        for (name, trait_) in self.traits.iter() {
            src.push_str("pub trait ");
            src.push_str(&name);
            src.push_str(" {\n");
            for f in trait_.methods.iter() {
                src.push_str(&f);
                src.push_str("\n");
            }
            src.push_str("}\n");
        }

        // Close the opening `mod`.
        if !self.opts.standalone {
            src.push_str("}\n");
        }

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
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut RustWasm, params: Vec<String>) -> FunctionBindgen<'_> {
        FunctionBindgen {
            gen,
            params,
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
            self.push_str(&format!(
                "if {layout}.size() != 0 {{\nalloc::dealloc({ptr}, {layout});\n}}\n"
            ));
        }
        if self.needs_cleanup_list {
            self.push_str(
                "for (ptr, layout) in cleanup_list {\n
                    if layout.size() != 0 {\n
                        alloc::dealloc(ptr, layout);\n
                    }\n
                }\n",
            );
        }
    }

    fn declare_import(
        &mut self,
        iface: &Interface,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        let module = iface.module.as_deref().unwrap_or(&iface.name);

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
        self.push_str("fn wit_import(");
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
        "wit_import".to_string()
    }

    fn ret_area_name(&self, iface: &Interface) -> String {
        // For imports, we allocate the return area on the stack; for exports,
        // we statically allocate it.
        if self.gen.in_import {
            format!("__{}_ret_area", iface.name.to_snake_case())
        } else {
            format!("__{}_RET_AREA", iface.name.to_shouty_snake_case())
        }
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

    fn return_pointer(&mut self, iface: &Interface, size: usize, align: usize) -> String {
        self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
        self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
        let tmp = self.tmp();

        if self.gen.in_import {
            self.push_str(&format!(
                "
                    #[repr(align({align}))]
                    struct {ty}([u8; {size}]);
                    let mut {name}: {ty} = {ty}([0; {size}]);
                ",
                ty = RustWasm::ret_area_type_name(iface),
                name = self.ret_area_name(iface),
            ));
        }

        self.push_str(&format!(
            "let ptr{} = {}.0.as_mut_ptr() as i32;\n",
            tmp,
            self.ret_area_name(iface),
        ));
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
                results.push(format!("wit_bindgen_guest_rust::rt::as_i64({})", s));
            }
            Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!("wit_bindgen_guest_rust::rt::as_i32({})", s));
            }

            Instruction::F32FromFloat32 => {
                let s = operands.pop().unwrap();
                results.push(format!("wit_bindgen_guest_rust::rt::as_f32({})", s));
            }
            Instruction::F64FromFloat64 => {
                let s = operands.pop().unwrap();
                results.push(format!("wit_bindgen_guest_rust::rt::as_f64({})", s));
            }
            Instruction::Float32FromF32
            | Instruction::Float64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }
            Instruction::S8FromI32 => top_as("i8"),
            Instruction::U8FromI32 => top_as("u8"),
            Instruction::S16FromI32 => top_as("i16"),
            Instruction::U16FromI32 => top_as("u16"),
            Instruction::U32FromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),
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
                wit_bindgen_gen_rust_lib::bitcast(casts, operands, results)
            }

            Instruction::I32FromBool => {
                results.push(format!("match {} {{ true => 1, false => 0 }}", operands[0]));
            }
            Instruction::BoolFromI32 => {
                if unchecked {
                    results.push(format!(
                        "core::mem::transmute::<u8, bool>({} as u8)",
                        operands[0],
                    ));
                } else {
                    results.push(format!(
                        "match {} {{
                            0 => false,
                            1 => true,
                            _ => panic!(\"invalid bool discriminant\"),
                        }}",
                        operands[0],
                    ));
                }
            }

            Instruction::FlagsLower { flags, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
                for i in 0..flags.repr().count() {
                    results.push(format!("(flags{}.bits() >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLift { name, flags, .. } => {
                let repr = RustFlagsRepr::new(flags);
                let name = name.to_upper_camel_case();
                let mut result = format!("{}::empty()", name);
                for (i, op) in operands.iter().enumerate() {
                    result.push_str(&format!(
                        " | {}::from_bits_preserve((({} as {repr}) << {}) as _)",
                        name,
                        op,
                        i * 32
                    ));
                }
                results.push(result);
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
                    let case_name = case.name.to_upper_camel_case();
                    self.push_str(&format!("{name}::{case_name}"));
                    if case.ty.is_some() {
                        self.push_str(&format!("(e) => {block},\n"));
                    } else {
                        self.push_str(&format!(" => {{\n{block}\n}}\n"));
                    }
                }
                self.push_str("};\n");
            }

            // In unchecked mode when this type is a named enum then we know we
            // defined the type so we can transmute directly into it.
            Instruction::VariantLift { name, variant, .. }
                if variant.cases.iter().all(|c| c.ty.is_none()) && unchecked =>
            {
                self.blocks.drain(self.blocks.len() - variant.cases.len()..);
                let mut result = format!("core::mem::transmute::<_, ");
                result.push_str(&name.to_upper_camel_case());
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push_str(" as ");
                result.push_str(int_repr(variant.tag()));
                result.push_str(")");
                results.push(result);
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
                    let pat = if i == variant.cases.len() - 1 && unchecked {
                        String::from("_")
                    } else {
                        i.to_string()
                    };
                    let block = if case.ty.is_some() {
                        format!("({block})")
                    } else {
                        String::new()
                    };
                    let case = case.name.to_upper_camel_case();
                    result.push_str(&format!("{pat} => {name}::{case}{block},\n"));
                }
                if !unchecked {
                    result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                }
                result.push_str("}");
                results.push(result);
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
                for (i, (case_name, block)) in self
                    .gen
                    .union_case_names(iface, union)
                    .into_iter()
                    .zip(blocks)
                    .enumerate()
                {
                    let pat = if i == union.cases.len() - 1 && unchecked {
                        String::from("_")
                    } else {
                        i.to_string()
                    };
                    let name = self.typename_lift(iface, *ty);
                    result.push_str(&format!("{pat} => {name}::{case_name}({block}),\n"));
                }
                if !unchecked {
                    result.push_str("_ => panic!(\"invalid union discriminant\"),\n");
                }
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
                        None => {{\n{none}\n}},
                    }};"
                ));
            }

            Instruction::OptionLift { .. } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                assert_eq!(none, "()");
                let operand = &operands[0];
                let invalid = if unchecked {
                    "core::hint::unreachable_unchecked()"
                } else {
                    "panic!(\"invalid enum discriminant\")"
                };
                results.push(format!(
                    "match {operand} {{
                        0 => None,
                        1 => Some({some}),
                        _ => {invalid},
                    }}"
                ));
            }

            Instruction::ResultLower {
                results: result_types,
                result,
                ..
            } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                self.let_results(result_types.len(), results);
                let operand = &operands[0];
                let ok_binding = if result.ok.is_some() { "e" } else { "_" };
                let err_binding = if result.err.is_some() { "e" } else { "_" };
                self.push_str(&format!(
                    "match {operand} {{
                        Ok({ok_binding}) => {{ {ok} }},
                        Err({err_binding}) => {{ {err} }},
                    }};"
                ));
            }

            Instruction::ResultLift { .. } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                let operand = &operands[0];
                let invalid = if unchecked {
                    "core::hint::unreachable_unchecked()"
                } else {
                    "panic!(\"invalid enum discriminant\")"
                };
                results.push(format!(
                    "match {operand} {{
                        0 => Ok({ok}),
                        1 => Err({err}),
                        _ => {invalid},
                    }}"
                ));
            }

            Instruction::EnumLower { enum_, name, .. } => {
                let mut result = format!("match {} {{\n", operands[0]);
                let name = name.to_upper_camel_case();
                for (i, case) in enum_.cases.iter().enumerate() {
                    let case = case.name.to_upper_camel_case();
                    result.push_str(&format!("{name}::{case} => {i},\n"));
                }
                result.push_str("}");
                results.push(result);
            }

            // In unchecked mode when this type is a named enum then we know we
            // defined the type so we can transmute directly into it.
            Instruction::EnumLift { enum_, name, .. } if unchecked => {
                let mut result = format!("core::mem::transmute::<_, ");
                result.push_str(&name.to_upper_camel_case());
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push_str(" as ");
                result.push_str(int_repr(enum_.tag()));
                result.push_str(")");
                results.push(result);
            }

            Instruction::EnumLift { enum_, name, .. } => {
                let mut result = format!("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                let name = name.to_upper_camel_case();
                for (i, case) in enum_.cases.iter().enumerate() {
                    let case = case.name.to_upper_camel_case();
                    result.push_str(&format!("{i} => {name}::{case},\n"));
                }
                result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                result.push_str("}");
                results.push(result);
            }

            Instruction::ListCanonLower { realloc, .. } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = operands.pop().unwrap();
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

            Instruction::ListCanonLift { .. } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                results.push(result);
            }

            Instruction::StringLower { realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = format!("{}.into_bytes()", operands[0]);
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

            Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                if self.gen.opts.raw_strings {
                    results.push(result);
                } else if unchecked {
                    results.push(format!("String::from_utf8_unchecked({})", result));
                } else {
                    results.push(format!("String::from_utf8({}).unwrap()", result));
                }
            }

            Instruction::ListLower { element, realloc } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{tmp}");
                let result = format!("result{tmp}");
                let layout = format!("layout{tmp}");
                let len = format!("len{tmp}");
                self.push_str(&format!(
                    "let {vec} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!("let {len} = {vec}.len() as i32;\n"));
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                self.push_str(&format!(
                    "let {layout} = alloc::Layout::from_size_align_unchecked({vec}.len() * {size}, {align});\n",
                ));
                self.push_str(&format!(
                    "let {result} = if {layout}.size() != 0\n{{\nlet ptr = alloc::alloc({layout});\n",
                ));
                self.push_str(&format!(
                    "if ptr.is_null()\n{{\nalloc::handle_alloc_error({layout});\n}}\nptr\n}}",
                ));
                self.push_str(&format!("else {{\ncore::ptr::null_mut()\n}};\n",));
                self.push_str(&format!("for (i, e) in {vec}.into_iter().enumerate() {{\n",));
                self.push_str(&format!(
                    "let base = {result} as i32 + (i as i32) * {size};\n",
                ));
                self.push_str(&body);
                self.push_str("}\n");
                results.push(format!("{result} as i32"));
                results.push(len);

                if realloc.is_none() {
                    // If an allocator isn't requested then we must clean up the
                    // allocation ourselves since our callee isn't taking
                    // ownership.
                    self.cleanup.push((result, layout));
                }
            }

            Instruction::ListLift { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{tmp}");
                let base = format!("base{tmp}");
                let result = format!("result{tmp}");
                self.push_str(&format!(
                    "let {base} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!(
                    "let {len} = {operand1};\n",
                    operand1 = operands[1]
                ));
                self.push_str(&format!(
                    "let mut {result} = Vec::with_capacity({len} as usize);\n",
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
                self.push_str(&format!(
                    "wit_bindgen_guest_rust::rt::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                ));
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm { iface, name, sig } => {
                let func = self.declare_import(iface, name, &sig.params, &sig.results);

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

            Instruction::CallInterface { module, func } => {
                self.let_results(func.results.len(), results);
                match &func.kind {
                    FunctionKind::Freestanding => {
                        if self.gen.opts.standalone {
                            // For standalone mode, use the macro identifier
                            self.push_str(&format!(
                                "<$t as {t}>::{}",
                                func.name.to_snake_case(),
                                t = module.to_upper_camel_case(),
                            ));
                        } else {
                            self.push_str(&format!(
                                "<super::{m} as {m}>::{}",
                                func.name.to_snake_case(),
                                m = module.to_upper_camel_case()
                            ));
                        }
                    }
                }
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(")");
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

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { size, align } => {
                self.push_str(&format!(
                    "wit_bindgen_guest_rust::rt::dealloc({}, {}, {});\n",
                    operands[0], size, align
                ));
            }

            Instruction::GuestDeallocateString => {
                self.push_str(&format!(
                    "wit_bindgen_guest_rust::rt::dealloc({}, ({}) as usize, 1);\n",
                    operands[0], operands[1],
                ));
            }

            Instruction::GuestDeallocateVariant { blocks } => {
                let max = blocks - 1;
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                self.src.push_str(&format!("match {op0} {{\n"));
                for (i, block) in blocks.into_iter().enumerate() {
                    let pat = if i == max {
                        String::from("_")
                    } else {
                        i.to_string()
                    };
                    self.src.push_str(&format!("{pat} => {block},\n"));
                }
                self.src.push_str("}\n");
            }

            Instruction::GuestDeallocateList { element } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{tmp}");
                let base = format!("base{tmp}");
                self.push_str(&format!(
                    "let {base} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!(
                    "let {len} = {operand1};\n",
                    operand1 = operands[1]
                ));

                if body != "()" {
                    self.push_str("for i in 0..");
                    self.push_str(&len);
                    self.push_str(" {\n");
                    self.push_str("let base = ");
                    self.push_str(&base);
                    self.push_str(" + i *");
                    self.push_str(&size.to_string());
                    self.push_str(";\n");
                    self.push_str(&body);
                    self.push_str("\n}\n");
                }
                self.push_str(&format!(
                    "wit_bindgen_guest_rust::rt::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                ));
            }
        }
    }
}
