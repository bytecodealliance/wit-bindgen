use anyhow::{bail, Result};
use heck::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_bindgen_core::abi::{self, AbiVariant, Bindgen, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Source, TypeInfo, Types,
    WorldGenerator,
};
use wit_bindgen_rust_lib::{
    dealias, int_repr, to_rust_ident, to_upper_camel_case, wasm_type, FnSig, Ownership,
    RustFlagsRepr, RustFunctionGenerator, RustGenerator, TypeMode,
};

#[derive(Default, Copy, Clone, PartialEq, Eq)]
enum Direction {
    #[default]
    Import,
    Export,
}

#[derive(Default)]
struct ResourceInfo {
    // Note that a resource can be both imported and exported (e.g. when
    // importing and exporting the same interface which contains one or more
    // resources).  In that case, this field will be `Import` while we're
    // importing the interface and later change to `Export` while we're
    // exporting the interface.
    direction: Direction,
    owned: bool,
    docs: Docs,
}

#[derive(Default)]
struct RustWasm {
    types: Types,
    src: Source,
    opts: Opts,
    import_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    export_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    skip: HashSet<String>,
    interface_names: HashMap<InterfaceId, String>,
    resources: HashMap<TypeId, ResourceInfo>,
    import_funcs_called: bool,
}

#[cfg(feature = "clap")]
fn parse_exports(s: &str) -> Result<HashMap<ExportKey, String>, String> {
    if s.is_empty() {
        Ok(HashMap::default())
    } else {
        s.split(',')
            .map(|entry| {
                let (key, value) = entry.split_once('=').ok_or_else(|| {
                    format!("expected string of form `<key>=<value>[,<key>=<value>...]`; got `{s}`")
                })?;
                Ok((
                    match key {
                        "world" => ExportKey::World,
                        _ => ExportKey::Name(key.to_owned()),
                    },
                    value.to_owned(),
                ))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExportKey {
    World,
    Name(String),
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rustfmt: bool,

    /// If true, code generation should qualify any features that depend on
    /// `std` with `cfg(feature = "std")`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub std_feature: bool,

    /// If true, code generation should pass borrowed string arguments as
    /// `&[u8]` instead of `&str`. Strings are still required to be valid
    /// UTF-8, but this avoids the need for Rust code to do its own UTF-8
    /// validation if it doesn't already have a `&str`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub raw_strings: bool,

    /// Names of functions to skip generating bindings for.
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip: Vec<String>,

    /// Names of the concrete types which implement the traits representing any
    /// functions, interfaces, and/or resources exported by the world.
    ///
    /// Example: `--exports world=MyWorld,ns:pkg/iface1=MyIface1,ns:pkg/iface1/resource1=MyResource1`,
    #[cfg_attr(feature = "clap", arg(long, value_parser = parse_exports, default_value = ""))]
    pub exports: HashMap<ExportKey, String>,

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
    ///
    /// - `owning`: Generated types will be composed entirely of owning fields,
    /// regardless of whether they are used as parameters to imports or not.
    ///
    /// - `borrowing`: Generated types used as parameters to imports will be
    /// "deeply borrowing", i.e. contain references rather than owned values
    /// when applicable.
    ///
    /// - `borrowing-duplicate-if-necessary`: As above, but generating distinct
    /// types for borrowing and owning, if necessary.
    #[cfg_attr(feature = "clap", arg(long, default_value_t = Ownership::Owning))]
    pub ownership: Ownership,

    /// The optional path to the wit-bindgen runtime module to use.
    ///
    /// This defaults to `wit_bindgen::rt`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub runtime_path: Option<String>,

    /// The optional path to the bitflags crate to use.
    ///
    /// This defaults to `wit_bindgen::bitflags`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub bitflags_path: Option<String>,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = RustWasm::new();
        r.skip = self.skip.iter().cloned().collect();
        r.opts = self;
        Box::new(r)
    }
}

impl RustWasm {
    fn new() -> RustWasm {
        RustWasm::default()
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
            uwriteln!(self.src, "pub mod {} {{", ns.to_snake_case());
            for (pkg, modules) in pkgs {
                uwriteln!(self.src, "pub mod {} {{", pkg.to_snake_case());
                for module in modules {
                    uwriteln!(self.src, "{module}");
                }
                uwriteln!(self.src, "}}");
            }
            uwriteln!(self.src, "}}");
        }
    }

    fn runtime_path(&self) -> &str {
        self.opts
            .runtime_path
            .as_deref()
            .unwrap_or("wit_bindgen::rt")
    }

    fn bitflags_path(&self) -> &str {
        self.opts
            .bitflags_path
            .as_deref()
            .unwrap_or("wit_bindgen::bitflags")
    }

    fn lookup_export(&self, key: &ExportKey) -> Result<String> {
        if let Some(key) = self.opts.exports.get(key) {
            return Ok(key.clone());
        }
        if self.opts.stubs {
            return Ok("Stub".to_owned());
        }
        let key = match key {
            ExportKey::World => "world",
            ExportKey::Name(name) => name,
        };
        if self.opts.exports.is_empty() {
            bail!("no `exports` map provided in configuration but key is required for `{key}`")
        }
        bail!("expected `exports` map to contain key `{key}`")
    }
}

impl WorldGenerator for RustWasm {
    fn preprocess(&mut self, resolve: &Resolve, _world: WorldId) {
        wit_bindgen_core::generated_preamble(&mut self.src, env!("CARGO_PKG_VERSION"));
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
    ) -> Result<()> {
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
                format!("{}:{}/", pkg.namespace, pkg.name)
            } else {
                String::new()
            }
        );
        let mut gen = self.interface(Identifier::Interface(id, name), None, resolve, false);
        let (snake, path_to_root, pkg) = gen.start_append_submodule(name);
        gen.types(id);
        gen.generate_exports(
            &ExportKey::Name(path),
            Some(name),
            resolve.interfaces[id].functions.values(),
        )?;
        gen.finish_append_submodule(&snake, &path_to_root, pkg);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let mut gen = self.interface(Identifier::World(world), None, resolve, false);
        gen.generate_exports(&ExportKey::World, None, funcs.iter().map(|f| f.1))?;
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

    fn finish_imports(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) {
        if !self.import_funcs_called {
            // We call `import_funcs` even if the world doesn't import any
            // functions since one of the side effects of that method is to
            // generate `struct`s for any imported resources.
            self.import_funcs(resolve, world, &[], files);
        }
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) {
        let name = &resolve.worlds[world].name;
        let imports = mem::take(&mut self.import_modules);
        self.emit_modules(&imports);
        let exports = mem::take(&mut self.export_modules);
        if !exports.is_empty() {
            self.src.push_str("pub mod exports {\n");
            self.emit_modules(&exports);
            self.src.push_str("}\n");
        }

        self.src.push_str("\n#[cfg(target_arch = \"wasm32\")]\n");

        // The custom section name here must start with "component-type" but
        // otherwise is attempted to be unique here to ensure that this doesn't get
        // concatenated to other custom sections by LLD by accident since LLD will
        // concatenate custom sections of the same name.
        self.src
            .push_str(&format!("#[link_section = \"component-type:{}\"]\n", name,));

        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        let component_type = wit_component::metadata::encode(
            resolve,
            world,
            wit_component::StringEncoding::UTF8,
            Some(&producers),
        )
        .unwrap();

        self.src.push_str("#[doc(hidden)]\n");
        self.src.push_str(&format!(
            "pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; {}] = ",
            component_type.len()
        ));
        self.src.push_str(&format!("{:?};\n", component_type));

        self.src.push_str(
            "
            #[inline(never)]
            #[doc(hidden)]
            #[cfg(target_arch = \"wasm32\")]
            pub fn __link_section() {}
        ",
        );

        if self.opts.stubs {
            self.src.push_str("\n#[derive(Debug)]\npub struct Stub;\n");
            let world_id = world;
            let world = &resolve.worlds[world];
            let mut funcs = Vec::new();
            for (name, export) in world.exports.iter() {
                let (pkg, name) = match name {
                    WorldKey::Name(name) => (None, name),
                    WorldKey::Interface(id) => {
                        let interface = &resolve.interfaces[*id];
                        (
                            Some(&resolve.packages[interface.package.unwrap()].name),
                            interface.name.as_ref().unwrap(),
                        )
                    }
                };
                match export {
                    WorldItem::Function(func) => {
                        funcs.push(func);
                    }
                    WorldItem::Interface(id) => {
                        for (resource, funcs) in
                            group_by_resource(resolve.interfaces[*id].functions.values())
                        {
                            let mut gen =
                                self.interface(Identifier::World(world_id), None, resolve, false);
                            gen.generate_stub(resource, pkg, name, true, &funcs);
                            let stub = gen.finish();
                            self.src.push_str(&stub);
                        }
                    }
                    WorldItem::Type(_) => unreachable!(),
                }
            }

            for (resource, funcs) in group_by_resource(funcs.into_iter()) {
                let mut gen = self.interface(Identifier::World(world_id), None, resolve, false);
                gen.generate_stub(resource, None, &world.name, false, &funcs);
                let stub = gen.finish();
                self.src.push_str(&stub);
            }
        }

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

        let module_name = name.to_snake_case();
        files.push(&format!("{module_name}.rs"), src.as_bytes());
    }
}

enum Identifier<'a> {
    World(WorldId),
    Interface(InterfaceId, &'a WorldKey),
}

struct InterfaceGenerator<'a> {
    src: Source,
    identifier: Identifier<'a>,
    in_import: bool,
    sizes: SizeAlign,
    gen: &'a mut RustWasm,
    wasm_import_module: Option<&'a str>,
    resolve: &'a Resolve,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
}

impl InterfaceGenerator<'_> {
    fn generate_exports<'a>(
        &mut self,
        export_key: &ExportKey,
        interface_name: Option<&WorldKey>,
        funcs: impl Iterator<Item = &'a Function>,
    ) -> Result<()> {
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
                format!(
                    "Guest{}",
                    self.resolve.types[ty]
                        .name
                        .as_deref()
                        .unwrap()
                        .to_upper_camel_case()
                )
            } else {
                "Guest".to_string()
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
                    sig.self_arg = Some("&self".into());
                    sig.self_is_first_param = true;
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
                    let path = match &export_key {
                        ExportKey::World => panic!("can't export resources from worlds"),
                        ExportKey::Name(path) => path,
                    };
                    let name = self.resolve.types[ty].name.as_deref().unwrap();
                    let path = format!("{path}/{name}");
                    let export_key = ExportKey::Name(path);
                    let impl_name = self.gen.lookup_export(&export_key)?;
                    let name = to_upper_camel_case(name);
                    uwriteln!(self.src, "pub use {path_to_root}{impl_name} as {name};");
                } else {
                    let impl_name = self.gen.lookup_export(&export_key)?;
                    uwriteln!(self.src, "use {path_to_root}{impl_name} as _GuestImpl;");
                }
                if saw_export {
                    self.src.push_str("const _: () = {\n");
                    for &func in &funcs {
                        self.generate_guest_export(func, interface_name);
                    }
                    self.src.push_str("};\n");
                }

                if let Some(ty) = resource {
                    self.finish_resource_export(
                        ty,
                        interface_name.expect("resources can only be exported from interfaces"),
                    );
                }
            }
        }

        Ok(())
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

                uwriteln!(
                    self.src,
                    r#"
                        #[derive(Debug)]
                        pub struct {camel} {{
                            handle: i32,
                        }}

                        impl Drop for {camel} {{
                             fn drop(&mut self) {{
                                 unsafe {{
                                     #[cfg(not(target_arch = "wasm32"))]
                                     unsafe fn wit_import(_n: i32) {{ unreachable!() }}

                                     #[cfg(target_arch = "wasm32")]
                                     #[link(wasm_import_module = "{wasm_import_module}")]
                                     extern "C" {{
                                         #[link_name = "[resource-drop]{name}"]
                                         fn wit_import(_: i32);
                                     }}

                                     wit_import(self.handle)
                                 }}
                             }}
                        }}

                        impl {camel} {{
                            #[doc(hidden)]
                            pub unsafe fn from_handle(handle: i32) -> Self {{
                                Self {{ handle }}
                            }}

                            #[doc(hidden)]
                            pub fn into_handle(self) -> i32 {{
                                ::core::mem::ManuallyDrop::new(self).handle
                            }}
                    "#
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
        if self.return_pointer_area_align > 0 {
            uwrite!(
                self.src,
                "
                    #[allow(unused_imports)]
                    use {rt}::{{alloc, vec::Vec, string::String}};

                    #[repr(align({align}))]
                    struct _RetArea([u8; {size}]);
                    static mut _RET_AREA: _RetArea = _RetArea([0; {size}]);
                ",
                rt = self.gen.runtime_path(),
                align = self.return_pointer_area_align,
                size = self.return_pointer_area_size,
            );
        }

        mem::take(&mut self.src).into()
    }

    fn finish_resource_export(&mut self, id: TypeId, interface_name: &WorldKey) {
        self.gen.resources.entry(id).or_default();
        let info = &self.gen.resources[&id];
        let name = self.resolve.types[id].name.as_deref().unwrap();
        let camel = name.to_upper_camel_case();
        let snake = to_rust_ident(name);
        let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        let module = match &self.resolve.types[id].owner {
            TypeOwner::Interface(_) => self.resolve.name_world_key(interface_name),
            TypeOwner::World(_) | TypeOwner::None => unreachable!(),
        };
        let rt = self.gen.runtime_path();

        uwriteln!(
            self.src,
            r#"
                const _: () = {{
                    #[doc(hidden)]
                    #[export_name = "{export_prefix}{module}#[dtor]{name}"]
                    #[allow(non_snake_case)]
                    unsafe extern "C" fn __export_dtor_{snake}(arg0: i32) {{
                        #[allow(unused_imports)]
                        use {rt}::boxed::Box;

                        drop(Box::from_raw(::core::mem::transmute::<isize, *mut {camel}>(
                            arg0.try_into().unwrap(),
                        )))
                    }}
                }};
            "#
        );

        if info.owned {
            uwriteln!(
                self.src,
                r#"
                    #[derive(Debug)]
                    pub struct Own{camel} {{
                        handle: i32,
                    }}

                    impl Own{camel} {{
                        #[doc(hidden)]
                        pub unsafe fn from_handle(handle: i32) -> Self {{
                            Self {{ handle }}
                        }}

                        #[doc(hidden)]
                        pub fn into_handle(self) -> i32 {{
                            ::core::mem::ManuallyDrop::new(self).handle
                        }}

                        pub fn new(rep: {camel}) -> Own{camel} {{
                            #[allow(unused_imports)]
                            use {rt}::boxed::Box;

                            unsafe {{
                                #[cfg(target_arch = "wasm32")]
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-new]{name}"]
                                    fn wit_import(_: i32) -> i32;
                                }}

                                #[cfg(not(target_arch = "wasm32"))]
                                unsafe fn wit_import(_n: i32) -> i32 {{ unreachable!() }}

                                Own{camel} {{
                                    handle: wit_import(
                                        ::core::mem::transmute::<*mut {camel}, isize>(
                                            Box::into_raw(Box::new(rep))
                                        )
                                            .try_into()
                                            .unwrap(),
                                    ),
                                }}
                            }}
                        }}
                    }}

                    impl core::ops::Deref for Own{camel} {{
                        type Target = {camel};

                        fn deref(&self) -> &{camel} {{
                            unsafe {{
                                #[cfg(target_arch = "wasm32")]
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-rep]{name}"]
                                    fn wit_import(_: i32) -> i32;
                                }}

                                #[cfg(not(target_arch = "wasm32"))]
                                unsafe fn wit_import(_n: i32) -> i32 {{ unreachable!() }}

                                ::core::mem::transmute::<isize, &{camel}>(
                                    wit_import(self.handle).try_into().unwrap()
                                )
                            }}
                        }}
                    }}

                    impl core::ops::DerefMut for Own{camel} {{
                        fn deref_mut(&mut self) -> &mut {camel} {{
                            unsafe {{
                                #[cfg(target_arch = "wasm32")]
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-rep]{name}"]
                                    fn wit_import(_: i32) -> i32;
                                }}

                                #[cfg(not(target_arch = "wasm32"))]
                                unsafe fn wit_import(_n: i32) -> i32 {{ unreachable!() }}

                                ::core::mem::transmute::<isize, &mut {camel}>(
                                    wit_import(self.handle).try_into().unwrap()
                                )
                            }}
                        }}
                    }}

                    impl Drop for Own{camel} {{
                        fn drop(&mut self) {{
                            unsafe {{
                                #[cfg(target_arch = "wasm32")]
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-drop]{name}"]
                                    fn wit_import(_: i32);
                                }}

                                #[cfg(not(target_arch = "wasm32"))]
                                fn wit_import(_n: i32) {{ unreachable!() }}

                                wit_import(self.handle)
                            }}
                        }}
                    }}
                "#
            );
        }
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
        path_to_root: &str,
        pkg: Option<PackageName>,
    ) {
        let module = self.finish();
        let module = format!(
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[used]
                    #[doc(hidden)]
                    #[cfg(target_arch = \"wasm32\")]
                    static __FORCE_SECTION_REF: fn() = {path_to_root}__link_section;
                    {module}
                }}
            ",
        );
        let map = if self.in_import {
            &mut self.gen.import_modules
        } else {
            &mut self.gen.export_modules
        };
        map.entry(pkg).or_insert(Vec::new()).push(module);
    }

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
                if let FunctionKind::Method(_) = &func.kind {
                    sig.self_arg = Some("&self".into());
                    sig.self_is_first_param = true;
                }
            }
        }
        self.src.push_str("#[allow(clippy::all)]\n");
        let params = self.print_signature(func, param_mode, &sig);
        self.src.push_str("{\n");
        self.src.push_str(&format!(
            "
                #[allow(unused_imports)]
                use {rt}::{{alloc, vec::Vec, string::String}};
            ",
            rt = self.gen.runtime_path()
        ));
        self.src.push_str("unsafe {\n");

        let mut f = FunctionBindgen::new(self, params);
        abi::call(
            f.gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_cleanup_list,
            src,
            import_return_pointer_area_size,
            import_return_pointer_area_align,
            ..
        } = f;

        if needs_cleanup_list {
            self.src.push_str("let mut cleanup_list = Vec::new();\n");
        }
        if import_return_pointer_area_size > 0 {
            uwrite!(
                self.src,
                "
                    #[repr(align({import_return_pointer_area_align}))]
                    struct RetArea([u8; {import_return_pointer_area_size}]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                ",
            );
        }
        self.src.push_str(&String::from(src));

        self.src.push_str("}\n");
        self.src.push_str("}\n");
    }

    fn generate_guest_export(&mut self, func: &Function, interface_name: Option<&WorldKey>) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let name_snake = func.name.to_snake_case().replace('.', "_");
        let wasm_module_export_name = interface_name.map(|k| self.resolve.name_world_key(k));
        let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        let export_name = func.core_export_name(wasm_module_export_name.as_deref());
        uwrite!(
            self.src,
            "
                #[doc(hidden)]
                #[export_name = \"{export_prefix}{export_name}\"]
                #[allow(non_snake_case)]
                unsafe extern \"C\" fn __export_{name_snake}(\
            ",
        );

        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            uwrite!(self.src, "{name}: {},", wasm_type(*param));
            params.push(name);
        }
        self.src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                uwrite!(self.src, " -> {}", wasm_type(sig.results[0]));
            }
            _ => unimplemented!(),
        }

        self.push_str(" {");

        uwrite!(
            self.src,
            "
                #[allow(unused_imports)]
                use {rt}::{{alloc, vec::Vec, string::String}};

                // Before executing any other code, use this function to run all static
                // constructors, if they have not yet been run. This is a hack required
                // to work around wasi-libc ctors calling import functions to initialize
                // the environment.
                //
                // This functionality will be removed once rust 1.69.0 is stable, at which
                // point wasi-libc will no longer have this behavior.
                //
                // See
                // https://github.com/bytecodealliance/preview2-prototyping/issues/99
                // for more details.
                #[cfg(target_arch=\"wasm32\")]
                {rt}::run_ctors_once();

            ",
            rt = self.gen.runtime_path()
        );

        let mut f = FunctionBindgen::new(self, params);
        abi::call(
            f.gen.resolve,
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

        if abi::guest_export_needs_post_return(self.resolve, func) {
            let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
            uwrite!(
                self.src,
                "
                    const _: () = {{
                    #[doc(hidden)]
                    #[export_name = \"{export_prefix}cabi_post_{export_name}\"]
                    #[allow(non_snake_case)]
                    unsafe extern \"C\" fn __post_return_{name_snake}(\
                "
            );
            let mut params = Vec::new();
            for (i, result) in sig.results.iter().enumerate() {
                let name = format!("arg{}", i);
                uwrite!(self.src, "{name}: {},", wasm_type(*result));
                params.push(name);
            }
            self.src.push_str(") {\n");

            let mut f = FunctionBindgen::new(self, params);
            abi::post_return(f.gen.resolve, func, &mut f);
            let FunctionBindgen {
                needs_cleanup_list,
                src,
                ..
            } = f;
            assert!(!needs_cleanup_list);
            self.src.push_str(&String::from(src));
            self.src.push_str("}\n");
            self.src.push_str("};\n");
        }
    }

    fn generate_stub(
        &mut self,
        resource: Option<TypeId>,
        pkg: Option<&PackageName>,
        name: &str,
        in_interface: bool,
        funcs: &[&Function],
    ) {
        let path = if let Some(pkg) = pkg {
            format!(
                "{}::{}::{}",
                to_rust_ident(&pkg.namespace),
                to_rust_ident(&pkg.name),
                to_rust_ident(name),
            )
        } else {
            to_rust_ident(name)
        };

        let name = resource
            .map(|ty| {
                format!(
                    "Guest{}",
                    self.resolve.types[ty]
                        .name
                        .as_deref()
                        .unwrap()
                        .to_upper_camel_case()
                )
            })
            .unwrap_or_else(|| "Guest".to_string());

        let qualified_name = if in_interface {
            format!("exports::{path}::{name}")
        } else {
            name
        };

        uwriteln!(self.src, "impl {qualified_name} for Stub {{");

        for &func in funcs {
            if self.gen.skip.contains(&func.name) {
                continue;
            }
            let mut sig = FnSig::default();
            sig.use_item_name = true;
            sig.private = true;
            if let FunctionKind::Method(_) = &func.kind {
                sig.self_arg = Some("&self".into());
                sig.self_is_first_param = true;
            }
            self.print_signature(func, TypeMode::Owned, &sig);
            self.src.push_str("{ unreachable!() }\n");
        }

        self.src.push_str("}\n");
    }
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
                path.push_str("super::");
            }
            match name {
                WorldKey::Name(_) => {
                    path.push_str("super::");
                }
                WorldKey::Interface(_) => {
                    path.push_str("super::super::super::");
                }
            }
        }
        let name = &self.gen.interface_names[&interface];
        path.push_str(&name);
        Some(path)
    }

    fn std_feature(&self) -> bool {
        self.gen.opts.std_feature
    }

    fn use_raw_strings(&self) -> bool {
        self.gen.opts.raw_strings
    }

    fn push_vec_name(&mut self) {
        self.push_str(&format!("{rt}::vec::Vec", rt = self.gen.runtime_path()));
    }

    fn is_exported_resource(&self, mut ty: TypeId) -> bool {
        loop {
            let def = &self.resolve.types[ty];
            if let TypeOwner::World(_) = &def.owner {
                // Worlds cannot export types of any kind as of this writing.
                return false;
            }
            match &def.kind {
                TypeDefKind::Type(Type::Id(id)) => ty = *id,
                _ => break,
            }
        }

        matches!(
            self.gen.resources.get(&ty).map(|info| info.direction),
            Some(Direction::Export)
        )
    }

    fn mark_resource_owned(&mut self, resource: TypeId) {
        self.gen
            .resources
            .entry(dealias(self.resolve, resource))
            .or_default()
            .owned = true;
    }

    fn push_string_name(&mut self) {
        self.push_str(&format!(
            "{rt}::string::String",
            rt = self.gen.runtime_path()
        ));
    }

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

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if self.gen.opts.raw_strings {
            self.push_str("[u8]");
        } else {
            self.push_str("str");
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &Record, docs: &Docs) {
        self.print_typedef_record(id, record, docs, false);
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

    fn type_tuple(&mut self, id: TypeId, _name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_typedef_tuple(id, tuple, docs);
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.src.push_str(&format!(
            "{bitflags}::bitflags! {{\n",
            bitflags = self.gen.bitflags_path()
        ));
        self.rustdoc(docs);
        let repr = RustFlagsRepr::new(flags);
        self.src.push_str(&format!(
            "#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]\npub struct {}: {repr} {{\n",
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
    }

    fn type_variant(&mut self, id: TypeId, _name: &str, variant: &Variant, docs: &Docs) {
        self.print_typedef_variant(id, variant, docs, false);
    }

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        self.print_typedef_option(id, payload, docs);
    }

    fn type_result(&mut self, id: TypeId, _name: &str, result: &Result_, docs: &Docs) {
        self.print_typedef_result(id, result, docs);
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs, &[], Box::new(|_| String::new()));

        let name = to_upper_camel_case(name);
        let mut cases = String::new();
        let repr = int_repr(enum_.tag());
        for (i, case) in enum_.cases.iter().enumerate() {
            let case = case.name.to_upper_camel_case();
            cases.push_str(&format!("{i} => {name}::{case},\n"));
        }
        uwriteln!(
            self.src,
            r#"
                impl {name} {{
                    pub(crate) unsafe fn _lift(val: {repr}) -> {name} {{
                        if !cfg!(debug_assertions) {{
                            return ::core::mem::transmute(val);
                        }}

                        match val {{
                            {cases}
                            _ => panic!("invalid enum discriminant"),
                        }}
                    }}
                }}
            "#
        );
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(id, ty, docs);
    }

    fn type_list(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_type_list(id, ty, docs);
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_upper_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(ty, TypeMode::Owned);
        self.src.push_str(";\n");
    }
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    params: Vec<String>,
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
    fn new(gen: &'b mut InterfaceGenerator<'a>, params: Vec<String>) -> FunctionBindgen<'a, 'b> {
        FunctionBindgen {
            gen,
            params,
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
        module_name: &str,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        // Define the actual function we're calling inline
        let mut sig = "(".to_owned();
        for param in params.iter() {
            sig.push_str("_: ");
            sig.push_str(wasm_type(*param));
            sig.push_str(", ");
        }
        sig.push_str(")");
        assert!(results.len() < 2);
        for result in results.iter() {
            sig.push_str(" -> ");
            sig.push_str(wasm_type(*result));
        }
        uwrite!(
            self.src,
            "
                #[cfg(target_arch = \"wasm32\")]
                #[link(wasm_import_module = \"{module_name}\")]
                extern \"C\" {{
                    #[link_name = \"{name}\"]
                    fn wit_import{sig};
                }}

                #[cfg(not(target_arch = \"wasm32\"))]
                fn wit_import{sig} {{ unreachable!() }}
            "
        );
        "wit_import".to_string()
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
            self.blocks.push(format!("{{\n{}}}", &src[..]));
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
            uwrite!(self.src, "let ptr{tmp} = ret_area.as_mut_ptr() as i32;");
        } else {
            self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
            self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
            uwriteln!(self.src, "let ptr{tmp} = _RET_AREA.0.as_mut_ptr() as i32;");
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
                results.push(format!(
                    "{rt}::as_i64({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }
            Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_i32({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }

            Instruction::F32FromFloat32 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_f32({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }
            Instruction::F64FromFloat64 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_f64({s})",
                    rt = self.gen.gen.runtime_path()
                ));
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
                results.push(format!(
                    "{}::char_lift({} as u32)",
                    self.gen.gen.runtime_path(),
                    operands[0]
                ));
            }

            Instruction::Bitcasts { casts } => {
                wit_bindgen_rust_lib::bitcast(casts, operands, results)
            }

            Instruction::I32FromBool => {
                results.push(format!("match {} {{ true => 1, false => 0 }}", operands[0]));
            }
            Instruction::BoolFromI32 => {
                results.push(format!(
                    "{}::bool_lift({} as u8)",
                    self.gen.gen.runtime_path(),
                    operands[0]
                ));
            }

            Instruction::FlagsLower { flags, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
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
                results.push(format!("({op}).handle"))
            }

            Instruction::HandleLift { handle, .. } => {
                let op = &operands[0];
                let (prefix, resource) = match handle {
                    Handle::Borrow(resource) => ("&", resource),
                    Handle::Own(resource) => ("", resource),
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
                                    "::core::mem::transmute::<isize, &{name}>\
                                     ({op}.try_into().unwrap())"
                                )
                            }
                            Handle::Own(_) => {
                                let name = self.gen.type_path(resource, true);
                                format!("{name}::from_handle({op})")
                            }
                        }
                    } else {
                        let name = self.gen.type_path(resource, true);
                        format!("{prefix}{name}::from_handle({op})")
                    },
                );
            }

            Instruction::RecordLower { ty, record, .. } => {
                self.record_lower(*ty, record, &operands[0], results);
            }
            Instruction::RecordLift { ty, record, .. } => {
                self.record_lift(*ty, record, operands, results);
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
                let name = self.typename_lower(*ty);
                let name = if name.contains("::") {
                    let tmp = self.tmp();
                    uwriteln!(self.src, "use {name} as V{tmp};");
                    format!("V{tmp}")
                } else {
                    name
                };
                self.let_results(result_types.len(), results);
                let op0 = &operands[0];
                self.push_str(&format!("match {op0} {{\n"));
                for (case, block) in variant.cases.iter().zip(blocks) {
                    let case_name = case.name.to_upper_camel_case();
                    self.push_str(&format!("{name}::{case_name}"));
                    if case.ty.is_some() {
                        self.push_str(&format!("(e) => {block},\n"));
                    } else {
                        self.push_str(&format!(" => {{\n{block}\n}}\n"));
                    }
                }
                if results.len() == 0 {
                    self.push_str("}\n");
                } else {
                    self.push_str("};\n");
                }
            }

            Instruction::VariantLift { variant, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                let tmp = self.tmp();
                let name = self.typename_lift(*ty);
                let name = if name.contains("::") {
                    uwriteln!(self.src, "use {name} as V{tmp};");
                    format!("V{tmp}")
                } else {
                    name
                };
                uwriteln!(self.src, "let v{tmp} = match {op0} {{");
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    if i == variant.cases.len() - 1 {
                        uwriteln!(
                            self.src,
                            "n => {{
                                debug_assert_eq!(n, {i}, \"invalid enum discriminant\");\
                            "
                        );
                    } else {
                        uwriteln!(self.src, "{i} => {{");
                    }
                    let case_name = case.name.to_upper_camel_case();
                    if case.ty.is_none() {
                        uwriteln!(self.src, "{name}::{case_name}");
                    } else {
                        uwriteln!(self.src, "let e{tmp} = {block};");
                        uwriteln!(self.src, "{name}::{case_name}(e{tmp})");
                    }
                    uwriteln!(self.src, "}}");
                }
                uwriteln!(self.src, "}};");
                results.push(format!("v{tmp}"));
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
                results.push(format!(
                    "match {operand} {{
                        0 => None,
                        1 => {{
                            let e = {some};
                            Some(e)
                        }}
                        _ => {rt}::invalid_enum_discriminant(),
                    }}",
                    rt = self.gen.gen.runtime_path(),
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
                results.push(format!(
                    "match {operand} {{
                        0 => {{
                            let e = {ok};
                            Ok(e)
                        }}
                        1 => {{
                            let e = {err};
                            Err(e)
                        }}
                        _ => {rt}::invalid_enum_discriminant(),
                    }}",
                    rt = self.gen.gen.runtime_path(),
                ));
            }

            Instruction::EnumLower { .. } => {
                results.push(format!("{}.clone() as i32", operands[0]));
            }

            Instruction::EnumLift { enum_, ty, .. } => {
                let name = self.gen.type_path(*ty, true);
                let repr = int_repr(enum_.tag());
                let op = &operands[0];
                let result = format!("{name}::_lift({op} as {repr})");
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
                    self.push_str(&format!("::core::mem::forget({});\n", val));
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
                    self.push_str(&format!("::core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                uwriteln!(self.src, "let {len} = {} as usize;", operands[1]);
                uwriteln!(
                    self.src,
                    "let bytes{tmp} = Vec::from_raw_parts({} as *mut _, {len}, {len});",
                    operands[0],
                );
                if self.gen.gen.opts.raw_strings {
                    results.push(format!("bytes{tmp}"));
                } else {
                    results.push(format!(
                        "{}::string_lift(bytes{tmp})",
                        self.gen.gen.runtime_path()
                    ));
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
                self.push_str(&format!("else {{\n::core::ptr::null_mut()\n}};\n",));
                self.push_str(&format!("for (i, e) in {vec}.into_iter().enumerate() {{\n",));
                self.push_str(&format!(
                    "let base = {result} as i32 + (i as i32) * {size};\n",
                ));
                self.push_str(&body);
                self.push_str("\n}\n");
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

                uwriteln!(self.src, "for i in 0..{len} {{");
                uwriteln!(self.src, "let base = {base} + i * {size};");
                uwriteln!(self.src, "let e{tmp} = {body};");
                uwriteln!(self.src, "{result}.push(e{tmp});");
                uwriteln!(self.src, "}}");
                results.push(result);
                self.push_str(&format!(
                    "{rt}::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
                ));
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
                    self.push_str("let ret = ");
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
                            "<_GuestImpl as Guest>::{}",
                            to_rust_ident(&func.name)
                        ));
                    }
                    FunctionKind::Method(ty) | FunctionKind::Static(ty) => {
                        self.push_str(&format!(
                            "<{0} as Guest{0}>::{1}",
                            resolve.types[*ty]
                                .name
                                .as_deref()
                                .unwrap()
                                .to_upper_camel_case(),
                            to_rust_ident(func.item_name())
                        ));
                    }
                    FunctionKind::Constructor(ty) => {
                        self.gen.mark_resource_owned(*ty);
                        self.push_str(&format!(
                            "Own{0}::new(<{0} as Guest{0}>::new",
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
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = *(({} + {offset}) as *const i32);",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::I32Load8U { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = i32::from(*(({} + {offset}) as *const u8));",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::I32Load8S { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = i32::from(*(({} + {offset}) as *const i8));",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::I32Load16U { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = i32::from(*(({} + {offset}) as *const u16));",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::I32Load16S { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = i32::from(*(({} + {offset}) as *const i16));",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::I64Load { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = *(({} + {offset}) as *const i64);",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::F32Load { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = *(({} + {offset}) as *const f32);",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
            }
            Instruction::F64Load { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "let l{tmp} = *(({} + {offset}) as *const f64);",
                    operands[0]
                );
                results.push(format!("l{tmp}"));
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
                    "{rt}::dealloc({op}, {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
                    op = operands[0]
                ));
            }

            Instruction::GuestDeallocateString => {
                self.push_str(&format!(
                    "{rt}::dealloc({op0}, ({op1}) as usize, 1);\n",
                    rt = self.gen.gen.runtime_path(),
                    op0 = operands[0],
                    op1 = operands[1],
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
                    "{rt}::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
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
