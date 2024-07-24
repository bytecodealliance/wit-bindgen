use anyhow::Result;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    iter, mem,
    ops::Deref,
};
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Handle, Int, InterfaceId, Record,
        Resolve, Result_, SizeAlign, Tuple, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, Variant,
        WorldId, WorldKey,
    },
    Direction, Files, InterfaceGenerator as _, Ns, Source, WorldGenerator,
};

// Assumptions:
// Data: u8 -> Byte, s8 | s16 | u16 | s32 -> Int, u32 -> UInt, s64 -> Int64, u64 -> UInt64, f32 | f64 -> Double, address -> Int
// Encoding: UTF16

const FFI: &str = r#"
pub extern "wasm" fn extend16(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend16_s)

pub extern "wasm" fn extend8(value : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.extend8_s)

pub extern "wasm" fn store8(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store8)

pub extern "wasm" fn load8_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_u)

pub extern "wasm" fn load8(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load8_s)

pub extern "wasm" fn store16(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store16)

pub extern "wasm" fn load16(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_s)

pub extern "wasm" fn load16_u(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load16_u)

pub extern "wasm" fn store32(offset : Int, value : Int) =
  #|(func (param i32) (param i32) local.get 0 local.get 1 i32.store)

pub extern "wasm" fn load32(offset : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 i32.load)

pub extern "wasm" fn store64(offset : Int, value : Int64) =
  #|(func (param i32) (param i64) local.get 0 local.get 1 i64.store)

pub extern "wasm" fn load64(offset : Int) -> Int64 =
  #|(func (param i32) (result i64) local.get 0 i64.load)

pub extern "wasm" fn storef32(offset : Int, value : Double) =
  #|(func (param i32) (param i64) local.get 0 local.get 1 f32.demote_f64 f32.store)

pub extern "wasm" fn loadf32(offset : Int) -> Double =
  #|(func (param i32) (result f64) local.get 0 f32.load f64.promote_f32)

pub extern "wasm" fn storef64(offset : Int, value : Double) =
  #|(func (param i32) (param f64) local.get 0 local.get 1 f64.store)

pub extern "wasm" fn loadf64(offset : Int) -> Double =
  #|(func (param i32) (result f64) local.get 0 f64.load)

pub extern "wasm" fn malloc(size : Int) -> Int =
  #|(func (param i32) (result i32) local.get 0 call $rael.malloc)

pub extern "wasm" fn free(position : Int) =
  #|(func (param i32) local.get 0 call $rael.free)

pub extern "wasm" fn copy(dest : Int, src : Int, len : Int) =
  #|(func (param i32) (param i32) (param i32) local.get 0 local.get 1 local.get 2 memory.copy)

pub fn read_utf16(buffer : Int, offset : Int) -> (Char, Int) {
  let value = load16_u(buffer + offset)
  if value < 0xD800 || value >= 0xE000 {
    (Char::from_int(value), 2)
  } else {
    let hi = value & 0x3FF
    let lo = load16_u(buffer + offset + 2) & 0x3FF
    (Char::from_int(0x10000 | (hi << 10) | lo), 4)
  }
}

pub fn write_utf16(char : Char, buffer : Int, offset : Int) -> Int {
  let code = char.to_int()
  if code < 0x10000 {
    store16(buffer + offset, code & 0xFFFF)
    2
  } else if code < 0x110000 {
    store16(buffer + offset, ((code - 0x10000) >> 10) + 0xD800)
    store16(buffer + offset + 2, ((code - 0x10000) & 0x3FF) + 0xDC00)
    4
  } else {
    panic()
  }
}

pub extern "wasm" fn str2ptr(str: String) -> Int =
  #|(func (param i32) (result i32) local.get 0 call $rael.decref local.get 0 i32.const 8 i32.add)

pub trait Any {}
pub struct Cleanup {
    address : Int
    size : Int
    align : Int
}
"#;

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
    needs_cleanup: bool,
    import_interface_fragments: HashMap<String, Vec<InterfaceFragment>>,
    export_interface_fragments: HashMap<String, Vec<InterfaceFragment>>,
    import_world_fragments: Vec<InterfaceFragment>,
    export_world_fragments: Vec<InterfaceFragment>,
    sizes: SizeAlign,
    import_interface_names: HashMap<InterfaceId, String>,
    export_interface_names: HashMap<InterfaceId, String>,
    export: HashMap<String, String>,
}

impl MoonBit {
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
        self.import_interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(&resolve.name_world_key(key), func);
        }

        gen.add_interface_fragment(Direction::Import);

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

        gen.add_world_fragment(Direction::Import);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = interface_name(resolve, key, Direction::Export);
        self.export_interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(Some(&resolve.name_world_key(key)), func);
        }

        gen.add_interface_fragment(Direction::Export);
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

        gen.add_world_fragment(Direction::Export);
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

        gen.add_world_fragment(Direction::Import);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        let name = world_name(resolve, id);
        let (package, name) = split_qualified_name(&name);

        let mut src = Source::default();
        let version = env!("CARGO_PKG_VERSION");
        wit_bindgen_core::generated_preamble(&mut src, version);

        // Import world fragments
        src.push_str(
            &self
                .import_world_fragments
                .iter()
                .map(|f| f.src.deref())
                .collect::<Vec<_>>()
                .join("\n"),
        );

        if self.needs_cleanup {
            // TODO: generate on demand
        }

        let directory = package.replace('.', "/");
        files.push(&format!("{directory}/{name}.mbt"), indent(&src).as_bytes());
        files.push(&format!("{directory}/moon.pkg.json"), "{}".as_bytes());

        // Export world fragments
        src.push_str(
            &self
                .export_world_fragments
                .iter()
                .map(|f| f.src.deref())
                .collect::<Vec<_>>()
                .join("\n"),
        );

        if self.needs_cleanup {
            // TODO: generate on demand
        }

        files.push(&format!("{name}.mbt"), indent(&src).as_bytes());

        let generate_stub = |name, fragments: &[InterfaceFragment], files: &mut Files| {
            let b = fragments
                .iter()
                .map(|f| f.stub.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let mut body = Source::default();
            wit_bindgen_core::generated_preamble(&mut body, version);
            uwriteln!(&mut body, "{b}");

            files.push(&format!("{name}.mbt"), indent(&body).as_bytes());
        };

        if self.opts.generate_stub {
            generate_stub(format!("{name}Impl"), &self.export_world_fragments, files);
        }

        // Import interface fragments
        for (name, fragments) in &self.import_interface_fragments {
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
            // Avoid conflict between fragments
            files.remove(&format!("{directory}/moon.pkg.json"));
            files.push(
                &format!("{directory}/moon.pkg.json"),
                "{\"import\": [\"wasi-bindgen/ffi\"]}".as_bytes(),
            );
        }

        // Export interface fragments
        for (name, fragments) in &self.export_interface_fragments {
            let (_package, name) = split_qualified_name(name);

            let b = fragments
                .iter()
                .map(|f| f.src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let mut body = Source::default();
            wit_bindgen_core::generated_preamble(&mut body, version);
            uwriteln!(&mut body, "{b}");

            files.push(&format!("{name}.mbt"), indent(&body).as_bytes());

            if self.opts.generate_stub {
                generate_stub(format!("{name}Impl"), fragments, files);
            }
        }

        // Export project files
        // ffi utils
        files.push(&format!("ffi/ffi.mbt"), FFI.as_bytes());
        files.push(&format!("ffi/moon.pkg.json"), "{}".as_bytes());

        let mut body = Source::default();
        uwriteln!(&mut body, "{{\"name\": \"wasi-bindgen\"}}");
        files.push(&format!("moon.mod.json"), body.as_bytes());

        let mut body = Source::default();
        let exports = self
            .export
            .iter()
            .map(|(k, v)| format!("\"{k}:{v}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let imports = self
            .import_interface_names
            .values()
            .map(|v| {
                let path = v.split(".").collect::<Vec<_>>();
                format!(
                    "{{\"path\" : \"wasi-bindgen/{}\", \"alias\" : \"{}\" }}",
                    path.iter()
                        .copied()
                        .take(path.len() - 1)
                        .collect::<Vec<_>>()
                        .join("/"),
                    path.iter().nth_back(1).unwrap()
                )
            })
            .collect::<HashSet<_>>()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        uwrite!(
            &mut body,
            r#"
            {{
                "import": [
                    {imports}
                    {}"wasi-bindgen/ffi"
                ],
                "link": {{
                    "wasm": {{
                        "export": [{exports}]
                    }}
                }}
            }}
            "#,
            if imports.is_empty() { "" } else { "," },
        );
        files.push(&format!("moon.pkg.json"), body.as_bytes());

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
    fn qualifier(&self, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.gen.import_interface_names.get(id) {
                let name_path = name.split('.').collect::<Vec<_>>();
                let self_name_path = self.name.split('.').collect::<Vec<_>>();
                if name_path
                    .iter()
                    .copied()
                    .take(name_path.len() - 1)
                    .collect::<Vec<_>>()
                    .join("/")
                    != self_name_path
                        .iter()
                        .copied()
                        .take(self_name_path.len() - 1)
                        .collect::<Vec<_>>()
                        .join("/")
                {
                    return format!("@{}.", name_path[name_path.len() - 2]);
                }
            }
        }

        String::new()
    }

    fn add_interface_fragment(self, direction: Direction) {
        match direction {
            Direction::Import => {
                self.gen
                    .import_interface_fragments
                    .entry(self.name.to_owned())
                    .or_default()
                    .push(InterfaceFragment {
                        src: self.src,
                        stub: self.stub,
                    });
            }
            Direction::Export => {
                self.gen
                    .export_interface_fragments
                    .entry(self.name.to_owned())
                    .or_default()
                    .push(InterfaceFragment {
                        src: self.src,
                        stub: self.stub,
                    });
            }
        }
    }

    fn add_world_fragment(self, direction: Direction) {
        match direction {
            Direction::Import => {
                self.gen.import_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                });
            }
            Direction::Export => {
                self.gen.export_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                });
            }
        }
    }

    fn import(&mut self, module: &str, func: &Function) {
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

            r#"let cleanupList : Array[@ffi.Cleanup] = []
               let ignoreList : Array[@ffi.Any] = []"#
                .into()
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

        let sig = self.sig_string(func);

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
            let sig = self.sig_string(func);

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

    fn type_name(&mut self, ty: &Type, type_variable: bool) -> String {
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
                    TypeDefKind::Type(ty) => self.type_name(ty, type_variable),
                    TypeDefKind::List(ty) => {
                        if type_variable {
                            format!("Array[{}]", self.type_name(ty, type_variable))
                        } else {
                            "Array".into()
                        }
                    }
                    TypeDefKind::Tuple(tuple) => {
                        if type_variable {
                            format!(
                                "({})",
                                tuple
                                    .types
                                    .iter()
                                    .map(|ty| self.type_name(ty, type_variable))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        } else {
                            unreachable!()
                        }
                    }
                    TypeDefKind::Option(ty) => {
                        if type_variable {
                            format!("{}?", self.type_name(ty, type_variable))
                        } else {
                            "Option".into()
                        }
                    }
                    TypeDefKind::Result(result) => {
                        if type_variable {
                            let mut name = |ty: &Option<Type>| {
                                ty.as_ref()
                                    .map(|ty| self.type_name(ty, true))
                                    .unwrap_or_else(|| "Unit".into())
                            };
                            let ok = name(&result.ok);
                            let err = name(&result.err);

                            format!("Result[{ok}, {err}]")
                        } else {
                            "Result".into()
                        }
                    }
                    TypeDefKind::Handle(handle) => {
                        let ty = match handle {
                            Handle::Own(ty) => ty,
                            Handle::Borrow(ty) => ty,
                        };
                        let ty = &self.resolve.types[*ty];
                        if let Some(name) = &ty.name {
                            format!("{}{}", self.qualifier(ty), name.to_upper_camel_case())
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            format!("{}{}", self.qualifier(ty), name.to_upper_camel_case())
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
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

    fn sig_string(&mut self, func: &Function) -> String {
        let name = match func.kind {
            FunctionKind::Freestanding => func.name.to_moonbit_ident(),
            FunctionKind::Constructor(_) => {
                func.name.replace("[constructor]", "").to_moonbit_ident()
            }
            _ => func.name.split(".").last().unwrap().to_moonbit_ident(),
        };
        let type_name = match func.kind {
            FunctionKind::Freestanding => "".into(),
            FunctionKind::Method(ty) | FunctionKind::Constructor(ty) | FunctionKind::Static(ty) => {
                format!("{}::", self.type_name(&Type::Id(ty), true))
            }
        };

        let result_type = match func.results.len() {
            0 => "Unit".into(),
            1 => self.type_name(func.results.iter_types().next().unwrap(), true),
            _ => {
                format!(
                    "({})",
                    func.results
                        .iter_types()
                        .map(|ty| self.type_name(ty, true))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name(ty, true);
                let name = name.to_moonbit_ident();
                format!("{name} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("pub fn {type_name}{name}({params}) -> {result_type}")
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
                    self.type_name(&field.ty, true),
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

        let name = name.to_upper_camel_case();
        uwrite!(
            self.src,
            "
            pub type {name} Int
            "
        )
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

        let cases = flags
            .flags
            .iter()
            .map(|flag| flag.name.to_shouty_snake_case())
            .collect::<Vec<_>>()
            .join("; ");

        let map_to_int = flags
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
                format!("{flag_name} => ((1{suffix} << {i}){cast})")
            })
            .collect::<Vec<_>>()
            .join("\n    ");

        uwrite!(
            self.src,
            "
            type {name} {ty} derive(Default)
            pub enum {name}Flag {{
                {cases}
            }}
            fn {name}Flag::value(self : {name}Flag) -> {ty} {{
              match self {{
                {map_to_int}
              }}
            }}
            pub fn {name}::set(self : {name}, other: {name}Flag) -> {name} {{
              self.0.lor(other.value())
            }}
            pub fn {name}::unset(self : {name}, other: {name}Flag) -> {name} {{
              self.0.land(other.value().lnot())
            }}
            pub fn {name}::is_set(self : {name}, other: {name}Flag) -> Bool {{
              (self.0.land(other.value()) == other.value())
            }}
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
                    let ty = self.type_name(&ty, true);
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
        // self.type_name(&Type::Id(id));
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
enum Cleanup {
    Memory {
        address: String,
        size: String,
        align: usize,
    },
    Object(String),
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
        is_result: bool,
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

                let payload = if self.gen.non_empty_type(ty.as_ref()).is_some() || is_result {
                    payload
                } else {
                    String::new()
                };

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
                        }}",
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
        is_result: bool,
    ) {
        let blocks = self
            .blocks
            .drain(self.blocks.len() - cases.len()..)
            .collect::<Vec<_>>();

        // Hacky way to get the type name without type parameter
        let ty = self.gen.type_name(ty, false);
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

                if payload.is_empty() && !is_result {
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
                             {constructor}({})
                         }}",
                        if payload.is_empty() {
                            "()".into()
                        } else {
                            payload
                        }
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
            | Instruction::CoreF32FromF32
            | Instruction::CoreF64FromF64
            | Instruction::F32FromCoreF32
            | Instruction::F64FromCoreF64 => results.push(operands[0].clone()),

            Instruction::CharFromI32 => results.push(format!("Char::from_int({})", operands[0])),
            Instruction::I32FromChar => results.push(format!("({}).to_int()", operands[0])),

            Instruction::I32FromU8 => results.push(format!("({}).to_int()", operands[0])),
            Instruction::U8FromI32 => results.push(format!("({}).to_byte()", operands[0])),

            Instruction::I32FromS8 => results.push(format!("@ffi.extend8({})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("({}.land(0xFF))", operands[0])),

            Instruction::U16FromI32 | Instruction::S16FromI32 => {
                results.push(format!("({}.land(0xFFFF))", operands[0]))
            }
            Instruction::I32FromS16 => results.push(format!("@ffi.extend16({})", operands[0])),

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
                    results.push(format!("({}).0.to_int()", operands[0]));
                }
                Int::U64 => {
                    let op = &operands[0];
                    results.push(format!("(({op}).0.to_int())"));
                    results.push(format!("((({op}).0.lsr(32)).to_int())"));
                }
            },

            Instruction::FlagsLift { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 => {
                    results.push(format!(
                        "{}({}.to_byte())",
                        self.gen.type_name(&Type::Id(*ty), true),
                        operands[0]
                    ));
                }
                Int::U16 | Int::U32 => {
                    results.push(format!(
                        "{}({}.to_uint())",
                        self.gen.type_name(&Type::Id(*ty), true),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "{}(({}).to_uint().to_uint64().lor(({}).to_uint().to_uint64.lsl(32)))",
                        self.gen.type_name(&Type::Id(*ty), true),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::HandleLower { .. } => {
                let op = &operands[0];
                results.push(format!("{op}.0"));
            }
            Instruction::HandleLift { ty, .. } => {
                let op = &operands[0];
                results.push(format!(
                    "{}({})",
                    self.gen.type_name(&Type::Id(*ty), true),
                    op
                ));
            }

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

                results.push(format!(
                    "{}::{{{ops}}}",
                    self.gen.type_name(&Type::Id(*ty), true)
                ));
            }

            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                // Empty tuple is Unit
                // (T) is T
                if tuple.types.len() == 0 {
                    results.push("()".into());
                } else if tuple.types.len() == 1 {
                    results.push(format!("{}", operands[0]));
                } else {
                    for i in 0..tuple.types.len() {
                        results.push(format!("({op}).{i}"));
                    }
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
                false,
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
                false,
            ),

            Instruction::OptionLower {
                results: lowered_types,
                ..
            } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let _none_payload = self.payloads.pop().unwrap();

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

                let block = |Block { body, results, .. }| {
                    let assignments = results
                        .iter()
                        .map(|result| format!("{result}"))
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!(
                        "{body}
                         ({assignments})"
                    )
                };

                let none = block(none);
                let some = block(some);

                if declarations.is_empty() {
                    uwrite!(
                        self.src,
                        r#"
                        match (({op})) {{
                            None => {{
                                {none}
                            }}
                            Some({some_payload}) => {{
                                {some}
                            }}
                        }}
                        "#
                    );
                } else {
                    uwrite!(
                        self.src,
                        r#"
                        let ({declarations}) = match (({op})) {{
                            None => {{
                                {none}
                            }}
                            Some({some_payload}) => {{
                                {some}
                            }}
                        }}
                        "#
                    );
                }
            }

            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap();
                let _none = self.blocks.pop().unwrap();

                let ty = self.gen.type_name(&Type::Id(*ty), true);
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
                    let {lifted} : {ty} = @option.unless(({op}) == 0, fn () {{
                        {some}
                        {payload}
                    }})
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
                true,
            ),

            Instruction::ResultLift { result, ty } => self.lift_variant(
                &Type::Id(*ty),
                &[("Ok", result.ok), ("Err", result.err)],
                &operands[0],
                results,
                true,
            ),

            Instruction::EnumLower { .. } => results.push(format!("{}.ordinal()", operands[0])),

            Instruction::EnumLift { ty, .. } => results.push(format!(
                "{}::from({})",
                self.gen.type_name(&Type::Id(*ty), true),
                operands[0]
            )),

            Instruction::ListCanonLower {
                element,
                realloc: _,
            } => {
                let (_size, _ty) = list_element_info(element);
                unimplemented!()
            }

            Instruction::ListCanonLift { element, .. } => {
                let (_, _ty) = list_element_info(element);
                unimplemented!()
            }

            Instruction::StringLower { realloc } => {
                let op = &operands[0];

                if realloc.is_none() {
                    results.push(format!("@ffi.str2ptr({op})"));
                    self.cleanup.push(Cleanup::Object(op.clone()));
                } else {
                    let address = self.locals.tmp("address");
                    let offset = self.locals.tmp("offset");
                    let ch = self.locals.tmp("ch");

                    uwrite!(
                        self.src,
                        "
                        let {address} = @ffi.malloc({op}.length() * 6)
                        let mut {offset} = 0
                        {op}.iter().each(fn({ch}) {{ {offset} += @ffi.write_utf16({ch}, {address}, {offset}) }})
                        "
                    );

                    results.push(format!("{address}"));
                }
                results.push(format!("{op}.length()"));
            }

            Instruction::StringLift { .. } => {
                let buffer = self.locals.tmp("bytes");
                let index = self.locals.tmp("i");
                let addr = self.locals.tmp("addr");
                let len = self.locals.tmp("length");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    let {buffer} : Buffer = Buffer::new()
                    let {addr} = {address}
                    let {len} = {length}
                    for {index} = {addr}; {index} < {addr} + {len}; {index} = {index} + 1 {{
                        {buffer}.write_byte(@ffi.load8_u({index}).to_byte())
                    }}
                    "
                );

                results.push(format!("{buffer}.to_string()"));
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
                let ty = self.gen.type_name(element, true);
                let index = self.locals.tmp("index");

                uwrite!(
                    self.src,
                    "
                    let {address} = @ffi.malloc(({op}).length() * {size});
                    for {index} = 0; {index} < ({op}).length(); {index} = {index} + 1 {{
                        let {block_element} : {ty} = ({op})[({index})]
                        let {base} = {address} + ({index} * {size});
                        {body}
                    }}
                    "
                );

                if realloc.is_none() {
                    self.cleanup.push(Cleanup::Memory {
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
                let ty = self.gen.type_name(element, true);
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
                    @ffi.free({address})
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
                let assignment = match func.results.len() {
                    0 => "let _ = ".into(),
                    _ => {
                        let ty = format!(
                            "({})",
                            func.results
                                .iter_types()
                                .map(|ty| self.gen.type_name(ty, true))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );

                        let result = func
                            .results
                            .iter_types()
                            .map(|_ty| {
                                let result = self.locals.tmp("result");
                                results.push(result.clone());
                                result
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        let assignment = format!("let ({result}) : {ty} = ");

                        assignment
                    }
                };

                let name = match func.kind {
                    FunctionKind::Freestanding => func.name.to_moonbit_ident(),
                    FunctionKind::Constructor(ty) => {
                        let name = self.gen.type_name(&Type::Id(ty), false);
                        format!(
                            "{}::{}",
                            name,
                            func.name.replace("[constructor]", "").to_moonbit_ident()
                        )
                    }
                    FunctionKind::Method(ty) | FunctionKind::Static(ty) => {
                        let name = self.gen.type_name(&Type::Id(ty), false);
                        format!(
                            "{}::{}",
                            name,
                            func.name.split(".").last().unwrap().to_moonbit_ident()
                        )
                    }
                };

                let args = operands.join(", ");

                uwrite!(
                    self.src,
                    "
                    {assignment}{name}({args});
                    "
                );
            }

            Instruction::Return { amt, .. } => {
                for clean in &self.cleanup {
                    match clean {
                        Cleanup::Memory {
                            address,
                            size: _,
                            align: _,
                        } => uwriteln!(self.src, "@ffi.free({address})"),
                        Cleanup::Object(obj) => uwriteln!(self.src, "ignore({obj})"),
                    }
                }

                if self.needs_cleanup_list {
                    uwrite!(
                        self.src,
                        "
                        cleanupList.each(fn(cleanup) {{
                            @ffi.free(cleanup.address);
                        }})
                        ignore(ignoreList)
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
                results.push(format!("@ffi.load32(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load8U { offset } => {
                results.push(format!("@ffi.load8_u(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load8S { offset } => {
                results.push(format!("@ffi.load8(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load16U { offset } => {
                results.push(format!("@ffi.load16_u(({}) + {offset})", operands[0]))
            }

            Instruction::I32Load16S { offset } => {
                results.push(format!("@ffi.load16(({}) + {offset})", operands[0]))
            }

            Instruction::I64Load { offset } => {
                results.push(format!("@ffi.load64(({}) + {offset})", operands[0]))
            }

            Instruction::F32Load { offset } => {
                results.push(format!("@ffi.loadf32(({}) + {offset})", operands[0]))
            }

            Instruction::F64Load { offset } => {
                results.push(format!("@ffi.loadf64(({}) + {offset})", operands[0]))
            }

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => uwriteln!(
                self.src,
                "@ffi.store32(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store8 { offset } => uwriteln!(
                self.src,
                "@ffi.store8(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store16 { offset } => uwriteln!(
                self.src,
                "@ffi.store16(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::I64Store { offset } => uwriteln!(
                self.src,
                "@ffi.store64(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::F32Store { offset } => uwriteln!(
                self.src,
                "@ffi.storef32(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),

            Instruction::F64Store { offset } => uwriteln!(
                self.src,
                "@ffi.storef64(({}) + {offset}, {})",
                operands[1],
                operands[0]
            ),
            // TODO: see what we can do with align
            Instruction::Malloc { size, .. } => uwriteln!(self.src, "@ffi.malloc({})", size),

            Instruction::GuestDeallocate { .. } => {
                uwriteln!(self.src, "@ffi.free({})", operands[0])
            }

            Instruction::GuestDeallocateString => uwriteln!(self.src, "@ffi.free({})", operands[0]),

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

                uwriteln!(self.src, "@ffi.free({address})");
            }
        }
    }

    fn return_pointer(&mut self, size: usize, _align: usize) -> String {
        let address = self.locals.tmp("return_area");
        uwriteln!(self.src, "let {address} = @ffi.malloc({})", size,);
        address
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

            for cleanup in &self.cleanup {
                match cleanup {
                    Cleanup::Memory {
                        address,
                        size,
                        align,
                    } => uwriteln!(
                        self.src,
                        "cleanupList.push({{address: {address}, size: {size}, align: {align}}})",
                    ),
                    Cleanup::Object(obj) => uwriteln!(self.src, "ignoreList.push({obj})",),
                }
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

fn is_primitive(_ty: &Type) -> bool {
    // TODO: treat primitives
    false
    // matches!(
    //     ty,
    //     Type::U8 | Type::U32 | Type::S32 | Type::U64 | Type::S64 | Type::F64 | Type::Char
    // )
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
            "continue" | "for" | "match" | "if" | "pub" | "priv" | "readonly" | "break"
            | "raise" | "try" | "except" | "catch" | "else" | "enum" | "struct" | "type"
            | "trait" | "return" | "let" | "mut" | "while" | "loop" | "extern" | "with"
            | "throw" => {
                format!("{self}_")
            }
            _ => self.to_lower_camel_case(),
        }
    }
}
