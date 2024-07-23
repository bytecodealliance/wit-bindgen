use anyhow::Result;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToUpperCamelCase};
use std::{collections::HashMap, fmt::Write, iter, mem, ops::Deref};
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Int, InterfaceId, Record, Resolve,
        Result_, SizeAlign, Tuple, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, Variant, WorldId,
        WorldKey,
    },
    Direction, Files, InterfaceGenerator as _, Ns, Source, WorldGenerator,
};

// Assumptions:
// Data: u8 -> Byte, s8 | s16 | u16 | s32 -> Int, u32 -> UInt, s64 -> Int64, u64 -> UInt64, f32 | f64 -> Double, address -> Int

/* FFI:
extern "wasm" fn extend16(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend16_s)

extern "wasm" fn extend8(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend8_s)

extern "wasm" fn store8(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store8)

extern "wasm" fn load8_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_u)

extern "wasm" fn load8(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_s)

extern "wasm" fn store16(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store16)

extern "wasm" fn load16(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_s)

extern "wasm" fn load16_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_u)

extern "wasm" fn store32(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store)

extern "wasm" fn load32(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load)

extern "wasm" fn store64(offset : Int, value : Int64) =
  #|(func (param i32) (param i64) local.get 0 local.get 1 i64.store)

extern "wasm" fn load64(offset : Int) -> Int =
  #|(func (param i32) (result i64) local.get 0 i64.load)

extern "wasm" fn storef32(offset : Int, value : Double) =
  #|(func (param i32) (param i64) local.get 0 local.get 1 f32.demote_f64 f32.store)

extern "wasm" fn loadf32(offset : Int) -> Double =
  #|(func (param i32) (result f64) local.get 0 f32.load f64.promote_f32)

extern "wasm" fn storef64(offset : Int, value : Double) =
  #|(func (param i32) (param f64) local.get 0 local.get 1 f64.store)

extern "wasm" fn loadf64(offset : Int) -> Int =
  #|(func (param i32) (result f64) local.get 0 f64.load)

extern "wasm" fn malloc(size : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 call $rael.malloc)

extern "wasm" fn free(position : Int) =
  #|(func (param i32) local.get 0 call $rael.free)

extern "wasm" fn copy(dest : Int, src : Int, len : Int) =
  #|(func (param i32) (param i32) (param i32) local.get 0 local.get 1 local.get 2 memory.copy)
 */

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(MoonBit {
            opts: self.clone(),
            ..MoonBit::default()
        })
    }
}

struct InterfaceFragment {
    src: String,
    stub: String,
}

#[derive(Default)]
pub struct MoonBit {
    opts: Opts,
    name: String,
    return_area_size: usize,
    return_area_align: usize,
    needs_cleanup: bool,
    interface_fragments: HashMap<String, Vec<InterfaceFragment>>,
    world_fragments: Vec<InterfaceFragment>,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, String>,
    export: HashMap<String, String>,
}

impl MoonBit {
    fn qualifier(&self) -> String {
        format!("{}.", self.name)
    }

    fn interface<'a>(&'a mut self, resolve: &'a Resolve, name: &'a str) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            stub: String::new(),
            gen: self,
            resolve,
            name,
        }
    }
}

impl WorldGenerator for MoonBit {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.name = world_name(resolve, world);
        self.sizes.fill(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = interface_name(resolve, key, Direction::Import);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(&resolve.name_world_key(key), func);
        }

        gen.add_interface_fragment();

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = world_name(resolve, world);
        let mut gen = self.interface(resolve, &name);

        for (_, func) in funcs {
            gen.import("$root", func);
        }

        gen.add_world_fragment();
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = interface_name(resolve, key, Direction::Export);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(Some(&resolve.name_world_key(key)), func);
        }

        gen.add_interface_fragment();
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = world_name(resolve, world);
        let mut gen = self.interface(resolve, &name);

        for (_, func) in funcs {
            gen.export(None, func);
        }

        gen.add_world_fragment();
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = world_name(resolve, world);
        let mut gen = self.interface(resolve, &name);

        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        let name = world_name(resolve, id);
        let (package, name) = split_qualified_name(&name);

        let mut src = Source::default();
        let version = env!("CARGO_PKG_VERSION");
        wit_bindgen_core::generated_preamble(&mut src, version);

        src.push_str(
            &self
                .world_fragments
                .iter()
                .map(|f| f.src.deref())
                .collect::<Vec<_>>()
                .join("\n"),
        );

        if self.needs_cleanup {
            src.push_str(
                "
                pub struct Cleanup {
                    address : Int
                    size : Int
                    align : Int
                }
                ",
            );
        }

        if self.return_area_align > 0 {
            let size = self.return_area_size;
            // let align = self.return_area_align;

            uwriteln!(src, "let wasi_RETURN_AREA : Int = malloc({size})",);
        }

        let directory = package.replace('.', "/");
        files.push(&format!("{directory}/{name}.mbt"), indent(&src).as_bytes());

        let generate_stub =
            |package: &str, name, fragments: &[InterfaceFragment], files: &mut Files| {
                let b = fragments
                    .iter()
                    .map(|f| f.stub.deref())
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut body = Source::default();
                wit_bindgen_core::generated_preamble(&mut body, version);
                uwriteln!(&mut body, "{b}");

                let directory = package.replace('.', "/");
                files.push(&format!("{directory}/{name}.mbt"), indent(&body).as_bytes());
            };

        if self.opts.generate_stub {
            generate_stub(
                &package,
                format!("{name}Impl"),
                &self.world_fragments,
                files,
            );
        }

        for (name, fragments) in &self.interface_fragments {
            let (package, name) = split_qualified_name(name);

            let b = fragments
                .iter()
                .map(|f| f.src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let mut body = Source::default();
            wit_bindgen_core::generated_preamble(&mut body, version);
            uwriteln!(&mut body, "{b}");

            let directory = package.replace('.', "/");
            files.push(&format!("{directory}/{name}.mbt"), indent(&body).as_bytes());

            if self.opts.generate_stub {
                generate_stub(&package, format!("{name}Impl"), fragments, files);
            }
        }

        let mut body = Source::default();
        uwriteln!(&mut body, "{{\"name\": \"wasi-bindgen\"}}");
        files.push(&format!("moon.mod.json"), indent(&body).as_bytes());

        let mut body = Source::default();
        let exports = self
            .export
            .iter()
            .map(|(k, v)| format!("\"{k}:{v}\""))
            .collect::<Vec<_>>()
            .join(", ");
        uwrite!(
            &mut body,
            r#"
            {{
                "link": {{
                    "wasm": {{
                        "export": [{exports}]
                    }}
                }}
            }}
            "#
        );
        files.push(&format!("moon.pkg.json"), indent(&body).as_bytes());
        
        Ok(())
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    stub: String,
    gen: &'a mut MoonBit,
    resolve: &'a Resolve,
    name: &'a str,
}

impl InterfaceGenerator<'_> {
    fn qualifier(&self, when: bool, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.gen.interface_names.get(id) {
                if name != self.name {
                    return format!("{name}.");
                }
            }
        }

        if when {
            format!("{}.", self.name)
        } else {
            String::new()
        }
    }

    fn add_interface_fragment(self) {
        self.gen
            .interface_fragments
            .entry(self.name.to_owned())
            .or_default()
            .push(InterfaceFragment {
                src: self.src,
                stub: self.stub,
            });
    }

    fn add_world_fragment(self) {
        self.gen.world_fragments.push(InterfaceFragment {
            src: self.src,
            stub: self.stub,
        });
    }

    fn import(&mut self, module: &str, func: &Function) {
        if func.kind != FunctionKind::Freestanding {
            todo!("resources");
        }

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            func.params
                .iter()
                .map(|(name, _)| name.to_moonbit_ident())
                .collect(),
        );

        abi::call(
            bindgen.gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
        );

        let src = bindgen.src;

        let cleanup_list = if bindgen.needs_cleanup_list {
            self.gen.needs_cleanup = true;

            format!(
                "let cleanupList : Array[{}Cleanup] = []\n",
                self.gen.qualifier()
            )
        } else {
            String::new()
        };

        let name = &func.name;

        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        let result_type = match &sig.results[..] {
            [] => "".into(),
            [result] => format!("-> {}", wasm_type(*result)),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("p{i} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let sig = self.sig_string(func, false);

        uwrite!(
            self.src,
            r#"fn wasmImport{camel_name}({params}) {result_type} = "{module}" "{name}";

            {sig} {{
              {cleanup_list}
              {src}
            }}
            "#
        );
    }

    fn export(&mut self, interface_name: Option<&str>, func: &Function) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let export_name = func.core_export_name(interface_name);

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        );

        abi::call(
            bindgen.gen.resolve,
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );

        assert!(!bindgen.needs_cleanup_list);

        let src = bindgen.src;

        let result_type = match &sig.results[..] {
            [] => "Unit",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("p{i} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        uwrite!(
            self.src,
            r#"
            /// @Export(name = "{export_name}")
            pub fn wasmExport{camel_name}({params}) -> {result_type} {{
                {src}
            }}
            "#
        );
        self.gen
            .export
            .insert(format!("wasmExport{camel_name}"), format!("{export_name}"));

        if abi::guest_export_needs_post_return(self.resolve, func) {
            let params = sig
                .results
                .iter()
                .enumerate()
                .map(|(i, param)| {
                    let ty = wasm_type(*param);
                    format!("p{i} : {ty}")
                })
                .collect::<Vec<_>>()
                .join(", ");

            let mut bindgen = FunctionBindgen::new(
                self,
                "INVALID",
                (0..sig.results.len()).map(|i| format!("p{i}")).collect(),
            );

            abi::post_return(bindgen.gen.resolve, func, &mut bindgen);

            let src = bindgen.src;

            uwrite!(
                self.src,
                r#"
                /// @Export(name = "cabi_post_{export_name}")
                pub fn wasmExport{camel_name}PostReturn({params}) -> Unit {{
                    {src}
                }}
                "#
            );
            self.gen.export.insert(
                format!("wasmExport{camel_name}PostReturn"),
                format!("cabi_post_{export_name}"),
            );
        }

        if self.gen.opts.generate_stub {
            let sig = self.sig_string(func, true);

            uwrite!(
                self.stub,
                r#"
                {sig} {{
                    abort("todo")
                }}
                "#
            );
        }
    }

    fn constructor_name(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "Byte".into(),
            Type::S32 | Type::S8 | Type::U16 | Type::S16 => "Int".into(),
            Type::U32 => "UInt".into(),
            Type::Char => "Char".into(),
            Type::U64 => "UInt64".into(),
            Type::S64 => "Int64".into(),
            Type::F32 | Type::F64 => "Double".into(),
            Type::String => "String".into(),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.constructor_name(ty),
                    TypeDefKind::List(_) => "Array".into(),
                    TypeDefKind::Tuple(_) => panic!(),
                    TypeDefKind::Option(_) => "Option".into(),
                    TypeDefKind::Result(_) => "Result".into(),
                    _ => {
                        if let Some(name) = &ty.name {
                            name.to_upper_camel_case()
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    fn type_name(&mut self, ty: &Type) -> String {
        self.type_name_with_qualifier(ty, false)
    }

    fn type_name_with_qualifier(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "Byte".into(),
            Type::S32 | Type::S8 | Type::U16 | Type::S16 => "Int".into(),
            Type::U32 => "UInt".into(),
            Type::Char => "Char".into(),
            Type::U64 => "UInt64".into(),
            Type::S64 => "Int64".into(),
            Type::F32 | Type::F64 => "Double".into(),
            Type::String => "String".into(),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.type_name_with_qualifier(ty, qualifier),
                    TypeDefKind::List(ty) => {
                        format!("Array[{}]", self.type_name_boxed(ty, qualifier))
                    }
                    TypeDefKind::Tuple(tuple) => {
                        format!(
                            "({})",
                            tuple
                                .types
                                .iter()
                                .map(|ty| self.type_name_boxed(ty, qualifier))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                    TypeDefKind::Option(ty) => {
                        format!("{}?", self.type_name_boxed(ty, qualifier))
                    }
                    TypeDefKind::Result(result) => {
                        let mut name = |ty: &Option<Type>| {
                            ty.as_ref()
                                .map(|ty| self.type_name_boxed(ty, qualifier))
                                .unwrap_or_else(|| "Unit".into())
                        };
                        let ok = name(&result.ok);
                        let err = name(&result.err);

                        format!("Result[{ok}, {err}]")
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(qualifier, ty),
                                name.to_upper_camel_case()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    fn type_name_boxed(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "Byte".into(),
            Type::S8 | Type::U16 | Type::S16 | Type::S32 => "Int".into(),
            Type::U32 => "UInt".into(),
            Type::Char => "Char".into(),
            Type::U64 => "UInt64".into(),
            Type::S64 => "Int64".into(),
            Type::F32 | Type::F64 => "Double".into(),
            Type::Id(id) => {
                let def = &self.resolve.types[*id];
                match &def.kind {
                    TypeDefKind::Type(ty) => self.type_name_boxed(ty, qualifier),
                    _ => self.type_name_with_qualifier(ty, qualifier),
                }
            }
            _ => self.type_name_with_qualifier(ty, qualifier),
        }
    }

    fn print_docs(&mut self, docs: &Docs) {
        if let Some(docs) = &docs.contents {
            let lines = docs
                .trim()
                .lines()
                .map(|line| format!("/// {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            uwrite!(self.src, "{}", lines)
        }
    }

    fn non_empty_type<'a>(&self, ty: Option<&'a Type>) -> Option<&'a Type> {
        if let Some(ty) = ty {
            let id = match ty {
                Type::Id(id) => *id,
                _ => return Some(ty),
            };
            match &self.resolve.types[id].kind {
                TypeDefKind::Type(t) => self.non_empty_type(Some(t)).map(|_| ty),
                TypeDefKind::Record(r) => (!r.fields.is_empty()).then_some(ty),
                TypeDefKind::Tuple(t) => (!t.types.is_empty()).then_some(ty),
                _ => Some(ty),
            }
        } else {
            None
        }
    }

    fn sig_string(&mut self, func: &Function, qualifier: bool) -> String {
        let name = func.name.to_moonbit_ident();

        let result_type = match func.results.len() {
            0 => "Unit".into(),
            1 => {
                self.type_name_with_qualifier(func.results.iter_types().next().unwrap(), qualifier)
            }
            _ => {
                format!(
                    "({})",
                    func.results
                        .iter_types()
                        .map(|ty| self.type_name_boxed(ty, qualifier))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name_with_qualifier(ty, qualifier);
                let name = name.to_moonbit_ident();
                format!("{name} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("pub fn {name}({params}) -> {result_type}")
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let parameters = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{} : {}",
                    field.name.to_moonbit_ident(),
                    self.type_name(&field.ty),
                )
            })
            .collect::<Vec<_>>()
            .join("; ");

        uwrite!(
            self.src,
            "
            pub struct {name} {{
                {parameters}
            }}
            "
        );
    }

    fn type_resource(&mut self, _id: TypeId, name: &str, docs: &Docs) {
        self.print_docs(docs);

        uwrite!(self.src, "pub type {name} UInt")
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let ty = match flags.repr() {
            FlagsRepr::U8 => "Byte",
            FlagsRepr::U16 | FlagsRepr::U32(1) => "UInt",
            FlagsRepr::U32(2) => "UInt64",
            repr => todo!("flags {repr:?}"),
        };

        let flags = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let flag_name = flag.name.to_shouty_snake_case();
                let suffix = if matches!(flags.repr(), FlagsRepr::U32(2)) {
                    "UL"
                } else {
                    "U"
                };
                let cast = if matches!(flags.repr(), FlagsRepr::U8) {
                    ".to_byte()"
                } else {
                    ""
                };
                format!("let {flag_name} : {name} = {name}((1{suffix} << {i}){cast})")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            pub type {name} {ty}
            pub fn {name}::lor(self : {name}, other: {name}) -> {name} {{
              self.0 | other.0
            }}
            {flags}
            "
        );
    }

    fn type_tuple(&mut self, _id: TypeId, _name: &str, _tuple: &Tuple, _docs: &Docs) {
        // Not needed
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let cases = variant
            .cases
            .iter()
            .map(|case| {
                let name = case.name.to_upper_camel_case();
                if let Some(ty) = case.ty {
                    let ty = self.type_name(&ty);
                    format!("{name}({ty})")
                } else {
                    format!("{name}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n  ");

        uwrite!(
            self.src,
            "
            pub enum {name} {{
              {cases}
            }}
            "
        );
    }

    fn type_option(&mut self, _id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        // Not needed
    }

    fn type_result(&mut self, _id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        // Not needed
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        // Type definition
        let cases = enum_
            .cases
            .iter()
            .map(|case| case.name.to_shouty_snake_case())
            .collect::<Vec<_>>()
            .join("; ");

        uwrite!(
            self.src,
            "
            pub enum {name} {{
                {cases}
            }}
            "
        );

        // Case to integer
        let cases = enum_
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| format!("{} => {i}", case.name.to_shouty_snake_case()))
            .collect::<Vec<_>>()
            .join("\n  ");

        uwrite!(
            self.src,
            "
            pub fn ordinal(self : {name}) -> Int {{
              match self {{
                {cases}
              }}
            }}
            "
        );

        // Integer to case
        let cases = enum_
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| format!("{i} => {}", case.name.to_shouty_snake_case()))
            .collect::<Vec<_>>()
            .join("\n  ");

        uwrite!(
            self.src,
            "
            pub fn {name}::from(self : Int) -> {name} {{
              match self {{
                {cases}
                _ => panic()
              }}
            }}
            "
        );
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        // TODO: Implement correct type aliasing
        self.type_name(&Type::Id(id));
    }

    fn type_list(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        // Not needed
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unimplemented!();
    }
}

struct Block {
    body: String,
    results: Vec<String>,
    element: String,
    base: String,
}

struct Cleanup {
    address: String,
    size: String,
    align: usize,
}

struct BlockStorage {
    body: String,
    element: String,
    base: String,
    cleanup: Vec<Cleanup>,
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    func_name: &'b str,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    payloads: Vec<String>,
    cleanup: Vec<Cleanup>,
    needs_cleanup_list: bool,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'b mut InterfaceGenerator<'a>,
        func_name: &'b str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        Self {
            gen,
            func_name,
            params,
            src: String::new(),
            locals: Ns::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            payloads: Vec::new(),
            cleanup: Vec::new(),
            needs_cleanup_list: false,
        }
    }

    fn lower_variant(
        &mut self,
        cases: &[(&str, Option<Type>)],
        lowered_types: &[WasmType],
        op: &str,
        results: &mut Vec<String>,
    ) {
        let blocks = self
            .blocks
            .drain(self.blocks.len() - cases.len()..)
            .collect::<Vec<_>>();

        let payloads = self
            .payloads
            .drain(self.payloads.len() - cases.len()..)
            .collect::<Vec<_>>();

        let lowered = lowered_types
            .iter()
            .map(|_| self.locals.tmp("lowered"))
            .collect::<Vec<_>>();

        results.extend(lowered.iter().cloned());

        let declarations = lowered.join(",");

        let cases = cases
            .iter()
            .zip(blocks)
            .zip(payloads)
            .map(|(((name, ty), Block { body, results, .. }), payload)| {
                let name = name.to_upper_camel_case();
                let assignments = results
                    .iter()
                    .map(|result| format!("{result}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let payload = if self.gen.non_empty_type(ty.as_ref()).is_some() {
                    payload
                } else {
                    String::new()
                };

                // TODO: This may cause empty constructor of Result::OK
                if payload.is_empty() {
                    format!(
                        "{name} => {{
                          {body}
                          ({assignments})
                        }}"
                    )
                } else {
                    format!(
                        "{name}({payload}) => {{
                          {body}
                          ({assignments})
                        }}"
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        if declarations.is_empty() {
            uwrite!(
                self.src,
                r#"
                match {op} {{
                    {cases}
                    _ => panic()
                }}
                "#
            );
        } else {
            uwrite!(
                self.src,
                r#"
                let ({declarations}) = match {op} {{
                    {cases}
                    _ => panic()
                }}
                "#
            );
        }
    }

    fn lift_variant(
        &mut self,
        ty: &Type,
        cases: &[(&str, Option<Type>)],
        op: &str,
        results: &mut Vec<String>,
    ) {
        let blocks = self
            .blocks
            .drain(self.blocks.len() - cases.len()..)
            .collect::<Vec<_>>();

        // Hacky way to get the type name without type parameter
        let ty = self.gen.constructor_name(ty);
        let lifted = self.locals.tmp("lifted");

        let cases = cases
            .iter()
            .zip(blocks)
            .enumerate()
            .map(|(i, ((case_name, case_ty), Block { body, results, .. }))| {
                let payload = if self.gen.non_empty_type(case_ty.as_ref()).is_some() {
                    results.into_iter().next().unwrap()
                } else {
                    String::new()
                };

                let constructor = format!("{ty}::{}", case_name.to_upper_camel_case());

                if payload.is_empty() {
                    format!(
                        "{i} => {{
                             {body}
                             {constructor}
                         }}"
                    )
                } else {
                    format!(
                        "{i} => {{
                             {body}
                             {constructor}({payload})
                         }}"
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            r#"
            let {lifted} = match ({op}) {{
                {cases}
                _ => panic()
            }}
            "#
        );

        results.push(lifted);
    }
}

impl Bindgen for FunctionBindgen<'_, '_> {
    type Operand = String;

    fn emit(
        &mut self,
        _resolve: &Resolve,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(format!("({})", val.to_string())),
            Instruction::ConstZero { tys } => results.extend(tys.iter().map(|ty| {
                match ty {
                    WasmType::I32 => "0",
                    WasmType::I64 => "0L",
                    WasmType::F32 => "0.0",
                    WasmType::F64 => "0.0",
                    WasmType::Pointer => "0",
                    WasmType::PointerOrI64 => "0L",
                    WasmType::Length => "0",
                }
                .to_owned()
            })),

            Instruction::Bitcasts { casts } => results.extend(
                casts
                    .iter()
                    .zip(operands)
                    .map(|(cast, op)| perform_cast(op, cast)),
            ),

            Instruction::I32FromU16
            | Instruction::I32FromS32
            | Instruction::I64FromS64
            | Instruction::S32FromI32
            | Instruction::S64FromI64
            | Instruction::CharFromI32
            | Instruction::I32FromChar
            | Instruction::CoreF32FromF32
            | Instruction::CoreF64FromF64
            | Instruction::F32FromCoreF32
            | Instruction::F64FromCoreF64 => results.push(operands[0].clone()),

            Instruction::I32FromU8 => results.push(format!("({}).to_int()", operands[0])),
            Instruction::U8FromI32 => results.push(format!("({}).to_byte()", operands[0])),

            Instruction::I32FromS8 => results.push(format!("extend8({})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("({}.land(0xFF))", operands[0])),

            Instruction::U16FromI32 | Instruction::S16FromI32 => {
                results.push(format!("({}.land(0xFFFF))", operands[0]))
            }
            Instruction::I32FromS16 => results.push(format!("extend16({})", operands[0])),

            Instruction::U32FromI32 => results.push(format!("({}).to_uint()", operands[0])),
            Instruction::I32FromU32 => results.push(format!("({}).to_int()", operands[0])),

            Instruction::U64FromI64 => results.push(format!("({}).to_uint64()", operands[0])),
            Instruction::I64FromU64 => results.push(format!("({}).to_int64()", operands[0])),

            Instruction::I32FromBool => {
                results.push(format!("(if {} {{ 1 }} else {{ 0 }})", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            // TODO: checked
            Instruction::FlagsLower { flags, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(format!("({}).0", operands[0]));
                }
                Int::U64 => {
                    let op = &operands[0];
                    results.push(format!("(({op}).0.to_uint())"));
                    results.push(format!("((({op}).0.lsr(32)).to_uint())"));
                }
            },

            Instruction::FlagsLift { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(format!(
                        "{}({})",
                        self.gen.type_name(&Type::Id(*ty)),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "{}(({}).lor(({}).lsl(32)))",
                        self.gen.type_name(&Type::Id(*ty)),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::HandleLower { .. } | Instruction::HandleLift { .. } => todo!(),

            Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for field in record.fields.iter() {
                    results.push(format!("({op}).{}", field.name.to_moonbit_ident()));
                }
            }
            Instruction::RecordLift { ty, record, .. } => {
                let ops = operands
                    .iter()
                    .enumerate()
                    .map(|(i, op)| {
                        format!(
                            "{} : {}",
                            record.fields[i].name.to_moonbit_ident(),
                            op.to_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!("{}::{{{ops}}}", self.gen.type_name(&Type::Id(*ty))));
            }

            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                for i in 0..tuple.types.len() {
                    results.push(format!("({op}).{i}"));
                }
            }
            Instruction::TupleLift { .. } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                results.push(format!("({ops})"));
            }

            Instruction::VariantPayloadName => {
                let payload = self.locals.tmp("payload");
                results.push(payload.clone());
                self.payloads.push(payload);
            }

            Instruction::VariantLower {
                variant,
                results: lowered_types,
                ..
            } => self.lower_variant(
                &variant
                    .cases
                    .iter()
                    .map(|case| (case.name.deref(), case.ty))
                    .collect::<Vec<_>>(),
                lowered_types,
                &operands[0],
                results,
            ),

            Instruction::VariantLift { variant, ty, .. } => self.lift_variant(
                &Type::Id(*ty),
                &variant
                    .cases
                    .iter()
                    .map(|case| (case.name.deref(), case.ty))
                    .collect::<Vec<_>>(),
                &operands[0],
                results,
            ),

            Instruction::OptionLower {
                results: lowered_types,
                payload,
                ..
            } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();

                let lowered = lowered_types
                    .iter()
                    .map(|_| self.locals.tmp("lowered"))
                    .collect::<Vec<_>>();

                results.extend(lowered.iter().cloned());

                let declarations = lowered
                    .iter()
                    .map(|lowered| format!("{lowered}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let op = &operands[0];

                let block = |ty: Option<&Type>, Block { body, results, .. }| {
                    let assignments = lowered
                        .iter()
                        .zip(&results)
                        .map(|(lowered, result)| format!("{lowered} = {result};\n"))
                        .collect::<Vec<_>>()
                        .concat();

                    format!(
                        "{body}
                         {assignments}"
                    )
                };

                let none = block(None, none);
                let some = block(Some(payload), some);

                uwrite!(
                    self.src,
                    r#"
                    let {declarations} = match (({op})) {{
                        None => {none}
                        Some({some_payload}) => {{
                            {some}
                        }}
                    }}
                    "#
                );
            }

            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap();
                let _none = self.blocks.pop().unwrap();

                let ty = self.gen.type_name(&Type::Id(*ty));
                let lifted = self.locals.tmp("lifted");
                let op = &operands[0];

                let payload = if self.gen.non_empty_type(Some(*payload)).is_some() {
                    some.results.into_iter().next().unwrap()
                } else {
                    "None".into()
                };

                let some = some.body;

                uwrite!(
                    self.src,
                    r#"
                    let {lifted} : {ty} = match ({op}) {{
                        0 => Option::None

                        1 => {{
                            {some}
                            Option::Some({payload})
                        }}

                        _ => panic()
                    }}
                    "#
                );

                results.push(lifted);
            }

            Instruction::ResultLower {
                results: lowered_types,
                result,
                ..
            } => self.lower_variant(
                &[("Ok", result.ok), ("Err", result.err)],
                lowered_types,
                &operands[0],
                results,
            ),

            Instruction::ResultLift { result, ty } => self.lift_variant(
                &Type::Id(*ty),
                &[("Ok", result.ok), ("Err", result.err)],
                &operands[0],
                results,
            ),

            Instruction::EnumLower { .. } => results.push(format!("{}.ordinal()", operands[0])),

            Instruction::EnumLift { ty, .. } => results.push(format!(
                "{}::from({})",
                self.gen.type_name(&Type::Id(*ty)),
                operands[0]
            )),

            Instruction::ListCanonLower { element, realloc } => {
                let op = &operands[0];
                let (size, ty) = list_element_info(element);

                // Note that we can only reliably use `Address.ofData` for elements with alignment <= 4 because as
                // of this writing TeaVM does not guarantee 64 bit items are aligned on 8 byte boundaries.
                if realloc.is_none() && size <= 4 {
                    results.push(format!("org.teavm.interop.Address.ofData({op}).toInt()"));
                } else {
                    let address = self.locals.tmp("address");
                    let ty = ty.to_upper_camel_case();

                    uwrite!(
                        self.src,
                        "
                        org.teavm.interop.Address {address} = Memory.malloc({size} * ({op}).length, {size});
                        Memory.put{ty}s({address}, {op}, 0, ({op}).length);
                        "
                    );

                    if realloc.is_none() {
                        self.cleanup.push(Cleanup {
                            address: format!("{address}.toInt()"),
                            size: format!("{size} * ({op}).length"),
                            align: size,
                        });
                    }

                    results.push(format!("{address}.toInt()"));
                }
                results.push(format!("({op}).length"));
            }

            Instruction::ListCanonLift { element, .. } => {
                let (_, ty) = list_element_info(element);
                let ty_upper = ty.to_upper_camel_case();
                let array = self.locals.tmp("array");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    {ty}[] {array} = new {ty}[{length}];
                    Memory.get{ty_upper}s(org.teavm.interop.Address.fromInt({address}), {array}, 0, ({array}).length);
                    "
                );

                results.push(array);
            }

            Instruction::StringLower { realloc } => {
                let op = &operands[0];
                let bytes = self.locals.tmp("bytes");
                uwriteln!(
                    self.src,
                    "byte[] {bytes} = ({op}).getBytes(StandardCharsets.UTF_8);"
                );

                if realloc.is_none() {
                    results.push(format!("org.teavm.interop.Address.ofData({bytes}).toInt()"));
                } else {
                    let address = self.locals.tmp("address");

                    uwrite!(
                        self.src,
                        "
                        org.teavm.interop.Address {address} = Memory.malloc({bytes}.length, 1);
                        Memory.putBytes({address}, {bytes}, 0, {bytes}.length);
                        "
                    );

                    results.push(format!("{address}.toInt()"));
                }
                results.push(format!("{bytes}.length"));
            }

            Instruction::StringLift { .. } => {
                let bytes = self.locals.tmp("bytes");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    byte[] {bytes} = new byte[{length}];
                    Memory.getBytes(org.teavm.interop.Address.fromInt({address}), {bytes}, 0, {length});
                    "
                );

                results.push(format!("new String({bytes}, StandardCharsets.UTF_8)"));
            }

            Instruction::ListLower { element, realloc } => {
                let Block {
                    body,
                    results: block_results,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                assert!(block_results.is_empty());

                let op = &operands[0];
                let size = self.gen.gen.sizes.size(element);
                let align = self.gen.gen.sizes.align(element);
                let address = self.locals.tmp("address");
                let ty = self.gen.type_name(element);
                let index = self.locals.tmp("index");

                uwrite!(
                    self.src,
                    "
                    let {address} = malloc(({op}).length() * {size});
                    for {index} = 0; {index} < ({op}).length(); {index} = {index} + 1 {{
                        let {block_element} : {ty} = ({op})[({index})]
                        let {base} = {address} + ({index} * {size});
                        {body}
                    }}
                    "
                );

                if realloc.is_none() {
                    self.cleanup.push(Cleanup {
                        address: address.clone(),
                        size: format!("({op}).length() * {size}"),
                        align,
                    });
                }

                results.push(address);
                results.push(format!("({op}).length()"));
            }

            Instruction::ListLift { element, .. } => {
                let Block {
                    body,
                    results: block_results,
                    base,
                    ..
                } = self.blocks.pop().unwrap();
                let address = &operands[0];
                let length = &operands[1];
                let array = self.locals.tmp("array");
                let ty = self.gen.type_name(element);
                let size = self.gen.gen.sizes.size(element);
                // let align = self.gen.gen.sizes.align(element);
                let index = self.locals.tmp("index");

                let result = match &block_results[..] {
                    [result] => result,
                    _ => todo!("result count == {}", results.len()),
                };

                uwrite!(
                    self.src,
                    "
                    let {array} : Array[{ty}] = [];
                    for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                        let {base} = ({address}) + ({index} * {size})
                        {body}
                        {array}.push({result})
                    }}
                    free({address})
                    "
                );

                results.push(array);
            }

            Instruction::IterElem { .. } => {
                results.push(self.block_storage.last().unwrap().element.clone())
            }

            Instruction::IterBasePointer => {
                results.push(self.block_storage.last().unwrap().base.clone())
            }

            Instruction::CallWasm { sig, .. } => {
                let assignment = match &sig.results[..] {
                    [result] => {
                        let ty = wasm_type(*result);
                        let result = self.locals.tmp("result");
                        let assignment = format!("let {result} : {ty} = ");
                        results.push(result);
                        assignment
                    }

                    [] => String::new(),

                    _ => unreachable!(),
                };

                let func_name = self.func_name.to_upper_camel_case();

                let operands = operands.join(", ");

                uwriteln!(self.src, "{assignment} wasmImport{func_name}({operands});");
            }

            Instruction::CallInterface { func, .. } => {
                let (assignment, destructure) = match func.results.len() {
                    0 => (String::new(), String::new()),
                    1 => {
                        let ty = self
                            .gen
                            .type_name(func.results.iter_types().next().unwrap());
                        let result = self.locals.tmp("result");
                        let assignment = format!("let {result} : {ty} = ");
                        results.push(result);
                        (assignment, String::new())
                    }
                    count => {
                        let ty = format!(
                            "{}Tuple{count}<{}>",
                            self.gen.gen.qualifier(),
                            func.results
                                .iter_types()
                                .map(|ty| self.gen.type_name_boxed(ty, false))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );

                        let result = self.locals.tmp("result");
                        let assignment = format!("let {result} : {ty} = ");

                        let destructure = func
                            .results
                            .iter_types()
                            .enumerate()
                            .map(|(index, ty)| {
                                let ty = self.gen.type_name(ty);
                                let my_result = self.locals.tmp("result");
                                let assignment =
                                    format!("let {my_result} : {ty} = {result}.f{index};");
                                results.push(my_result);
                                assignment
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        (assignment, destructure)
                    }
                };

                let module = self.gen.name;
                let name = func.name.to_moonbit_ident();

                let args = operands.join(", ");

                uwrite!(
                    self.src,
                    "
                    {assignment}{module}Impl.{name}({args});
                    {destructure}
                    "
                );
            }

            Instruction::Return { amt, .. } => {
                for Cleanup { address, .. } in &self.cleanup {
                    uwriteln!(self.src, "free({address})");
                }

                if self.needs_cleanup_list {
                    uwrite!(
                        self.src,
                        "
                        cleanupList.each(fn(cleanup) {{
                            free(cleanup.address);
                        }})
                        "
                    );
                }

                match *amt {
                    0 => (),
                    1 => uwriteln!(self.src, "return {}", operands[0]),
                    _ => {
                        let results = operands.join(", ");
                        uwriteln!(self.src, "return ({results})");
                    }
                }
            }

            Instruction::I32Load { offset }
            | Instruction::PointerLoad { offset }
            | Instruction::LengthLoad { offset } => {
                results.push(format!("load32(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load8U { offset } => {
                results.push(format!("load8_u(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load8S { offset } => {
                results.push(format!("load8(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load16U { offset } => {
                results.push(format!("load16_u(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load16S { offset } => {
                results.push(format!("load16(({}) + {offset})", operands[0]))
            }

            Instruction::I64Load { offset } => {
                results.push(format!("load64(({}) + {offset})", operands[0]))
            }

            Instruction::F32Load { offset } => {
                results.push(format!("loadf32(({}) + {offset}).", operands[0]))
            }

            Instruction::F64Load { offset } => {
                results.push(format!("loadf64(({}) + {offset})", operands[0]))
            }

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => uwriteln!(
                self.src,
                "store32(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store8 { offset } => uwriteln!(
                self.src,
                "store8(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store16 { offset } => uwriteln!(
                self.src,
                "store16(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I64Store { offset } => uwriteln!(
                self.src,
                "store64(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::F32Store { offset } => uwriteln!(
                self.src,
                "storef32(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::F64Store { offset } => uwriteln!(
                self.src,
                "storef64(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),
            // TODO: see what we can do with align
            Instruction::Malloc { size, .. } => uwriteln!(self.src, "malloc({})", size),

            Instruction::GuestDeallocate { .. } => {
                uwriteln!(self.src, "free({})", operands[0])
            }

            Instruction::GuestDeallocateString => uwriteln!(self.src, "free({})", operands[0]),

            Instruction::GuestDeallocateVariant { blocks } => {
                let cases = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .enumerate()
                    .map(|(i, Block { body, results, .. })| {
                        assert!(results.is_empty());
                        if body.is_empty() {
                            format!("{i} => ()")
                        } else {
                            format!(
                                "{i} => {{
                                   {body}
                                 }}"
                            )
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let op = &operands[0];

                uwrite!(
                    self.src,
                    "
                    match ({op}) {{
                        {cases}
                    }}
                    "
                );
            }

            Instruction::GuestDeallocateList { element } => {
                let Block {
                    body,
                    results,
                    base,
                    ..
                } = self.blocks.pop().unwrap();
                assert!(results.is_empty());

                let address = &operands[0];
                let length = &operands[1];

                let size = self.gen.gen.sizes.size(element);
                // let align = self.gen.gen.sizes.align(element);

                if !body.trim().is_empty() {
                    let index = self.locals.tmp("index");

                    uwrite!(
                        self.src,
                        "
                        for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                            let {base} = ({address}) + ({index} * {size})
                            {body}
                        }}
                        "
                    );
                }

                uwriteln!(self.src, "free({address})");
            }
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        self.gen.gen.return_area_size = self.gen.gen.return_area_size.max(size);
        self.gen.gen.return_area_align = self.gen.gen.return_area_align.max(align);
        format!("{}wasi_RETURN_AREA", self.gen.gen.qualifier())
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            element: self.locals.tmp("element"),
            base: self.locals.tmp("base"),
            cleanup: mem::take(&mut self.cleanup),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage {
            body,
            element,
            base,
            cleanup,
        } = self.block_storage.pop().unwrap();

        if !self.cleanup.is_empty() {
            self.needs_cleanup_list = true;

            for Cleanup {
                address,
                size,
                align,
            } in &self.cleanup
            {
                uwriteln!(
                    self.src,
                    "cleanupList.push({}Cleanup::{{address: {address}, size: {size}, align: {align}}})",
                    self.gen.gen.qualifier()
                );
            }
        }

        self.cleanup = cleanup;

        self.blocks.push(Block {
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
            element,
            base,
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.gen.sizes
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        is_primitive(element)
    }
}

fn perform_cast(op: &str, cast: &Bitcast) -> String {
    match cast {
        Bitcast::I32ToF32 => {
            format!("Int::to_double({op})")
        }
        Bitcast::I64ToF32 => format!("Int64::to_double({op})"),
        Bitcast::F32ToI32 => {
            format!("Double::to_int({op})")
        }
        Bitcast::F32ToI64 => format!("Double::to_int64({op})"),
        Bitcast::I64ToF64 => {
            format!("Int64::to_double({op})")
        }
        Bitcast::F64ToI64 => {
            format!("Double::to_int64({op})")
        }
        Bitcast::LToI64 | Bitcast::PToP64 | Bitcast::I32ToI64 => format!("Int::to_int64({op})"),
        Bitcast::I64ToL | Bitcast::P64ToP | Bitcast::I64ToI32 => format!("Int64::to_int({op})"),
        Bitcast::I64ToP64 => format!("{op}"),
        Bitcast::P64ToI64 => format!("{op}"),
        Bitcast::I32ToP
        | Bitcast::PToI32
        | Bitcast::I32ToL
        | Bitcast::LToI32
        | Bitcast::LToP
        | Bitcast::PToL
        | Bitcast::None => op.to_owned(),

        Bitcast::Sequence(sequence) => {
            let [first, second] = &**sequence;
            perform_cast(&perform_cast(op, first), second)
        }
    }
}

fn int_type(int: Int) -> &'static str {
    match int {
        Int::U8 => "Byte",
        Int::U16 => "UInt32",
        Int::U32 => "UInt32",
        Int::U64 => "UInt64",
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "Int",
        WasmType::I64 => "Int64",
        WasmType::F32 => "Double",
        WasmType::F64 => "Double",
        WasmType::Pointer => "Int",
        WasmType::PointerOrI64 => "Int64",
        WasmType::Length => "Int",
    }
}

fn flags_repr(flags: &Flags) -> Int {
    match flags.repr() {
        FlagsRepr::U8 => Int::U8,
        FlagsRepr::U16 => Int::U16,
        FlagsRepr::U32(1) => Int::U32,
        FlagsRepr::U32(2) => Int::U64,
        repr => panic!("unimplemented flags {repr:?}"),
    }
}

fn list_element_info(ty: &Type) -> (usize, &'static str) {
    match ty {
        Type::U8 => (1, "byte"),
        Type::S32 => (4, "Int"),
        Type::U32 => (4, "UInt"),
        Type::S64 => (8, "Int64"),
        Type::U64 => (8, "UInt64"),
        Type::F64 => (8, "Double"),
        Type::Char => (4, "Char"),
        _ => unreachable!(),
    }
}

fn indent(code: &str) -> String {
    let mut indented = String::with_capacity(code.len());
    let mut indent = 0;
    let mut was_empty = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if was_empty {
                continue;
            }
            was_empty = true;
        } else {
            was_empty = false;
        }

        if trimmed.starts_with('}') {
            indent -= 1;
        }
        indented.extend(iter::repeat(' ').take(indent * 4));
        indented.push_str(trimmed);
        if trimmed.ends_with('{') {
            indent += 1;
        }
        indented.push('\n');
    }
    indented
}

fn is_primitive(ty: &Type) -> bool {
    matches!(
        ty,
        Type::U8 | Type::U32 | Type::S32 | Type::U64 | Type::S64 | Type::F64 | Type::Char
    )
}

fn world_name(resolve: &Resolve, world: WorldId) -> String {
    format!(
        "worlds.{}",
        resolve.worlds[world].name.to_upper_camel_case()
    )
}

fn interface_name(resolve: &Resolve, name: &WorldKey, direction: Direction) -> String {
    let pkg = match name {
        WorldKey::Name(_) => None,
        WorldKey::Interface(id) => {
            let pkg = resolve.interfaces[*id].package.unwrap();
            Some(resolve.packages[pkg].name.clone())
        }
    };

    let name = match name {
        WorldKey::Name(name) => name,
        WorldKey::Interface(id) => resolve.interfaces[*id].name.as_ref().unwrap(),
    }
    .to_upper_camel_case();

    format!(
        "wit.{}.{}{name}",
        match direction {
            Direction::Import => "imports",
            Direction::Export => "exports",
        },
        if let Some(name) = &pkg {
            format!(
                "{}.{}.",
                name.namespace.to_moonbit_ident(),
                name.name.to_moonbit_ident()
            )
        } else {
            String::new()
        }
    )
}

fn split_qualified_name(name: &str) -> (String, &str) {
    let tokens = name.split('.').collect::<Vec<_>>();

    let package = tokens
        .iter()
        .copied()
        .take(tokens.len() - 1)
        .collect::<Vec<_>>()
        .join(".");

    let name = tokens.last().unwrap();

    (package, name)
}

trait ToMoonBitIdent: ToOwned {
    fn to_moonbit_ident(&self) -> Self::Owned;
}

impl ToMoonBitIdent for str {
    fn to_moonbit_ident(&self) -> String {
        // Escape MoonBit keywords
        match self {
            "continue" | "for" | "match" | "if" | "pub" | "priv" | "readonly" | "self"
            | "break" | "raise" | "try" | "except" | "catch" | "else" | "enum" | "struct"
            | "type" | "trait" | "return" | "let" | "mut" | "while" | "loop" => {
                format!("{self}_")
            }
            _ => self.to_lower_camel_case(),
        }
    }
}
