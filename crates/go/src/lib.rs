use anyhow::Result;
use heck::{ToLowerCamelCase as _, ToSnakeCase as _, ToUpperCamelCase as _};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, hash_map};
use std::fmt;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::iter;
use std::mem;
use std::process::Command;
use std::str::FromStr;
use std::thread;
use wit_bindgen_core::abi::{
    self, AbiVariant, Bindgen, Bitcast, FlatTypes, Instruction, LiftLower, WasmType,
};
use wit_bindgen_core::wit_parser::{
    Alignment, ArchitectureSize, Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Handle, Int,
    InterfaceId, Package, PackageName, Param, Record, Resolve, Result_, SizeAlign, Tuple, Type,
    TypeDefKind, TypeId, TypeOwner, Variant, WorldId, WorldKey,
};
use wit_bindgen_core::{
    AsyncFilterSet, Direction, Files, InterfaceGenerator as _, Ns, WorldGenerator, uwriteln,
};

const MAX_FLAT_PARAMS: usize = 16;

const POINTER_SIZE_EXPRESSION: &str = "4";
const VARIANT_PAYLOAD_NAME: &str = "payload";
const ITER_BASE_POINTER: &str = "base";
const ITER_ELEMENT: &str = "element";
const IMPORT_RETURN_AREA: &str = "returnArea";
const EXPORT_RETURN_AREA: &str = "exportReturnArea";
const SYNC_EXPORT_PINNER: &str = "syncExportPinner";
const PINNER: &str = "pinner";

/// Adds the wit-bindgen GitHub repository prefix to a package name.
fn remote_pkg(name: &str) -> String {
    format!(r#""github.com/bytecodealliance/wit-bindgen/{name}""#)
}

#[derive(Default, Debug, Copy, Clone)]
pub enum Format {
    #[default]
    True,
    False,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::True => "true",
                Self::False => "false",
            }
        )
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Format, String> {
        match s {
            "true" => Ok(Format::True),
            "false" => Ok(Format::False),
            _ => Err(format!("expected `true` or `false`; got `{s}`")),
        }
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Whether or not `gofmt` should be used (if present) to format generated
    /// code.
    #[cfg_attr(
        feature = "clap",
        arg(
            long,
            default_value = "true",
            default_missing_value = "true",
            num_args = 0..=1,
            require_equals = true,
        )
    )]
    pub format: Format,

    #[cfg_attr(feature = "clap", clap(flatten))]
    pub async_: AsyncFilterSet,

    /// If true, generate stub functions for any exported functions and/or
    /// resources.
    #[cfg_attr(feature = "clap", clap(long))]
    pub generate_stubs: bool,

    /// If specified, organize the bindings into a package for use as a library;
    /// otherwise (if `None`), the bindings will be organized for use as a
    /// standalone executable.
    #[cfg_attr(feature = "clap", clap(long))]
    pub pkg_name: Option<String>,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(Go {
            opts: self.clone(),
            ..Go::default()
        })
    }
}

#[derive(Default)]
struct InterfaceData {
    code: String,
    imports: BTreeSet<String>,
    need_unsafe: bool,
    need_runtime: bool,
    need_math: bool,
}

impl InterfaceData {
    fn extend(&mut self, data: InterfaceData) {
        self.code.push_str(&data.code);
        self.imports.extend(data.imports);
        self.need_unsafe |= data.need_unsafe;
        self.need_runtime |= data.need_runtime;
        self.need_math |= data.need_math;
    }

    fn imports(&self) -> String {
        self.imports
            .iter()
            .map(|s| s.to_string())
            .chain(self.need_unsafe.then(|| r#""unsafe""#.into()))
            .chain(self.need_runtime.then(|| r#""runtime""#.into()))
            .chain(self.need_math.then(|| r#""math""#.into()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn from_generator_and_code(generator: FunctionGenerator<'_>, code: String) -> Self {
        Self {
            code,
            imports: generator.imports,
            need_unsafe: generator.need_unsafe,
            need_runtime: generator.need_pinner,
            need_math: generator.need_math,
        }
    }
}

impl From<InterfaceGenerator<'_>> for InterfaceData {
    fn from(generator: InterfaceGenerator<'_>) -> Self {
        Self {
            code: generator.src,
            imports: generator.imports,
            need_unsafe: generator.need_unsafe,
            need_runtime: generator.need_runtime,
            need_math: false,
        }
    }
}

#[derive(Default)]
struct Go {
    opts: Opts,
    src: String,
    sizes: SizeAlign,
    return_area_size: ArchitectureSize,
    return_area_align: Alignment,
    imports: BTreeSet<String>,
    tuples: BTreeSet<usize>,
    need_option: bool,
    need_result: bool,
    need_math: bool,
    need_unit: bool,
    need_future: bool,
    need_stream: bool,
    need_unsafe: bool,
    interface_names: HashMap<InterfaceId, WorldKey>,
    interfaces: BTreeMap<String, InterfaceData>,
    export_interfaces: BTreeMap<String, InterfaceData>,
    types: HashSet<TypeId>,
    resources: HashMap<TypeId, Direction>,
    futures_and_streams: HashMap<(TypeId, bool), Option<WorldKey>>,
}

impl Go {
    /// Adds the bindings module prefix to a package name.
    fn mod_pkg(&self, name: &str) -> String {
        let prefix = self.opts.pkg_name.as_deref().unwrap_or("wit_component");
        format!(r#""{prefix}/{name}""#)
    }

    fn package_for_owner(
        &mut self,
        resolve: &Resolve,
        owner: Option<&WorldKey>,
        id: TypeId,
        local: Option<&WorldKey>,
        in_import: bool,
        imports: &mut BTreeSet<String>,
    ) -> String {
        let exported = self.has_exported_resource(resolve, Type::Id(id));

        if local == owner && (exported ^ in_import) {
            String::new()
        } else {
            let package = interface_name(resolve, owner);
            let package = if exported {
                format!("export_{package}")
            } else {
                package
            };
            let prefix = format!("{package}.");
            imports.insert(self.mod_pkg(&package));
            prefix
        }
    }

    fn package(
        &mut self,
        resolve: &Resolve,
        id: TypeId,
        local: Option<&WorldKey>,
        in_import: bool,
        imports: &mut BTreeSet<String>,
    ) -> String {
        let ty = &resolve.types[id];
        let owner = match ty.owner {
            TypeOwner::World(_) => None,
            TypeOwner::Interface(id) => Some(
                self.interface_names
                    .get(&id)
                    .cloned()
                    .unwrap_or(WorldKey::Interface(id)),
            ),
            TypeOwner::None => unreachable!(),
        };

        self.package_for_owner(resolve, owner.as_ref(), id, local, in_import, imports)
    }

    fn type_name(
        &mut self,
        resolve: &Resolve,
        ty: Type,
        local: Option<&WorldKey>,
        in_import: bool,
        imports: &mut BTreeSet<String>,
    ) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "uint8".into(),
            Type::S8 => "int8".into(),
            Type::U16 => "uint16".into(),
            Type::S16 => "int16".into(),
            Type::U32 => "uint32".into(),
            Type::S32 => "int32".into(),
            Type::U64 => "uint64".into(),
            Type::S64 => "int64".into(),
            Type::F32 => "float32".into(),
            Type::F64 => "float64".into(),
            Type::Char => "rune".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &resolve.types[id];
                match &ty.kind {
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Resource => {
                        let package = self.package(resolve, id, local, in_import, imports);
                        let name = ty.name.as_ref().unwrap().to_upper_camel_case();
                        format!("{package}{name}")
                    }
                    TypeDefKind::Handle(Handle::Own(ty) | Handle::Borrow(ty)) => {
                        let name =
                            self.type_name(resolve, Type::Id(*ty), local, in_import, imports);
                        format!("*{name}")
                    }
                    TypeDefKind::Option(ty) => {
                        imports.insert(remote_pkg("wit_types"));
                        let ty = self.type_name(resolve, *ty, local, in_import, imports);
                        format!("wit_types.Option[{ty}]")
                    }
                    TypeDefKind::List(ty) => {
                        let ty = self.type_name(resolve, *ty, local, in_import, imports);
                        format!("[]{ty}")
                    }
                    TypeDefKind::Result(result) => {
                        imports.insert(remote_pkg("wit_types"));
                        let ok_type = result
                            .ok
                            .map(|ty| self.type_name(resolve, ty, local, in_import, imports))
                            .unwrap_or_else(|| {
                                self.need_unit = true;
                                "wit_types.Unit".into()
                            });
                        let err_type = result
                            .err
                            .map(|ty| self.type_name(resolve, ty, local, in_import, imports))
                            .unwrap_or_else(|| {
                                self.need_unit = true;
                                "wit_types.Unit".into()
                            });
                        format!("wit_types.Result[{ok_type}, {err_type}]")
                    }
                    TypeDefKind::Tuple(tuple) => {
                        imports.insert(remote_pkg("wit_types"));
                        let count = tuple.types.len();
                        if count > 16 {
                            todo!(
                                "tuples can not have a capacity greater than 16: {:?}",
                                ty.kind
                            )
                        }
                        self.tuples.insert(count);
                        let types = tuple
                            .types
                            .iter()
                            .map(|ty| self.type_name(resolve, *ty, local, in_import, imports))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("wit_types.Tuple{count}[{types}]")
                    }
                    TypeDefKind::Future(ty) => {
                        self.need_future = true;
                        imports.insert(remote_pkg("wit_types"));
                        let ty = ty
                            .map(|ty| self.type_name(resolve, ty, local, in_import, imports))
                            .unwrap_or_else(|| {
                                self.need_unit = true;
                                "wit_types.Unit".into()
                            });
                        format!("*wit_types.FutureReader[{ty}]")
                    }
                    TypeDefKind::Stream(ty) => {
                        self.need_stream = true;
                        imports.insert(remote_pkg("wit_types"));
                        let ty = ty
                            .map(|ty| self.type_name(resolve, ty, local, in_import, imports))
                            .unwrap_or_else(|| {
                                self.need_unit = true;
                                "wit_types.Unit".into()
                            });
                        format!("*wit_types.StreamReader[{ty}]")
                    }
                    TypeDefKind::Type(ty) => {
                        self.type_name(resolve, *ty, local, in_import, imports)
                    }
                    _ => todo!("{:?}", ty.kind),
                }
            }
            _ => todo!("{ty:?}"),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "required context codegen")]
    fn future_or_stream(
        &mut self,
        resolve: &Resolve,
        ty: TypeId,
        index: usize,
        in_import: bool,
        imported_type: bool,
        interface: Option<&WorldKey>,
        func_name: &str,
    ) -> InterfaceData {
        let prefix = if in_import { "" } else { "[export]" };

        let module = format!(
            "{prefix}{}",
            interface
                .as_ref()
                .map(|name| resolve.name_world_key(name))
                .unwrap_or_else(|| "$root".into())
        );

        let (payload_ty, kind, count) = match &resolve.types[ty].kind {
            TypeDefKind::Future(ty) => (*ty, "future", ""),
            TypeDefKind::Stream(ty) => (*ty, "stream", ", count uint32"),
            _ => unreachable!(),
        };

        let upper_kind = kind.to_upper_camel_case();

        let mut data = InterfaceData {
            need_unsafe: true,
            ..InterfaceData::default()
        };
        data.imports.insert(remote_pkg("wit_types"));

        let (payload, snake) = if let Some(ty) = payload_ty {
            (
                self.type_name(resolve, ty, interface, imported_type, &mut data.imports),
                self.mangle_name(resolve, ty, interface),
            )
        } else {
            self.need_unit = true;
            ("wit_types.Unit".into(), "unit".into())
        };
        let camel = snake.to_upper_camel_case();

        let abi = self.sizes.record(payload_ty.as_ref());
        let size = abi.size.format(POINTER_SIZE_EXPRESSION);
        let align = abi.align.format(POINTER_SIZE_EXPRESSION);

        // TODO: Skip lifting/lowering other types that can be used directly in
        // their canonical form:
        let (lift, lift_name, lower, lower_name) = match payload_ty {
            None => (
                format!(
                    "func wasm_{kind}_lift_{snake}(src unsafe.Pointer) {payload} {{
	return wit_types.Unit{{}}
}}
"
                ),
                format!("wasm_{kind}_lift_{snake}"),
                String::new(),
                "nil".to_string(),
            ),
            Some(Type::U8 | Type::S8) => (
                String::new(),
                "nil".to_string(),
                String::new(),
                "nil".to_string(),
            ),
            Some(ty) => {
                data.need_runtime = true;

                let mut generator = FunctionGenerator::new(
                    self,
                    None,
                    None,
                    interface,
                    "INVALID",
                    Vec::new(),
                    false,
                    imported_type,
                );
                generator.collect_lifters = true;

                let lift_result =
                    abi::lift_from_memory(resolve, &mut generator, "src".to_string(), &ty);
                let lift = mem::take(&mut generator.src);

                abi::lower_to_memory(
                    resolve,
                    &mut generator,
                    "dst".to_string(),
                    "value".to_string(),
                    &ty,
                );

                let lifter_count = generator.lifter_count;
                let (prefix, suffix) = if lifter_count > 0 {
                    (
                        format!("lifters := make([]func(), 0, {lifter_count})\n"),
                        "\nreturn func() {
        for _, lifter := range lifters {
                lifter()
        }
}",
                    )
                } else {
                    (String::new(), "\nreturn func() {}")
                };

                let lower = mem::take(&mut generator.src);
                data.extend(InterfaceData::from_generator_and_code(
                    generator,
                    String::new(),
                ));

                (
                    format!(
                        "func wasm_{kind}_lift_{snake}(src unsafe.Pointer) {payload} {{
        {lift}
	return {lift_result}
}}
"
                    ),
                    format!("wasm_{kind}_lift_{snake}"),
                    format!(
                        "func wasm_{kind}_lower_{snake}(
        pinner *runtime.Pinner,
        value {payload},
        dst unsafe.Pointer,
) func() {{
        {prefix}{lower}{suffix}
}}
"
                    ),
                    format!("wasm_{kind}_lower_{snake}"),
                )
            }
        };

        data.code = format!(
            r#"
//go:wasmimport {module} [{kind}-new-{index}]{func_name}
func wasm_{kind}_new_{snake}() uint64

//go:wasmimport {module} [async-lower][{kind}-read-{index}]{func_name}
func wasm_{kind}_read_{snake}(handle int32, item unsafe.Pointer{count}) uint32

//go:wasmimport {module} [async-lower][{kind}-write-{index}]{func_name}
func wasm_{kind}_write_{snake}(handle int32, item unsafe.Pointer{count}) uint32

//go:wasmimport {module} [{kind}-drop-readable-{index}]{func_name}
func wasm_{kind}_drop_readable_{snake}(handle int32)

//go:wasmimport {module} [{kind}-drop-writable-{index}]{func_name}
func wasm_{kind}_drop_writable_{snake}(handle int32)

{lift}

{lower}

var wasm_{kind}_vtable_{snake} = wit_types.{upper_kind}Vtable[{payload}]{{
	{size},
	{align},
	wasm_{kind}_read_{snake},
	wasm_{kind}_write_{snake},
	nil,
	nil,
	wasm_{kind}_drop_readable_{snake},
	wasm_{kind}_drop_writable_{snake},
	{lift_name},
	{lower_name},
}}

func Make{upper_kind}{camel}() (*wit_types.{upper_kind}Writer[{payload}], *wit_types.{upper_kind}Reader[{payload}]) {{
	pair := wasm_{kind}_new_{snake}()
	return wit_types.Make{upper_kind}Writer[{payload}](&wasm_{kind}_vtable_{snake}, int32(pair >> 32)),
		wit_types.Make{upper_kind}Reader[{payload}](&wasm_{kind}_vtable_{snake}, int32(pair & 0xFFFFFFFF))
}}

func Lift{upper_kind}{camel}(handle int32) *wit_types.{upper_kind}Reader[{payload}] {{
	return wit_types.Make{upper_kind}Reader[{payload}](&wasm_{kind}_vtable_{snake}, handle)
}}
"#
        );

        data
    }

    fn mangle_name(&self, resolve: &Resolve, ty: Type, local: Option<&WorldKey>) -> String {
        // TODO: Ensure the returned name is always distinct for distinct types
        // (e.g. by incorporating interface version numbers and/or additional
        // mangling as needed).
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "u8".into(),
            Type::U16 => "u16".into(),
            Type::U32 => "u32".into(),
            Type::U64 => "u64".into(),
            Type::S8 => "s8".into(),
            Type::S16 => "s16".into(),
            Type::S32 => "s32".into(),
            Type::S64 => "s64".into(),
            Type::ErrorContext => "error_context".into(),
            Type::F32 => "f32".into(),
            Type::F64 => "f64".into(),
            Type::Char => "char".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &resolve.types[id];
                match &ty.kind {
                    TypeDefKind::Record(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Resource => {
                        let package = match ty.owner {
                            TypeOwner::Interface(interface) => {
                                let key = self
                                    .interface_names
                                    .get(&interface)
                                    .cloned()
                                    .unwrap_or(WorldKey::Interface(interface));

                                if local == Some(&key) {
                                    String::new()
                                } else {
                                    format!(
                                        "{}_",
                                        interface_name(
                                            resolve,
                                            Some(
                                                &self
                                                    .interface_names
                                                    .get(&interface)
                                                    .cloned()
                                                    .unwrap_or(WorldKey::Interface(interface))
                                            )
                                        )
                                    )
                                }
                            }
                            _ => String::new(),
                        };

                        let name = ty.name.as_ref().unwrap().to_snake_case();

                        format!("{package}{name}")
                    }
                    TypeDefKind::Option(some) => {
                        format!("option_{}", self.mangle_name(resolve, *some, local))
                    }
                    TypeDefKind::Result(result) => format!(
                        "result_{}_{}",
                        result
                            .ok
                            .map(|ty| self.mangle_name(resolve, ty, local))
                            .unwrap_or_else(|| "unit".into()),
                        result
                            .err
                            .map(|ty| self.mangle_name(resolve, ty, local))
                            .unwrap_or_else(|| "unit".into())
                    ),
                    TypeDefKind::List(ty) => {
                        format!("list_{}", self.mangle_name(resolve, *ty, local))
                    }
                    TypeDefKind::Tuple(tuple) => {
                        let types = tuple
                            .types
                            .iter()
                            .map(|ty| self.mangle_name(resolve, *ty, local))
                            .collect::<Vec<_>>()
                            .join("_");
                        format!("tuple{}_{types}", tuple.types.len())
                    }
                    TypeDefKind::Handle(Handle::Own(ty) | Handle::Borrow(ty)) => {
                        self.mangle_name(resolve, Type::Id(*ty), local)
                    }
                    TypeDefKind::Type(ty) => self.mangle_name(resolve, *ty, local),
                    TypeDefKind::Stream(ty) => {
                        format!(
                            "stream_{}",
                            ty.map(|ty| self.mangle_name(resolve, ty, local))
                                .unwrap_or_else(|| "unit".into())
                        )
                    }
                    TypeDefKind::Future(ty) => {
                        format!(
                            "future_{}",
                            ty.map(|ty| self.mangle_name(resolve, ty, local))
                                .unwrap_or_else(|| "unit".into())
                        )
                    }
                    kind => todo!("{kind:?}"),
                }
            }
        }
    }
}

impl WorldGenerator for Go {
    // FIXME(#1527): this caused failures in CI at
    // https://github.com/bytecodealliance/wit-bindgen/actions/runs/21880247244/job/63160400774?pr=1526
    // and should be fixed at some point by deleting this method and getting
    // tests passing again.
    fn uses_nominal_type_ids(&self) -> bool {
        false
    }

    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        _ = world;
        self.sizes.fill(resolve);
        self.imports.insert(remote_pkg("wit_runtime"));
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        if let WorldKey::Name(_) = name {
            self.interface_names.insert(id, name.clone());
        }

        let mut data = {
            let mut generator = InterfaceGenerator::new(self, resolve, Some((id, name)), true);
            for (name, ty) in resolve.interfaces[id].types.iter() {
                if !generator.generator.types.contains(ty) {
                    generator.generator.types.insert(*ty);
                    generator.define_type(name, *ty);
                }
            }
            InterfaceData::from(generator)
        };

        for (_, func) in &resolve.interfaces[id].functions {
            data.extend(self.import(resolve, func, Some(name)));
        }
        self.interfaces
            .entry(interface_name(resolve, Some(name)))
            .or_default()
            .extend(data);

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let mut data = InterfaceData::default();
        for (_, func) in funcs {
            data.extend(self.import(resolve, func, None));
        }
        self.interfaces
            .entry(interface_name(resolve, None))
            .or_default()
            .extend(data);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        if let WorldKey::Name(_) = name {
            self.interface_names.insert(id, name.clone());
        }

        for (type_name, ty) in &resolve.interfaces[id].types {
            let exported = matches!(resolve.types[*ty].kind, TypeDefKind::Resource)
                || self.has_exported_resource(resolve, Type::Id(*ty));

            let mut generator = InterfaceGenerator::new(self, resolve, Some((id, name)), false);

            if exported || !generator.generator.types.contains(ty) {
                generator.generator.types.insert(*ty);
                generator.define_type(type_name, *ty);
            }

            let data = generator.into();

            if exported {
                &mut self.export_interfaces
            } else {
                &mut self.interfaces
            }
            .entry(interface_name(resolve, Some(name)))
            .or_default()
            .extend(data);
        }

        for (_, func) in &resolve.interfaces[id].functions {
            let code = self.export(resolve, func, Some(name));
            self.src.push_str(&code);
        }

        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        for (_, func) in funcs {
            let code = self.export(resolve, func, None);
            self.src.push_str(&code);
        }
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut generator = InterfaceGenerator::new(self, resolve, None, true);
        for (name, ty) in types {
            if !generator.generator.types.contains(ty) {
                generator.generator.types.insert(*ty);
                generator.define_type(name, *ty);
            }
        }
        let data = generator.into();
        self.interfaces
            .entry(interface_name(resolve, None))
            .or_default()
            .extend(data);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        _ = (resolve, id);

        let version = env!("CARGO_PKG_VERSION");
        let packages = resolve
            .packages
            .iter()
            .map(
                |(
                    _,
                    Package {
                        name:
                            PackageName {
                                namespace,
                                name,
                                version,
                            },
                        ..
                    },
                )| {
                    let version = if let Some(version) = version {
                        format!("@{version}")
                    } else {
                        String::new()
                    };
                    format!("//     {namespace}:{name}{version}")
                },
            )
            .collect::<Vec<_>>()
            .join("\n");
        let header = &format!(
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
//
// This code was generated from the following packages:
{packages}
"
        );

        let src = mem::take(&mut self.src);
        let align = self.return_area_align.format(POINTER_SIZE_EXPRESSION);
        let size = self.return_area_size.format(POINTER_SIZE_EXPRESSION);
        let imports = self
            .imports
            .iter()
            .map(|s| s.as_str())
            .chain(self.need_math.then_some(r#""math""#))
            .chain(self.need_unsafe.then_some(r#""unsafe""#))
            .collect::<Vec<_>>()
            .join("\n");

        let (exports_file_path, package_name, main_func) = if self.opts.pkg_name.is_some() {
            // If a module name is specified, the generated files will be used as a library.
            ("wit_exports/wit_exports.go", "wit_exports", "")
        } else {
            // This is the literal location of the Go package.
            let replacement_pkg = concat!(
                "github.com/bytecodealliance/wit-bindgen/crates/go/src/package v",
                env!("CARGO_PKG_VERSION")
            );

            files.push(
                "go.mod",
                format!(
                    "module {}\n\ngo 1.25\n\nreplace github.com/bytecodealliance/wit-bindgen => {}",
                    self.opts.pkg_name.as_deref().unwrap_or("wit_component"),
                    replacement_pkg
                )
                .as_bytes(),
            );

            // If a module name is NOT specified, the generated files will be used as a
            // standalone executable.
            (
                "wit_exports.go",
                "main",
                r#"// Unused, but present to make the compiler happy
func main() {}
"#,
            )
        };

        files.push(
            exports_file_path,
            &maybe_gofmt(
                self.opts.format,
                format!(
                    r#"{header}
package {package_name}

import (
        "runtime"
        {imports}
)

var staticPinner = runtime.Pinner{{}}
var {EXPORT_RETURN_AREA} = uintptr(wit_runtime.Allocate(&staticPinner, {size}, {align}))
var {SYNC_EXPORT_PINNER} = runtime.Pinner{{}}

{src}

{main_func}
"#
                )
                .as_bytes(),
            ),
        );

        for (prefix, interfaces) in [("export_", &self.export_interfaces), ("", &self.interfaces)] {
            for (name, data) in interfaces {
                let imports = data.imports();
                let code = &data.code;

                files.push(
                    &format!("{prefix}{name}/wit_bindings.go"),
                    &maybe_gofmt(
                        self.opts.format,
                        format!(
                            "{header}
package {prefix}{name}

import (
        {imports}
)

{code}"
                        )
                        .as_bytes(),
                    ),
                );
            }
        }

        Ok(())
    }
}

impl Go {
    fn import(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        interface: Option<&WorldKey>,
    ) -> InterfaceData {
        self.visit_futures_and_streams(true, resolve, func, interface);

        let async_ = self.opts.async_.is_async(resolve, interface, func, true);

        let (variant, prefix) = if async_ {
            (AbiVariant::GuestImportAsync, "[async-lower]")
        } else {
            (AbiVariant::GuestImport, "")
        };

        let sig = resolve.wasm_signature(variant, func);
        let import_name = &func.name;
        let name = func.name.to_snake_case().replace('.', "_");
        let (camel, has_self) = func_declaration(resolve, func);

        let module = match interface {
            Some(name) => resolve.name_world_key(name),
            None => "$root".to_string(),
        };

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| format!("arg{i} {}", wasm_type(*param)))
            .collect::<Vec<_>>()
            .join(", ");

        let results = match &sig.results[..] {
            [] => "",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let mut imports = BTreeSet::new();
        let go_params =
            self.func_params(resolve, func, interface, true, &mut imports, has_self, "");
        let go_results = self.func_results(resolve, func, interface, true, &mut imports);

        let raw_name = format!("wasm_import_{name}");

        let go_param_names = has_self
            .then(|| "self".to_string())
            .into_iter()
            .chain(
                func.params
                    .iter()
                    .skip(if has_self { 1 } else { 0 })
                    .map(|Param { name, .. }| name.to_lower_camel_case()),
            )
            .collect::<Vec<_>>();

        let mut generator = FunctionGenerator::new(
            self,
            None,
            interface,
            interface,
            &raw_name,
            go_param_names.clone(),
            false,
            true,
        );
        generator.imports = imports;

        let code = if async_ {
            generator.imports.insert(remote_pkg("wit_async"));

            let (lower, wasm_params) = if sig.indirect_params {
                generator.imports.insert(remote_pkg("wit_runtime"));

                let params_pointer = generator.locals.tmp("params");
                let abi = generator
                    .generator
                    .sizes
                    .record(func.params.iter().map(|Param { ty, .. }| ty));
                let size = abi.size.format(POINTER_SIZE_EXPRESSION);
                let align = abi.align.format(POINTER_SIZE_EXPRESSION);
                let offsets = generator
                    .generator
                    .sizes
                    .field_offsets(func.params.iter().map(|Param { ty, .. }| ty));

                for (name, (offset, ty)) in go_param_names.iter().zip(offsets) {
                    let offset = offset.format(POINTER_SIZE_EXPRESSION);
                    abi::lower_to_memory(
                        resolve,
                        &mut generator,
                        format!("unsafe.Add(unsafe.Pointer({params_pointer}), {offset})"),
                        name.clone(),
                        ty,
                    );
                }

                let code = mem::take(&mut generator.src);
                generator.need_pinner = true;
                (
                    format!(
                        "{params_pointer} := wit_runtime.Allocate({PINNER}, {size}, {align})\n{code}"
                    ),
                    vec![format!("uintptr({params_pointer})")],
                )
            } else {
                let wasm_params = go_param_names
                    .iter()
                    .zip(&func.params)
                    .flat_map(|(name, Param { ty, .. })| {
                        abi::lower_flat(resolve, &mut generator, name.clone(), ty)
                    })
                    .collect();
                (mem::take(&mut generator.src), wasm_params)
            };

            let wasm_params = wasm_params
                .iter()
                .map(|v| v.as_str())
                .chain(func.result.map(|_| IMPORT_RETURN_AREA))
                .collect::<Vec<_>>()
                .join(", ");

            let lift = if let Some(ty) = func.result {
                let result = abi::lift_from_memory(
                    resolve,
                    &mut generator,
                    IMPORT_RETURN_AREA.to_string(),
                    &ty,
                );
                let code = mem::take(&mut generator.src);
                if let Type::Id(ty) = ty
                    && let TypeDefKind::Tuple(tuple) = &resolve.types[ty].kind
                {
                    let count = tuple.types.len();
                    let tuple = generator.locals.tmp("tuple");

                    let results = (0..count)
                        .map(|index| format!("{tuple}.F{index}"))
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!(
                        "{code}
{tuple} := {result}
return {results}"
                    )
                } else {
                    format!("{code}\nreturn {result}")
                }
            } else {
                String::new()
            };

            format!(
                "{lower}
wit_async.SubtaskWait(uint32({raw_name}({wasm_params})))
{lift}
"
            )
        } else {
            abi::call(
                resolve,
                variant,
                LiftLower::LowerArgsLiftResults,
                func,
                &mut generator,
                false,
            );
            mem::take(&mut generator.src)
        };

        let return_area = |generator: &mut FunctionGenerator<'_>,
                           size: ArchitectureSize,
                           align: Alignment| {
            generator.imports.insert(remote_pkg("wit_runtime"));
            generator.need_pinner = true;
            let size = size.format(POINTER_SIZE_EXPRESSION);
            let align = align.format(POINTER_SIZE_EXPRESSION);
            format!(
                "{IMPORT_RETURN_AREA} := uintptr(wit_runtime.Allocate({PINNER}, {size}, {align}))"
            )
        };

        let return_area = if async_ && func.result.is_some() {
            let abi = generator.generator.sizes.record(func.result.as_ref());
            return_area(&mut generator, abi.size, abi.align)
        } else if !(async_ || generator.return_area_size.is_empty()) {
            let size = generator.return_area_size;
            let align = generator.return_area_align;
            return_area(&mut generator, size, align)
        } else {
            String::new()
        };

        let pinner = if generator.need_pinner {
            format!(
                "{PINNER} := &runtime.Pinner{{}}
defer {PINNER}.Unpin()
"
            )
        } else {
            String::new()
        };

        InterfaceData::from_generator_and_code(
            generator,
            format!(
                "
//go:wasmimport {module} {prefix}{import_name}
func {raw_name}({params}) {results}

func {camel}({go_params}) {go_results} {{
        {pinner}
        {return_area}
        {code}
}}
"
            ),
        )
    }

    fn export(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        interface: Option<&WorldKey>,
    ) -> String {
        self.visit_futures_and_streams(false, resolve, func, interface);

        let async_ = self.opts.async_.is_async(resolve, interface, func, false);

        let (variant, prefix) = if async_ {
            (AbiVariant::GuestExportAsync, "[async-lift]")
        } else {
            (AbiVariant::GuestExport, "")
        };

        let sig = resolve.wasm_signature(variant, func);
        let core_module_name = interface.map(|v| resolve.name_world_key(v));
        let export_name = func.legacy_core_export_name(core_module_name.as_deref());
        let name = func_name(resolve, interface, func);

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| format!("arg{i} {}", wasm_type(*param)))
            .collect::<Vec<_>>()
            .join(", ");

        let results = match &sig.results[..] {
            [] => "",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let unpin_params =
            sig.indirect_params || abi::guest_export_params_have_allocations(resolve, func);

        let param_names = (0..sig.params.len()).map(|i| format!("arg{i}")).collect();
        let mut generator = FunctionGenerator::new(
            self,
            Some(&name),
            interface,
            None,
            "INVALID",
            param_names,
            unpin_params,
            false,
        );
        abi::call(
            resolve,
            variant,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut generator,
            async_,
        );
        let code = generator.src;
        let imports = generator.imports;
        let need_unsafe = generator.need_unsafe;
        self.need_math |= generator.need_math;
        self.need_unsafe |= need_unsafe;
        self.imports.extend(imports);

        let (pinner, other, start, end) = if async_ {
            self.imports.insert(remote_pkg("wit_async"));

            let module = match interface {
                Some(name) => resolve.name_world_key(name),
                None => "$root".to_string(),
            };

            let function = &func.name;

            let task_return_params = func
                .result
                .map(|ty| {
                    let mut storage = vec![WasmType::I32; MAX_FLAT_PARAMS];
                    let mut flat = FlatTypes::new(&mut storage);
                    if resolve.push_flat(&ty, &mut flat) {
                        flat.to_vec()
                    } else {
                        vec![WasmType::I32]
                    }
                })
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .map(|(i, ty)| {
                    let ty = wasm_type(ty);
                    format!("arg{i} {ty}")
                })
                .collect::<Vec<_>>()
                .join(", ");

            (
                if abi::guest_export_needs_post_return(resolve, func) {
                    format!("{PINNER} := &runtime.Pinner{{}}")
                } else {
                    String::new()
                },
                format!(
                    "

//go:wasmexport [callback]{prefix}{export_name}
func wasm_export_callback_{name}(event0 uint32, event1 uint32, event2 uint32) uint32 {{
        return wit_async.Callback(event0, event1, event2)
}}

//go:wasmimport [export]{module} [task-return]{function}
func wasm_export_task_return_{name}({task_return_params})
"
                ),
                "return int32(wit_async.Run(func() {",
                "}))",
            )
        } else if abi::guest_export_needs_post_return(resolve, func) {
            (
                format!("{PINNER} := &{SYNC_EXPORT_PINNER}"),
                format!(
                    "

//go:wasmexport cabi_post_{export_name}
func wasm_export_post_return_{name}(result {results}) {{
        syncExportPinner.Unpin()
}}
"
                ),
                "",
                "",
            )
        } else {
            (String::new(), String::new(), "", "")
        };

        if self.opts.generate_stubs {
            let (camel, has_self) = func_declaration(resolve, func);

            let mut imports = BTreeSet::new();
            let params =
                self.func_params(resolve, func, interface, false, &mut imports, has_self, "_");
            let results = self.func_results(resolve, func, interface, false, &mut imports);

            self.export_interfaces
                .entry(interface_name(resolve, interface))
                .or_default()
                .extend(InterfaceData {
                    code: format!(
                        r#"
func {camel}({params}) {results} {{
        panic("not implemented")
}}
"#
                    ),
                    imports,
                    ..InterfaceData::default()
                });
        }

        format!(
            "
//go:wasmexport {prefix}{export_name}
func wasm_export_{name}({params}) {results} {{
        {start}
        {pinner}
        {code}
        {end}
}}{other}
"
        )
    }

    #[expect(clippy::too_many_arguments, reason = "required context for codegen")]
    fn func_params(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        interface: Option<&WorldKey>,
        in_import: bool,
        imports: &mut BTreeSet<String>,
        has_self: bool,
        prefix: &str,
    ) -> String {
        func.params
            .iter()
            .skip(if has_self { 1 } else { 0 })
            .map(|Param { name, ty, .. }| {
                let name = name.to_lower_camel_case();
                let ty = self.type_name(resolve, *ty, interface, in_import, imports);
                format!("{prefix}{name} {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn func_results(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        interface: Option<&WorldKey>,
        in_import: bool,
        imports: &mut BTreeSet<String>,
    ) -> String {
        if let Some(ty) = &func.result {
            if let Type::Id(id) = ty
                && let TypeDefKind::Tuple(tuple) = &resolve.types[*id].kind
            {
                let types = tuple
                    .types
                    .iter()
                    .map(|ty| self.type_name(resolve, *ty, interface, in_import, imports))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({types})")
            } else {
                self.type_name(resolve, *ty, interface, in_import, imports)
            }
        } else {
            String::new()
        }
    }

    fn visit_futures_and_streams(
        &mut self,
        in_import: bool,
        resolve: &Resolve,
        func: &Function,
        interface: Option<&WorldKey>,
    ) {
        for (index, ty) in func
            .find_futures_and_streams(resolve)
            .into_iter()
            .enumerate()
        {
            let payload_type = match &resolve.types[ty].kind {
                TypeDefKind::Future(ty) => {
                    self.need_future = true;
                    ty
                }
                TypeDefKind::Stream(ty) => {
                    self.need_stream = true;
                    ty
                }
                _ => unreachable!(),
            };

            let exported = payload_type
                .map(|ty| self.has_exported_resource(resolve, ty))
                .unwrap_or(false);

            if let hash_map::Entry::Vacant(e) = self.futures_and_streams.entry((ty, exported)) {
                e.insert(interface.cloned());

                let data = self.future_or_stream(
                    resolve,
                    ty,
                    index,
                    in_import,
                    in_import || !exported,
                    interface,
                    &func.name,
                );

                if in_import || !exported {
                    &mut self.interfaces
                } else {
                    &mut self.export_interfaces
                }
                .entry(interface_name(resolve, interface))
                .or_default()
                .extend(data);
            }
        }
    }

    fn has_exported_resource(&self, resolve: &Resolve, ty: Type) -> bool {
        any(resolve, ty, &|ty| {
            if let Type::Id(id) = ty
                && let TypeDefKind::Resource = &resolve.types[id].kind
                && let Direction::Export = self.resources.get(&id).unwrap()
            {
                true
            } else {
                false
            }
        })
    }
}

struct FunctionGenerator<'a> {
    generator: &'a mut Go,
    name: Option<&'a str>,
    interface: Option<&'a WorldKey>,
    interface_for_types: Option<&'a WorldKey>,
    function_to_call: &'a str,
    param_names: Vec<String>,
    unpin_params: bool,
    in_import: bool,
    locals: Ns,
    src: String,
    block_storage: Vec<String>,
    blocks: Vec<(String, Vec<String>)>,
    need_unsafe: bool,
    need_pinner: bool,
    need_math: bool,
    collect_lifters: bool,
    lifter_count: u32,
    return_area_size: ArchitectureSize,
    return_area_align: Alignment,
    imports: BTreeSet<String>,
}

impl<'a> FunctionGenerator<'a> {
    #[expect(clippy::too_many_arguments, reason = "required context for codegen")]
    fn new(
        generator: &'a mut Go,
        name: Option<&'a str>,
        interface: Option<&'a WorldKey>,
        interface_for_types: Option<&'a WorldKey>,
        function_to_call: &'a str,
        param_names: Vec<String>,
        unpin_params: bool,
        in_import: bool,
    ) -> Self {
        let mut locals = Ns::default();
        for name in &param_names {
            locals.insert(name).unwrap();
        }

        Self {
            generator,
            name,
            interface,
            interface_for_types,
            function_to_call,
            param_names,
            unpin_params,
            in_import,
            locals,
            src: String::new(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            need_unsafe: false,
            need_pinner: false,
            need_math: false,
            collect_lifters: false,
            lifter_count: 0,
            return_area_size: ArchitectureSize::default(),
            return_area_align: Alignment::default(),
            imports: BTreeSet::new(),
        }
    }

    fn type_name(&mut self, resolve: &Resolve, ty: Type) -> String {
        self.generator.type_name(
            resolve,
            ty,
            self.interface_for_types,
            self.in_import,
            &mut self.imports,
        )
    }

    fn package_for_owner(
        &mut self,
        resolve: &Resolve,
        owner: Option<&WorldKey>,
        ty: TypeId,
    ) -> String {
        self.generator.package_for_owner(
            resolve,
            owner,
            ty,
            self.interface_for_types,
            self.in_import,
            &mut self.imports,
        )
    }
}

impl Bindgen for FunctionGenerator<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.generator.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, to_restore);
        self.blocks.push((src, mem::take(operands)));
    }

    fn return_pointer(&mut self, size: ArchitectureSize, align: Alignment) -> String {
        if self.in_import {
            self.return_area_size = self.return_area_size.max(size);
            self.return_area_align = self.return_area_align.max(align);

            if !self.return_area_size.is_empty() {
                self.need_pinner = true;
                self.imports.insert(remote_pkg("wit_runtime"));
            }

            IMPORT_RETURN_AREA.into()
        } else {
            self.generator.return_area_size = self.generator.return_area_size.max(size);
            self.generator.return_area_align = self.generator.return_area_align.max(align);
            EXPORT_RETURN_AREA.into()
        }
    }

    fn is_list_canonical(&self, _: &Resolve, ty: &Type) -> bool {
        matches!(
            ty,
            Type::U8
                | Type::S8
                | Type::U16
                | Type::S16
                | Type::U32
                | Type::S32
                | Type::U64
                | Type::S64
                | Type::F32
                | Type::F64
        )
    }

    fn emit(
        &mut self,
        resolve: &Resolve,
        instruction: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        let store = |me: &mut Self, src, pointer, offset: &ArchitectureSize, ty| {
            me.need_unsafe = true;
            let offset = offset.format(POINTER_SIZE_EXPRESSION);
            uwriteln!(
                me.src,
                "*(*{ty})(unsafe.Add(unsafe.Pointer({pointer}), {offset})) = {src}"
            );
        };
        let load = |me: &mut Self,
                    results: &mut Vec<String>,
                    pointer,
                    offset: &ArchitectureSize,
                    ty,
                    cast: &dyn Fn(String) -> String| {
            me.need_unsafe = true;
            let offset = offset.format(POINTER_SIZE_EXPRESSION);
            results.push(cast(format!(
                "*(*{ty})(unsafe.Add(unsafe.Pointer({pointer}), {offset}))"
            )));
        };

        match instruction {
            Instruction::GetArg { nth } => results.push(self.param_names[*nth].clone()),
            Instruction::StringLower { .. } => {
                self.need_pinner = true;
                self.need_unsafe = true;
                let string = &operands[0];
                let utf8 = self.locals.tmp("utf8");
                uwriteln!(
                    self.src,
                    "{utf8} := unsafe.Pointer(unsafe.StringData({string}))\n\
                     {PINNER}.Pin({utf8})"
                );
                results.push(format!("uintptr({utf8})"));
                results.push(format!("uint32(len({string}))"));
            }
            Instruction::StringLift { .. } => {
                self.need_unsafe = true;
                let pointer = &operands[0];
                let length = &operands[1];
                let value = self.locals.tmp("value");
                uwriteln!(
                    self.src,
                    "{value} := unsafe.String((*uint8)(unsafe.Pointer({pointer})), {length})"
                );
                results.push(value)
            }
            Instruction::ListCanonLower { .. } => {
                self.need_pinner = true;
                self.need_unsafe = true;
                let slice = &operands[0];
                let data = self.locals.tmp("data");
                uwriteln!(
                    self.src,
                    "{data} := unsafe.Pointer(unsafe.SliceData({slice}))\n\
                     {PINNER}.Pin({data})"
                );
                results.push(format!("uintptr({data})"));
                results.push(format!("uint32(len({slice}))"));
            }
            Instruction::ListCanonLift { element, .. } => {
                self.need_unsafe = true;
                let pointer = &operands[0];
                let length = &operands[1];
                let ty = self.type_name(resolve, **element);
                let value = self.locals.tmp("value");
                uwriteln!(
                    self.src,
                    "{value} := unsafe.Slice((*{ty})(unsafe.Pointer({pointer})), {length})"
                );
                results.push(value)
            }
            Instruction::ListLower { element, .. } => {
                self.need_unsafe = true;
                self.need_pinner = true;
                self.imports.insert(remote_pkg("wit_runtime"));
                let (body, _) = self.blocks.pop().unwrap();
                let value = &operands[0];
                let slice = self.locals.tmp("slice");
                let result = self.locals.tmp("result");
                let length = self.locals.tmp("length");
                let size = self
                    .generator
                    .sizes
                    .size(element)
                    .format(POINTER_SIZE_EXPRESSION);
                let align = self
                    .generator
                    .sizes
                    .align(element)
                    .format(POINTER_SIZE_EXPRESSION);
                uwriteln!(
                    self.src,
                    "{slice} := {value}
{length} := uint32(len({slice}))
{result} := wit_runtime.Allocate({PINNER}, uintptr({length} * {size}), {align})
for index, {ITER_ELEMENT} := range {slice} {{
        {ITER_BASE_POINTER} := unsafe.Add({result}, index * {size})
        {body}
}}
"
                );
                results.push(format!("uintptr({result})"));
                results.push(length);
            }
            Instruction::ListLift { element, .. } => {
                self.need_unsafe = true;
                let (body, body_results) = self.blocks.pop().unwrap();
                let value = &operands[0];
                let length = &operands[1];
                let result = self.locals.tmp("result");
                let size = self
                    .generator
                    .sizes
                    .size(element)
                    .format(POINTER_SIZE_EXPRESSION);
                let element_type = self.type_name(resolve, **element);
                let body_result = &body_results[0];
                uwriteln!(
                    self.src,
                    "{result} := make([]{element_type}, 0, {length})
for index := 0; index < int({length}); index++ {{
        {ITER_BASE_POINTER} := unsafe.Add(unsafe.Pointer({value}), index * {size})
        {body}
        {result} = append({result}, {body_result})
}}
"
                );
                results.push(result);
            }
            Instruction::CallInterface { func, .. } => {
                if self.unpin_params {
                    self.imports.insert(remote_pkg("wit_runtime"));
                    uwriteln!(self.src, "wit_runtime.Unpin()");
                }

                let name = func.item_name().to_upper_camel_case();
                let package = format!("export_{}", interface_name(resolve, self.interface));

                let call = match &func.kind {
                    FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                        let args = operands.join(", ");
                        let call = format!("{package}.{name}({args})");
                        self.imports.insert(self.generator.mod_pkg(&package));
                        call
                    }
                    FunctionKind::Constructor(ty) => {
                        let args = operands.join(", ");
                        let ty = resolve.types[*ty]
                            .name
                            .as_ref()
                            .unwrap()
                            .to_upper_camel_case();
                        let call = format!("{package}.Make{ty}({args})");
                        self.imports.insert(self.generator.mod_pkg(&package));
                        call
                    }
                    FunctionKind::Method(_) | FunctionKind::AsyncMethod(_) => {
                        let target = &operands[0];
                        let args = operands[1..].join(", ");
                        format!("({target}).{name}({args})")
                    }
                    FunctionKind::Static(ty) | FunctionKind::AsyncStatic(ty) => {
                        let args = operands.join(", ");
                        let ty = self.type_name(resolve, Type::Id(*ty));
                        format!("{ty}{name}({args})")
                    }
                };

                if let Some(ty) = func.result {
                    let result = self.locals.tmp("result");
                    if let Type::Id(ty) = ty
                        && let TypeDefKind::Tuple(tuple) = &resolve.types[ty].kind
                    {
                        let count = tuple.types.len();
                        self.generator.tuples.insert(count);
                        self.imports.insert(remote_pkg("wit_types"));

                        let results = (0..count)
                            .map(|_| self.locals.tmp("result"))
                            .collect::<Vec<_>>()
                            .join(", ");

                        let types = tuple
                            .types
                            .iter()
                            .map(|&ty| self.type_name(resolve, ty))
                            .collect::<Vec<_>>()
                            .join(", ");

                        uwriteln!(
                            self.src,
                            "{results} := {call}
{result} := wit_types.Tuple{count}[{types}]{{{results}}}"
                        );
                    } else {
                        uwriteln!(self.src, "{result} := {call}");
                    }
                    results.push(result);
                } else {
                    uwriteln!(self.src, "{call}");
                }
            }
            Instruction::Return { func, .. } => {
                if let Some(ty) = func.result {
                    let result = &operands[0];
                    if self.in_import
                        && let Type::Id(ty) = ty
                        && let TypeDefKind::Tuple(tuple) = &resolve.types[ty].kind
                    {
                        let count = tuple.types.len();
                        let tuple = self.locals.tmp("tuple");

                        let results = (0..count)
                            .map(|index| format!("{tuple}.F{index}"))
                            .collect::<Vec<_>>()
                            .join(", ");

                        uwriteln!(
                            self.src,
                            "{tuple} := {result}
return {results}"
                        );
                    } else {
                        uwriteln!(self.src, "return {result}");
                    }
                }
            }
            Instruction::AsyncTaskReturn { .. } => {
                let name = self.name.unwrap();
                let args = operands.join(", ");
                uwriteln!(self.src, "wasm_export_task_return_{name}({args})");
            }
            Instruction::LengthStore { offset } => store(
                self,
                &format!("uint32({})", operands[0]),
                &operands[1],
                offset,
                "uint32",
            ),
            Instruction::PointerStore { offset } => store(
                self,
                &format!("uint32(uintptr({}))", operands[0]),
                &operands[1],
                offset,
                "uint32",
            ),
            Instruction::I32Store8 { offset } => store(
                self,
                &format!("int8({})", operands[0]),
                &operands[1],
                offset,
                "int8",
            ),
            Instruction::I32Store16 { offset } => store(
                self,
                &format!("int16({})", operands[0]),
                &operands[1],
                offset,
                "int16",
            ),
            Instruction::I32Store { offset } => {
                store(self, &operands[0], &operands[1], offset, "int32")
            }
            Instruction::I64Store { offset } => {
                store(self, &operands[0], &operands[1], offset, "int64")
            }
            Instruction::F32Store { offset } => {
                store(self, &operands[0], &operands[1], offset, "float32")
            }
            Instruction::F64Store { offset } => {
                store(self, &operands[0], &operands[1], offset, "float64")
            }
            Instruction::LengthLoad { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| v)
            }
            Instruction::PointerLoad { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| {
                    format!("uintptr({v})")
                })
            }
            Instruction::I32Load8U { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| {
                    format!("uint8({v})")
                })
            }
            Instruction::I32Load8S { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| {
                    format!("int8({v})")
                })
            }
            Instruction::I32Load16U { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| {
                    format!("uint16({v})")
                })
            }
            Instruction::I32Load16S { offset } => {
                load(self, results, &operands[0], offset, "uint32", &|v| {
                    format!("int16({v})")
                })
            }
            Instruction::I32Load { offset } => {
                load(self, results, &operands[0], offset, "int32", &|v| v)
            }
            Instruction::I64Load { offset } => {
                load(self, results, &operands[0], offset, "int64", &|v| v)
            }
            Instruction::F32Load { offset } => {
                load(self, results, &operands[0], offset, "float32", &|v| v)
            }
            Instruction::F64Load { offset } => {
                load(self, results, &operands[0], offset, "float64", &|v| v)
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),
            Instruction::U8FromI32 => results.push(format!("uint8({})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("int8({})", operands[0])),
            Instruction::U16FromI32 => results.push(format!("uint16({})", operands[0])),
            Instruction::S16FromI32 => results.push(format!("int16({})", operands[0])),
            Instruction::U32FromI32 => results.push(format!("uint32({})", operands[0])),
            Instruction::S32FromI32 | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap())
            }
            Instruction::U64FromI64 => results.push(format!("uint64({})", operands[0])),
            Instruction::I32FromBool => {
                let value = &operands[0];
                let result = self.locals.tmp("result");
                uwriteln!(
                    self.src,
                    "var {result} int32
if {value} {{
        {result} = 1
}} else {{
        {result} = 0
}}"
                );
                results.push(result);
            }
            Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32 => {
                results.push(format!("int32({})", operands[0]));
            }
            Instruction::I32FromS32 | Instruction::I64FromS64 => {
                results.push(operands.pop().unwrap())
            }
            Instruction::I64FromU64 => results.push(format!("int64({})", operands[0])),
            Instruction::CoreF32FromF32
            | Instruction::CoreF64FromF64
            | Instruction::F32FromCoreF32
            | Instruction::F64FromCoreF64 => results.push(operands.pop().unwrap()),
            Instruction::CharFromI32 => results.push(format!("rune({})", operands[0])),
            Instruction::I32FromChar => results.push(format!("int32({})", operands[0])),
            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                for index in 0..tuple.types.len() {
                    results.push(format!("({op}).F{index}"));
                }
            }
            Instruction::TupleLift { tuple, .. } => {
                let count = tuple.types.len();
                self.generator.tuples.insert(count);
                let types = tuple
                    .types
                    .iter()
                    .map(|&ty| self.type_name(resolve, ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                let fields = operands.join(", ");
                self.imports.insert(remote_pkg("wit_types"));
                results.push(format!("wit_types.Tuple{count}[{types}]{{{fields}}}"));
            }
            Instruction::FlagsLower { .. } => {
                let value = operands.pop().unwrap();
                results.push(format!("int32({value})"))
            }
            Instruction::FlagsLift { flags, .. } => {
                let value = operands.pop().unwrap();
                let repr = flags_repr(flags);
                results.push(format!("{repr}({value})"))
            }
            Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for field in &record.fields {
                    let field = field.name.to_upper_camel_case();
                    results.push(format!("({op}).{field}"));
                }
            }
            Instruction::RecordLift { ty, .. } => {
                let name = self.type_name(resolve, Type::Id(*ty));
                let fields = operands.join(", ");
                results.push(format!("{name}{{{fields}}}"));
            }
            Instruction::OptionLower {
                results: result_types,
                ..
            } => {
                self.generator.need_option = true;
                self.imports.insert(remote_pkg("wit_types"));
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                let value = &operands[0];

                let result_names = (0..result_types.len())
                    .map(|_| self.locals.tmp("option"))
                    .collect::<Vec<_>>();

                let declarations = result_types
                    .iter()
                    .zip(&result_names)
                    .map(|(ty, name)| {
                        let ty = wasm_type(*ty);
                        format!("var {name} {ty}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let some_result_assignments = some_results
                    .iter()
                    .zip(&result_names)
                    .map(|(result, name)| format!("{name} = {result}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                let none_result_assignments = none_results
                    .iter()
                    .zip(&result_names)
                    .map(|(result, name)| format!("{name} = {result}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                results.extend(result_names);

                uwriteln!(
                    self.src,
                    r#"{declarations}
switch {value}.Tag() {{
case wit_types.OptionNone:
        {none}
        {none_result_assignments}
case wit_types.OptionSome:
        {VARIANT_PAYLOAD_NAME} := {value}.Some()
        {some}
        {some_result_assignments}
default:
        panic("unreachable")
}}"#
                );
            }
            Instruction::OptionLift { ty, payload } => {
                self.generator.need_option = true;
                self.imports.insert(remote_pkg("wit_types"));
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.is_empty());
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];
                let ty = self.type_name(resolve, Type::Id(*ty));
                let some_type = self.type_name(resolve, **payload);
                let result = self.locals.tmp("option");
                let tag = &operands[0];
                uwriteln!(
                    self.src,
                    r#"var {result} {ty}
switch {tag} {{
case 0:
        {none}
        {result} = wit_types.None[{some_type}]()
case 1:
        {some}
        {result} = wit_types.Some[{some_type}]({some_result})
default:
        panic("unreachable")
}}"#
                );
                results.push(result);
            }
            Instruction::ResultLower {
                result,
                results: result_types,
                ..
            } => {
                self.generator.need_result = true;
                self.imports.insert(remote_pkg("wit_types"));
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let value = &operands[0];

                let result_names = (0..result_types.len())
                    .map(|_| self.locals.tmp("option"))
                    .collect::<Vec<_>>();

                let declarations = result_types
                    .iter()
                    .zip(&result_names)
                    .map(|(ty, name)| {
                        let ty = wasm_type(*ty);
                        format!("var {name} {ty}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let ok_result_assignments = ok_results
                    .iter()
                    .zip(&result_names)
                    .map(|(result, name)| format!("{name} = {result}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                let err_result_assignments = err_results
                    .iter()
                    .zip(&result_names)
                    .map(|(result, name)| format!("{name} = {result}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                results.extend(result_names);

                let ok_set_payload = if result.ok.is_some() {
                    format!("{VARIANT_PAYLOAD_NAME} := {value}.Ok()")
                } else {
                    self.generator.need_unit = true;
                    String::new()
                };

                let err_set_payload = if result.err.is_some() {
                    format!("{VARIANT_PAYLOAD_NAME} := {value}.Err()")
                } else {
                    self.generator.need_unit = true;
                    String::new()
                };

                uwriteln!(
                    self.src,
                    r#"{declarations}
switch {value}.Tag() {{
case wit_types.ResultOk:
        {ok_set_payload}
        {ok}
        {ok_result_assignments}
case wit_types.ResultErr:
        {err_set_payload}
        {err}
        {err_result_assignments}
default:
        panic("unreachable")
}}"#
                );
            }
            Instruction::ResultLift { ty, result, .. } => {
                self.generator.need_result = true;
                self.imports.insert(remote_pkg("wit_types"));
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                assert_eq!(ok_results.is_empty(), result.ok.is_none());
                assert_eq!(err_results.is_empty(), result.err.is_none());
                let ok_result = if result.ok.is_some() {
                    &ok_results[0]
                } else {
                    self.generator.need_unit = true;
                    "wit_types.Unit{}"
                };
                let err_result = if result.err.is_some() {
                    &err_results[0]
                } else {
                    self.generator.need_unit = true;
                    "wit_types.Unit{}"
                };
                let ty = self.type_name(resolve, Type::Id(*ty));
                let ok_type = result
                    .ok
                    .map(|ty| self.type_name(resolve, ty))
                    .unwrap_or_else(|| {
                        self.generator.need_unit = true;
                        "wit_types.Unit".into()
                    });
                let err_type = result
                    .err
                    .map(|ty| self.type_name(resolve, ty))
                    .unwrap_or_else(|| {
                        self.generator.need_unit = true;
                        "wit_types.Unit".into()
                    });
                let result = self.locals.tmp("result");
                let tag = &operands[0];
                uwriteln!(
                    self.src,
                    r#"var {result} {ty}
switch {tag} {{
case 0:
        {ok}
        {result} = wit_types.Ok[{ok_type}, {err_type}]({ok_result})
case 1:
        {err}
        {result} = wit_types.Err[{ok_type}, {err_type}]({err_result})
default:
        panic("unreachable")
}}"#
                );
                results.push(result);
            }
            Instruction::EnumLower { .. } => results.push(format!("int32({})", operands[0])),
            Instruction::EnumLift { enum_, .. } => {
                results.push(format!("{}({})", int_repr(enum_.tag()), operands[0]))
            }
            Instruction::VariantLower {
                ty,
                variant,
                results: result_types,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let ty = self.type_name(resolve, Type::Id(*ty));
                let value = &operands[0];

                let result_names = (0..result_types.len())
                    .map(|_| self.locals.tmp("variant"))
                    .collect::<Vec<_>>();

                let declarations = result_types
                    .iter()
                    .zip(&result_names)
                    .map(|(ty, name)| {
                        let ty = wasm_type(*ty);
                        format!("var {name} {ty}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let cases = variant
                    .cases
                    .iter()
                    .zip(blocks)
                    .map(|(case, (block, block_results))| {
                        let assignments = result_names
                            .iter()
                            .zip(&block_results)
                            .map(|(name, result)| format!("{name} = {result}"))
                            .collect::<Vec<_>>()
                            .join("\n");

                        let name = case.name.to_upper_camel_case();

                        let set_payload = if case.ty.is_some() {
                            format!("{VARIANT_PAYLOAD_NAME} := {value}.{name}()")
                        } else {
                            String::new()
                        };

                        format!(
                            "case {ty}{name}:
        {set_payload}
        {block}
        {assignments}
"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                results.extend(result_names);

                uwriteln!(
                    self.src,
                    r#"{declarations}
switch {value}.Tag() {{
{cases}
default:
        panic("unreachable")
}}"#
                );
            }
            Instruction::VariantLift { ty, variant, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let ty = self.type_name(resolve, Type::Id(*ty));
                let result = self.locals.tmp("variant");
                let tag = &operands[0];

                let (package, name) = if let Some(index) = ty.find('.') {
                    (&ty[..index + 1], &ty[index + 1..])
                } else {
                    ("", ty.as_str())
                };

                let cases = variant
                    .cases
                    .iter()
                    .zip(blocks)
                    .enumerate()
                    .map(|(index, (case, (block, block_results)))| {
                        assert_eq!(block_results.is_empty(), case.ty.is_none());
                        let payload = if case.ty.is_some() {
                            &block_results[0]
                        } else {
                            ""
                        };
                        let case = case.name.to_upper_camel_case();
                        format!(
                            "case {index}:
        {block}
        {result} = {package}Make{name}{case}({payload})
"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                uwriteln!(
                    self.src,
                    r#"var {result} {ty}
switch {tag} {{
{cases}
default:
        panic("unreachable")
}}"#
                );
                results.push(result);
            }
            Instruction::VariantPayloadName => results.push(VARIANT_PAYLOAD_NAME.into()),
            Instruction::IterElem { .. } => results.push(ITER_ELEMENT.into()),
            Instruction::IterBasePointer => results.push(ITER_BASE_POINTER.into()),
            Instruction::I32Const { val } => results.push(format!("int32({val})")),
            Instruction::ConstZero { tys } => {
                results.extend(iter::repeat_with(|| "0".into()).take(tys.len()));
            }
            Instruction::Bitcasts { casts } => {
                results.extend(
                    casts
                        .iter()
                        .zip(operands)
                        .map(|(which, op)| cast(op, which, &mut self.need_math)),
                );
            }
            Instruction::FutureLower { .. }
            | Instruction::StreamLower { .. }
            | Instruction::HandleLower {
                handle: Handle::Own(_),
                ..
            } => {
                let op = &operands[0];
                if self.collect_lifters {
                    self.lifter_count += 1;
                    let resource = self.locals.tmp("resource");
                    let handle = self.locals.tmp("handle");
                    uwriteln!(
                        self.src,
                        "{resource} := {op}
{handle} := {resource}.TakeHandle()
lifters = append(lifters, func() {{
        {resource}.SetHandle({handle})
}})"
                    );
                    results.push(handle)
                } else {
                    results.push(format!("({op}).TakeHandle()"))
                }
            }
            Instruction::HandleLower {
                handle: Handle::Borrow(_),
                ..
            } => results.push(format!("({}).Handle()", operands[0])),
            Instruction::HandleLift { handle, .. } => {
                let (which, resource) = match handle {
                    Handle::Borrow(resource) => ("Borrow", resource),
                    Handle::Own(resource) => ("Own", resource),
                };
                let handle = &operands[0];
                let ty = self.type_name(resolve, Type::Id(*resource));
                results.push(format!("{ty}From{which}Handle(int32(uintptr({handle})))"))
            }
            Instruction::CallWasm { sig, .. } => {
                let assignment = match &sig.results[..] {
                    [] => String::new(),
                    [_] => {
                        let result = self.locals.tmp("result");
                        let assignment = format!("{result} := ");
                        results.push(result);
                        assignment
                    }
                    _ => unreachable!(),
                };
                let name = &self.function_to_call;
                let params = operands.join(", ");
                uwriteln!(self.src, "{assignment}{name}({params})")
            }
            Instruction::Flush { amt } => {
                for op in operands.iter().take(*amt) {
                    let result = self.locals.tmp("result");
                    uwriteln!(self.src, "{result} := {op};");
                    results.push(result);
                }
            }
            Instruction::FutureLift { ty, .. } => {
                let exported = self.generator.has_exported_resource(resolve, Type::Id(*ty));
                let owner = self
                    .generator
                    .futures_and_streams
                    .get(&(*ty, exported))
                    .unwrap()
                    .clone();
                let package = self.package_for_owner(resolve, owner.as_ref(), *ty);
                let TypeDefKind::Future(payload_ty) = &resolve.types[*ty].kind else {
                    unreachable!()
                };
                let camel = if let Some(ty) = payload_ty {
                    self.generator
                        .mangle_name(resolve, *ty, owner.as_ref())
                        .to_upper_camel_case()
                } else {
                    "Unit".into()
                };
                let handle = &operands[0];
                results.push(format!("{package}LiftFuture{camel}({handle})"));
            }
            Instruction::StreamLift { ty, .. } => {
                let exported = self.generator.has_exported_resource(resolve, Type::Id(*ty));
                let owner = self
                    .generator
                    .futures_and_streams
                    .get(&(*ty, exported))
                    .unwrap()
                    .clone();
                let package = self.package_for_owner(resolve, owner.as_ref(), *ty);
                let TypeDefKind::Stream(payload_ty) = &resolve.types[*ty].kind else {
                    unreachable!()
                };
                let camel = if let Some(ty) = payload_ty {
                    self.generator
                        .mangle_name(resolve, *ty, owner.as_ref())
                        .to_upper_camel_case()
                } else {
                    "Unit".into()
                };
                let handle = &operands[0];
                results.push(format!("{package}LiftStream{camel}({handle})"));
            }
            Instruction::GuestDeallocate { .. } => {
                // Nothing to do here; should be handled when calling `pinner.Unpin()`
            }
            _ => unimplemented!("{instruction:?}"),
        }
    }
}

struct InterfaceGenerator<'a> {
    generator: &'a mut Go,
    resolve: &'a Resolve,
    interface: Option<(InterfaceId, &'a WorldKey)>,
    in_import: bool,
    src: String,
    imports: BTreeSet<String>,
    need_unsafe: bool,
    need_runtime: bool,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(
        generator: &'a mut Go,
        resolve: &'a Resolve,
        interface: Option<(InterfaceId, &'a WorldKey)>,
        in_import: bool,
    ) -> Self {
        Self {
            generator,
            resolve,
            interface,
            in_import,
            src: String::new(),
            imports: BTreeSet::new(),
            need_unsafe: false,
            need_runtime: false,
        }
    }

    fn type_name(&mut self, resolve: &Resolve, ty: Type) -> String {
        self.generator.type_name(
            resolve,
            ty,
            self.interface.map(|(_, key)| key),
            self.in_import || !self.generator.has_exported_resource(resolve, ty),
            &mut self.imports,
        )
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _: TypeId, name: &str, record: &Record, docs: &Docs) {
        let name = name.to_upper_camel_case();

        let fields = record
            .fields
            .iter()
            .map(|field| {
                let ty = self.type_name(self.resolve, field.ty);
                let docs = format_docs(&field.docs);
                let field = field.name.to_upper_camel_case();
                format!("{docs}{field} {ty}")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let docs = format_docs(docs);

        uwriteln!(
            self.src,
            "
{docs}type {name} struct {{
        {fields}
}}"
        )
    }

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        self.generator.resources.insert(
            id,
            if self.in_import {
                Direction::Import
            } else {
                Direction::Export
            },
        );

        let camel = name.to_upper_camel_case();
        let module = self
            .interface
            .map(|(_, key)| self.resolve.name_world_key(key))
            .unwrap_or_else(|| "$root".into());

        if self.in_import {
            self.imports.insert(remote_pkg("wit_runtime"));
            self.need_runtime = true;
            let docs = format_docs(docs);
            uwriteln!(
                self.src,
                r#"
//go:wasmimport {module} [resource-drop]{name}
func resourceDrop{camel}(handle int32)

{docs}type {camel} struct {{
        handle *wit_runtime.Handle
}}

func (self *{camel}) TakeHandle() int32 {{
        return self.handle.Take()
}}

func (self *{camel}) SetHandle(handle int32) {{
        self.handle.Set(handle)
}}

func (self *{camel}) Handle() int32 {{
        return self.handle.Use()
}}

func (self *{camel}) Drop() {{
	handle := self.handle.TakeOrNil()
	if handle != 0 {{
		resourceDrop{camel}(handle)
	}}
}}

func {camel}FromOwnHandle(handleValue int32) *{camel} {{
        handle := wit_runtime.MakeHandle(handleValue)
        value := &{camel}{{handle}}
        runtime.AddCleanup(value, func(_ int) {{
                handleValue := handle.TakeOrNil()
                if handleValue != 0 {{
                        resourceDrop{camel}(handleValue)
                }}
        }}, 0)
        return value
}}

func {camel}FromBorrowHandle(handleValue int32) *{camel} {{
	handle := wit_runtime.MakeHandle(handleValue)
	return &{camel}{{handle}}
}}
"#
            );
        } else {
            self.need_unsafe = true;
            uwriteln!(
                self.src,
                r#"
//go:wasmimport [export]{module} [resource-new]{name}
func resourceNew{camel}(pointer unsafe.Pointer) int32

//go:wasmimport [export]{module} [resource-rep]{name}
func resourceRep{camel}(handle int32) unsafe.Pointer

//go:wasmimport [export]{module} [resource-drop]{name}
func resourceDrop{camel}(handle int32)

//go:wasmexport {module}#[dtor]{name}
func resourceDtor{camel}(rep int32) {{
        val := (*{camel})(unsafe.Pointer(uintptr(rep)))
        val.handle = 0
        val.pinner.Unpin()
        val.OnDrop()
}}

func (self *{camel}) TakeHandle() int32 {{
	self.pinner.Pin(self)
	self.handle = resourceNew{camel}(unsafe.Pointer(self))
	return self.handle
}}

func (self *{camel}) SetHandle(handle int32) {{
        if self.handle != handle {{
                panic("invalid handle")
        }}
}}

func (self *{camel}) Drop() {{
	handle := self.handle
	if self.handle != 0 {{
		self.handle = 0
		resourceDrop{camel}(handle)
		self.pinner.Unpin()
                self.OnDrop()
	}}
}}

func {camel}FromOwnHandle(handle int32) *{camel} {{
	return (*{camel})(unsafe.Pointer(resourceRep{camel}(handle)))
}}

func {camel}FromBorrowHandle(rep int32) *{camel} {{
	return (*{camel})(unsafe.Pointer(uintptr(rep)))
}}
"#
            );

            if self.generator.opts.generate_stubs {
                self.need_runtime = true;
                uwriteln!(
                    self.src,
                    r#"
type {camel} struct {{
        pinner runtime.Pinner
        handle int32
}}

func (self *{camel}) OnDrop() {{}}
"#
                );
            }
        }
    }

    fn type_flags(&mut self, _: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        let repr = flags_repr(flags);

        let name = name.to_upper_camel_case();

        let constants = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let docs = format_docs(&flag.docs);
                let flag = flag.name.to_upper_camel_case();
                format!("{docs}{name}{flag} {repr} = 1 << {i}")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let docs = format_docs(docs);

        uwriteln!(
            self.src,
            "
const (
{constants}
)

{docs}type {name} = {repr}"
        )
    }

    fn type_tuple(&mut self, _: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.imports.insert(remote_pkg("wit_types"));
        let count = tuple.types.len();
        self.generator.tuples.insert(count);
        let name = name.to_upper_camel_case();
        let docs = format_docs(docs);
        let types = tuple
            .types
            .iter()
            .map(|ty| self.type_name(self.resolve, *ty))
            .collect::<Vec<_>>()
            .join(", ");

        uwriteln!(
            self.src,
            "{docs}type {name} = wit_types.Tuple{count}[{types}]"
        );
    }

    fn type_variant(&mut self, _: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        let repr = int_repr(variant.tag());

        let name = name.to_upper_camel_case();

        let constants = variant
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| {
                let docs = format_docs(&case.docs);
                let case = case.name.to_upper_camel_case();
                format!("{docs}{name}{case} {repr} = {i}")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let getters = variant
            .cases
            .iter()
            .filter_map(|case| {
                case.ty.map(|ty| {
                    let case = case.name.to_upper_camel_case();
                    let ty = self.type_name(self.resolve, ty);
                    format!(
                        r#"func (self {name}) {case}() {ty} {{
        if self.tag != {name}{case} {{
                panic("tag mismatch")
        }}
        return self.value.({ty})
}}
"#
                    )
                })
            })
            .collect::<Vec<_>>()
            .concat();

        let constructors = variant
            .cases
            .iter()
            .map(|case| {
                let (param, value) = if let Some(ty) = case.ty {
                    let ty = self.type_name(self.resolve, ty);
                    (format!("value {ty}"), "value")
                } else {
                    (String::new(), "nil")
                };
                let case = case.name.to_upper_camel_case();
                format!(
                    r#"func Make{name}{case}({param}) {name} {{
        return {name}{{{name}{case}, {value}}}
}}
"#
                )
            })
            .collect::<Vec<_>>()
            .concat();

        let docs = format_docs(docs);

        uwriteln!(
            self.src,
            "
const (
{constants}
)

{docs}type {name} struct {{
        tag {repr}
        value any
}}

func (self {name}) Tag() {repr} {{
        return self.tag
}}

{getters}
{constructors}
"
        )
    }

    fn type_option(&mut self, _: TypeId, name: &str, payload: &Type, docs: &Docs) {
        self.generator.need_option = true;
        self.imports.insert(remote_pkg("wit_types"));
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, *payload);
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = wit_types.Option[{ty}]");
    }

    fn type_result(&mut self, _: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.generator.need_result = true;
        self.imports.insert(remote_pkg("wit_types"));
        let name = name.to_upper_camel_case();
        let ok_type = result
            .ok
            .map(|ty| self.type_name(self.resolve, ty))
            .unwrap_or_else(|| {
                self.generator.need_unit = true;
                "wit_types.Unit".into()
            });
        let err_type = result
            .err
            .map(|ty| self.type_name(self.resolve, ty))
            .unwrap_or_else(|| {
                self.generator.need_unit = true;
                "wit_types.Unit".into()
            });
        let docs = format_docs(docs);
        uwriteln!(
            self.src,
            "{docs}type {name} = wit_types.Result[{ok_type}, {err_type}]"
        );
    }

    fn type_enum(&mut self, _: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let repr = int_repr(enum_.tag());

        let name = name.to_upper_camel_case();

        let constants = enum_
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| {
                let docs = format_docs(&case.docs);
                let case = case.name.to_upper_camel_case();
                format!("{docs}{name}{case} {repr} = {i}")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let docs = format_docs(docs);

        uwriteln!(
            self.src,
            "
const (
        {constants}
)
{docs}type {name} = {repr}"
        )
    }

    fn type_alias(&mut self, _: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, *ty);
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = {ty}");
    }

    fn type_list(&mut self, _: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, *ty);
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = []{ty}");
    }

    fn type_fixed_length_list(&mut self, _: TypeId, name: &str, ty: &Type, size: u32, docs: &Docs) {
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, *ty);
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = [{size}]{ty}");
    }

    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        _ = (id, name, ty, docs);
        todo!()
    }

    fn type_future(&mut self, id: TypeId, name: &str, _: &Option<Type>, docs: &Docs) {
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, Type::Id(id));
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = {ty}");
    }

    fn type_stream(&mut self, id: TypeId, name: &str, _: &Option<Type>, docs: &Docs) {
        let name = name.to_upper_camel_case();
        let ty = self.type_name(self.resolve, Type::Id(id));
        let docs = format_docs(docs);
        uwriteln!(self.src, "{docs}type {name} = {ty}");
    }
}

fn interface_name(resolve: &Resolve, interface: Option<&WorldKey>) -> String {
    match interface {
        Some(WorldKey::Name(name)) => name.to_snake_case(),
        Some(WorldKey::Interface(id)) => {
            let interface = &resolve.interfaces[*id];
            let package = &resolve.packages[interface.package.unwrap()];
            let package_has_multiple_versions = resolve.packages.iter().any(|(_, p)| {
                p.name.namespace == package.name.namespace
                    && p.name.name == package.name.name
                    && p.name.version != package.name.version
            });
            let version = if package_has_multiple_versions {
                if let Some(version) = &package.name.version {
                    format!("{}_", version.to_string().replace(['.', '-', '+'], "_"))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let namespace = package.name.namespace.to_snake_case();
            let package = package.name.name.to_snake_case();
            let interface = interface.name.as_ref().unwrap().to_snake_case();
            format!("{namespace}_{package}_{version}{interface}")
        }
        None => "wit_world".into(),
    }
}

fn func_name(resolve: &Resolve, interface: Option<&WorldKey>, func: &Function) -> String {
    let prefix = interface_name(resolve, interface);
    let name = func.name.to_snake_case().replace('.', "_");

    format!("{prefix}_{name}")
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int32",
        WasmType::I64 => "int64",
        WasmType::F32 => "float32",
        WasmType::F64 => "float64",
        WasmType::Pointer => "uintptr",
        WasmType::PointerOrI64 => "int64",
        WasmType::Length => "uint32",
    }
}

fn format_docs(docs: &Docs) -> String {
    docs.contents
        .as_ref()
        .map(|v| {
            v.trim()
                .lines()
                .map(|line| format!("// {line}\n"))
                .collect::<Vec<_>>()
                .concat()
        })
        .unwrap_or_default()
}

fn flags_repr(flags: &Flags) -> &'static str {
    match flags.repr() {
        FlagsRepr::U8 => "uint8",
        FlagsRepr::U16 => "uint16",
        FlagsRepr::U32(1) => "uint32",
        _ => unreachable!(),
    }
}

fn int_repr(int: Int) -> &'static str {
    match int {
        Int::U8 => "uint8",
        Int::U16 => "uint16",
        Int::U32 => "uint32",
        Int::U64 => unreachable!(),
    }
}

fn cast(op: &str, which: &Bitcast, need_math: &mut bool) -> String {
    match which {
        Bitcast::I32ToF32 | Bitcast::I64ToF32 => {
            *need_math = true;
            format!("math.Float32frombits(uint32({op}))")
        }
        Bitcast::F32ToI32 => {
            *need_math = true;
            format!("int32(math.Float32bits({op}))")
        }
        Bitcast::F32ToI64 => {
            *need_math = true;
            format!("int64(math.Float32bits({op}))")
        }
        Bitcast::I64ToF64 => {
            *need_math = true;
            format!("math.Float64frombits(uint64({op}))")
        }
        Bitcast::F64ToI64 => {
            *need_math = true;
            format!("int64(math.Float64bits({op}))")
        }
        Bitcast::I32ToI64 | Bitcast::LToI64 => {
            format!("int64({op})")
        }
        Bitcast::PToP64 => {
            format!("int64({op})")
        }
        Bitcast::I64ToI32 | Bitcast::I64ToL | Bitcast::PToI32 => {
            format!("int32({op})")
        }
        Bitcast::I64ToP64 | Bitcast::P64ToI64 => op.into(),
        Bitcast::P64ToP | Bitcast::LToP | Bitcast::I32ToP => {
            format!("uintptr({op})")
        }
        Bitcast::PToL => {
            format!("uint32({op})")
        }
        Bitcast::I32ToL => {
            format!("uint32({op})")
        }
        Bitcast::LToI32 => {
            format!("uint32({op})")
        }
        Bitcast::None => op.to_string(),
        Bitcast::Sequence(sequence) => {
            let [first, second] = &**sequence;
            let inner = cast(op, first, need_math);
            cast(&inner, second, need_math)
        }
    }
}

fn any(resolve: &Resolve, ty: Type, fun: &dyn Fn(Type) -> bool) -> bool {
    if fun(ty) {
        return true;
    }

    match ty {
        Type::Bool
        | Type::U8
        | Type::S8
        | Type::U16
        | Type::S16
        | Type::U32
        | Type::S32
        | Type::U64
        | Type::S64
        | Type::F32
        | Type::F64
        | Type::Char
        | Type::String => false,
        Type::Id(id) => {
            let ty = &resolve.types[id];
            match &ty.kind {
                TypeDefKind::Flags(_) | TypeDefKind::Enum(_) | TypeDefKind::Resource => false,
                TypeDefKind::Handle(Handle::Own(resource) | Handle::Borrow(resource)) => {
                    any(resolve, Type::Id(*resource), fun)
                }
                TypeDefKind::Record(record) => record
                    .fields
                    .iter()
                    .any(|field| any(resolve, field.ty, fun)),
                TypeDefKind::Variant(variant) => variant
                    .cases
                    .iter()
                    .any(|case| case.ty.map(|ty| any(resolve, ty, fun)).unwrap_or(false)),
                TypeDefKind::Option(ty) | TypeDefKind::List(ty) | TypeDefKind::Type(ty) => {
                    any(resolve, *ty, fun)
                }
                TypeDefKind::Result(result) => result
                    .ok
                    .map(|ty| any(resolve, ty, fun))
                    .or_else(|| result.err.map(|ty| any(resolve, ty, fun)))
                    .unwrap_or(false),
                TypeDefKind::Tuple(tuple) => tuple.types.iter().any(|ty| any(resolve, *ty, fun)),
                TypeDefKind::Future(ty) | TypeDefKind::Stream(ty) => {
                    ty.map(|ty| any(resolve, ty, fun)).unwrap_or(false)
                }
                _ => todo!("{:?}", ty.kind),
            }
        }
        _ => todo!("{ty:?}"),
    }
}

fn func_declaration(resolve: &Resolve, func: &Function) -> (String, bool) {
    match &func.kind {
        FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
            (func.item_name().to_upper_camel_case(), false)
        }
        FunctionKind::Constructor(ty) => {
            let ty = resolve.types[*ty]
                .name
                .as_ref()
                .unwrap()
                .to_upper_camel_case();
            (format!("Make{ty}"), false)
        }
        FunctionKind::Method(ty) | FunctionKind::AsyncMethod(ty) => {
            let ty = resolve.types[*ty]
                .name
                .as_ref()
                .unwrap()
                .to_upper_camel_case();
            let camel = func.item_name().to_upper_camel_case();
            (format!("(self *{ty}) {camel}"), true)
        }
        FunctionKind::Static(ty) | FunctionKind::AsyncStatic(ty) => {
            let ty = resolve.types[*ty]
                .name
                .as_ref()
                .unwrap()
                .to_upper_camel_case();
            let camel = func.item_name().to_upper_camel_case();
            (format!("{ty}{camel}"), false)
        }
    }
}

fn maybe_gofmt<'a>(format: Format, code: &'a [u8]) -> Cow<'a, [u8]> {
    thread::scope(|s| {
        if let Format::True = format
            && let Ok((reader, mut writer)) = io::pipe()
        {
            s.spawn(move || {
                _ = writer.write_all(code);
            });

            if let Ok(output) = Command::new("gofmt").stdin(reader).output()
                && output.status.success()
            {
                return Cow::Owned(output.stdout);
            }
        }

        Cow::Borrowed(code)
    })
}
