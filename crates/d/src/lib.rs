use anyhow::Result;
use heck::*;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::mem::take;
use std::path::PathBuf;
use wit_bindgen_core::{
    Direction, Files, InterfaceGenerator, Source, Types, WorldGenerator, abi::WasmType,
    wit_parser::*,
};

#[derive(Default)]
struct D {
    used_interfaces: HashSet<(WorldKey, InterfaceId)>,

    interface_imports: Vec<String>,
    interface_exports: Vec<String>,
    type_imports_src: Source,
    function_imports_src: Source,
    function_exports_src: Source,

    opts: Opts,

    world_id: Option<WorldId>,
    world_fqn: String,
    interface_fqns: HashMap<InterfaceId, InterfaceFQNSet>,

    cur_interface: Option<InterfaceId>,

    types: Types,
}

#[derive(Default, Debug)]
struct InterfaceFQNSet {
    import: Option<String>,
    export: Option<String>,
    common: Option<String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Where to place output files
    #[cfg_attr(feature = "clap", arg(skip))]
    out_dir: Option<PathBuf>,
}

impl Opts {
    pub fn build(mut self, out_dir: Option<&PathBuf>) -> Box<dyn WorldGenerator> {
        let mut r = D::default();
        self.out_dir = out_dir.cloned();
        r.opts = self.clone();
        Box::new(r)
    }
}

fn escape_d_identifier(name: &str) -> &str {
    match name {
        // Escape D keywords.
        // Source: https://dlang.org/spec/lex.html#keywords
        "abstract" => "abstract_",
        "alias" => "alias_",
        "align" => "align_",
        "asm" => "asm_",
        "assert" => "assert_",
        "auto" => "auto_",

        "body" => "body_",
        "bool" => "bool_",
        "break" => "break_",
        "byte" => "byte_",

        "case" => "case_",
        "cast" => "cast_",
        "catch" => "catch_",
        "cdouble" => "cdouble_",
        "cent" => "cent_",
        "cfloat" => "cfloat_",
        "char" => "char_",
        "class" => "class_",
        "const" => "const_",
        "continue" => "continue_",
        "creal" => "creal_",

        "dchar" => "dchar_",
        "debug" => "debug_",
        "default" => "default_",
        "delegate" => "delegate_",
        "delete" => "delete_",
        "deprecated" => "deprecated_",
        "do" => "do_",
        "double" => "double_",

        "else" => "else_",
        "enum" => "enum_",
        "export" => "export_",
        "extern" => "extern_",

        "false" => "false_",
        "final" => "final_",
        "finally" => "finally_",
        "float" => "float_",
        "for" => "for_",
        "foreach" => "foreach_",
        "foreach_reverse" => "foreach_reverse_",
        "function" => "function_",

        "goto" => "goto_",

        "idouble" => "idouble_",
        "if" => "if_",
        "ifloat" => "ifloat_",
        "immutable" => "immutable_",
        "import" => "import_",
        "in" => "in_",
        "inout" => "inout_",
        "int" => "int_",
        "interface" => "interface_",
        "invariant" => "invariant_",
        "ireal" => "ireal_",
        "is" => "is_",

        "lazy" => "lazy_",
        "long" => "long_",

        "macro" => "macro_",
        "mixin" => "mixin_",
        "module" => "module_",

        "new" => "new_",
        "nothrow" => "nothrow_",
        "null" => "null_",

        "out" => "out_",
        "override" => "override_",

        "package" => "package_",
        "pragma" => "pragma_",
        "private" => "private_",
        "protected" => "protected_",
        "public" => "public_",
        "pure" => "pure_",

        "real" => "real_",
        "ref" => "ref_",
        "return" => "return_",

        "scope" => "scope_",
        "shared" => "shared_",
        "short" => "short_",
        "static" => "static_",
        "struct" => "struct_",
        "super" => "super_",
        "switch" => "switch_",
        "synchronized" => "synchronized_",

        "template" => "template_",
        "this" => "this_",
        "throw" => "throw_",
        "true" => "true_",
        "try" => "try_",
        "typeid" => "typeid_",
        "typeof" => "typeof_",

        "ubyte" => "ubyte_",
        "ucent" => "ucent_",
        "uint" => "uint_",
        "ulong" => "ulong_",
        "union" => "union_",
        "unittest" => "unittest_",
        "ushort" => "ushort_",

        "version" => "version_",
        "void" => "void_",

        "wchar" => "wchar_",
        "while" => "while_",
        "with" => "with_",

        // Symbols we define as part of the bindings we want to avoid creating conflicts with
        "WitList" => "WitList_",
        "WitString" => "WitString_",
        "WitFlags" => "WitFlags_",
        "Option" => "Option_",
        "Result" => "Result_",
        "bits" => "bits_",     // part of WitFlags
        "borrow" => "borrow_", // part of the expansion of `resource`
        "drop" => "drop_",     // part of the expansion of `resource`

        s => s,
    }
}

pub fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "uint",
        WasmType::I64 => "ulong",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
        WasmType::Pointer => "void*",
        WasmType::PointerOrI64 => "ulong",
        WasmType::Length => "size_t",
    }
}

fn get_package_fqn(id: PackageId, resolve: &Resolve) -> String {
    let pkg = &resolve.packages[id];
    let pkg_has_multiple_versions = resolve.packages.iter().any(|(_, p)| {
        p.name.namespace == pkg.name.namespace
            && p.name.name == pkg.name.name
            && p.name.version != pkg.name.version
    });

    format!(
        "wit.{}.{}{}",
        escape_d_identifier(&pkg.name.namespace.to_snake_case()),
        escape_d_identifier(&pkg.name.name.to_snake_case()),
        if pkg_has_multiple_versions {
            if let Some(version) = &pkg.name.version {
                let version = version
                    .to_string()
                    .replace('.', "_")
                    .replace('-', "_")
                    .replace('+', "_");
                format!(".{version}")
            } else {
                String::default()
            }
        } else {
            String::default()
        }
    )
}

fn get_interface_fqn(
    interface_id: &WorldKey,
    world_fqn: &str,
    resolve: &Resolve,
    direction: Option<Direction>,
) -> String {
    match interface_id {
        WorldKey::Name(name) => {
            format!(
                "{}.{}.{}",
                world_fqn,
                match direction {
                    None => panic!(
                        "Inline interfaces can only generate `import` or `export` module variant"
                    ),
                    Some(Direction::Import) => "imports",
                    Some(Direction::Export) => "exports",
                },
                escape_d_identifier(&name.to_snake_case())
            )
        }
        WorldKey::Interface(id) => {
            let iface = &resolve.interfaces[*id];

            format!(
                "{}.{}.{}",
                get_package_fqn(iface.package.unwrap(), resolve),
                escape_d_identifier(&iface.name.as_ref().unwrap().to_snake_case()),
                match direction {
                    None => "common",
                    Some(Direction::Import) => "imports",
                    Some(Direction::Export) => "exports",
                },
            )
        }
    }
}

fn get_world_fqn(id: WorldId, resolve: &Resolve) -> String {
    let world = &resolve.worlds[id];
    format!(
        "{}.{}",
        get_package_fqn(world.package.unwrap(), resolve),
        escape_d_identifier(&world.name.to_snake_case())
    )
}

impl D {
    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        direction: Option<Direction>,
        name: Option<&'a WorldKey>,
        wasm_import_module: Option<&'a str>,
    ) -> DInterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(resolve);

        DInterfaceGenerator {
            src: Source::default(),
            fqn: "",
            r#gen: self,
            resolve,
            interface: None,
            name: name,
            sizes,
            direction,

            wasm_import_module,
        }
    }

    fn lookup_interface_fqn(&self, id: InterfaceId, direction: Option<Direction>) -> Option<&str> {
        let all_fqns = &self.interface_fqns[&id];
        match direction {
            None => all_fqns.common.as_deref(),
            Some(Direction::Import) => all_fqns.import.as_deref(),
            Some(Direction::Export) => all_fqns.export.as_deref(),
        }
    }
}

impl WorldGenerator for D {
    fn uses_nominal_type_ids(&self) -> bool {
        false
    }

    fn preprocess(&mut self, resolve: &Resolve, world_id: WorldId) {
        self.world_fqn = get_world_fqn(world_id, resolve);
        self.world_id = Some(world_id);
        self.types.analyze(resolve);

        let world = &resolve.worlds[world_id];

        for (name, import) in world.imports.iter() {
            match import {
                WorldItem::Interface { id, .. } => {
                    let fqns = self.interface_fqns.entry(*id).or_insert_with(|| {
                        let mut result = InterfaceFQNSet::default();

                        match name {
                            WorldKey::Interface(_) => {
                                result.common =
                                    Some(get_interface_fqn(&name, &self.world_fqn, resolve, None));
                            }
                            WorldKey::Name(_) => {
                                // For anonymous/inline imports, the common types are in the same file as the imports
                                result.common = Some(get_interface_fqn(
                                    &name,
                                    &self.world_fqn,
                                    resolve,
                                    Some(Direction::Import),
                                ));
                            }
                        }

                        result
                    });
                    (*fqns).import = Some(get_interface_fqn(
                        &name,
                        &self.world_fqn,
                        resolve,
                        Some(Direction::Import),
                    ))
                }
                _ => {}
            }
        }

        for (name, export) in world.exports.iter() {
            match export {
                WorldItem::Interface { id, .. } => {
                    let fqns = self.interface_fqns.entry(*id).or_insert_with(|| {
                        let mut result = InterfaceFQNSet::default();

                        match name {
                            WorldKey::Interface(_) => {
                                result.common =
                                    Some(get_interface_fqn(&name, &self.world_fqn, resolve, None));
                            }
                            WorldKey::Name(_) => {
                                // For anonymous/inline exports, the common types are in the same file as the exports
                                result.common = Some(get_interface_fqn(
                                    &name,
                                    &self.world_fqn,
                                    resolve,
                                    Some(Direction::Export),
                                ));
                            }
                        }

                        result
                    });
                    (*fqns).export = Some(get_interface_fqn(
                        &name,
                        &self.world_fqn,
                        resolve,
                        Some(Direction::Export),
                    ))
                }
                _ => {}
            }
        }
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        files: &mut Files,
    ) -> Result<()> {
        self.used_interfaces.insert((name.clone(), id));

        self.cur_interface = Some(id);

        let fqn = self.interface_fqns[&id].import.as_ref().unwrap().clone();

        self.interface_imports.push(fqn.clone());

        let wasm_import_module = resolve.name_world_key(name);
        let mut r#gen = self.interface(
            resolve,
            Some(Direction::Import),
            Some(name),
            Some(&wasm_import_module),
        );
        r#gen.fqn = &fqn;
        r#gen.interface = Some(id);
        r#gen.prologue();

        r#gen.src.push_str("// Types");
        if let WorldKey::Name(_) = name {
            // We have an inline interface imported in a world.
            // Emit the "common" types as well

            r#gen.direction = None;
            r#gen.types(id);
            r#gen.direction = Some(Direction::Import);
        }

        r#gen.types(id);

        r#gen.src.push_str("\n// Functions\n");
        for (_name, func) in &resolve.interfaces[id].functions {
            match func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                    r#gen.import_func(func);
                }
                _ => {}
            }
        }

        let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
        interface_filepath.add_extension("d");

        files.push(interface_filepath.to_str().unwrap(), r#gen.src.as_bytes());

        //self.interface_imports.push(interface_src.fqn.clone());
        //interface_src.src.push_str("\n// Function imports\n");
        //interface_src.src.append_src(&tmp_src);

        self.cur_interface = None;
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut r#gen = self.interface(resolve, Some(Direction::Import), None, Some("$root"));
        for (name, id) in types.iter() {
            r#gen.define_type(name, *id);
        }

        let src = take(&mut r#gen.src);
        self.type_imports_src.append_src(&src);
    }

    fn import_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let _name = WorldKey::Name("$root".to_string());
        //let wasm_import_module = resolve.name_world_key(&name);

        for (name, _func) in funcs {
            self.function_imports_src
                .push_str(&format!("// Import function - {name}\n"));
        }
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        files: &mut Files,
    ) -> Result<()> {
        self.used_interfaces.insert((name.clone(), id));

        self.cur_interface = Some(id);

        let fqn = self.interface_fqns[&id].export.as_ref().unwrap().clone();

        self.interface_exports.push(fqn.clone());

        let wasm_import_module = resolve.name_world_key(name);
        let mut r#gen = self.interface(
            resolve,
            Some(Direction::Export),
            Some(name),
            Some(&wasm_import_module),
        );
        r#gen.interface = Some(id);
        r#gen.prologue();

        if let WorldKey::Name(_) = name {
            // We have an inline interface exported in a world.
            // Emit the "common" types as well

            r#gen.direction = None;
            r#gen.types(id);
            r#gen.direction = Some(Direction::Export);
        }

        r#gen.types(id);

        let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
        interface_filepath.add_extension("d");

        files.push(interface_filepath.to_str().unwrap(), r#gen.src.as_bytes());

        self.cur_interface = None;
        Ok(())
    }

    fn export_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        for (name, _func) in funcs {
            self.function_exports_src
                .push_str(&format!("// Export function: {name}\n"));
        }
        Ok(())
    }

    fn finish(&mut self, resolve: &Resolve, world_id: WorldId, files: &mut Files) -> Result<()> {
        for (name, id) in take(&mut self.used_interfaces) {
            if let WorldKey::Interface(_) = name {
                let fqn = self.interface_fqns[&id].common.as_ref().unwrap().clone();

                let wasm_import_module = resolve.name_world_key(&name);
                let mut r#gen =
                    self.interface(resolve, None, Some(&name), Some(&wasm_import_module));
                r#gen.interface = Some(id);
                r#gen.prologue();
                r#gen.types(id);

                let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
                interface_filepath.add_extension("d");

                files.push(interface_filepath.to_str().unwrap(), r#gen.src.as_bytes());
            }
        }

        let mut world_src = Source::default();

        let world = &resolve.worlds[world_id];

        world_src.push_str(&format!(
            "/++\n{}\n+/\n",
            world.docs.contents.as_deref().unwrap_or_default()
        ));

        world_src.push_str(&format!("module {};\n\n", self.world_fqn));
        world_src.push_str("import wit.common;\n\n");
        world_src.push_str("// Interface imports\n");
        world_src.push_str(
            &self
                .interface_imports
                .iter()
                .map(|fqn| format!("public import {fqn};"))
                .collect::<Vec<String>>()
                .join("\n"),
        );

        world_src.push_str("\n\n// Type imports\n");
        world_src.append_src(&self.type_imports_src);

        world_src.push_str("\n// Function imports\n");
        world_src.append_src(&self.function_imports_src);

        world_src.push_str("\n// Interface exports\n");
        world_src.push_str(
            &self
                .interface_exports
                .iter()
                .map(|fqn| format!("public import {fqn};"))
                .collect::<Vec<String>>()
                .join("\n"),
        );

        world_src.push_str("\n\nprivate alias AliasSeq(T...) = T;\n");
        world_src.push_str("template Exports(Impl...) {\n");
        world_src.push_str("// Interface exports\n");

        world_src.push_str("alias InterfaceExports = AliasSeq!(\n");
        world_src.indent(1);
        world_src.push_str(
            &self
                .interface_exports
                .iter()
                .map(|fqn| format!("{fqn}.Exports!Impl"))
                .collect::<Vec<String>>()
                .join(",\n"),
        );
        world_src.deindent(1);
        world_src.push_str("\n);\n");

        world_src.push_str("\n// Function exports\n");
        world_src.append_src(&self.function_exports_src);
        world_src.push_str("}\n");

        let mut world_filepath = PathBuf::from_iter(get_world_fqn(world_id, resolve).split("."));
        world_filepath.push("package.d");

        files.push(world_filepath.to_str().unwrap(), world_src.as_bytes());

        files.push("wit/common.d", include_bytes!("wit_common.d"));
        Ok(())
    }
}

struct DInterfaceGenerator<'a> {
    src: Source,
    direction: Option<Direction>,
    r#gen: &'a mut D,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
    name: Option<&'a WorldKey>,
    wasm_import_module: Option<&'a str>,
    fqn: &'a str,

    sizes: SizeAlign,
}

impl<'a> DInterfaceGenerator<'a> {
    fn scoped_type_name(&self, id: TypeId, from_module_fqn: &str) -> String {
        let ty = &self.resolve.types[id];

        let owner_fqn = self.type_owner_fqn(&ty.owner).unwrap();

        let upper_name = ty.name.as_ref().unwrap().to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        if from_module_fqn == owner_fqn {
            escaped_name.into()
        } else {
            format!("{owner_fqn}.{escaped_name}")
        }
    }
    fn type_name(&self, ty: &Type, from_module_fqn: &str) -> Cow<'static, str> {
        match ty {
            Type::Bool => Cow::Borrowed("bool"),
            Type::Char => Cow::Borrowed("dchar"),
            Type::U8 => Cow::Borrowed("ubyte"),
            Type::S8 => Cow::Borrowed("byte"),
            Type::U16 => Cow::Borrowed("ushort"),
            Type::S16 => Cow::Borrowed("short"),
            Type::U32 => Cow::Borrowed("uint"),
            Type::S32 => Cow::Borrowed("int"),
            Type::U64 => Cow::Borrowed("ulong"),
            Type::S64 => Cow::Borrowed("long"),
            Type::F32 => Cow::Borrowed("float"),
            Type::F64 => Cow::Borrowed("double"),
            Type::String => Cow::Borrowed("WitString"),
            Type::Id(id) => {
                let typedef = &self.resolve.types[*id];

                match typedef.owner {
                    TypeOwner::None => match &typedef.kind {
                        TypeDefKind::Record(_) => {
                            Cow::Owned(self.scoped_type_name(*id, from_module_fqn))
                        }
                        TypeDefKind::Resource => {
                            Cow::Owned(self.scoped_type_name(*id, from_module_fqn))
                        }
                        TypeDefKind::Handle(Handle::Own(id)) => {
                            Cow::Owned(self.scoped_type_name(*id, from_module_fqn))
                        }
                        TypeDefKind::Handle(Handle::Borrow(_id)) => {
                            Cow::Owned(self.scoped_type_name(*id, from_module_fqn) + ".Borrow")
                        }
                        TypeDefKind::Tuple(t) => Cow::Owned(format!(
                            "Tuple!({})",
                            t.types
                                .iter()
                                .map(|ty| self.type_name(ty, from_module_fqn).into_owned())
                                .collect::<Vec<String>>()
                                .join(", ")
                        )),
                        TypeDefKind::Option(o) => {
                            Cow::Owned(format!("Option!({})", self.type_name(o, from_module_fqn)))
                        }
                        TypeDefKind::Result(r) => Cow::Owned(format!(
                            "Result!({}, {})",
                            match r.ok {
                                Some(ok_type) => self.type_name(&ok_type, from_module_fqn),
                                None => Cow::Borrowed("void"),
                            },
                            match r.err {
                                Some(err_type) => self.type_name(&err_type, from_module_fqn),
                                None => Cow::Borrowed("void"),
                            }
                        )),
                        TypeDefKind::List(ty) => Cow::Owned(format!(
                            "WitList!({})",
                            self.type_name(&ty, from_module_fqn)
                        )),
                        TypeDefKind::Future(_) => {
                            Cow::Borrowed("/* todo - type_name of `future` */")
                        }
                        TypeDefKind::Stream(_) => {
                            Cow::Borrowed("/* todo - type_name of `stream` */")
                        }
                        TypeDefKind::FixedLengthList(ty, size) => {
                            Cow::Owned(format!("{}[{size}]", self.type_name(ty, from_module_fqn)))
                        }
                        TypeDefKind::Map(_, _) => todo!(),
                        TypeDefKind::Unknown => unimplemented!(),
                        unhandled => {
                            panic!(
                                "Encountered unexpected `type_name` invocation of ownerless typedef: {unhandled:?}."
                            );
                        }
                    },
                    _ => Cow::Owned(self.scoped_type_name(*id, from_module_fqn)),
                }
            }
            Type::ErrorContext => todo!(),
        }
    }

    fn type_owner_fqn(&self, owner: &TypeOwner) -> Option<&str> {
        match &owner {
            TypeOwner::None => None,
            TypeOwner::Interface(interface_id) => match self.direction {
                Some(_) => self
                    .r#gen
                    .lookup_interface_fqn(*interface_id, self.direction)
                    .or_else(|| self.r#gen.lookup_interface_fqn(*interface_id, None)),
                None => self.r#gen.lookup_interface_fqn(*interface_id, None),
            },
            TypeOwner::World(world_id) => {
                if *world_id != self.r#gen.world_id.unwrap() {
                    panic!("Dealing with type from different world?");
                }

                Some(&self.r#gen.world_fqn)
            }
        }
    }

    fn prologue(&mut self) {
        let id = self.interface.unwrap();

        let fqn = self.r#gen.lookup_interface_fqn(id, self.direction).unwrap();

        let interface = &self.resolve.interfaces[self.interface.unwrap()];

        self.src.push_str(&format!(
            "/++\n{}\n+/\n",
            interface.docs.contents.as_deref().unwrap_or_default()
        ));

        self.src.push_str(&format!("module {};\n\n", fqn));

        self.src.push_str("import wit.common;\n\n");
        if self.direction.is_some()
            && let Some(WorldKey::Interface(_)) = self.name
        {
            self.src.push_str("public import ");
            self.src
                .push_str(self.r#gen.lookup_interface_fqn(id, None).unwrap());
            self.src.push_str(";\n\n");
        }

        let mut deps = BTreeSet::new();

        for dep_id in self.resolve.interface_direct_deps(id) {
            deps.insert(dep_id);
        }

        for dep_id in deps {
            let common_fqn = self.r#gen.lookup_interface_fqn(dep_id, None).unwrap();
            let directional_fqn = self.r#gen.lookup_interface_fqn(dep_id, self.direction);

            if let Some(WorldKey::Interface(_)) = self.name {
                self.src.push_str(&format!(
                    "static import {};\n",
                    match self.direction {
                        Some(_) => directional_fqn.unwrap_or(common_fqn),
                        None => common_fqn,
                    }
                ));
            } else {
                self.src
                    .push_str(&format!("static import {};\n", common_fqn));

                if let Some(fqn) = directional_fqn {
                    self.src.push_str(&format!("static import {};\n", fqn));
                };
            }
        }
        self.src.push_str("\n");
    }

    fn type_is_direction_sensitive(&self, id: TypeId) -> bool {
        let type_info = &self.r#gen.types.get(id);

        type_info.has_resource
    }

    fn import_func(&mut self, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Method(_) => {}
            kind => {
                self.src
                    .push_str(&format!("// TODO: Import {kind:?} - {}\n", func.name));
                return;
            }
        }

        let sig = self
            .resolve
            .wasm_signature(abi::AbiVariant::GuestImport, func);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            func.docs.contents.as_deref().unwrap_or_default()
        ));
        self.src.push_str(&format!(
            "@wasmImport!(\"{}\", \"{}\")\n",
            self.wasm_import_module.unwrap(),
            func.name
        ));
        // The mangle is not important, as long as it won't conflict with other symbols
        // WebAssembly symbol identifiers are much more permissive than C (can be any UTF-8).
        // Yet, LDC before 1.42 don't allow full use of this fact. We make some substitutions.
        self.src.push_str(&format!(
            "pragma(mangle, \"__wit_import_{}__{}\")\n",
            self.wasm_import_module
                .unwrap()
                .replace("/", "__")
                .replace("-", "_"),
            func.name
                .replace("-", "_")
                .replace("[", ":")
                .replace("]", ":")
        ));

        let split_name = match &func.kind {
            FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => &func.name,
            FunctionKind::Method(_)
            | FunctionKind::Static(_)
            | FunctionKind::Constructor(_)
            | FunctionKind::AsyncMethod(_)
            | FunctionKind::AsyncStatic(_) => {
                self.src.push_str("static ");

                func.name.split(".").skip(1).next().unwrap()
            }
        };

        let lower_name = split_name.to_lower_camel_case();
        let escaped_name = escape_d_identifier(&lower_name);

        self.src.push_str(&format!(
            "/*private*/ extern(C) {} __import_{escaped_name}({});\n",
            match sig.results.len() {
                0 => "void",
                1 => wasm_type(sig.results[0]),
                _ => unimplemented!("multi-value return not supported"),
            },
            sig.params
                .iter()
                .map(|param| wasm_type(*param))
                .collect::<Vec<&str>>()
                .join(", ")
        ));
    }
}

impl<'a> InterfaceGenerator<'a> for DInterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    // Override `types` to filter by `self.direction`
    fn types(&mut self, iface: InterfaceId) {
        let iface = &self.resolve().interfaces[iface];
        for (name, id) in iface.types.iter() {
            if self.direction.is_some() == self.type_is_direction_sensitive(*id) {
                self.define_type(name, *id);
            }
        }
    }

    fn type_record(&mut self, id: TypeId, name: &str, record: &Record, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner)
            .unwrap()
            .to_string();

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));
        self.src.push_str(&format!("struct {escaped_name} {{\n"));

        let mut is_first = true;
        for field in &record.fields {
            if is_first {
                is_first = false;
            } else {
                self.src.push_str("\n");
            }

            self.src.push_str(&format!(
                "/++\n{}\n+/\n",
                field.docs.contents.as_deref().unwrap_or_default()
            ));
            self.src.push_str(&format!(
                "{} {};\n",
                self.type_name(&field.ty, &owner_fqn),
                field.name.to_lower_camel_case()
            ));
        }

        self.src.push_str("}\n");
    }

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        let ty = &self.resolve.types[id];

        match self.direction {
            None => panic!("Resources can only be generated for imports, or exports. Not common."),
            Some(Direction::Import) => match ty.owner {
                TypeOwner::Interface(owner_id) => {
                    if let Some(cur_interface) = self.interface
                        && cur_interface == owner_id
                    {
                    } else {
                        panic!("Emitting resource from `interface` outside that interface?");
                    }

                    self.src.push_str(&format!(
                        "\n/++\n{}\n+/\n",
                        docs.contents.as_deref().unwrap_or_default()
                    ));

                    self.src.push_str(&format!(
                        "struct {escaped_name} {{
        package(wit) int __handle = 0;

        package(wit) this(int handle) {{
            __handle = handle;
        }}

        @disable this();

        // TODO: make RAII? disable copy for the own


        auto borrow() => Borrow(__handle);
        alias borrow this;

        "
                    ));

                    for (_, func) in &self.resolve.interfaces[owner_id].functions {
                        if match &func.kind {
                            FunctionKind::Freestanding => false,
                            FunctionKind::Method(_) => false,
                            FunctionKind::Static(mid) => *mid == id,
                            FunctionKind::Constructor(mid) => *mid == id,
                            FunctionKind::AsyncFreestanding => false,
                            FunctionKind::AsyncMethod(_) => false,
                            FunctionKind::AsyncStatic(_) => todo!(),
                        } {
                            self.import_func(func);
                        }
                    }

                    self.src.push_str(&format!(
                        "struct Borrow {{
            package(wit) int __handle = 0;

            package(wit) this(int handle) {{
                __handle = handle;
            }}

            @disable this();

                        "
                    ));

                    for (_, func) in &self.resolve.interfaces[owner_id].functions {
                        if match &func.kind {
                            FunctionKind::Freestanding => false,
                            FunctionKind::Method(mid) => *mid == id,
                            FunctionKind::Static(_) => false,
                            FunctionKind::Constructor(_) => false,
                            FunctionKind::AsyncFreestanding => false,
                            FunctionKind::AsyncMethod(_) => todo!(),
                            FunctionKind::AsyncStatic(_) => false,
                        } {
                            self.import_func(func);
                        }
                    }

                    self.src.push_str("}\n");

                    self.src.push_str("}\n");
                }
                TypeOwner::World(_) => todo!("resources in worlds"),
                TypeOwner::None => {
                    panic!("Resource definition without owner?");
                }
            },
            Some(Direction::Export) => todo!("export of resource"),
        }
        //todo!("def of `resource`")
    }

    fn type_tuple(&mut self, id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self.type_owner_fqn(&self.resolve.types[id].owner).unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = Tuple!({});",
            tuple
                .types
                .iter()
                .map(|ty| self.type_name(ty, owner_fqn).into_owned())
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        let storage_type = match flags.repr() {
            FlagsRepr::U8 => "ubyte",
            FlagsRepr::U16 => "ushort",
            FlagsRepr::U32(1) => "uint",
            FlagsRepr::U32(2) => "ulong",
            repr => todo!("flags {repr:?}"),
        };

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));
        self.src.push_str(&format!("struct {escaped_name} {{\n"));

        self.src
            .push_str(&format!("mixin WitFlags!{storage_type};\n\n"));

        for (index, flag) in flags.flags.iter().enumerate() {
            if index != 0 {
                self.src.push_str("\n");
            }
            self.src.push_str(&format!(
                "/++\n{}\n+/\n",
                flag.docs.contents.as_deref().unwrap_or_default()
            ));
            self.src.push_str(&format!(
                "enum {} = {escaped_name}[{index}];\n",
                escape_d_identifier(&flag.name.to_lower_camel_case())
            ));
        }
        self.src.push_str(&format!("}}\n"));
    }

    fn type_variant(&mut self, id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        let storage_type = match variant.tag() {
            Int::U8 => "ubyte",
            Int::U16 => "ushort",
            Int::U32 => "uint",
            Int::U64 => "ulong",
        };

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner)
            .unwrap()
            .to_string();

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));
        self.src.push_str(&format!("struct {escaped_name} {{\n"));

        self.src.push_str("mixin WitVariant!(\n");
        self.src.indent(1);

        for case in &variant.cases {
            self.src.push_str(&format!(
                "{}, // {}\n",
                match &case.ty {
                    None => Cow::Borrowed("void"),
                    Some(ty) => self.type_name(ty, &owner_fqn),
                },
                escape_d_identifier(&case.name.to_lower_camel_case())
            ));
        }

        self.src.deindent(1);
        self.src.push_str(");\n");

        self.src.deindent(1);
        //self.src.push_str("@safe @nogc nothrow:\n");
        self.src.indent(1);

        self.src.deindent(1);
        self.src.push_str("\npublic:\n");
        self.src.indent(1);

        self.src
            .push_str(&format!("enum Tag : {storage_type} {{\n"));

        let mut is_first = true;
        for case in &variant.cases {
            if is_first {
                is_first = false;
            } else {
                self.src.push_str("\n");
            }
            self.src.push_str(&format!(
                "/++\n{}\n+/\n",
                case.docs.contents.as_deref().unwrap_or_default()
            ));
            self.src.push_str(&format!(
                "{},\n",
                escape_d_identifier(&case.name.to_lower_camel_case())
            ));
        }

        self.src.push_str("}\n");

        self.src.push_str("Tag tag() => _tag;\n");

        for case in &variant.cases {
            self.src.push_str(&format!(
                "\n/++\n{}\n+/\n",
                case.docs.contents.as_deref().unwrap_or_default()
            ));
            let upper_case_name = case.name.to_upper_camel_case();
            let escaped_upper_case_name = escape_d_identifier(&upper_case_name);

            let lower_case_name = case.name.to_lower_camel_case();
            let escaped_lower_case_name = escape_d_identifier(&lower_case_name);

            self.src.push_str(&format!(
                "alias {escaped_lower_case_name} = _create!(Tag.{escaped_lower_case_name});\n",
            ));
            self.src.push_str(&format!(
                "/// ditto\nbool is{escaped_upper_case_name}() const => _tag == Tag.{escaped_lower_case_name};\n",
            ));

            if let Some(ty) = &case.ty {
                self.src.push_str(&format!(
                    "///ditto\nalias get{escaped_upper_case_name} = _get!(Tag.{escaped_lower_case_name});\n",
                ));
            }
        }
        self.src.push_str("}\n");
    }

    fn type_option(&mut self, id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self.type_owner_fqn(&self.resolve.types[id].owner).unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = Option!({});",
            self.type_name(payload, owner_fqn)
        ));
    }

    fn type_result(&mut self, id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self.type_owner_fqn(&self.resolve.types[id].owner).unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = Result!({}, {});",
            match result.ok {
                Some(ok_type) => self.type_name(&ok_type, owner_fqn),
                None => Cow::Borrowed("void"),
            },
            match result.err {
                Some(err_type) => self.type_name(&err_type, owner_fqn),
                None => Cow::Borrowed("void"),
            }
        ));
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        let storage_type = match enum_.tag() {
            Int::U8 => "ubyte",
            Int::U16 => "ushort",
            Int::U32 => "uint",
            Int::U64 => "ulong",
        };

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));
        self.src
            .push_str(&format!("enum {escaped_name} : {storage_type} {{\n"));

        let mut is_first = true;
        for case in &enum_.cases {
            if is_first {
                is_first = false;
            } else {
                self.src.push_str("\n");
            }
            self.src.push_str(&format!(
                "/++\n{}\n+/\n",
                case.docs.contents.as_deref().unwrap_or_default()
            ));
            self.src.push_str(&format!(
                "{},\n",
                escape_d_identifier(&case.name.to_lower_camel_case())
            ));
        }

        self.src.push_str("}");
    }

    fn type_alias(&mut self, id: TypeId, name: &str, alias_ty: &Type, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let typename = self.type_name(
            alias_ty,
            self.type_owner_fqn(&self.resolve.types[id].owner).unwrap(),
        );

        self.src
            .push_str(&format!("alias {escaped_name} = {typename};\n"));
    }

    fn type_list(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self.type_owner_fqn(&self.resolve.types[id].owner).unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = WitList!({});",
            self.type_name(ty, owner_fqn)
        ));
    }

    fn type_fixed_length_list(
        &mut self,
        id: TypeId,
        name: &str,
        ty: &Type,
        size: u32,
        docs: &Docs,
    ) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self.type_owner_fqn(&self.resolve.types[id].owner).unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = {}[{size}];",
            self.type_name(ty, owner_fqn)
        ));
    }

    fn type_future(&mut self, _id: TypeId, name: &str, _ty: &Option<Type>, _docs: &Docs) {
        todo!("def of `future` - {name}");
    }

    fn type_stream(&mut self, _id: TypeId, name: &str, _ty: &Option<Type>, _docs: &Docs) {
        todo!("def of `stream` - {name}");
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, _ty: &Type, _docs: &Docs) {
        todo!("def of `builtin` - {name}");
    }
}
