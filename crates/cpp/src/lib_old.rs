use heck::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::mem;
use wit_bindgen_core::abi::{self, AbiVariant, Bindgen, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{
    uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Source, TypeInfo, Types,
    WorldGenerator,
};
use wit_bindgen_cpp_host::RESOURCE_BASE_CLASS_NAME;
use wit_bindgen_rust::{
    dealias, FnSig, Ownership, RustFlagsRepr, RustFunctionGenerator, RustGenerator, TypeMode,
};

#[derive(Default, Copy, Clone, PartialEq, Eq)]
enum Direction {
    #[default]
    Import,
    Export,
}

#[derive(Default)]
struct ResourceInfo {
    direction: Direction,
    owned: bool,
    docs: Docs,
}

#[derive(Default)]
struct Cpp {
    types: Types,
    src: Source,
    opts: Opts,
    import_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    export_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    skip: HashSet<String>,
    interface_names: HashMap<InterfaceId, String>,
    resources: HashMap<TypeId, ResourceInfo>,
    import_funcs_called: bool,
    world: Option<WorldId>,
}

#[cfg(feature = "clap")]
fn parse_map(s: &str) -> Result<HashMap<String, String>, String> {
    if s.is_empty() {
        Ok(HashMap::default())
    } else {
        s.split(',')
            .map(|entry| {
                let (key, value) = entry.split_once('=').ok_or_else(|| {
                    format!("expected string of form `<key>=<value>[,<key>=<value>...]`; got `{s}`")
                })?;
                Ok((key.to_owned(), value.to_owned()))
            })
            .collect()
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Names of functions to skip generating bindings for.
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip: Vec<String>,

    /// Name of the concrete type which implements the trait representing any
    /// top-level functions exported by the world.
    #[cfg_attr(feature = "clap", arg(long))]
    pub world_exports: Option<String>,

    /// Names of the concrete types which implement the traits representing any
    /// interfaces exported by the world.
    #[cfg_attr(feature = "clap", arg(long, value_parser = parse_map, default_value = ""))]
    pub interface_exports: HashMap<String, String>,

    /// Names of the concrete types which implement the traits representing any
    /// resources exported by the world.
    #[cfg_attr(feature = "clap", arg(long, value_parser = parse_map, default_value = ""))]
    pub resource_exports: HashMap<String, String>,

    /// If true, generate stub implementations for any exported functions,
    /// interfaces, and/or resources.
    #[cfg_attr(feature = "clap", arg(long))]
    pub stubs: bool,

    /// Optionally prefix any export names with the specified value.
    ///
    /// This is useful to avoid name conflicts when testing.
    #[cfg_attr(feature = "clap", arg(long))]
    pub export_prefix: Option<String>,

    /// Whether to generate owning or borrowing type definitions.
    ///
    /// Valid values include:
    /// - `owning`: Generated types will be composed entirely of owning fields,
    /// regardless of whether they are used as parameters to imports or not.
    /// - `borrowing`: Generated types used as parameters to imports will be
    /// "deeply borrowing", i.e. contain references rather than owned values
    /// when applicable.
    /// - `borrowing-duplicate-if-necessary`: As above, but generating distinct
    /// types for borrowing and owning, if necessary.
    #[cfg_attr(feature = "clap", arg(long, default_value_t = Ownership::Owning))]
    pub ownership: Ownership,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = Cpp::new();
        r.skip = self.skip.iter().cloned().collect();
        r.opts = self;
        Box::new(r)
    }
}

impl Cpp {
    fn new() -> Cpp {
        Cpp::default()
    }

    fn interface<'a>(
        &'a mut self,
        identifier: Identifier<'a>,
        wasm_import_module: Option<&'a str>,
        resolve: &'a Resolve,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(resolve);

        InterfaceGenerator {
            identifier,
            wasm_import_module,
            src: Source::default(),
            in_import,
            gen: self,
            sizes,
            resolve,
            return_pointer_area_size: 0,
            return_pointer_area_align: 0,
        }
    }

    fn emit_modules(&mut self, modules: &BTreeMap<Option<PackageName>, Vec<String>>) {
        let mut map = BTreeMap::new();
        for (pkg, modules) in modules {
            match pkg {
                Some(pkg) => {
                    let prev = map
                        .entry(&pkg.namespace)
                        .or_insert(BTreeMap::new())
                        .insert(&pkg.name, modules);
                    assert!(prev.is_none());
                }
                None => {
                    for module in modules {
                        uwriteln!(self.src, "{module}");
                    }
                }
            }
        }
        for (ns, pkgs) in map {
            uwriteln!(self.src, "namespace {} {{", ns.to_snake_case());
            for (pkg, modules) in pkgs {
                uwriteln!(self.src, "namespace {} {{", pkg.to_snake_case());
                for module in modules {
                    uwriteln!(self.src, "{module}");
                }
                uwriteln!(self.src, "}}");
            }
            uwriteln!(self.src, "}}");
        }
    }
}

impl WorldGenerator for Cpp {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.world = Some(world);
        let version = env!("CARGO_PKG_VERSION");
        uwriteln!(
            self.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );
        uwriteln!(
            self.src,
            r#"#include "{}_cpp.h"
            #include <utility>

            extern "C" void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size);

            __attribute__((__weak__, __export_name__("cabi_realloc")))
            void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {{
                (void) old_size;
                if (new_size == 0) return (void*) align;
                void *ret = realloc(ptr, new_size);
                if (!ret) abort();
                return ret;
            }}

            "#,
            resolve.worlds[world].name.to_snake_case(),
        );
        self.types.analyze(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let wasm_import_module = resolve.name_world_key(name);
        let mut gen = self.interface(
            Identifier::Interface(id, name),
            Some(&wasm_import_module),
            resolve,
            true,
        );
        let (snake, path_to_root, pkg) = gen.start_append_submodule(name);
        gen.types(id);

        gen.generate_imports(resolve.interfaces[id].functions.values());

        gen.finish_append_submodule(&snake, &path_to_root, pkg);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        self.import_funcs_called = true;

        let mut gen = self.interface(Identifier::World(world), Some("$root"), resolve, true);

        gen.generate_imports(funcs.iter().map(|(_, func)| *func));

        let src = gen.finish();
        self.src.push_str(&src);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> std::result::Result<(), anyhow::Error> {
        let (pkg, inner_name) = match name {
            WorldKey::Name(name) => (None, name),
            WorldKey::Interface(id) => {
                let interface = &resolve.interfaces[*id];
                (
                    Some(&resolve.packages[interface.package.unwrap()].name),
                    interface.name.as_ref().unwrap(),
                )
            }
        };
        let path = format!(
            "{}{inner_name}",
            if let Some(pkg) = pkg {
                format!("{}::{}::", pkg.namespace, pkg.name)
            } else {
                String::new()
            }
        );
        let impl_name = self
            .opts
            .interface_exports
            .get(&path)
            .cloned()
            .or_else(|| self.opts.stubs.then(|| "Stub".to_owned()))
            .ok_or_else(|| format!("interface export implementation required for `{path}`"));
        let mut gen = self.interface(Identifier::Interface(id, name), None, resolve, false);
        let (snake, path_to_root, pkg) = gen.start_append_submodule(name);
        gen.types(id);
        gen.generate_exports(
            &inner_name.to_upper_camel_case(),
            Some(&path),
            impl_name.as_deref(),
            Some(name),
            resolve.interfaces[id].functions.values(),
        );
        gen.finish_append_submodule(&snake, &path_to_root, pkg);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> std::result::Result<(), anyhow::Error> {
        let world_name = &resolve.worlds[world].name;
        let impl_name = self
            .opts
            .world_exports
            .clone()
            .or_else(|| self.opts.stubs.then(|| "Stub".to_owned()))
            .ok_or_else(|| format!("world export implementation required"));
        let trait_name = world_name.to_upper_camel_case();
        let mut gen = self.interface(Identifier::World(world), None, resolve, false);
        gen.generate_exports(
            &trait_name,
            None,
            impl_name.as_deref(),
            None,
            funcs.iter().map(|f| f.1),
        );
        let src = gen.finish();
        self.src.push_str(&src);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(Identifier::World(world), None, resolve, true);
        for (name, ty) in types {
            gen.define_type(name, *ty);
        }
        let src = gen.finish();
        self.src.push_str(&src);
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) {
        if !self.import_funcs_called {
            // We call `import_funcs` even if the world doesn't import any
            // functions since one of the side effects of that method is to
            // generate `struct`s for any imported resources.
            self.import_funcs(resolve, world, &[], files);
        }

        let name = &resolve.worlds[world].name;
        let imports = mem::take(&mut self.import_modules);
        self.emit_modules(&imports);
        let _exports = mem::take(&mut self.export_modules);
        // if !exports.is_empty() {
        //     self.src.push_str("pub mod exports {\n");
        //     self.emit_modules(&exports);
        //     self.src.push_str("}\n");
        // }

        // The custom section name here must start with "component-type" but
        // otherwise is attempted to be unique here to ensure that this doesn't get
        // concatenated to other custom sections by LLD by accident since LLD will
        // concatenate custom sections of the same name.
        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        let _component_type = wit_component::metadata::encode(
            resolve,
            world,
            wit_component::StringEncoding::UTF8,
            Some(&producers),
        )
        .unwrap();

        // if self.opts.stubs {
        //     self.src.push_str("\npub struct Stub;\n");
        //     let world_id = world;
        //     let world = &resolve.worlds[world];
        //     let mut funcs = Vec::new();
        //     for (name, export) in world.exports.iter() {
        //         let (pkg, name) = match name {
        //             WorldKey::Name(name) => (None, name),
        //             WorldKey::Interface(id) => {
        //                 let interface = &resolve.interfaces[*id];
        //                 (
        //                     Some(&resolve.packages[interface.package.unwrap()].name),
        //                     interface.name.as_ref().unwrap(),
        //                 )
        //             }
        //         };
        //         match export {
        //             WorldItem::Function(func) => {
        //                 funcs.push(func);
        //             }
        //             WorldItem::Interface(id) => {
        //                 for (resource, funcs) in
        //                     group_by_resource(resolve.interfaces[*id].functions.values())
        //                 {
        //                     let mut gen =
        //                         self.interface(Identifier::World(world_id), None, resolve, false);
        //                     gen.generate_stub(resource, pkg, name, true, &funcs);
        //                     let stub = gen.finish();
        //                     self.src.push_str(&stub);
        //                 }
        //             }
        //             WorldItem::Type(_) => unreachable!(),
        //         }
        //     }

        //     for (resource, funcs) in group_by_resource(funcs.into_iter()) {
        //         let mut gen = self.interface(Identifier::World(world_id), None, resolve, false);
        //         gen.generate_stub(resource, None, &world.name, false, &funcs);
        //         let stub = gen.finish();
        //         self.src.push_str(&stub);
        //     }
        // }

        let src = mem::take(&mut self.src);
        let module_name = name.to_snake_case();
        files.push(&format!("{module_name}.cpp"), src.as_bytes());
    }
}

#[derive(Clone)]
enum Identifier<'a> {
    World(WorldId),
    Interface(InterfaceId, &'a WorldKey),
}

struct InterfaceGenerator<'a> {
    src: Source,
    identifier: Identifier<'a>,
    in_import: bool,
    sizes: SizeAlign,
    gen: &'a mut Cpp,
    wasm_import_module: Option<&'a str>,
    resolve: &'a Resolve,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
}

impl InterfaceGenerator<'_> {
    fn generate_exports<'a>(
        &mut self,
        trait_name: &str,
        path: Option<&str>,
        impl_name: Result<&str, &String>,
        interface_name: Option<&WorldKey>,
        funcs: impl Iterator<Item = &'a Function>,
    ) {
        let mut by_resource = group_by_resource(funcs);

        // Make sure we generate code for resources with no methods:
        match self.identifier {
            Identifier::Interface(id, _) => {
                for ty in self.resolve.interfaces[id].types.values() {
                    if let TypeDefKind::Resource = &self.resolve.types[*ty].kind {
                        by_resource.entry(Some(*ty)).or_default();
                    }
                }
            }
            Identifier::World(id) => {
                let world = &self.resolve.worlds[id];
                for item in world.exports.values() {
                    if let WorldItem::Type(_) = item {
                        // As of this writing, there's no way this can be represented in WIT, but it should be easy
                        // to handle if that changes.
                        todo!()
                    }
                }
            }
        }

        for (resource, funcs) in by_resource {
            let trait_name = if let Some(ty) = resource {
                self.resolve.types[ty]
                    .name
                    .as_deref()
                    .unwrap()
                    .to_upper_camel_case()
            } else {
                trait_name.to_owned()
            };
            let mut saw_export = false;
            uwriteln!(self.src, "pub trait {trait_name} {{");
            for &func in &funcs {
                if self.gen.skip.contains(&func.name) {
                    continue;
                }
                saw_export = true;
                let mut sig = FnSig::default();
                sig.use_item_name = true;
                sig.private = true;
                if let FunctionKind::Method(_) = &func.kind {
                    // sig.self_arg = Some("&self".into());
                    // sig.self_is_first_param = true;
                }
                self.print_signature(func, TypeMode::Owned, &sig);
                self.src.push_str(";\n");
            }
            uwriteln!(self.src, "}}");

            if saw_export || resource.is_some() {
                let mut path_to_root = String::new();
                if let Some(key) = interface_name {
                    if !self.in_import {
                        path_to_root.push_str("super::");
                    }
                    if let WorldKey::Interface(_) = key {
                        path_to_root.push_str("super::super::");
                    }
                    path_to_root.push_str("super::");
                }
                if let Some(ty) = resource {
                    let name = self.resolve.types[ty].name.as_deref().unwrap();
                    let path = if let Some(path) = path {
                        format!("{path}::{name}")
                    } else {
                        name.to_owned()
                    };
                    let impl_name = self
                        .gen
                        .opts
                        .resource_exports
                        .get(&path)
                        .cloned()
                        .or_else(|| self.gen.opts.stubs.then(|| "Stub".to_owned()))
                        .ok_or_else(|| {
                            format!("resource export implementation required for `{path}`")
                        })
                        .unwrap();

                    uwriteln!(
                        self.src,
                        "use {path_to_root}{impl_name} as Rep{trait_name};"
                    );
                } else {
                    let impl_name = impl_name.unwrap();
                    uwriteln!(
                        self.src,
                        "use {path_to_root}{impl_name} as {trait_name}Impl;"
                    );
                }
                if saw_export {
                    self.src.push_str("const _: () = {\n");
                    for &func in &funcs {
                        self.generate_guest_export(func, interface_name, &trait_name);
                    }
                    self.src.push_str("};\n");
                }

                if let Some(ty) = resource {
                    self.finish_resource_export(ty);
                }
            }
        }
    }

    fn make_export_name(input: &str) -> String {
        input
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' => c,
                _ => '_',
            })
            .collect()
    }

    fn export_name2(module_name: &str, name: &str) -> String {
        let mut res = Self::make_export_name(module_name);
        res.push('_');
        res.push_str(&Self::make_export_name(name));
        res
    }

    fn declare_import2(
        module_name: &str,
        name: &str,
        args: &str,
        result: &str,
    ) -> (String, String) {
        let extern_name = Self::export_name2(module_name, name);
        let import = format!("extern __attribute__((import_module(\"{module_name}\")))\n __attribute__((import_name(\"{name}\")))\n {result} {extern_name}({args});\n");
        (extern_name, import)
    }

    fn generate_imports<'a>(&mut self, funcs: impl Iterator<Item = &'a Function>) {
        let wasm_import_module = self.wasm_import_module.unwrap();
        let mut by_resource = group_by_resource(funcs);

        // Make sure we generate code for resources with no methods:
        match self.identifier {
            Identifier::Interface(id, _) => {
                for ty in self.resolve.interfaces[id].types.values() {
                    if let TypeDefKind::Resource = &self.resolve.types[*ty].kind {
                        by_resource.entry(Some(*ty)).or_default();
                    }
                }
            }
            Identifier::World(id) => {
                let world = &self.resolve.worlds[id];
                for item in world.imports.values() {
                    if let WorldItem::Type(ty) = item {
                        if let TypeDefKind::Resource = &self.resolve.types[*ty].kind {
                            by_resource.entry(Some(*ty)).or_default();
                        }
                    }
                }
            }
        }

        for (resource, funcs) in by_resource {
            if let Some(resource) = resource {
                let name = self.resolve.types[resource].name.as_deref().unwrap();

                let camel = name.to_upper_camel_case();

                let (name_drop, code) = Self::declare_import2(
                    wasm_import_module,
                    &format!("[resource-drop]{name}"),
                    "int32_t",
                    "void",
                );
                // destructor
                uwriteln!(
                    self.src,
                    r#"{camel}::~{camel}() {{
                            {code}
                            if (handle>=0)
                                {name_drop}(handle);
                    }}
                    "#
                );
                // construct from handle (in binding)
                let world = self
                    .gen
                    .world
                    .map(|w| &self.resolve.worlds[w].name)
                    .unwrap()
                    .to_snake_case();
                let base_name = format!("{world}::{RESOURCE_BASE_CLASS_NAME}");
                uwriteln!(
                    self.src,
                    r#"{camel}::{camel}({base_name}&& handle) : {base_name}(std::move(handle)) {{}}"#
                );
            }
            for func in funcs {
                self.generate_guest_import(func);
            }
            if resource.is_some() {
                self.src.push_str("}\n");
            }
        }
    }

    fn finish(&mut self) -> String {
        // if self.return_pointer_area_align > 0 {
        //     uwrite!(
        //         self.src,
        //         "
        //             #[allow(unused_imports)]
        //             use wit_bindgen::rt::{{alloc, vec::Vec, string::String}};

        //             #[repr(align({align}))]
        //             struct _RetArea([u8; {size}]);
        //             static mut _RET_AREA: _RetArea = _RetArea([0; {size}]);
        //         ",
        //         align = self.return_pointer_area_align,
        //         size = self.return_pointer_area_size,
        //     );
        // }

        mem::take(&mut self.src).into()
    }

    fn finish_resource_export(&mut self, id: TypeId) {
        let _info = self.gen.resources.entry(id).or_default();
        let name = self.resolve.types[id].name.as_deref().unwrap();
        let _camel = name.to_upper_camel_case();
        let _snake = to_rust_ident(name);
        let _export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        let _interface_name = if let TypeOwner::Interface(id) = self.resolve.types[id].owner {
            &self.gen.interface_names[&id]
        } else {
            unreachable!()
        };
    }
    fn start_append_submodule(&mut self, name: &WorldKey) -> (String, String, Option<PackageName>) {
        let snake = match name {
            WorldKey::Name(name) => to_rust_ident(name),
            WorldKey::Interface(id) => {
                to_rust_ident(self.resolve.interfaces[*id].name.as_ref().unwrap())
            }
        };
        let mut path_to_root = String::from("super::");
        let pkg = match name {
            WorldKey::Name(_) => None,
            WorldKey::Interface(id) => {
                let pkg = self.resolve.interfaces[*id].package.unwrap();
                Some(self.resolve.packages[pkg].name.clone())
            }
        };
        if let Identifier::Interface(id, _) = self.identifier {
            let mut path = String::new();
            if !self.in_import {
                path.push_str("exports::");
                path_to_root.push_str("super::");
            }
            if let Some(name) = &pkg {
                path.push_str(&format!(
                    "{}::{}::",
                    name.namespace.to_snake_case(),
                    name.name.to_snake_case()
                ));
                path_to_root.push_str("super::super::");
            }
            path.push_str(&snake);
            self.gen.interface_names.insert(id, path);
        }
        (snake, path_to_root, pkg)
    }

    fn finish_append_submodule(
        mut self,
        snake: &str,
        _path_to_root: &str,
        pkg: Option<PackageName>,
    ) {
        let module = self.finish();
        let module = format!(
            "
                namespace {snake} {{
                    {module}
            ",
        );
        let map = if self.in_import {
            &mut self.gen.import_modules
        } else {
            &mut self.gen.export_modules
        };
        map.entry(pkg).or_insert(Vec::new()).push(module);
    }

    // fn print_signature_cpp(
    //     &mut self,
    //     func: &Function,
    //     param_mode: TypeMode,
    //     sig: &FnSig,
    // ) -> Vec<String> {
    //     if !matches!(func.kind, FunctionKind::Constructor(_)) {
    //         self.print_results_cpp(&func.results, TypeMode::Owned);
    //         self.push_str(" ");
    //     }
    //     let params = self.print_docs_and_params_cpp(func, param_mode, &sig);
    //     params
    // }

    // fn print_docs_and_params_cpp(
    //     &mut self,
    //     func: &Function,
    //     param_mode: TypeMode,
    //     sig: &FnSig,
    // ) -> Vec<String> {
    //     // self.rustdoc(&func.docs);
    //     // self.rustdoc_params(&func.params, "Parameters");
    //     // TODO: re-add this when docs are back
    //     // self.rustdoc_params(&func.results, "Return");

    //     let object = match &func.kind {
    //         FunctionKind::Freestanding => None,
    //         FunctionKind::Method(i) => Some(i),
    //         FunctionKind::Static(i) => Some(i),
    //         FunctionKind::Constructor(i) => Some(i),
    //     }
    //     .map(|i| {
    //         self.resolve.types[*i]
    //             .name
    //             .as_ref()
    //             .unwrap()
    //             .to_pascal_case()
    //     })
    //     .unwrap_or_default();
    //     let func_name = if sig.use_item_name {
    //         if let FunctionKind::Constructor(i) = &func.kind {
    //             format!("{object}::{object}")
    //         } else {
    //             format!("{object}::{}", func.item_name().to_pascal_case())
    //         }
    //     } else {
    //         func.name.to_pascal_case()
    //     };
    //     self.push_str(&func_name);
    //     if let Some(generics) = &sig.generics {
    //         self.push_str(generics);
    //     }
    //     self.push_str("(");
    //     if let Some(arg) = &sig.self_arg {
    //         self.push_str(arg);
    //         self.push_str(",");
    //     }
    //     let mut params = Vec::new();
    //     for (i, (name, param)) in func.params.iter().enumerate() {
    //         params.push(name.clone());
    //         if i == 0 && sig.self_is_first_param {
    //             // params.push("self".to_string());
    //             continue;
    //         }
    //         if i == 0 && name == "self" {
    //             continue;
    //         }
    //         let name = to_rust_ident(name);
    //         self.print_ty_cpp(param, param_mode);
    //         self.push_str(" ");
    //         self.push_str(&name);
    //         if i + 1 != func.params.len() {
    //             self.push_str(",");
    //         }
    //     }
    //     self.push_str(")");
    //     params
    // }

    // fn print_tyid_cpp(&mut self, id: TypeId, mode: TypeMode) {
    //     let info = self.info(id);
    //     let lt = self.lifetime_for(&info, mode);
    //     let ty = &RustGenerator::resolve(self).types[id];
    //     if ty.name.is_some() {
    //         // If this type has a list internally, no lifetime is being printed,
    //         // but we're in a borrowed mode, then that means we're in a borrowed
    //         // context and don't want ownership of the type but we're using an
    //         // owned type definition. Inject a `&` in front to indicate that, at
    //         // the API level, ownership isn't required.
    //         if info.has_list && lt.is_none() {
    //             if let TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) = mode {
    //                 self.push_str("&");
    //                 if lt != "'_" {
    //                     self.push_str(lt);
    //                     self.push_str(" ");
    //                 }
    //             }
    //         }
    //         let name = self.type_path(id, lt.is_none());
    //         self.push_str(&name);

    //         // If the type recursively owns data and it's a
    //         // variant/record/list, then we need to place the
    //         // lifetime parameter on the type as well.
    //         if info.has_list && needs_generics(RustGenerator::resolve(self), &ty.kind) {
    //             self.print_generics(lt);
    //         }

    //         return;

    //         fn needs_generics(resolve: &Resolve, ty: &TypeDefKind) -> bool {
    //             match ty {
    //                 TypeDefKind::Variant(_)
    //                 | TypeDefKind::Record(_)
    //                 | TypeDefKind::Option(_)
    //                 | TypeDefKind::Result(_)
    //                 | TypeDefKind::Future(_)
    //                 | TypeDefKind::Stream(_)
    //                 | TypeDefKind::List(_)
    //                 | TypeDefKind::Flags(_)
    //                 | TypeDefKind::Enum(_)
    //                 | TypeDefKind::Tuple(_)
    //                 | TypeDefKind::Union(_) => true,
    //                 TypeDefKind::Type(Type::Id(t)) => {
    //                     needs_generics(resolve, &resolve.types[*t].kind)
    //                 }
    //                 TypeDefKind::Type(Type::String) => true,
    //                 TypeDefKind::Resource | TypeDefKind::Handle(_) | TypeDefKind::Type(_) => false,
    //                 TypeDefKind::Unknown => unreachable!(),
    //             }
    //         }
    //     }

    //     match &ty.kind {
    //         TypeDefKind::List(t) => self.print_list(t, mode),

    //         TypeDefKind::Option(t) => {
    //             self.push_str("Option<");
    //             self.print_ty(t, mode);
    //             self.push_str(">");
    //         }

    //         TypeDefKind::Result(r) => {
    //             self.push_str("Result<");
    //             self.print_optional_ty(r.ok.as_ref(), mode);
    //             self.push_str(",");
    //             self.print_optional_ty(r.err.as_ref(), mode);
    //             self.push_str(">");
    //         }

    //         TypeDefKind::Variant(_) => panic!("unsupported anonymous variant"),

    //         // Tuple-like records are mapped directly to Rust tuples of
    //         // types. Note the trailing comma after each member to
    //         // appropriately handle 1-tuples.
    //         TypeDefKind::Tuple(t) => {
    //             self.push_str("(");
    //             for ty in t.types.iter() {
    //                 self.print_ty(ty, mode);
    //                 self.push_str(",");
    //             }
    //             self.push_str(")");
    //         }
    //         TypeDefKind::Resource => {
    //             panic!("unsupported anonymous type reference: resource")
    //         }
    //         TypeDefKind::Record(_) => {
    //             panic!("unsupported anonymous type reference: record")
    //         }
    //         TypeDefKind::Flags(_) => {
    //             panic!("unsupported anonymous type reference: flags")
    //         }
    //         TypeDefKind::Enum(_) => {
    //             panic!("unsupported anonymous type reference: enum")
    //         }
    //         TypeDefKind::Union(_) => {
    //             panic!("unsupported anonymous type reference: union")
    //         }
    //         TypeDefKind::Future(ty) => {
    //             self.push_str("Future<");
    //             self.print_optional_ty(ty.as_ref(), mode);
    //             self.push_str(">");
    //         }
    //         TypeDefKind::Stream(stream) => {
    //             self.push_str("Stream<");
    //             self.print_optional_ty(stream.element.as_ref(), mode);
    //             self.push_str(",");
    //             self.print_optional_ty(stream.end.as_ref(), mode);
    //             self.push_str(">");
    //         }

    //         TypeDefKind::Handle(Handle::Own(ty)) => {
    //             self.mark_resource_owned(*ty);
    //             self.print_ty(&Type::Id(*ty), mode);
    //         }

    //         TypeDefKind::Handle(Handle::Borrow(ty)) => {
    //             self.push_str("&");
    //             self.print_ty(&Type::Id(*ty), mode);
    //         }

    //         TypeDefKind::Type(t) => self.print_ty(t, mode),

    //         TypeDefKind::Resource => {
    //             todo!("implement resources")
    //         }

    //         TypeDefKind::Unknown => unreachable!(),
    //     }
    // }

    // fn print_ty_cpp(&mut self, ty: &Type, mode: TypeMode) {
    //     match ty {
    //         Type::Id(t) => self.print_tyid_cpp(*t, mode),
    //         Type::Bool => self.push_str("bool"),
    //         Type::U8 => self.push_str("uint8_t"),
    //         Type::U16 => self.push_str("uint16_t"),
    //         Type::U32 => self.push_str("uint32_t"),
    //         Type::U64 => self.push_str("uint64_t"),
    //         Type::S8 => self.push_str("int8_t"),
    //         Type::S16 => self.push_str("int16_t"),
    //         Type::S32 => self.push_str("int32_t"),
    //         Type::S64 => self.push_str("int64_t"),
    //         Type::Float32 => self.push_str("float"),
    //         Type::Float64 => self.push_str("double"),
    //         Type::Char => self.push_str("int32_t"),
    //         Type::String => match mode {
    //             TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) => {
    //                 self.push_str("std::string_view");
    //             }
    //             TypeMode::Owned => {
    //                 self.push_str("std::string");
    //             }
    //         },
    //     }
    // }

    // fn print_option_ty_cpp(&mut self, ty: Option<&Type>, mode: TypeMode) {
    //     match ty {
    //         Some(ty) => self.print_ty_cpp(ty, mode),
    //         None => self.push_str("void"),
    //     }
    // }

    // fn print_results_cpp(&mut self, results: &Results, mode: TypeMode) {
    //     match results.len() {
    //         0 | 1 => self.print_option_ty_cpp(results.iter_types().next(), mode),
    //         _ => todo!(),
    //     }
    // }

    fn generate_guest_import(&mut self, func: &Function) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let mut sig = FnSig::default();
        let param_mode = TypeMode::AllBorrowed("'_");
        match &func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Method(_) | FunctionKind::Static(_) | FunctionKind::Constructor(_) => {
                sig.use_item_name = true;
                // if let FunctionKind::Method(_) = &func.kind {
                //     sig.self_arg = Some("&self".into());
                //     sig.self_is_first_param = true;
                // }
            }
        }
        // self.src.push_str("#[allow(clippy::all)]\n");
        let params = self.print_signature(func, param_mode, &sig);
        if matches!(func.kind, FunctionKind::Method(_)) {
            self.src.push_str("const");
        }
        self.src.push_str("{\n");

        let mut f = FunctionBindgen::new(self, params, None);
        abi::call(
            f.gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            // needs_cleanup_list,
            src,
            import_return_pointer_area_size,
            import_return_pointer_area_align,
            ..
        } = f;

        // if needs_cleanup_list {
        //     self.src.push_str("let mut cleanup_list = Vec::new();\n");
        // }
        if import_return_pointer_area_size > 0 {
            let align = import_return_pointer_area_align.max(4);
            let elems = (import_return_pointer_area_size + (align - 1)) / align;
            let tp = match align {
                4 => "uint32_t",
                8 => "uint64_t",
                _ => todo!(),
            };
            uwriteln!(self.src, " {tp} ret_area[{elems}];")
        }
        //     uwrite!(
        //         self.src,
        //         "
        //             #[repr(align({import_return_pointer_area_align}))]
        //             struct RetArea([u8; {import_return_pointer_area_size}]);
        //             let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
        //         ",
        //     );
        // }
        self.src.push_str(&String::from(src));

        // self.src.push_str("}\n");
        self.src.push_str("}\n");
    }

    fn generate_guest_export(
        &mut self,
        func: &Function,
        _interface_name: Option<&WorldKey>,
        _trait_name: &str,
    ) {
        if self.gen.skip.contains(&func.name) {
            return;
        }
        todo!();

        // let name_snake = func.name.to_snake_case().replace('.', "_");
        // let wasm_module_export_name = interface_name.map(|k| self.resolve.name_world_key(k));
        // let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        // let export_name = func.core_export_name(wasm_module_export_name.as_deref());
        // uwrite!(
        //     self.src,
        //     "
        //         #[doc(hidden)]
        //         #[export_name = \"{export_prefix}{export_name}\"]
        //         #[allow(non_snake_case)]
        //         unsafe extern \"C\" fn __export_{name_snake}(\
        //     ",
        // );

        // let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);
        // let mut params = Vec::new();
        // for (i, param) in sig.params.iter().enumerate() {
        //     let name = format!("arg{}", i);
        //     uwrite!(self.src, "{name}: {},", wasm_type(*param));
        //     params.push(name);
        // }
        // self.src.push_str(")");

        // match sig.results.len() {
        //     0 => {}
        //     1 => {
        //         uwrite!(self.src, " -> {}", wasm_type(sig.results[0]));
        //     }
        //     _ => unimplemented!(),
        // }

        // self.push_str(" {");

        // let mut f = FunctionBindgen::new(self, params, Some(trait_name));
        // f.gen.resolve.call(
        //     AbiVariant::GuestExport,
        //     LiftLower::LiftArgsLowerResults,
        //     func,
        //     &mut f,
        // );
        // let FunctionBindgen {
        //     needs_cleanup_list,
        //     src,
        //     ..
        // } = f;
        // assert!(!needs_cleanup_list);
        // self.src.push_str(&String::from(src));
        // self.src.push_str("}\n");

        // if self.resolve.guest_export_needs_post_return(func) {
        //     let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        //     let mut params = Vec::new();
        //     for (i, result) in sig.results.iter().enumerate() {
        //         let name = format!("arg{}", i);
        //         uwrite!(self.src, "{name}: {},", wasm_type(*result));
        //         params.push(name);
        //     }
        //     self.src.push_str(") {\n");

        //     let mut f = FunctionBindgen::new(self, params, Some(trait_name));
        //     f.gen.resolve.post_return(func, &mut f);
        //     let FunctionBindgen {
        //         needs_cleanup_list,
        //         src,
        //         ..
        //     } = f;
        //     assert!(!needs_cleanup_list);
        //     self.src.push_str(&String::from(src));
        //     self.src.push_str("}\n");
        //     self.src.push_str("};\n");
        // }
    }

    // fn generate_stub(
    //     &mut self,
    //     resource: Option<TypeId>,
    //     pkg: Option<&PackageName>,
    //     name: &str,
    //     in_interface: bool,
    //     funcs: &[&Function],
    // ) {
    //     let path = if let Some(pkg) = pkg {
    //         format!(
    //             "{}::{}::{}",
    //             to_rust_ident(&pkg.namespace),
    //             to_rust_ident(&pkg.name),
    //             to_rust_ident(name),
    //         )
    //     } else {
    //         to_rust_ident(name)
    //     };

    //     let name = resource
    //         .map(|ty| {
    //             self.resolve.types[ty]
    //                 .name
    //                 .as_deref()
    //                 .unwrap()
    //                 .to_upper_camel_case()
    //         })
    //         .unwrap_or_else(|| name.to_upper_camel_case());

    //     let qualified_name = if in_interface {
    //         format!("exports::{path}::{name}")
    //     } else {
    //         name
    //     };

    //     uwriteln!(self.src, "impl {qualified_name} for Stub {{");

    //     for &func in funcs {
    //         if self.gen.skip.contains(&func.name) {
    //             continue;
    //         }
    //         let mut sig = FnSig::default();
    //         sig.use_item_name = true;
    //         sig.private = true;
    //         if let FunctionKind::Method(_) = &func.kind {
    //             // sig.self_arg = Some("&self".into());
    //             // sig.self_is_first_param = true;
    //         }
    //         self.print_signature(func, TypeMode::Owned, &sig);
    //         self.src.push_str("{ unreachable!() }\n");
    //     }

    //     self.src.push_str("}\n");
    // }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn ownership(&self) -> Ownership {
        self.gen.opts.ownership
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let mut path = String::new();
        if let Identifier::Interface(cur, name) = self.identifier {
            if cur == interface {
                return None;
            }
            if !self.in_import {
                //path.push_str("super::");
            }
            match name {
                WorldKey::Name(_) => {
                    //path.push_str("super::");
                }
                WorldKey::Interface(_) => {
                    //path.push_str("super::super::super::");
                }
            }
        }
        let name = &self.gen.interface_names[&interface];
        path.push_str(&name);
        Some(path)
    }

    fn is_exported_resource(&self, ty: TypeId) -> bool {
        matches!(
            self.gen
                .resources
                .get(&dealias(self.resolve, ty))
                .map(|info| info.direction),
            Some(Direction::Export)
        )
    }

    // fn add_own(&mut self, resource: TypeId, handle: TypeId) {
    //     self.gen
    //         .resources
    //         .entry(dealias(self.resolve, resource))
    //         .or_default()
    //         .own = Some(handle);
    // }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.gen.types.get(ty)
    }

    fn types_mut(&mut self) -> &mut Types {
        &mut self.gen.types
    }

    fn print_borrowed_slice(
        &mut self,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
        mode: TypeMode,
    ) {
        self.print_rust_slice(mutbl, ty, lifetime, mode);
    }

    fn print_borrowed_str(&mut self, _lifetime: &'static str) {
        self.push_str("&");
        // if self.gen.opts.raw_strings {
        //     self.push_str("[u8]");
        // } else {
        self.push_str("str");
        // }
    }

    fn push_vec_name(&mut self) {
        self.push_str("std::vector");
    }

    fn push_string_name(&mut self) {
        self.push_str("std::string");
    }

    fn mark_resource_owned(&mut self, resource: TypeId) {
        self.gen
            .resources
            .entry(dealias(self.resolve, resource))
            .or_default()
            .owned = true;
    }

    fn print_signature(
        &mut self,
        func: &Function,
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        if !matches!(func.kind, FunctionKind::Constructor(_)) {
            self.print_results(&func.results, TypeMode::Owned);
            self.push_str(" ");
        }
        let params = self.print_docs_and_params(func, param_mode, &sig);
        params
    }

    fn print_docs_and_params(
        &mut self,
        func: &Function,
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        // self.rustdoc(&func.docs);
        // self.rustdoc_params(&func.params, "Parameters");
        // TODO: re-add this when docs are back
        // self.rustdoc_params(&func.results, "Return");

        let object = match &func.kind {
            FunctionKind::Freestanding => None,
            FunctionKind::Method(i) => Some(i),
            FunctionKind::Static(i) => Some(i),
            FunctionKind::Constructor(i) => Some(i),
        }
        .map(|i| {
            self.resolve.types[*i]
                .name
                .as_ref()
                .unwrap()
                .to_pascal_case()
        })
        .unwrap_or_default();
        let func_name = if sig.use_item_name {
            if let FunctionKind::Constructor(_i) = &func.kind {
                format!("{object}::{object}")
            } else {
                format!("{object}::{}", func.item_name().to_pascal_case())
            }
        } else {
            func.name.to_pascal_case()
        };
        self.push_str(&func_name);
        if let Some(generics) = &sig.generics {
            self.push_str(generics);
        }
        self.push_str("(");
        if let Some(arg) = &sig.self_arg {
            self.push_str(arg);
            self.push_str(",");
        }
        let mut params = Vec::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            params.push(to_rust_ident(name));
            if i == 0 && sig.self_is_first_param {
                // params.push("self".to_string());
                continue;
            }
            if i == 0 && name == "self" {
                continue;
            }
            let name = to_rust_ident(name);
            self.print_ty(param, param_mode);
            self.push_str(" ");
            self.push_str(&name);
            if i + 1 != func.params.len() {
                self.push_str(",");
            }
        }
        self.push_str(")");
        params
    }

    fn print_tyid(&mut self, id: TypeId, mode: TypeMode) {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        let ty = &RustGenerator::resolve(self).types[id];
        if ty.name.is_some() {
            // If this type has a list internally, no lifetime is being printed,
            // but we're in a borrowed mode, then that means we're in a borrowed
            // context and don't want ownership of the type but we're using an
            // owned type definition. Inject a `&` in front to indicate that, at
            // the API level, ownership isn't required.
            if info.has_list && lt.is_none() {
                if let TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) = mode {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                        self.push_str(" ");
                    }
                }
            }
            let name = self.type_path(id, lt.is_none());
            self.push_str(&name);

            // If the type recursively owns data and it's a
            // variant/record/list, then we need to place the
            // lifetime parameter on the type as well.
            if info.has_list && needs_generics(RustGenerator::resolve(self), &ty.kind) {
                self.print_generics(lt);
            }

            return;

            fn needs_generics(resolve: &Resolve, ty: &TypeDefKind) -> bool {
                match ty {
                    TypeDefKind::Variant(_)
                    | TypeDefKind::Record(_)
                    | TypeDefKind::Option(_)
                    | TypeDefKind::Result(_)
                    | TypeDefKind::Future(_)
                    | TypeDefKind::Stream(_)
                    | TypeDefKind::List(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Tuple(_) => true,
                    TypeDefKind::Type(Type::Id(t)) => {
                        needs_generics(resolve, &resolve.types[*t].kind)
                    }
                    TypeDefKind::Type(Type::String) => true,
                    TypeDefKind::Resource | TypeDefKind::Handle(_) | TypeDefKind::Type(_) => false,
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
        }

        match &ty.kind {
            TypeDefKind::List(t) => self.print_list(t, mode),

            TypeDefKind::Option(t) => {
                self.push_str("std::option<");
                self.print_ty(t, mode);
                self.push_str(">");
            }

            TypeDefKind::Result(r) => {
                self.push_str("std::expected<");
                self.print_optional_ty(r.ok.as_ref(), mode);
                self.push_str(",");
                self.print_optional_ty(r.err.as_ref(), mode);
                self.push_str(">");
            }

            TypeDefKind::Variant(_) => panic!("unsupported anonymous variant"),

            // Tuple-like records are mapped directly to Rust tuples of
            // types. Note the trailing comma after each member to
            // appropriately handle 1-tuples.
            TypeDefKind::Tuple(t) => {
                self.push_str("(");
                for ty in t.types.iter() {
                    self.print_ty(ty, mode);
                    self.push_str(",");
                }
                self.push_str(")");
            }
            TypeDefKind::Resource => {
                panic!("unsupported anonymous type reference: resource")
            }
            TypeDefKind::Record(_) => {
                panic!("unsupported anonymous type reference: record")
            }
            TypeDefKind::Flags(_) => {
                panic!("unsupported anonymous type reference: flags")
            }
            TypeDefKind::Enum(_) => {
                panic!("unsupported anonymous type reference: enum")
            }
            TypeDefKind::Future(ty) => {
                self.push_str("Future<");
                self.print_optional_ty(ty.as_ref(), mode);
                self.push_str(">");
            }
            TypeDefKind::Stream(stream) => {
                self.push_str("Stream<");
                self.print_optional_ty(stream.element.as_ref(), mode);
                self.push_str(",");
                self.print_optional_ty(stream.end.as_ref(), mode);
                self.push_str(">");
            }

            TypeDefKind::Handle(Handle::Own(ty)) => {
                self.mark_resource_owned(*ty);
                self.print_ty(&Type::Id(*ty), mode);
            }

            TypeDefKind::Handle(Handle::Borrow(ty)) => {
                self.push_str("&");
                self.print_ty(&Type::Id(*ty), mode);
            }

            TypeDefKind::Type(t) => self.print_ty(t, mode),

            // TypeDefKind::Resource => {
            //     todo!("implement resources")
            // }
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn print_ty(&mut self, ty: &Type, mode: TypeMode) {
        match ty {
            Type::Id(t) => self.print_tyid(*t, mode),
            Type::Bool => self.push_str("bool"),
            Type::U8 => self.push_str("uint8_t"),
            Type::U16 => self.push_str("uint16_t"),
            Type::U32 => self.push_str("uint32_t"),
            Type::U64 => self.push_str("uint64_t"),
            Type::S8 => self.push_str("int8_t"),
            Type::S16 => self.push_str("int16_t"),
            Type::S32 => self.push_str("int32_t"),
            Type::S64 => self.push_str("int64_t"),
            Type::Float32 => self.push_str("float"),
            Type::Float64 => self.push_str("double"),
            Type::Char => self.push_str("int32_t"),
            Type::String => match mode {
                TypeMode::AllBorrowed(_lt) | TypeMode::LeafBorrowed(_lt) => {
                    self.push_str("std::string_view");
                }
                TypeMode::Owned => {
                    self.push_str("std::string");
                }
                TypeMode::HandlesBorrowed(_) => todo!(),
            },
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>, mode: TypeMode) {
        match ty {
            Some(ty) => self.print_ty(ty, mode),
            None => self.push_str("void"),
        }
    }

    fn print_results(&mut self, results: &Results, mode: TypeMode) {
        match results.len() {
            0 | 1 => self.print_optional_ty(results.iter_types().next(), mode),
            _ => todo!(),
        }
    }

    fn wasm_type(&mut self, ty: WasmType) {
        self.push_str(wasm_type(ty));
    }

    fn print_list(&mut self, ty: &Type, mode: TypeMode) {
        let next_mode = if matches!(self.ownership(), Ownership::Owning) {
            TypeMode::Owned
        } else {
            mode
        };
        match mode {
            TypeMode::AllBorrowed(lt) => {
                self.print_borrowed_slice(false, ty, lt, next_mode);
            }
            TypeMode::LeafBorrowed(lt) => {
                if RustGenerator::resolve(self).all_bits_valid(ty) {
                    self.print_borrowed_slice(false, ty, lt, next_mode);
                } else {
                    self.push_vec_name();
                    self.push_str("<");
                    self.print_ty(ty, next_mode);
                    self.push_str(">");
                }
            }
            TypeMode::Owned => {
                self.push_vec_name();
                self.push_str("<");
                self.print_ty(ty, next_mode);
                self.push_str(">");
            }
            TypeMode::HandlesBorrowed(_) => todo!(),
        }
    }

    fn print_rust_slice(
        &mut self,
        mutbl: bool,
        ty: &Type,
        _lifetime: &'static str,
        mode: TypeMode,
    ) {
        self.push_str("std::vector<");
        self.print_ty(ty, mode);
        self.push_str(">");
        if !mutbl {
            self.push_str(" const ");
        }
        self.push_str("&");
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int32_t",
        WasmType::I64 => "int64_t",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, _name: &str, _record: &Record, _docs: &Docs) {
        //self.print_typedef_record(id, record, docs, false);
    }

    fn type_resource(&mut self, id: TypeId, _name: &str, docs: &Docs) {
        let entry = self
            .gen
            .resources
            .entry(dealias(self.resolve, id))
            .or_default();
        if !self.in_import {
            entry.direction = Direction::Export;
        }
        entry.docs = docs.clone();
    }

    fn type_tuple(&mut self, _id: TypeId, _name: &str, _tuple: &Tuple, _docs: &Docs) {
        //self.print_typedef_tuple(id, tuple, docs);
    }

    fn type_flags(&mut self, _id: TypeId, _name: &str, _flags: &Flags, _docs: &Docs) {
        // self.src.push_str("wit_bindgen::bitflags::bitflags! {\n");
        // self.rustdoc(docs);
        // let repr = RustFlagsRepr::new(flags);
        // self.src.push_str(&format!(
        //     "pub struct {}: {repr} {{\n",
        //     name.to_upper_camel_case(),
        // ));
        // for (i, flag) in flags.flags.iter().enumerate() {
        //     self.rustdoc(&flag.docs);
        //     self.src.push_str(&format!(
        //         "const {} = 1 << {};\n",
        //         flag.name.to_shouty_snake_case(),
        //         i,
        //     ));
        // }
        // self.src.push_str("}\n");
        // self.src.push_str("}\n");
    }

    fn type_variant(&mut self, _id: TypeId, _name: &str, _variant: &Variant, _docs: &Docs) {
        //self.print_typedef_variant(id, variant, docs, false);
    }

    fn type_option(&mut self, _id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        //self.print_typedef_option(id, payload, docs);
    }

    fn type_result(&mut self, _id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        //self.print_typedef_result(id, result, docs);
    }

    fn type_enum(&mut self, _id: TypeId, _name: &str, _enum_: &Enum, _docs: &Docs) {
        //self.print_typedef_enum(id, name, enum_, docs, &[], Box::new(|_| String::new()));
    }

    fn type_alias(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        //self.print_typedef_alias(id, ty, docs);
    }

    fn type_list(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        //self.print_type_list(id, ty, docs);
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {}
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    params: Vec<String>,
    trait_name: Option<&'b str>,
    src: Source,
    blocks: Vec<String>,
    block_storage: Vec<(Source, Vec<(String, String)>)>,
    tmp: usize,
    needs_cleanup_list: bool,
    cleanup: Vec<(String, String)>,
    import_return_pointer_area_size: usize,
    import_return_pointer_area_align: usize,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'b mut InterfaceGenerator<'a>,
        params: Vec<String>,
        trait_name: Option<&'b str>,
    ) -> FunctionBindgen<'a, 'b> {
        FunctionBindgen {
            gen,
            params,
            trait_name,
            src: Default::default(),
            blocks: Vec::new(),
            block_storage: Vec::new(),
            tmp: 0,
            needs_cleanup_list: false,
            cleanup: Vec::new(),
            import_return_pointer_area_size: 0,
            import_return_pointer_area_align: 0,
        }
    }

    fn emit_cleanup(&mut self) {
        // for (ptr, layout) in mem::take(&mut self.cleanup) {
        //     self.push_str(&format!(
        //         "if {layout}.size() != 0 {{\nalloc::dealloc({ptr}, {layout});\n}}\n"
        //     ));
        // }
        // if self.needs_cleanup_list {
        //     self.push_str(
        //         "for (ptr, layout) in cleanup_list {\n
        //             if layout.size() != 0 {\n
        //                 alloc::dealloc(ptr, layout);\n
        //             }\n
        //         }\n",
        //     );
        // }
    }

    fn wasm_type_cpp(ty: WasmType) -> &'static str {
        wit_bindgen_c::wasm_type(ty)
        // match ty {
        //     WasmType::I32 => "int32_t",
        //     WasmType::I64 => "int64_t",
        //     WasmType::F32 => "float",
        //     WasmType::F64 => "double",
        // }
    }

    fn declare_import(
        &mut self,
        module_name: &str,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        let mut args = String::default();
        for (n, param) in params.iter().enumerate() {
            args.push_str(Self::wasm_type_cpp(*param));
            if n + 1 != params.len() {
                args.push_str(", ");
            }
        }
        let result = if results.is_empty() {
            "void"
        } else {
            Self::wasm_type_cpp(results[0])
        };
        let (name, code) = InterfaceGenerator::declare_import2(module_name, name, &args, result);
        self.src.push_str(&code);
        name
        // Define the actual function we're calling inline
        //todo!();
        // let mut sig = "(".to_owned();
        // for param in params.iter() {
        //     sig.push_str("_: ");
        //     sig.push_str(wasm_type(*param));
        //     sig.push_str(", ");
        // }
        // sig.push_str(")");
        // assert!(results.len() < 2);
        // for result in results.iter() {
        //     sig.push_str(" -> ");
        //     sig.push_str(wasm_type(*result));
        // }
        // uwriteln!(
        //     self.src,
        //     "
        //         #[cfg(target_arch = \"wasm32\")]
        //         #[link(wasm_import_module = \"{module_name}\")]
        //         extern \"C\" {{
        //             #[link_name = \"{name}\"]
        //             fn wit_import{sig};
        //         }}

        //         #[cfg(not(target_arch = \"wasm32\"))]
        //         fn wit_import{sig} {{ unreachable!() }}
        //     "
        // );
        // "wit_import".to_string()
    }
}

impl RustFunctionGenerator for FunctionBindgen<'_, '_> {
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

impl Bindgen for FunctionBindgen<'_, '_> {
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

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        let tmp = self.tmp();

        // Imports get a per-function return area to facilitate using the
        // stack whereas exports use a per-module return area to cut down on
        // stack usage. Note that for imports this also facilitates "adapter
        // modules" for components to not have data segments.
        if self.gen.in_import {
            self.import_return_pointer_area_size = self.import_return_pointer_area_size.max(size);
            self.import_return_pointer_area_align =
                self.import_return_pointer_area_align.max(align);
            uwriteln!(self.src, "auto ptr{tmp} = (int32_t)&ret_area;");
        } else {
            todo!();
            // self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
            // self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
            // uwriteln!(self.src, "auto ptr{tmp} = _RET_AREA.0.as_mut_ptr() as i32;");
        }
        format!("ptr{}", tmp)
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn is_list_canonical(&self, resolve: &Resolve, ty: &Type) -> bool {
        resolve.all_bits_valid(ty)
    }

    fn emit(
        &mut self,
        resolve: &Resolve,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        let mut top_as = |cvt: &str| {
            results.push(format!("({cvt})({})", operands.pop().unwrap()));
        };

        // work around the fact that some functions only push
        fn print_to_result<'a, 'b, 'c, T: FnOnce(&mut InterfaceGenerator<'a>)>(
            slf: &'a mut FunctionBindgen<'b, 'c>,
            resolve: &'a Resolve,
            f: T,
        ) -> String {
            let mut sizes = SizeAlign::default();
            sizes.fill(resolve);
            let mut gen = InterfaceGenerator {
                identifier: slf.gen.identifier.clone(),
                wasm_import_module: slf.gen.wasm_import_module.clone(),
                src: Source::default(),
                in_import: slf.gen.in_import.clone(),
                gen: slf.gen.gen,
                sizes,
                resolve,
                return_pointer_area_size: 0,
                return_pointer_area_align: 0,
            };
            f(&mut gen);
            //gen.print_optional_ty(result.ok.as_ref(), TypeMode::Owned);
            let mut ok_type = String::default();
            std::mem::swap(gen.src.as_mut_string(), &mut ok_type);
            ok_type
        }

        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(format!("(int32_t){}", val)),
            Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("(int32_t)0".to_string()),
                        WasmType::I64 => results.push("(int64_t)0".to_string()),
                        WasmType::F32 => results.push("0.0f".to_string()),
                        WasmType::F64 => results.push("0.0".to_string()),
                    }
                }
            }

            Instruction::I64FromU64 | Instruction::I64FromS64 => {
                let s = operands.pop().unwrap();
                results.push(format!("(int64_t)({})", s));
            }
            Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!("(int32_t)({})", s));
            }

            Instruction::F32FromFloat32 => {
                let s = operands.pop().unwrap();
                results.push(format!("(float)({})", s));
            }
            Instruction::F64FromFloat64 => {
                let s = operands.pop().unwrap();
                results.push(format!("(double)({})", s));
            }
            Instruction::Float32FromF32
            | Instruction::Float64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }
            Instruction::S8FromI32 => top_as("int8_t"),
            Instruction::U8FromI32 => top_as("uint8_t"),
            Instruction::S16FromI32 => top_as("int16_t"),
            Instruction::U16FromI32 => top_as("uint16_t"),
            Instruction::U32FromI32 => top_as("uint32_t"),
            Instruction::U64FromI64 => top_as("uint64_t"),
            Instruction::CharFromI32 => {
                todo!();
                // results.push(format!(
                //     "{{
                //         #[cfg(not(debug_assertions))]
                //         {{ ::core::char::from_u32_unchecked({} as u32) }}
                //         #[cfg(debug_assertions)]
                //         {{ ::core::char::from_u32({} as u32).unwrap() }}
                //     }}",
                //     operands[0], operands[0]
                // ));
            }

            Instruction::Bitcasts { casts } => {
                wit_bindgen_rust_lib::bitcast(casts, operands, results)
            }

            Instruction::I32FromBool => {
                results.push(format!("(int32_t)({})", operands[0]));
            }
            Instruction::BoolFromI32 => {
                results.push(format!("{}!=0", operands[0]));
            }

            Instruction::FlagsLower { flags, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("auto flags{} = {};\n", tmp, operands[0]));
                for i in 0..flags.repr().count() {
                    results.push(format!("(flags{}.bits() >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLift { flags, ty, .. } => {
                let repr = RustFlagsRepr::new(flags);
                let name = self.gen.type_path(*ty, true);
                let mut result = format!("{name}::empty()");
                for (i, op) in operands.iter().enumerate() {
                    result.push_str(&format!(
                        " | {name}::from_bits_retain((({op} as {repr}) << {}) as _)",
                        i * 32
                    ));
                }
                results.push(result);
            }

            Instruction::HandleLower {
                handle: Handle::Own(_),
                ..
            } => {
                let op = &operands[0];
                results.push(format!("({op}).into_handle()"))
            }

            Instruction::HandleLower {
                handle: Handle::Borrow(_),
                ..
            } => {
                let op = &operands[0];
                if op == "self" {
                    results.push("this->handle".into());
                } else {
                    results.push(format!("({op}).handle"));
                }
            }

            Instruction::HandleLift { handle, .. } => {
                let op = &operands[0];
                let (prefix, resource, _owned) = match handle {
                    Handle::Borrow(resource) => ("&", resource, false),
                    Handle::Own(resource) => ("", resource, true),
                };
                let resource = dealias(resolve, *resource);

                results.push(
                    if let Direction::Export = self.gen.gen.resources[&resource].direction {
                        match handle {
                            Handle::Borrow(_) => {
                                let name = resolve.types[resource]
                                    .name
                                    .as_deref()
                                    .unwrap()
                                    .to_upper_camel_case();
                                format!(
                                    "::core::mem::transmute::<isize, &Rep{name}>\
                                     ({op}.try_into().unwrap())"
                                )
                            }
                            Handle::Own(_) => {
                                let name = self.gen.type_path(resource, true);
                                format!("{name}::from_handle({op})")
                            }
                        }
                    } else {
                        op.clone()
                        // let name = self.gen.type_path(resource, true);
                        // let world = self.gen.gen.world.map(|w| &resolve.worlds[w].name).unwrap();
                        // format!("{prefix}{name}{{std::move({world}::{RESOURCE_BASE_CLASS_NAME}({op}))}}")
                    },
                );
            }

            Instruction::RecordLower { ty, record, .. } => {
                self.record_lower(*ty, record, &operands[0], results);
            }
            Instruction::RecordLift { ty, record, .. } => {
                let mut result = self.typename_lift(*ty);
                result.push_str("{");
                for (_field, val) in record.fields.iter().zip(operands) {
                    // result.push_str(&to_rust_ident(&field.name));
                    // result.push_str(":");
                    result.push_str(&val);
                    result.push_str(", ");
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::TupleLower { tuple, .. } => {
                self.tuple_lower(tuple, &operands[0], results);
            }
            Instruction::TupleLift { .. } => {
                self.tuple_lift(operands, results);
            }

            Instruction::VariantPayloadName => results.push("e".to_string()),

            Instruction::VariantLower {
                variant: _,
                results: _,
                ty,
                ..
            } => {
                let name = self.typename_lower(*ty);
                let op0 = &operands[0];
                self.push_str(&format!("({name}){op0}"));
            }

            Instruction::VariantLift { variant, ty, .. } => {
                let mut result = String::new();
                result.push_str("{");

                let named_enum = variant.cases.iter().all(|c| c.ty.is_none());
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];

                if named_enum {
                    // In unchecked mode when this type is a named enum then we know we
                    // defined the type so we can transmute directly into it.
                    // result.push_str("#[cfg(not(debug_assertions))]");
                    // result.push_str("{");
                    // result.push_str("::core::mem::transmute::<_, ");
                    // result.push_str(&name.to_upper_camel_case());
                    // result.push_str(">(");
                    // result.push_str(op0);
                    // result.push_str(" as ");
                    // result.push_str(int_repr(variant.tag()));
                    // result.push_str(")");
                    // result.push_str("}");
                }

                // if named_enum {
                //     result.push_str("#[cfg(debug_assertions)]");
                // }
                result.push_str("{");
                result.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lift(*ty);
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    let pat = i.to_string();
                    let block = if case.ty.is_some() {
                        format!("({block})")
                    } else {
                        String::new()
                    };
                    let case = case.name.to_upper_camel_case();
                    // if i == variant.cases.len() - 1 {
                    //     result.push_str("#[cfg(debug_assertions)]");
                    //     result.push_str(&format!("{pat} => {name}::{case}{block},\n"));
                    //     result.push_str("#[cfg(not(debug_assertions))]");
                    //     result.push_str(&format!("_ => {name}::{case}{block},\n"));
                    // } else {
                    result.push_str(&format!("{pat} => {name}::{case}{block},\n"));
                    // }
                }
                // result.push_str("#[cfg(debug_assertions)]");
                // result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                result.push_str("}");
                result.push_str("}");

                result.push_str("}");
                results.push(result);
            }

            Instruction::OptionLower {
                results: _result_types,
                ..
            } => {
                todo!();
                // let some = self.blocks.pop().unwrap();
                // let none = self.blocks.pop().unwrap();
                // self.let_results(result_types.len(), results);
                // let operand = &operands[0];
                // self.push_str(&format!(
                //     "match {operand} {{
                //         Some(e) => {some},
                //         None => {{\n{none}\n}},
                //     }};"
                // ));
            }

            Instruction::OptionLift { .. } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                assert_eq!(none, "()");
                let operand = &operands[0];
                results.push(format!(
                    "{operand}==1 ? std::optional<>(std::move({some})) : std::optional()"
                ));
            }

            Instruction::ResultLower {
                results: _result_types,
                // result,
                ..
            } => {
                todo!();
                // let err = self.blocks.pop().unwrap();
                // let ok = self.blocks.pop().unwrap();
                // self.let_results(result_types.len(), results);
                // let operand = &operands[0];
                // let ok_binding = if result.ok.is_some() { "e" } else { "_" };
                // let err_binding = if result.err.is_some() { "e" } else { "_" };
                // self.push_str(&format!(
                //     "match {operand} {{
                //         Ok({ok_binding}) => {{ {ok} }},
                //         Err({err_binding}) => {{ {err} }},
                //     }};"
                // ));
            }

            Instruction::ResultLift { result, .. } => {
                let mut err = self.blocks.pop().unwrap();
                let mut ok = self.blocks.pop().unwrap();
                if result.ok.is_none() {
                    ok.clear();
                } else {
                    ok = format!("std::move({ok})");
                }
                if result.err.is_none() {
                    err.clear();
                } else {
                    err = format!("std::move({err})");
                }
                let ok_type = print_to_result(self, resolve, |gen| {
                    gen.print_optional_ty(result.ok.as_ref(), TypeMode::Owned)
                });
                let err_type = print_to_result(self, resolve, |gen| {
                    gen.print_optional_ty(result.err.as_ref(), TypeMode::Owned)
                });
                let type_name = format!("std::expected<{ok_type}, {err_type}>",);
                let err_type = "std::unexpected";
                let operand = &operands[0];
                results.push(format!(
                    "{operand}==0 \n? {type_name}({ok}) \n: {type_name}({err_type}({err}))"
                ));
            }

            Instruction::EnumLower { enum_: _, ty, .. } => {
                let name = self.typename_lower(*ty);
                let op0 = &operands[0];
                let result = format!("({name}){op0}");
                results.push(result);
            }

            Instruction::EnumLift {
                enum_: _,
                ty: _,
                name,
            } => {
                results.push(format!("({name}){}", &operands[0]));
            }

            Instruction::ListCanonLower { realloc, .. } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                //                if realloc.is_none() {
                self.push_str(&format!("auto& {} = {};\n", val, operands[0]));
                // } else {
                //     let op0 = operands.pop().unwrap();
                //     self.push_str(&format!("auto {} = ({}).into_boxed_slice();\n", val, op0));
                // }
                self.push_str(&format!("auto {} = (int32_t)({}.data());\n", ptr, val));
                self.push_str(&format!("auto {} = (int32_t)({}.size());\n", len, val));
                if realloc.is_some() {
                    todo!();
                    // self.push_str(&format!("::core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListCanonLift { .. } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("auto {} = {};\n", len, operands[1]));
                let result = format!("std::vector((?*)({}), {len})", operands[0]);
                results.push(result);
            }

            Instruction::StringLower { realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("auto {} = {};\n", val, operands[0]));
                } else {
                    todo!();
                    // let op0 = format!("{}.into_bytes()", operands[0]);
                    // self.push_str(&format!("let {} = ({}).into_boxed_slice();\n", val, op0));
                }
                self.push_str(&format!("auto {} = (int32_t)({}.data());\n", ptr, val));
                self.push_str(&format!("auto {} = (int32_t)({}.size());\n", len, val));
                if realloc.is_some() {
                    todo!();
                    //                    self.push_str(&format!("::core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("auto {} = {};\n", len, operands[1]));
                let result = format!("std::string((char const*)({}), {len})", operands[0]);
                results.push(result);
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
                self.push_str(&format!("else {{\n::core::ptr::null_mut()\n}};\n",));
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
                let _align = self.gen.sizes.align(element);
                let len = format!("len{tmp}");
                let base = format!("base{tmp}");
                let result = format!("result{tmp}");
                self.push_str(&format!(
                    "auto {base} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!(
                    "auto {len} = {operand1};\n",
                    operand1 = operands[1]
                ));
                let elemtype =
                    print_to_result(self, resolve, |gen| gen.print_ty(element, TypeMode::Owned));
                self.push_str(&format!("auto {result} = std::vector<{elemtype}>();\n"));
                self.push_str(&format!("{result}.reserve({len});\n"));
                self.push_str(&format!("for (unsigned i=0;i<{len};++i) {{\n"));
                self.push_str(&format!("auto base = {base} + i * {size};\n"));
                self.push_str(&format!("{result}.push_back({body});\n"));
                self.push_str("}\n");
                results.push(result);
                self.push_str(&format!("free((void*){base});\n"));
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm { name, sig, .. } => {
                let func = self.declare_import(
                    self.gen.wasm_import_module.unwrap(),
                    name,
                    &sig.params,
                    &sig.results,
                );

                // ... then call the function with all our operands
                if sig.results.len() > 0 {
                    self.push_str("auto ret = ");
                    results.push("ret".to_string());
                }
                self.push_str(&func);
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");\n");
            }

            Instruction::CallInterface { func, .. } => {
                self.let_results(func.results.len(), results);
                match &func.kind {
                    FunctionKind::Freestanding => {
                        self.push_str(&format!(
                            "<{0}Impl as {0}>::{1}",
                            self.trait_name.unwrap(),
                            to_rust_ident(&func.name)
                        ));
                    }
                    FunctionKind::Method(ty) | FunctionKind::Static(ty) => {
                        self.push_str(&format!(
                            "<Rep{0} as {0}>::{1}",
                            resolve.types[*ty]
                                .name
                                .as_deref()
                                .unwrap()
                                .to_upper_camel_case(),
                            to_rust_ident(func.item_name())
                        ));
                    }
                    FunctionKind::Constructor(ty) => {
                        self.push_str(&format!(
                            "Own{0}::new(<Rep{0} as {0}>::new",
                            resolve.types[*ty]
                                .name
                                .as_deref()
                                .unwrap()
                                .to_upper_camel_case()
                        ));
                    }
                }
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(")");
                if let FunctionKind::Constructor(_) = &func.kind {
                    self.push_str(")");
                }
                self.push_str(";\n");
            }

            Instruction::Return { amt, func, .. } => {
                self.emit_cleanup();
                match amt {
                    0 => {}
                    1 => {
                        match &func.kind {
                            FunctionKind::Constructor(_) => {
                                // strange but works
                                self.push_str("this->handle = ");
                            }
                            _ => self.push_str("return "),
                        }
                        self.push_str(&operands[0]);
                        self.push_str(";\n");
                    }
                    _ => todo!(),
                }
            }

            Instruction::I32Load { offset } => {
                results.push(format!("*((int32_t const*)({} + {}))", operands[0], offset));
            }
            Instruction::I32Load8U { offset } => {
                results.push(format!(
                    "(int32_t)(*((uint8_t const*)({} + {})))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!(
                    "(int32_t)(*((int8_t const*)({} + {})))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!(
                    "(int32_t)(*((uint16_t const*)({} + {})))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!(
                    "(int32_t)(*((int16_t const*)({} + {})))",
                    operands[0], offset
                ));
            }
            Instruction::I64Load { offset } => {
                results.push(format!("*((int64_t const*)({} + {}))", operands[0], offset));
            }
            Instruction::F32Load { offset } => {
                results.push(format!("*((float const*)({} + {}))", operands[0], offset));
            }
            Instruction::F64Load { offset } => {
                results.push(format!("*((double const*)({} + {}))", operands[0], offset));
            }
            Instruction::I32Store { offset } => {
                self.push_str(&format!(
                    "*((int32_t*)({} + {})) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.push_str(&format!(
                    "*((int8_t*)({} + {})) = int8_t({});\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.push_str(&format!(
                    "*((uint16_t*)({} + {})) = uint16_t({});\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I64Store { offset } => {
                self.push_str(&format!(
                    "*((int64_t*)({} + {})) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F32Store { offset } => {
                self.push_str(&format!(
                    "*((float*)({} + {})) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F64Store { offset } => {
                self.push_str(&format!(
                    "*((double*)({} + {})) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { size, align } => {
                self.push_str(&format!(
                    "wit_bindgen::rt::dealloc({}, {}, {});\n",
                    operands[0], size, align
                ));
            }

            Instruction::GuestDeallocateString => {
                self.push_str(&format!(
                    "wit_bindgen::rt::dealloc({}, ({}) as usize, 1);\n",
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
                    "wit_bindgen::rt::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                ));
            }
        }
    }
}

fn group_by_resource<'a>(
    funcs: impl Iterator<Item = &'a Function>,
) -> BTreeMap<Option<TypeId>, Vec<&'a Function>> {
    let mut by_resource = BTreeMap::<_, Vec<_>>::new();
    for func in funcs {
        match &func.kind {
            FunctionKind::Freestanding => by_resource.entry(None).or_default().push(func),
            FunctionKind::Method(ty) | FunctionKind::Static(ty) | FunctionKind::Constructor(ty) => {
                by_resource.entry(Some(*ty)).or_default().push(func);
            }
        }
    }
    by_resource
}

fn to_rust_ident(name: &str) -> String {
    match name {
        // Escape C++ keywords.
        // Source: https://doc.rust-lang.org/reference/keywords.html
        "this" => "this_".into(),
        _ => wit_bindgen_c::to_c_ident(name),
    }
}
