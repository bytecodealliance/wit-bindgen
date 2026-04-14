use anyhow::Result;
use heck::*;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::mem::{replace, take};
use std::path::PathBuf;
use wit_bindgen_core::{
    Direction, Files, InterfaceGenerator, Source, Types, WorldGenerator,
    abi::{self, Bindgen, Bitcast, WasmType},
    wit_parser::*,
};

type DType = String;
#[derive(Default, Debug)]
struct DSig {
    static_member: bool,
    result: DType,
    arguments: Vec<(String, DType)>,
    name: String,
    implicit_self: bool,
    post_return: bool,
}

#[derive(Default)]
struct D {
    used_interfaces: HashSet<(WorldKey, InterfaceId)>,
    export_stubs: Vec<String>,

    interface_imports: Vec<String>,
    interface_exports: Vec<String>,
    type_imports_src: Source,
    function_imports_src: Source,
    function_exports_src: Source,
    export_stubs_src: Source,

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

    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    /// Whether stubs/declarations for exports should be emitted
    /// Only for testing purposes.
    emit_export_stubs: bool,
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

        // Common DRuntime & Phobos symbols
        "Object" => "Object_",
        "Error" => "Error_",
        "Throwable" => "Throwable_",
        "Exception" => "Exception_",
        "TypeInfo" => "TypeInfo_",

        // Symbols we define as part of the bindings we want to avoid creating conflicts with
        "WitList" => "WitList_",
        "WitString" => "WitString_",
        "WitFlags" => "WitFlags_",
        "Option" => "Option_",
        "Result" => "Result_",
        "bits" => "bits_",               // part of WitFlags
        "borrow" => "borrow_",           // part of the expansion of `resource`
        "drop" => "drop_",               // part of the expansion of `resource`
        "rep" => "rep_",                 // part of the expansion of `resource`
        "makeNew" => "makeNew_",         // part of the expansion of `resource`
        "constructor" => "constructor_", // part of the expansion of `resource`

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
                format!("_{version}")
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
            stub_src: Source::default(),
            stubs: Vec::default(),
            fqn: "",
            r#gen: self,
            resolve,
            interface: None,
            name: name,
            sizes,
            direction,

            wasm_import_module,

            return_pointer_area_size: Default::default(),
            return_pointer_area_align: Default::default(),
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

        if let WorldKey::Name(_) = name {
            // We have an inline interface imported in a world.
            // Emit the "common" types as well

            r#gen.direction = None;
            r#gen.types(id);
            r#gen.direction = Some(Direction::Import);
        }

        r#gen.types(id);

        for (_name, func) in &resolve.interfaces[id].functions {
            match func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                    r#gen.import_func(func);
                }
                _ => {}
            }
        }

        let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
        interface_filepath.set_extension("d");

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
        let fqn = self.world_fqn.clone();
        let mut r#gen = self.interface(resolve, Some(Direction::Import), None, Some("$root"));
        r#gen.fqn = &fqn;

        for (name, id) in types.iter() {
            r#gen.define_type(name, *id);
        }

        self.type_imports_src = take(&mut r#gen.src);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let fqn = self.world_fqn.clone();
        let mut r#gen = self.interface(resolve, Some(Direction::Import), None, Some("$root"));
        r#gen.fqn = &fqn;

        for (_name, func) in funcs {
            r#gen.import_func(func);
        }

        self.function_imports_src = take(&mut r#gen.src);
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
        let emit_exports_stubs = self.opts.emit_export_stubs;

        let mut r#gen = self.interface(
            resolve,
            Some(Direction::Export),
            Some(name),
            Some(&wasm_import_module),
        );
        r#gen.fqn = &fqn;
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

        r#gen
            .src
            .push_str("\npackage(wit) template Exports(Impl...) {\n");

        for (_name, func) in &resolve.interfaces[id].functions {
            match func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                    r#gen.export_func(func);
                }
                _ => {}
            }
        }

        for (type_name, type_id) in &resolve.interfaces[id].types {
            let ty = &resolve.types[*type_id];

            match &ty.kind {
                TypeDefKind::Resource => {
                    let upper_name = ty.name.as_ref().unwrap().to_upper_camel_case();
                    let escaped_name = escape_d_identifier(&upper_name);

                    r#gen.src.push_str(&format!(
                        "\n/++\n{}\n+/\n",
                        ty.docs.contents.as_deref().unwrap_or_default()
                    ));

                    r#gen
                        .src
                        .push_str(&format!("/// ditto\nstruct {escaped_name}_Wrappers {{\n"));

                    r#gen.src.push_str(&format!(
                        "alias _Resource_Impl = findWitExportResource!(\"{wasm_import_module}\", \"{type_name}\", Impl);\n"
                    ));

                    if emit_exports_stubs {
                        r#gen.stub_src.push_str(&format!(
                            "@witExport(\"{}\", \"{}\")\nstruct {escaped_name}_STUB {{\n",
                            wasm_import_module,
                            ty.name.as_ref().unwrap()
                        ));

                        r#gen.stubs.push(escaped_name.to_owned() + "_STUB");
                    }

                    for (_, func) in &resolve.interfaces[id].functions {
                        match func.kind {
                            FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {}
                            FunctionKind::Method(owner)
                            | FunctionKind::AsyncMethod(owner)
                            | FunctionKind::Constructor(owner)
                            | FunctionKind::Static(owner)
                            | FunctionKind::AsyncStatic(owner) => {
                                if owner == *type_id {
                                    r#gen.export_func(func);
                                }
                            }
                        }
                    }

                    r#gen.src.push_str(&format!(
                        "\n@wasmExport!(\"{}#[dtor]{}\")\n",
                        wasm_import_module,
                        ty.name.as_ref().unwrap()
                    ));
                    r#gen.src.push_str(&format!(
                        "pragma(mangle, \"__wit_export_{}__:dtor:{}\")\n",
                        wasm_import_module.replace("/", "__").replace("-", "_"),
                        ty.name.as_ref().unwrap().replace("-", "_")
                    ));
                    r#gen.src.push_str(
                        "static private extern(C) void __export_dtor(void* ptr) {
                            (*cast(_Resource_Impl*)ptr).destroy!false;
                            free(ptr);
                        }
                        ",
                    );

                    r#gen.src.push_str("}\n");

                    if emit_exports_stubs {
                        r#gen.stub_src.push_str("}\n");
                    }
                }
                _ => {}
            }
        }

        let ret_area_decl = r#gen.emit_ret_area_if_needed();

        r#gen.src.push_str(&ret_area_decl);
        r#gen.src.push_str("}\n\n");

        let DInterfaceGenerator {
            mut src,
            stub_src,
            stubs,
            ..
        } = r#gen;

        if self.opts.emit_export_stubs {
            src.append_src(&stub_src);

            src.push_str("alias STUBS = AliasSeq!(\n");
            src.indent(1);
            src.push_str(&stubs.join(",\n"));
            src.deindent(1);
            src.push_str("\n);\n");

            self.export_stubs.push(format!("{fqn}.STUBS"));
        }

        let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
        interface_filepath.set_extension("d");

        files.push(interface_filepath.to_str().unwrap(), src.as_bytes());

        self.cur_interface = None;
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let fqn = self.world_fqn.clone();
        let mut r#gen = self.interface(resolve, Some(Direction::Export), None, Some("$root"));
        r#gen.fqn = &fqn;

        for (_name, func) in funcs {
            match func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                    r#gen.export_func(func);
                }
                _ => {}
            }
        }

        let ret_area_decl = r#gen.emit_ret_area_if_needed();

        let DInterfaceGenerator {
            src,
            stub_src,
            mut stubs,
            ..
        } = r#gen;

        self.function_exports_src = src;
        self.function_exports_src.push_str(&ret_area_decl);

        if self.opts.emit_export_stubs {
            self.export_stubs_src.append_src(&stub_src);
            self.export_stubs.append(&mut stubs);
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
                r#gen.fqn = &fqn;
                r#gen.interface = Some(id);
                r#gen.prologue();
                r#gen.types(id);

                let mut interface_filepath = PathBuf::from_iter(fqn.split("."));
                interface_filepath.set_extension("d");

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
        world_src.push_str(
            &self
                .interface_imports
                .iter()
                .map(|fqn| format!("public import {fqn};"))
                .collect::<Vec<String>>()
                .join("\n"),
        );

        world_src.push_str("\n");

        world_src.push_str(
            &self
                .interface_exports
                .iter()
                .map(|fqn| format!("public import {fqn};"))
                .collect::<Vec<String>>()
                .join("\n"),
        );

        world_src.push_str("\n");

        world_src.append_src(&self.type_imports_src);

        world_src.append_src(&self.function_imports_src);

        world_src.push_str("\n\nprivate alias AliasSeq(T...) = T;\n");
        world_src.push_str("template Exports(Impl...) {\n");
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

        world_src.push_str(&self.function_exports_src.as_str());
        world_src.push_str("}\n");

        if self.opts.emit_export_stubs {
            self.export_stubs_src.push_str("alias STUBS = AliasSeq!(\n");
            self.export_stubs_src.indent(1);
            self.export_stubs_src
                .push_str(&self.export_stubs.join(",\n"));
            self.export_stubs_src.deindent(1);
            self.export_stubs_src.push_str("\n);\n");

            self.export_stubs_src
                .push_str("alias Exports_STUB_INVOKE = Exports!(STUBS);\n");

            world_src.append_src(&self.export_stubs_src);
        }

        let mut world_filepath = PathBuf::from_iter(get_world_fqn(world_id, resolve).split("."));
        world_filepath.push("package.d");

        files.push(world_filepath.to_str().unwrap(), world_src.as_bytes());

        files.push("wit/common.d", include_bytes!("wit_common.d"));
        Ok(())
    }
}

struct DInterfaceGenerator<'a> {
    src: Source,
    stub_src: Source,
    stubs: Vec<String>,
    direction: Option<Direction>,
    r#gen: &'a mut D,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
    name: Option<&'a WorldKey>,
    wasm_import_module: Option<&'a str>,
    fqn: &'a str,

    sizes: SizeAlign,

    return_pointer_area_size: ArchitectureSize,
    return_pointer_area_align: Alignment,
}

impl<'a> DInterfaceGenerator<'a> {
    fn scoped_type_name(&self, id: TypeId, from_module_fqn: &str) -> String {
        let ty = &self.resolve.types[id];

        let owner_fqn = self
            .type_owner_fqn(&ty.owner, self.r#gen.types.get(id).has_resource)
            .unwrap();

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
                        TypeDefKind::Handle(Handle::Borrow(id)) => {
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
                            self.optional_type_name(r.ok.as_ref(), from_module_fqn),
                            self.optional_type_name(r.err.as_ref(), from_module_fqn),
                        )),
                        TypeDefKind::List(ty) => Cow::Owned(format!(
                            "WitList!({})",
                            self.type_name(&ty, from_module_fqn)
                        )),
                        TypeDefKind::Future(_) => {
                            todo!("type_name of `future`")
                        }
                        TypeDefKind::Stream(_) => {
                            todo!("type_name of `stream`")
                        }
                        TypeDefKind::FixedLengthList(ty, size) => {
                            Cow::Owned(format!("{}[{size}]", self.type_name(ty, from_module_fqn)))
                        }
                        TypeDefKind::Map(_, _) => todo!("type_name of `map`"),
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

    fn optional_type_name(&self, ty: Option<&Type>, from_module_fqn: &str) -> Cow<'static, str> {
        match ty {
            Some(ty) => self.type_name(ty, from_module_fqn),
            None => Cow::Borrowed("void"),
        }
    }

    fn type_owner_fqn(&self, owner: &TypeOwner, imports_instead_of_common: bool) -> Option<&str> {
        match &owner {
            TypeOwner::None => None,
            TypeOwner::Interface(interface_id) => match self.direction {
                Some(_) => self
                    .r#gen
                    .lookup_interface_fqn(*interface_id, self.direction)
                    .or_else(|| {
                        if !imports_instead_of_common || self.direction != Some(Direction::Import) {
                            self.r#gen.lookup_interface_fqn(
                                *interface_id,
                                if imports_instead_of_common {
                                    Some(Direction::Import)
                                } else {
                                    None
                                },
                            )
                        } else {
                            None
                        }
                    }),
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

        let interface = &self.resolve.interfaces[self.interface.unwrap()];

        self.src.push_str(&format!(
            "/++\n{}\n+/\n",
            interface.docs.contents.as_deref().unwrap_or_default()
        ));

        self.src.push_str(&format!("module {};\n\n", self.fqn));

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

                if self.direction == Some(Direction::Export) {
                    if let Some(import_fqn) = self
                        .r#gen
                        .lookup_interface_fqn(dep_id, Some(Direction::Import))
                    {
                        self.src.push_str("static import ");
                        self.src.push_str(import_fqn);
                        self.src.push_str(";\n");
                    }
                }
            } else {
                self.src.push_str(&format!("static import {common_fqn};\n"));

                if let Some(fqn) = directional_fqn {
                    self.src.push_str(&format!("static import {fqn};\n"));
                };
            }
        }
        self.src.push_str("\n");
    }

    fn type_is_direction_sensitive(&self, id: TypeId) -> bool {
        let type_info = &self.r#gen.types.get(id);

        type_info.has_resource
    }

    fn get_d_signature(&mut self, func: &Function) -> DSig {
        match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Method(_)
            | FunctionKind::Static(_)
            | FunctionKind::Constructor(_) => {}

            FunctionKind::AsyncFreestanding
            | FunctionKind::AsyncMethod(_)
            | FunctionKind::AsyncStatic(_) => {
                todo!()
            }
        }

        let mut res = DSig::default();

        let split_name = match &func.kind {
            FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => &func.name,
            FunctionKind::Constructor(_) => "",
            FunctionKind::Method(_)
            | FunctionKind::Static(_)
            | FunctionKind::AsyncMethod(_)
            | FunctionKind::AsyncStatic(_) => func.name.split(".").skip(1).next().unwrap(),
        };

        let lower_name = split_name.to_lower_camel_case();
        let escaped_name = if let FunctionKind::Constructor(_) = &func.kind {
            match self.direction {
                Some(Direction::Import) => "makeNew",
                _ => "constructor",
            }
        } else {
            escape_d_identifier(&lower_name)
        };

        res.name = escaped_name.into();
        res.static_member = match &func.kind {
            FunctionKind::Static(_) => true,
            FunctionKind::Constructor(_) => true,
            _ => false,
        };

        res.post_return = self.direction == Some(Direction::Export)
            && abi::guest_export_needs_post_return(self.resolve, func);

        res.result
            .push_str(&(self.optional_type_name(func.result.as_ref(), self.fqn)));

        for (
            i,
            Param {
                name, ty: param, ..
            },
        ) in func.params.iter().enumerate()
        {
            if i == 0 && name == "self" {
                match &func.kind {
                    FunctionKind::Method(_) => {
                        res.implicit_self = true;
                        continue;
                    }
                    _ => {}
                }
            }

            let lower_param_name = name.to_lower_camel_case();
            let escaped_param_name = escape_d_identifier(&lower_param_name);

            let needs_in_qualifier = match param {
                Type::ErrorContext | Type::String | Type::Id(_) => true,
                _ => false,
            };

            res.arguments.push((
                escaped_param_name.into(),
                if needs_in_qualifier {
                    "in ".to_owned()
                } else {
                    "".to_owned()
                } + &self.type_name(&param, self.fqn),
            ));
        }

        res
    }

    fn import_func(&mut self, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Constructor(_)
            | FunctionKind::Method(_)
            | FunctionKind::Static(_) => {}
            kind => {
                todo!("Import {kind:?} - {}\n", func.name);
            }
        }

        let wasm_sig = self
            .resolve
            .wasm_signature(abi::AbiVariant::GuestImport, func);

        let d_sig = self.get_d_signature(func);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            func.docs.contents.as_deref().unwrap_or_default()
        ));

        if d_sig.static_member {
            self.src.push_str("static ");
        }
        self.src.push_str(&format!(
            "{} {}({}) {{\n",
            d_sig.result,
            d_sig.name,
            d_sig
                .arguments
                .iter()
                .map(|(name, ty)| ty.to_owned() + " " + name)
                .collect::<Vec<String>>()
                .join(", ")
        ));

        let mut params = Vec::new();

        if d_sig.implicit_self {
            params.push("this");
        }
        for (arg, _ty) in &d_sig.arguments {
            params.push(arg);
        }

        let mut f = FunctionBindgen::new(self, &params);
        abi::call(
            f.r#gen.resolve,
            abi::AbiVariant::GuestImport,
            abi::LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
            false,
        );
        let ret_area_decl = f.emit_ret_area_if_needed();

        let FunctionBindgen { src, .. } = f;
        self.src.push_str(&ret_area_decl);
        self.src.push_str(&src);

        self.src.push_str("}\n");

        self.src.push_str("/// ditto\n");
        self.src.push_str(&format!(
            "@wasmImport!(\"{}\", \"{}\")\n",
            self.wasm_import_module.unwrap(),
            func.name
        ));

        // The mangle is not important, as long as it won't conflict with other symbols
        // WebAssembly symbol identifiers are much more permissive than C (can be any UTF-8).
        // Yet, LDC before 1.42 doesn't allow full use of this fact. We make some substitutions.
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

        if d_sig.implicit_self || d_sig.static_member {
            self.src.push_str("static ");
        }
        self.src.push_str(&format!(
            "private extern(C) {} __import_{}({});\n",
            match wasm_sig.results.len() {
                0 => "void",
                1 => wasm_type(wasm_sig.results[0]),
                _ => unimplemented!("multi-value return not supported"),
            },
            d_sig.name,
            wasm_sig
                .params
                .iter()
                .map(|param| wasm_type(*param))
                .collect::<Vec<&str>>()
                .join(", ")
        ));
    }

    fn export_func(&mut self, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Constructor(_)
            | FunctionKind::Method(_)
            | FunctionKind::Static(_) => {}
            kind => {
                todo!("Export {kind:?} - {}\n", func.name);
            }
        }

        let wasm_sig = self
            .resolve
            .wasm_signature(abi::AbiVariant::GuestExport, func);

        let d_sig = self.get_d_signature(func);

        let mut params_data = Vec::new();
        let mut params = Vec::new();

        if d_sig.implicit_self {
            params.push("self");
        }
        for (arg, _ty) in wasm_sig.params.iter().enumerate() {
            params_data.push(format!("arg{arg}"));
        }
        for param in &params_data {
            params.push(&param);
        }

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            func.docs.contents.as_deref().unwrap_or_default()
        ));

        self.src.push_str(&format!(
            "alias {}_Sig = {} function({});\n",
            d_sig.name,
            d_sig.result,
            d_sig
                .arguments
                .iter()
                .map(|(name, ty)| ty.to_owned() + " " + name)
                .collect::<Vec<String>>()
                .join(", ")
        ));

        self.src.push_str(&format!(
            "/// ditto\nalias {}_Impl = findWitExportFunc!(\"{}\", \"{}\", {0}_Sig, {}, {});\n",
            d_sig.name,
            self.wasm_import_module.unwrap(),
            func.name,
            d_sig.implicit_self,
            match &func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => "Impl",
                _ => {
                    "witExportsIn!_Resource_Impl"
                }
            }
        ));

        if self.r#gen.opts.emit_export_stubs {
            self.stub_src.push_str(&format!(
                "@witExport(\"{}\", \"{}\")\n",
                self.wasm_import_module.unwrap(),
                func.name
            ));
            if d_sig.static_member {
                self.stub_src.push_str("static ");
            }
            self.stub_src.push_str(&format!(
                "{} {}_STUB({});\n",
                d_sig.result,
                d_sig.name,
                d_sig
                    .arguments
                    .iter()
                    .map(|(name, ty)| ty.to_owned() + " " + name)
                    .collect::<Vec<String>>()
                    .join(", ")
            ));

            match func.kind {
                FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => {
                    self.stubs.push(d_sig.name.clone() + "_STUB");
                }
                _ => {}
            }
        }

        let core_module_name = self.name.map(|s| self.resolve.name_world_key(s));
        let export_name = func.legacy_core_export_name(core_module_name.as_deref());

        self.src.push_str("/// ditto\n");
        self.src
            .push_str(&format!("@wasmExport!(\"{export_name}\")\n"));

        self.src.push_str(&format!(
            "pragma(mangle, \"__wit_export_{}\")\n",
            export_name
                .replace("/", "__")
                .replace("-", "_")
                .replace("[", ":")
                .replace("]", ":")
                .replace("#", "::")
        ));

        if d_sig.implicit_self || d_sig.static_member {
            self.src.push_str("static ");
        }
        self.src.push_str(&format!(
            "private extern(C) {} __export_{}({}) {{\n",
            match wasm_sig.results.len() {
                0 => "void",
                1 => wasm_type(wasm_sig.results[0]),
                _ => unimplemented!("multi-value return not supported"),
            },
            d_sig.name,
            wasm_sig
                .params
                .iter()
                .zip(&params)
                .map(|(ty, name)| format!("{} {name}", wasm_type(*ty)))
                .collect::<Vec<String>>()
                .join(", ")
        ));

        let mut f = FunctionBindgen::new(self, &params);
        abi::call(
            f.r#gen.resolve,
            abi::AbiVariant::GuestExport,
            abi::LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
            false,
        );

        let ret_area_decl = f.emit_ret_area_if_needed();

        let FunctionBindgen {
            src,
            return_pointer_area_size,
            return_pointer_area_align,
            ..
        } = f;
        self.return_pointer_area_size = self.return_pointer_area_size.max(return_pointer_area_size);
        self.return_pointer_area_align = self
            .return_pointer_area_align
            .max(return_pointer_area_align);

        self.src.push_str(&ret_area_decl);
        self.src.push_str(&src);

        self.src.push_str("}\n");

        if abi::guest_export_needs_post_return(self.resolve, func) {
            let mut param_data = Vec::new();
            let mut params = Vec::<&str>::new();

            for (arg, _ty) in wasm_sig.results.iter().enumerate() {
                param_data.push(format!("arg{arg}"));
            }
            for param in &param_data {
                params.push(&param);
            }

            self.src
                .push_str(&format!("@wasmExport!(\"cabi_post_{export_name}\")\n"));

            self.src.push_str(&format!(
                "pragma(mangle, \"__wit_cabi_post_{}\")\n",
                export_name
                    .replace("/", "__")
                    .replace("-", "_")
                    .replace("[", ":")
                    .replace("]", ":")
                    .replace("#", "::")
            ));

            if d_sig.implicit_self || d_sig.static_member {
                self.src.push_str("static ");
            }
            self.src.push_str(&format!(
                "private extern(C) void __cabi_post_{}({}) {{\n",
                d_sig.name,
                wasm_sig
                    .results
                    .iter()
                    .zip(&params)
                    .map(|(ty, name)| format!("{} {name}", wasm_type(*ty)))
                    .collect::<Vec<String>>()
                    .join(", ")
            ));

            let mut f = FunctionBindgen::new(self, &params);
            abi::post_return(f.r#gen.resolve, func, &mut f);

            let ret_area_decl = f.emit_ret_area_if_needed();

            let FunctionBindgen {
                src,
                return_pointer_area_size,
                return_pointer_area_align,
                ..
            } = f;
            self.return_pointer_area_size =
                self.return_pointer_area_size.max(return_pointer_area_size);
            self.return_pointer_area_align = self
                .return_pointer_area_align
                .max(return_pointer_area_align);

            self.src.push_str(&ret_area_decl);
            self.src.push_str(&src);

            self.src.push_str("}\n");
        }
    }

    fn emit_ret_area_if_needed(&self) -> String {
        if !self.return_pointer_area_size.is_empty() {
            format!(
                "\nalign({}) private void[{}] _exportsRetArea;\n",
                self.return_pointer_area_align.format("size_t.sizeof"),
                self.return_pointer_area_size.format("size_t.sizeof")
            )
        } else {
            String::new()
        }
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
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap()
            .to_string();

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));
        self.src.push_str(&format!("struct {escaped_name} {{\n"));

        let mut is_first = true;
        for field in &record.fields {
            let lower_name = field.name.to_lower_camel_case();
            let escaped_name = escape_d_identifier(&lower_name);

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
                "{} {escaped_name};\n",
                self.type_name(&field.ty, &owner_fqn)
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
            Some(Direction::Import) => {
                self.src.push_str(&format!(
                    "\n/++\n{}\n+/\n",
                    docs.contents.as_deref().unwrap_or_default()
                ));

                self.src.push_str(&format!(
                    "struct {escaped_name} {{
    package(wit) uint __handle = 0;

    package(wit) this(uint handle) {{
        __handle = handle;
    }}

    @disable this();

    "
                ));

                match ty.owner {
                    TypeOwner::Interface(owner_id) => {
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
                    }
                    TypeOwner::World(owner_id) => {
                        for (_, import) in &self.resolve.worlds[owner_id].imports {
                            match &import {
                                WorldItem::Function(func) => {
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
                                _ => {}
                            }
                        }
                    }
                    TypeOwner::None => {
                        panic!("Resource definition without owner?");
                    }
                }

                self.src
                    .push_str("\nvoid drop() {\n__import_drop(__handle);\n}\n");
                self.src.push_str(&format!(
                    "@wasmImport!(\"{}\", \"[resource-drop]{}\")\n",
                    self.wasm_import_module.unwrap(),
                    name
                ));
                self.src.push_str(&format!(
                    "pragma(mangle, \"__wit_import_{}__:resource_drop:{}\")\n",
                    self.wasm_import_module
                        .unwrap()
                        .replace("/", "__")
                        .replace("-", "_"),
                    name.replace("-", "_")
                ));
                self.src
                    .push_str("static private extern(C) void __import_drop(uint);\n\n");

                self.src.push_str(&format!(
                    "// TODO: make RAII? disable copy for the own

    auto borrow() => Borrow(__handle);
    alias borrow this;

    struct Borrow {{
    package(wit) uint __handle = 0;

    package(wit) this(uint handle) {{
        __handle = handle;
    }}

    @disable this();
                "
                ));

                match ty.owner {
                    TypeOwner::Interface(owner_id) => {
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
                    }
                    TypeOwner::World(owner_id) => {
                        for (_, import) in &self.resolve.worlds[owner_id].imports {
                            match &import {
                                WorldItem::Function(func) => {
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
                                _ => {}
                            }
                        }
                    }
                    TypeOwner::None => {
                        panic!("Resource definition without owner?");
                    }
                }
                self.src.push_str("}\n");
                self.src.push_str("}\n");
            }
            Some(Direction::Export) => match ty.owner {
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
        package(wit) uint __handle = 0;

        package(wit) this(uint handle) {{
            __handle = handle;
        }}

        @disable this();
"
                    ));

                    self.src.push_str(&format!(
                        "
                        static {escaped_name} makeNew(T)(scope void delegate(out T) dg) if (is(T == struct)) {{
                        auto ptr = cast(T*)malloc(T.sizeof);
                        if (ptr is null) return {escaped_name}.init;

                        dg(*ptr);
                        return {escaped_name}(__import_makeNew(ptr));
                        }}
                        ",
                    ));
                    self.src.push_str(&format!(
                        "@wasmImport!(\"{}\", \"[resource-new]{}\")\n",
                        self.wasm_import_module.unwrap(),
                        name
                    ));
                    self.src.push_str(&format!(
                        "pragma(mangle, \"__wit_import_{}__:resource_new:{}\")\n",
                        self.wasm_import_module
                            .unwrap()
                            .replace("/", "__")
                            .replace("-", "_"),
                        name.replace("-", "_")
                    ));
                    self.src
                        .push_str("static private extern(C) uint __import_makeNew(void*);\n\n");

                    self.src
                        .push_str("T* rep(T)() if (is(T == struct)) {\nreturn cast(T*)__import_rep(__handle);\n}\n");
                    self.src.push_str(&format!(
                        "@wasmImport!(\"{}\", \"[resource-rep]{}\")\n",
                        self.wasm_import_module.unwrap(),
                        name
                    ));
                    self.src.push_str(&format!(
                        "pragma(mangle, \"__wit_import_{}__:resource_rep:{}\")\n",
                        self.wasm_import_module
                            .unwrap()
                            .replace("/", "__")
                            .replace("-", "_"),
                        name.replace("-", "_")
                    ));
                    self.src
                        .push_str("static private extern(C) void __import_rep(uint);\n\n");

                    self.src
                        .push_str("void drop() {\n__import_drop(__handle);\n}\n");
                    self.src.push_str(&format!(
                        "@wasmImport!(\"{}\", \"[resource-drop]{}\")\n",
                        self.wasm_import_module.unwrap(),
                        name
                    ));
                    self.src.push_str(&format!(
                        "pragma(mangle, \"__wit_import_{}__:resource_drop:{}\")\n",
                        self.wasm_import_module
                            .unwrap()
                            .replace("/", "__")
                            .replace("-", "_"),
                        name.replace("-", "_")
                    ));
                    self.src
                        .push_str("static private extern(C) void __import_drop(uint);\n\n");

                    self.src.push_str(&format!(
                        "// TODO: make RAII? disable copy for the own
        auto borrow() => Borrow(__handle);
        alias borrow this;

        struct Borrow {{
            package(wit) uint __handle = 0;

            package(wit) this(uint handle) {{
                __handle = handle;
            }}

            @disable this();

                        "
                    ));

                    self.src.push_str("}\n");

                    self.src.push_str("}\n");
                }
                TypeOwner::World(_) => unimplemented!("resource exports in worlds"),
                TypeOwner::None => {
                    panic!("Resource definition without owner?");
                }
            },
        }
    }

    fn type_tuple(&mut self, id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        self.src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            docs.contents.as_deref().unwrap_or_default()
        ));

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap();
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
            .type_owner_fqn(&self.resolve.types[id].owner, false)
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
                self.optional_type_name(case.ty.as_ref(), &owner_fqn),
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

        self.src.push_str("Tag tag() const => _tag;\n");

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

            if case.ty.is_some() {
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

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap();
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

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = Result!({}, {});",
            self.optional_type_name(result.ok.as_ref(), owner_fqn),
            self.optional_type_name(result.err.as_ref(), owner_fqn),
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
            self.type_owner_fqn(&self.resolve.types[id].owner, false)
                .unwrap(),
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

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap();
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

        let owner_fqn = self
            .type_owner_fqn(&self.resolve.types[id].owner, false)
            .unwrap();
        self.src.push_str(&format!(
            "alias {escaped_name} = {}[{size}];",
            self.type_name(ty, owner_fqn)
        ));
    }

    fn type_map(&mut self, _id: TypeId, name: &str, _key: &Type, _value: &Type, _docs: &Docs) {
        todo!("def of `map` - {name}");
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

struct Block {
    body: String,
    results: Vec<String>,
    element: String,
    base: String,
}

struct BlockStorage {
    body: Source,
    element: String,
    base: String,
}

struct FunctionBindgen<'a, 'b> {
    r#gen: &'b mut DInterfaceGenerator<'a>,
    params: &'b [&'b str],
    tmp: usize,
    src: Source,
    block_storage: Vec<BlockStorage>,
    /// intermediate calculations for contained objects
    blocks: Vec<Block>,
    payloads: Vec<String>,
    return_pointer_area_size: ArchitectureSize,
    return_pointer_area_align: Alignment,
}

fn tempname(base: &str, idx: usize) -> String {
    format!("{base}{idx}")
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(r#gen: &'b mut DInterfaceGenerator<'a>, params: &'b [&'b str]) -> Self {
        Self {
            r#gen,
            params,
            tmp: 0,
            src: Default::default(),
            block_storage: Default::default(),
            blocks: Default::default(),
            payloads: Default::default(),
            return_pointer_area_size: Default::default(),
            return_pointer_area_align: Default::default(),
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

    fn load(
        &mut self,
        ty: &str,
        offset: ArchitectureSize,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        results.push(format!(
            "*(cast({}*)({} + {}))",
            ty,
            operands[0],
            offset.format("size_t.sizeof")
        ));
    }

    fn load_ext(
        &mut self,
        ty: &str,
        offset: ArchitectureSize,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        self.load(ty, offset, operands, results);
        let result = results.pop().unwrap();
        results.push(format!("cast(uint)({result})"));
    }

    fn store(&mut self, ty: &str, offset: ArchitectureSize, operands: &[String]) {
        self.push_str(&format!(
            "*cast({ty}*)({} + {}) = cast({ty})({});\n",
            operands[1],
            offset.format("size_t.sizeof"),
            operands[0]
        ));
    }

    /// Emits a shared return area declaration if needed by this function.
    ///
    /// During code generation, `return_pointer()` may be called multiple times for:
    /// - Indirect parameter storage (when too many/large params)
    /// - Return value storage (when return type is too large)
    ///
    /// **Safety:** This is safe because return pointers are used sequentially:
    /// 1. Parameter marshaling (before call)
    /// 2. Function execution
    /// 3. Return value unmarshaling (after call)
    ///
    /// The scratch space is reused but never accessed simultaneously.
    fn emit_ret_area_if_needed(&self) -> String {
        if !self.return_pointer_area_size.is_empty() {
            match self.r#gen.direction {
                Some(Direction::Import) => format!(
                    "align({}) void[{}] _retArea = void;\n",
                    self.return_pointer_area_align.format("size_t.sizeof"),
                    self.return_pointer_area_size.format("size_t.sizeof")
                ),
                Some(Direction::Export) => "alias _retArea = _exportsRetArea;\n".to_string(),
                None => {
                    unreachable!();
                }
            }
        } else {
            String::new()
        }
    }
}

fn perform_cast(op: &str, cast: &Bitcast) -> String {
    match cast {
        Bitcast::I32ToF32 | Bitcast::I64ToF32 => {
            format!("(cast(uint){op}).reinterpretCast!float")
        }
        Bitcast::F32ToI32 | Bitcast::F32ToI64 => {
            format!("({op}).reinterpretCast!uint")
        }
        Bitcast::I64ToF64 => {
            format!("({op}).reinterpretCast!double")
        }
        Bitcast::F64ToI64 => {
            format!("({op}).reinterpretCast!ulong")
        }
        Bitcast::I32ToI64 | Bitcast::LToI64 | Bitcast::PToP64 => {
            format!("cast(ulong)({op})")
        }
        Bitcast::I64ToI32 | Bitcast::PToI32 | Bitcast::LToI32 => {
            format!("cast(uint)({op})")
        }
        Bitcast::P64ToI64 | Bitcast::None | Bitcast::I64ToP64 => op.to_string(),
        Bitcast::P64ToP | Bitcast::I32ToP | Bitcast::LToP => {
            format!("cast(void*)({op})")
        }
        Bitcast::PToL | Bitcast::I32ToL | Bitcast::I64ToL => {
            format!("cast(size_t)({op})")
        }
        Bitcast::Sequence(sequence) => {
            let [first, second] = &**sequence;
            let inner = perform_cast(op, first);
            perform_cast(&inner, second)
        }
    }
}

impl<'a, 'b> Bindgen for FunctionBindgen<'a, 'b> {
    type Operand = String;

    fn emit(
        &mut self,
        _resolve: &Resolve,
        inst: &wit_bindgen_core::abi::Instruction<'_>,
        operands: &mut Vec<Self::Operand>,
        results: &mut Vec<Self::Operand>,
    ) {
        let mut top_as = |cvt: &str| {
            results.push(format!("cast({cvt})({})", operands.pop().unwrap()));
        };

        match inst {
            abi::Instruction::GetArg { nth } => {
                if *nth == 0 && &self.params[0] == &"self" {
                    results.push("this".into());
                } else {
                    results.push(self.params[*nth].into());
                }
            }

            abi::Instruction::I32Const { val } => results.push(val.to_string()),
            abi::Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    let op = perform_cast(op, cast);
                    results.push(op);
                }
            }
            abi::Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    results.push(
                        match ty {
                            WasmType::Pointer => "null",
                            _ => "0",
                        }
                        .to_string(),
                    );
                }
            }

            abi::Instruction::I32Load { offset } => self.load("uint", *offset, operands, results),
            abi::Instruction::I32Load8U { offset } => {
                self.load_ext("ubyte", *offset, operands, results)
            }
            abi::Instruction::I32Load8S { offset } => {
                self.load_ext("byte", *offset, operands, results)
            }
            abi::Instruction::I32Load16U { offset } => {
                self.load_ext("ushort", *offset, operands, results)
            }
            abi::Instruction::I32Load16S { offset } => {
                self.load_ext("short", *offset, operands, results)
            }
            abi::Instruction::I64Load { offset } => self.load("ulong", *offset, operands, results),
            abi::Instruction::F32Load { offset } => self.load("float", *offset, operands, results),
            abi::Instruction::F64Load { offset } => self.load("double", *offset, operands, results),

            abi::Instruction::PointerLoad { offset } => {
                self.load("void*", *offset, operands, results)
            }
            abi::Instruction::LengthLoad { offset } => {
                self.load("size_t", *offset, operands, results)
            }

            abi::Instruction::I32Store { offset } => self.store("uint", *offset, operands),
            abi::Instruction::I32Store8 { offset } => self.store("ubyte", *offset, operands),
            abi::Instruction::I32Store16 { offset } => self.store("ushort", *offset, operands),

            abi::Instruction::I64Store { offset } => self.store("ulong", *offset, operands),
            abi::Instruction::F32Store { offset } => self.store("float", *offset, operands),
            abi::Instruction::F64Store { offset } => self.store("double", *offset, operands),

            abi::Instruction::PointerStore { offset } => self.store("void*", *offset, operands),
            abi::Instruction::LengthStore { offset } => self.store("size_t", *offset, operands),

            abi::Instruction::I32FromChar
            | abi::Instruction::I32FromBool
            | abi::Instruction::I32FromU8
            | abi::Instruction::I32FromS8
            | abi::Instruction::I32FromU16
            | abi::Instruction::I32FromS16
            | abi::Instruction::I32FromS32 => top_as("uint"),
            abi::Instruction::I32FromU32 => results.push(operands.pop().unwrap()),

            abi::Instruction::I64FromU64 => results.push(operands.pop().unwrap()),
            abi::Instruction::I64FromS64 => top_as("ulong"),
            abi::Instruction::CoreF32FromF32 => results.push(operands.pop().unwrap()),
            abi::Instruction::CoreF64FromF64 => results.push(operands.pop().unwrap()),

            abi::Instruction::S8FromI32 => top_as("byte"),
            abi::Instruction::U8FromI32 => top_as("ubyte"),
            abi::Instruction::S16FromI32 => top_as("short"),
            abi::Instruction::U16FromI32 => top_as("ushort"),
            abi::Instruction::S32FromI32 => top_as("int"),
            abi::Instruction::U32FromI32 => results.push(operands.pop().unwrap()),
            abi::Instruction::S64FromI64 => top_as("long"),
            abi::Instruction::U64FromI64 => results.push(operands.pop().unwrap()),
            abi::Instruction::CharFromI32 => top_as("dchar"),
            abi::Instruction::F32FromCoreF32 => results.push(operands.pop().unwrap()),
            abi::Instruction::F64FromCoreF64 => results.push(operands.pop().unwrap()),
            abi::Instruction::BoolFromI32 => results.push(format!("({}) != 0", operands[0])),

            abi::Instruction::ListCanonLower { .. } | abi::Instruction::StringLower { .. } => {
                results.push(format!("cast(void*)({}.ptr)", operands[0]));
                results.push(format!("{}.length", operands[0]));
            }
            abi::Instruction::ListLower { element, .. } => {
                let Block {
                    body,
                    element: block_element,
                    base,
                    ..
                } = self.blocks.pop().unwrap();
                let tmp = self.tmp();

                let size = self.r#gen.sizes.size(element);
                let size_str = size.format("size_t.sizeof");

                let list = tempname("_list", tmp);
                let list_src = tempname("_listSrc", tmp);

                self.push_str(&format!(
                    "auto {list_src} = {};
                    auto {list} = wit.common.malloc({list_src}.length * ({size_str}));
                    scope(exit) {{ wit.common.free({list}); }}\n",
                    operands[0]
                ));

                self.push_str(&format!(
                    "foreach ({block_element}_idx, const ref {block_element}; {list_src}) {{\n"
                ));
                self.push_str(&format!(
                    "auto {base} = {list} + {block_element}_idx * ({size_str});\n"
                ));
                self.push_str(&body);
                //self.push_str(&format!("_targetElem = {};", body.1[0]));
                self.push_str("\n}\n");

                results.push(format!("{list}"));
                results.push(format!("{}.length", operands[0]));
            }

            abi::Instruction::ListCanonLift { element, ty, .. } => {
                let list_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                let elem_name = self.r#gen.type_name(element, self.r#gen.fqn);
                let tmp = self.tmp();

                let ptr = tempname("_ptr", tmp);
                let len = tempname("_len", tmp);

                self.push_str(&format!(
                    "auto {ptr} = cast({elem_name}*)({});
                    auto {len} = {};
                    ",
                    operands[0], operands[1]
                ));

                results.push(format!("{list_name}({ptr}[0..{len}])"));
            }
            abi::Instruction::StringLift => {
                let tmp = self.tmp();

                let ptr = tempname("_ptr", tmp);
                let len = tempname("_len", tmp);

                self.push_str(&format!(
                    "auto {ptr} = cast(char*)({});
                    auto {len} = {};
                    ",
                    operands[0], operands[1]
                ));

                results.push(format!("WitString({ptr}[0..{len}])"));
            }
            abi::Instruction::ListLift { ty, element, .. } => {
                let Block {
                    body,
                    results: block_results,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.r#gen.sizes.size(element);
                let size_str = size.format("size_t.sizeof");
                let elem_type_name = self.r#gen.type_name(element, self.r#gen.fqn);

                let list = tempname("_list", tmp);
                let list_len = tempname("_listLen", tmp);
                let list_src = tempname("_listSrcPtr", tmp);
                self.push_str(&format!("auto {list_src} = {};\n", operands[0]));
                self.push_str(&format!("auto {list_len} = {};\n", operands[1]));
                self.push_str(&format!(
                    "auto {list} = wit.common.mallocSlice!({elem_type_name})({list_len});\n",
                ));

                self.push_str(&format!(
                    "foreach ({block_element}_idx, ref {block_element}; {list}) {{\n",
                ));
                self.push_str(&format!(
                    "const auto {base} = {list_src} + {block_element}_idx * {size_str};\n"
                ));
                self.push_str(&body);
                self.push_str(&format!("{block_element} = {};", block_results[0]));
                self.push_str("\n}\n");

                let list_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                results.push(format!("{list_name}({list})"));
            }

            abi::Instruction::FixedLengthListLift { size, id, .. } => {
                let result = tempname("_arr", self.tmp());
                let type_name = self.r#gen.type_name(&Type::Id(*id), self.r#gen.fqn);
                self.push_str(&format!("{type_name} {result} = [\n",));
                self.src.indent(1);
                for op in operands.drain(0..(*size as usize)) {
                    self.push_str(&op);
                    self.push_str(", \n");
                }
                self.src.deindent(1);
                self.push_str("];\n");
                results.push(result);
            }
            abi::Instruction::FixedLengthListLower { size, .. } => {
                for i in 0..(*size as usize) {
                    results.push(format!("{}[{i}]", operands[0]));
                }
            }
            abi::Instruction::FixedLengthListLowerToMemory { element, .. } => {
                let Block {
                    body,
                    results: _,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                let arr_src = &operands[0];
                let arr_dst = &operands[1];
                let size_str = self.r#gen.sizes.size(element).format("size_t.sizeof");

                self.push_str(&format!(
                    "foreach ({block_element}_idx, const ref {block_element}; {arr_src}) {{\n"
                ));
                self.push_str(&format!(
                    "const auto {base} = {arr_dst} + {block_element}_idx * {size_str};\n"
                ));
                self.push_str(&body);
                self.push_str("\n}\n");
            }
            abi::Instruction::FixedLengthListLiftFromMemory { id, element, .. } => {
                let Block {
                    body,
                    results: block_results,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                let arr_src = &operands[0];
                let type_name = self.r#gen.type_name(&Type::Id(*id), self.r#gen.fqn);
                let size_str = self.r#gen.sizes.size(element).format("size_t.sizeof");

                let result = tempname("_arr", self.tmp());
                self.push_str(&format!("{type_name} {result} = void;\n"));

                self.push_str(&format!(
                    "foreach ({block_element}_idx, ref {block_element}; {result}) {{\n"
                ));
                self.push_str(&format!(
                    "const auto {base} = {arr_src} + {block_element}_idx * {size_str};\n"
                ));
                self.push_str(&body);
                self.push_str(&format!("{block_element} = {};", block_results[0]));
                self.push_str("\n}\n");

                results.push(result);
            }

            abi::Instruction::IterElem { .. } => {
                results.push(self.block_storage.last().unwrap().element.clone())
            }
            abi::Instruction::IterBasePointer => {
                results.push(self.block_storage.last().unwrap().base.clone())
            }

            abi::Instruction::RecordLower { record, .. } => {
                for field in record.fields.iter() {
                    let lower_name = field.name.to_lower_camel_case();
                    let escaped_name = escape_d_identifier(&lower_name);

                    results.push(format!("{}.{escaped_name}", operands[0]));
                }
            }
            abi::Instruction::RecordLift { ty, record, .. } => {
                let name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);

                let tmpvar = tempname("_record", self.tmp());

                self.push_str(&format!("{name} {tmpvar} = {{\n"));
                for (field, op) in record.fields.iter().zip(operands.iter()) {
                    let lower_name = field.name.to_lower_camel_case();
                    let escaped_name = escape_d_identifier(&lower_name);

                    self.push_str(&format!("{escaped_name}: {op},\n"));
                }
                self.push_str("};\n");

                results.push(tmpvar);
            }

            abi::Instruction::HandleLower { .. } => {
                let op = &operands[0];
                results.push(format!("{op}.__handle"))
            }
            abi::Instruction::HandleLift { ty, .. } => {
                let name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                results.push(format!("{name}({})", operands[0]));
            }

            abi::Instruction::TupleLower { tuple, .. } => {
                for i in 0..tuple.types.len() {
                    results.push(format!("{}[{i}]", &operands[0]));
                }
            }
            abi::Instruction::TupleLift { ty, .. } => {
                let name = tempname("_tuple", self.tmp());
                self.push_str(&format!(
                    "auto {name} = {}(\n",
                    self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn),
                ));
                self.src.indent(1);
                for op in operands.iter() {
                    self.push_str(op);
                    self.push_str(",\n");
                }
                self.src.deindent(1);
                self.push_str(");\n");
                results.push(name);
            }

            abi::Instruction::FlagsLower { flags, .. } => match flags.repr() {
                FlagsRepr::U8 | FlagsRepr::U16 | FlagsRepr::U32(1) => {
                    results.push(format!("cast(uint)({}.bits)", operands.pop().unwrap()));
                }
                FlagsRepr::U32(2) => {
                    let tempname = tempname("_flags", self.tmp());

                    self.push_str(&format!("auto {tempname} = {};", operands[0]));
                    results.push(format!("cast(uint)({tempname}.bits & 0xffffffff)"));
                    results.push(format!("cast(uint)(({tempname}.bits >> 32) & 0xffffffff)"));
                }
                _ => todo!(),
            },
            abi::Instruction::FlagsLift { flags, ty, .. } => {
                let type_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);

                match flags.repr() {
                    FlagsRepr::U8 => {
                        results.push(format!(
                            "{type_name}(cast(ubyte)({}))",
                            operands.pop().unwrap()
                        ));
                    }
                    FlagsRepr::U16 => {
                        results.push(format!(
                            "{type_name}(cast(ushort)({}))",
                            operands.pop().unwrap()
                        ));
                    }
                    FlagsRepr::U32(1) => {
                        results.push(format!("{type_name}({})", operands.pop().unwrap()));
                    }
                    FlagsRepr::U32(2) => {
                        results.push(format!(
                            "({type_name}({}) | {type_name}({} << 32))",
                            operands[0], operands[1]
                        ));
                    }
                    _ => todo!(),
                }
            }

            abi::Instruction::VariantPayloadName => {
                let name = tempname("_payload", self.tmp());
                results.push(name.clone());
                self.payloads.push(name);
            }
            abi::Instruction::VariantLower {
                ty,
                variant,
                results: result_types,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let payloads = self
                    .payloads
                    .drain(self.payloads.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let mut variant_results = Vec::with_capacity(result_types.len());
                for res_ty in result_types.iter() {
                    let name = tempname("_variantPart", self.tmp());
                    results.push(name.clone());
                    self.src
                        .push_str(&format!("{} {name} = void;\n", wasm_type(*res_ty)));
                    variant_results.push(name);
                }

                let ty_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);

                let tag_type = tempname("_Tag", self.tmp());

                self.push_str(&format!("alias {tag_type} = {ty_name}.Tag;\n"));
                self.push_str(&format!("final switch ({}.tag) {{\n", operands[0]));

                for ((case, block), payload) in variant.cases.iter().zip(blocks).zip(payloads) {
                    let lower_name = case.name.to_lower_camel_case();
                    let lower_escaped_name = escape_d_identifier(&lower_name);

                    let uppper_name = case.name.to_upper_camel_case();
                    let upper_escaped_name = escape_d_identifier(&uppper_name);

                    self.push_str(&format!("case {tag_type}.{lower_escaped_name}: {{\n"));
                    if let Some(ty) = case.ty.as_ref() {
                        let ty_name = self.r#gen.type_name(ty, self.r#gen.fqn);
                        self.push_str(&format!(
                            "const ref {ty_name} {payload} = {}.get{upper_escaped_name}();\n",
                            operands[0],
                        ));
                    }
                    self.src.push_str(&block.body);

                    for (name, result) in variant_results.iter().zip(&block.results) {
                        self.push_str(&format!("{name} = {result};\n"));
                    }
                    self.src.push_str("break;\n}\n");
                }

                self.src.push_str("}\n");
            }
            abi::Instruction::VariantLift { variant, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let ty = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);

                let tmp = self.tmp();
                let result = tempname("_variant", tmp);
                let tag = tempname("_tag", tmp);
                let tag_type = tempname("_Tag", tmp);

                self.push_str(&format!("{ty} {result} = void;\n"));
                self.push_str(&format!("auto {tag} = {};\n", operands[0]));

                self.push_str(&format!("alias {tag_type} = {ty}.Tag;\n"));
                self.push_str(&format!("final switch (cast({ty}.Tag){tag}) {{\n"));
                for (case, block) in variant.cases.iter().zip(blocks) {
                    let lower_name = case.name.to_lower_camel_case();
                    let escaped_name = escape_d_identifier(&lower_name);

                    let payload = tempname("_payload", self.tmp());

                    self.push_str(&format!("case {tag_type}.{escaped_name}: {{\n"));
                    self.src.push_str(&block.body);
                    assert!(block.results.len() == (case.ty.is_some() as usize));

                    let val = if let Some(_) = case.ty.as_ref() {
                        self.push_str(&format!("auto {payload} = {};\n", block.results[0]));
                        &payload
                    } else {
                        ""
                    };
                    self.push_str(&format!("{result} = {ty}.{escaped_name}({val});\n"));
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
                results.push(result);
            }

            abi::Instruction::EnumLower { .. } => {
                results.push(format!("cast(uint)({})", operands[0]))
            }
            abi::Instruction::EnumLift { ty, .. } => {
                let type_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                results.push(format!("cast({})({})", type_name, operands.pop().unwrap()))
            }

            abi::Instruction::OptionLower {
                results: result_types,
                ..
            } => {
                let Block {
                    body: mut some,
                    results: some_results,
                    ..
                } = self.blocks.pop().unwrap();
                let Block {
                    body: mut none,
                    results: none_results,
                    ..
                } = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let _none_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let name = tempname("_option", self.tmp());
                    results.push(name.clone());
                    self.push_str(&format!("{} {name} = void;\n", wasm_type(*ty)));
                    let some_result = &some_results[i];
                    some.push_str(&format!("{name} = {some_result};\n"));
                    let none_result = &none_results[i];
                    none.push_str(&format!("{name} = {none_result};\n"));
                }

                let bind_some = format!("ref {some_payload} = {}.unwrap();", operands[0]);

                self.push_str(&format!(
                    "\
                    if ({}.isSome) {{
                        {bind_some}
                        {some}}} else {{
                        {none}}}
                    ",
                    operands[0]
                ));
            }
            abi::Instruction::OptionLift { ty, .. } => {
                let Block {
                    body: some,
                    results: some_results,
                    ..
                } = self.blocks.pop().unwrap();
                let Block {
                    results: none_results,
                    ..
                } = self.blocks.pop().unwrap();
                assert!(none_results.is_empty());
                assert!(some_results.len() == 1);

                let type_name = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                let op0 = &operands[0];

                let tmp = self.tmp();
                let resultname = tempname("_option", tmp);
                let is_some = tempname("_isSome", tmp);
                let some_value = &some_results[0];
                self.push_str(&format!(
                    "{type_name} {resultname} = void;
                    bool {is_some} = ({op0}) != 0;
                    if ({is_some}) {{
                        {some}
                        {resultname} = {type_name}.some({some_value});
                    }} else {{
                        {resultname} = {type_name}.none;
                    }}
                    "
                ));
                results.push(format!("{resultname}"));
            }

            abi::Instruction::ResultLower {
                results: result_types,
                result,
                ..
            } => {
                let Block {
                    body: mut err,
                    results: err_results,
                    ..
                } = self.blocks.pop().unwrap();
                let Block {
                    body: mut ok,
                    results: ok_results,
                    ..
                } = self.blocks.pop().unwrap();
                let err_payload = self.payloads.pop().unwrap();
                let ok_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let tmp = self.tmp();
                    let name = tempname("_resultPart", tmp);
                    results.push(name.clone());
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    let ok_result = &ok_results[i];
                    ok.push_str(&format!("{name} = {ok_result};\n"));
                    let err_result = &err_results[i];
                    err.push_str(&format!("{name} = {err_result};\n"));
                }

                let op0 = &operands[0];
                let bind_ok = if let Some(_ok) = result.ok.as_ref() {
                    format!("ref {ok_payload} = {op0}.unwrap();")
                } else {
                    String::new()
                };
                let bind_err = if let Some(_err) = result.err.as_ref() {
                    format!("ref {err_payload} = {op0}.unwrapErr();")
                } else {
                    String::new()
                };

                self.push_str(&format!(
                    "\
                    if ({op0}.isErr) {{
                        {bind_err}
                        {err}}} else {{
                        {bind_ok}
                        {ok}}}
                    "
                ));
            }
            abi::Instruction::ResultLift { result, ty, .. } => {
                let Block {
                    body: err,
                    results: err_results,
                    ..
                } = self.blocks.pop().unwrap();
                assert!(err_results.len() == (result.err.is_some() as usize));
                let Block {
                    body: ok,
                    results: ok_results,
                    ..
                } = self.blocks.pop().unwrap();
                assert!(ok_results.len() == (result.ok.is_some() as usize));

                let full_type = self.r#gen.type_name(&Type::Id(*ty), self.r#gen.fqn);
                let op0 = &operands[0];

                let tmp = self.tmp();
                let resultname = tempname("_result", tmp);
                let is_err = tempname("_isErr", tmp);

                let ok_value = if result.ok.is_some() {
                    &ok_results[0]
                } else {
                    ""
                };

                let err_value = if result.err.is_some() {
                    &err_results[0]
                } else {
                    ""
                };

                self.push_str(&format!(
                    "{full_type} {resultname} = void;
                    bool {is_err} = ({op0}) != 0;
                    if ({is_err}) {{
                        {err}
                        {resultname} = {full_type}.err({err_value});
                    }} else {{
                        {ok}
                        {resultname} = {full_type}.ok({ok_value});
                    }}\n"
                ));
                results.push(resultname);
            }

            abi::Instruction::CallWasm { name, sig } => {
                let split_name = if name.contains('.') {
                    name.split(".").skip(1).next().unwrap()
                } else {
                    name
                };

                let lower_name = split_name.to_lower_camel_case();
                let escaped_name = if name.starts_with("[constructor]") {
                    "makeNew"
                } else {
                    escape_d_identifier(&lower_name)
                };

                if !sig.results.is_empty() {
                    self.src.push_str("auto _ret = ");
                    results.push("_ret".to_string());
                }
                self.push_str(&format!(
                    "__import_{escaped_name}({});\n",
                    operands.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            abi::Instruction::CallInterface { func, async_ } => {
                if *async_ {
                    todo!("CallInterface async");
                }

                if func.result.is_some() {
                    self.src.push_str("auto _ret = ");
                    results.push("_ret".to_string());
                }

                let split_name = match &func.kind {
                    FunctionKind::Freestanding | FunctionKind::AsyncFreestanding => &func.name,
                    FunctionKind::Constructor(_) => "",
                    FunctionKind::Method(_)
                    | FunctionKind::Static(_)
                    | FunctionKind::AsyncMethod(_)
                    | FunctionKind::AsyncStatic(_) => func.name.split(".").skip(1).next().unwrap(),
                };

                let lower_name = split_name.to_lower_camel_case();
                let escaped_name = if let FunctionKind::Constructor(_) = &func.kind {
                    "constructor"
                } else {
                    escape_d_identifier(&lower_name)
                };

                let implicit_self = match &func.kind {
                    FunctionKind::Freestanding
                    | FunctionKind::AsyncFreestanding
                    | FunctionKind::Static(_)
                    | FunctionKind::AsyncStatic(_)
                    | FunctionKind::Constructor(_) => {
                        self.src.push_str(&format!("{escaped_name}_Impl("));
                        false
                    }
                    FunctionKind::Method(_) | FunctionKind::AsyncMethod(_) => {
                        self.src.push_str(&format!(
                            "__traits(child, cast(_Resource_Impl*)self, {escaped_name}_Impl)("
                        ));
                        true
                    }
                };
                self.src.push_str(
                    &operands
                        .iter()
                        .skip(if implicit_self { 1 } else { 0 })
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                self.src.push_str(");\n");
            }
            abi::Instruction::Return { amt, .. } => match amt {
                0 => {}
                _ => {
                    assert!(*amt == operands.len());

                    if *amt == 1 {
                        self.push_str("return ");
                        self.src.push_str(&operands[0]);
                        self.push_str(";\n");
                    } else {
                        todo!();
                    }
                }
            },

            abi::Instruction::Malloc { .. } => {
                todo!("instr: Malloc")
            }
            abi::Instruction::GuestDeallocate { .. } => {
                self.push_str(&format!("free({});", operands[0]));
            }
            abi::Instruction::GuestDeallocateString { .. } => {
                self.push_str(&format!("if ({} > 0) {{\n", operands[1]));
                self.push_str(&format!("free({});\n", operands[0]));
                self.push_str("}\n");
            }
            abi::Instruction::GuestDeallocateList { element } => {
                let Block {
                    body,
                    results: _,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.r#gen.sizes.size(element);
                let size_str = size.format("size_t.sizeof");

                let list_len = tempname("_listLen", tmp);
                let list_src = tempname("_listSrcPtr", tmp);
                self.push_str(&format!("auto {list_src} = {};\n", operands[0]));
                self.push_str(&format!("auto {list_len} = {};\n", operands[1]));

                self.push_str(&format!(
                    "foreach ({block_element}_idx; 0..{list_len}) {{\n",
                ));
                self.push_str(&format!(
                    "const auto {base} = {list_src} + {block_element}_idx * {size_str};\n"
                ));
                self.push_str(&body);
                self.push_str("\n}\n");

                self.push_str(&format!("if ({} > 0) {{\n", operands[1]));
                self.push_str(&format!("free({});\n", operands[0]));
                self.push_str("}\n");
            }
            abi::Instruction::GuestDeallocateVariant {
                blocks: block_count,
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - block_count..)
                    .collect::<Vec<_>>();

                self.push_str(&format!("switch ({}) {{\n", operands[0]));
                for (i, block) in blocks.into_iter().enumerate() {
                    assert!(results.is_empty());

                    self.push_str(&format!("case {i}: {{\n"));
                    self.src.push_str(&block.body);
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("default: break;\n}\n");
            }
            abi::Instruction::DropHandle { .. } => {
                todo!("instr: DropHandle")
            }

            abi::Instruction::Flush { amt } => {
                for op in operands.iter().take(*amt) {
                    let result = tempname("_flush", self.tmp());
                    self.push_str(&format!("auto {result} = {op};\n"));
                    results.push(result);
                }
            }

            unk => todo!("emit instruction: {unk:?}"),
        }
    }

    fn return_pointer(&mut self, size: ArchitectureSize, align: Alignment) -> Self::Operand {
        // Track maximum return area requirements
        self.return_pointer_area_size = self.return_pointer_area_size.max(size);
        self.return_pointer_area_align = self.return_pointer_area_align.max(align);

        "_retArea.ptr".into()
    }

    fn push_block(&mut self) {
        let tmp = self.tmp();

        self.block_storage.push(BlockStorage {
            body: take(&mut self.src),
            element: tempname("_elem", tmp),
            base: tempname("_base", tmp),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<Self::Operand>) {
        let BlockStorage {
            body,
            element,
            base,
        } = self.block_storage.pop().unwrap();

        let src = replace(&mut self.src, body);
        self.blocks.push(Block {
            body: src.into(),
            results: take(operands),
            element,
            base,
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.r#gen.sizes
    }

    fn is_list_canonical(&self, _resolve: &Resolve, ty: &Type) -> bool {
        self.r#gen.resolve.all_bits_valid(ty)
    }
}
