use anyhow::Result;
use core::panic;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{collections::HashMap, fmt::Write, mem, ops::Deref};
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmSignature, WasmType},
    dealias, uwrite, uwriteln,
    wit_parser::{
        Alignment, ArchitectureSize, Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Handle,
        Int, InterfaceId, Record, Resolve, Result_, SizeAlign, Tuple, Type, TypeDef, TypeDefKind,
        TypeId, TypeOwner, Variant, WorldId, WorldKey,
    },
    AsyncFilterSet, Direction, Files, InterfaceGenerator as _, Ns, Source, WorldGenerator,
};

// Assumptions:
// - Data: u8 -> Byte, s8 | s16 | s32 -> Int, u16 | u32 -> UInt, s64 -> Int64, u64 -> UInt64, f32 | f64 -> Double, address -> Int
// - Encoding: UTF16
// - Lift/Lower list<T>: T == Int/UInt/Int64/UInt64/Float/Double -> FixedArray[T], T == Byte -> Bytes, T == Char -> String
// Organization:
// - one package per interface (export and import are treated as different interfaces)
// - ffi utils are under `./ffi`, and the project entrance (package as link target) is under `./gen`
// TODO: Export will share the type signatures with the import by using a newtype alias
pub(crate) const FFI_DIR: &str = "ffi";

pub(crate) const FFI: &str = include_str!("ffi.mbt");

pub(crate) const ASYNC_PRIMITIVE: &str = include_str!("./async-wasm/async_primitive.mbt");
pub(crate) const ASYNC_FUTURE: &str = include_str!("./async-wasm/future.mbt");
pub(crate) const ASYNC_WASM_PRIMITIVE: &str = include_str!("./async-wasm/wasm_primitive.mbt");
pub(crate) const ASYNC_WAITABLE_SET: &str = include_str!("./async-wasm/waitable_task.mbt");
pub(crate) const ASYNC_SUBTASK: &str = include_str!("./async-wasm/subtask.mbt");

pub(crate) const ASYNC_UTILS: [&str; 5] = [
    ASYNC_PRIMITIVE,
    ASYNC_FUTURE,
    ASYNC_WASM_PRIMITIVE,
    ASYNC_WAITABLE_SET,
    ASYNC_SUBTASK,
];

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Whether or not to derive Show for all types
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_show: bool,

    /// Whether or not to derive Eq for all types
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_eq: bool,

    /// Whether or not to declare as Error type for types ".*error"
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_error: bool,

    /// Whether or not to generate stub files ; useful for update after WIT change
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub ignore_stub: bool,

    /// Whether or not to generate moon.mod.json ; useful if the project is part of a larger project
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub ignore_module_file: bool,

    /// The package/dir to generate the program entrance
    #[cfg_attr(feature = "clap", arg(long, default_value = "gen"))]
    pub gen_dir: String,

    /// The project name ; or the package path prefix if the project is part of a larger project
    #[cfg_attr(feature = "clap", arg(long, default_value = None))]
    pub project_name: Option<String>,

    #[cfg_attr(feature = "clap", clap(flatten))]
    pub async_: AsyncFilterSet,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(MoonBit {
            opts: self.clone(),
            ..MoonBit::default()
        })
    }
}

struct MoonbitSignature {
    name: String,
    params: Vec<(String, Type)>,
    result_type: String,
}

struct InterfaceFragment {
    src: String,
    ffi: String,
    stub: String,
}

enum PayloadFor {
    Future,
    Stream,
}

#[derive(Default)]
struct Imports {
    packages: HashMap<String, String>,
    ns: Ns,
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
    interface_ns: Ns,
    // dependencies between packages
    package_import: HashMap<String, Imports>,
    export: HashMap<String, String>,
    export_ns: Ns,
    // return area allocation
    return_area_size: ArchitectureSize,
    return_area_align: Alignment,
    futures: Vec<TypeId>,
    is_async: bool,
}

impl MoonBit {
    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a str,
        module: &'a str,
        direction: Direction,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            stub: String::new(),
            ffi: String::new(),
            gen: self,
            resolve,
            name,
            module,
            direction,
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
        files: &mut Files,
    ) -> Result<()> {
        let name = interface_name(resolve, key);
        let name = self.interface_ns.tmp(&name);
        self.import_interface_names.insert(id, name.clone());

        if let Some(content) = &resolve.interfaces[id].docs.contents {
            if !content.is_empty() {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
        }

        let module = &resolve.name_world_key(key);
        let mut gen = self.interface(resolve, &name, module, Direction::Import);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(Some(key), func);
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
        let mut gen = self.interface(resolve, &name, "$root", Direction::Import);

        for (_, func) in funcs {
            gen.import(None, func); // None is "$root"
        }

        gen.add_world_fragment();
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        files: &mut Files,
    ) -> Result<()> {
        let name = format!("{}.{}", self.opts.gen_dir, interface_name(resolve, key));
        let name = self.interface_ns.tmp(&name);
        self.export_interface_names.insert(id, name.clone());

        if let Some(content) = &resolve.interfaces[id].docs.contents {
            if !content.is_empty() {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
        }

        let module = &resolve.name_world_key(key);
        let mut gen = self.interface(resolve, &name, module, Direction::Export);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(Some(key), func, Some(name.clone()));
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
        let name = format!("{}.{}", self.opts.gen_dir, world_name(resolve, world));
        let mut gen = self.interface(resolve, &name, "$root", Direction::Export);

        for (_, func) in funcs {
            gen.export(None, func, Some(name.clone()));
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
        let mut gen = self.interface(resolve, &name, "$root", Direction::Import);

        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        let project_name = self
            .opts
            .project_name
            .clone()
            .or(resolve.worlds[id].package.map(|id| {
                let package = &resolve.packages[id].name;
                format!("{}/{}", package.namespace, package.name)
            }))
            .unwrap_or("generated".into());
        let name = world_name(resolve, id);

        if let Some(content) = &resolve.worlds[id].docs.contents {
            if !content.is_empty() {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
        }

        let version = env!("CARGO_PKG_VERSION");

        let generate_pkg_definition = |name: &String, files: &mut Files| {
            let directory = name.replace('.', "/");
            let imports: Option<&Imports> = self.package_import.get(name);
            if let Some(imports) = imports {
                let mut deps = imports
                    .packages
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{{ \"path\" : \"{project_name}/{}\", \"alias\" : \"{}\" }}",
                            k.replace(".", "/"),
                            v
                        )
                    })
                    .collect::<Vec<_>>();
                deps.sort();

                files.push(
                    &format!("{directory}/moon.pkg.json"),
                    format!("{{ \"import\": [{}] }}", deps.join(", ")).as_bytes(),
                );
            } else {
                files.push(
                    &format!("{directory}/moon.pkg.json"),
                    format!("{{ }}").as_bytes(),
                );
            }
        };

        // Import world fragments
        let mut src = Source::default();
        let mut ffi = Source::default();
        wit_bindgen_core::generated_preamble(&mut src, version);
        wit_bindgen_core::generated_preamble(&mut ffi, version);
        self.import_world_fragments.iter().for_each(|f| {
            uwriteln!(src, "{}", f.src);
            uwriteln!(ffi, "{}", f.ffi);
            assert!(f.stub.is_empty());
        });

        let directory = name.replace('.', "/");
        files.push(&format!("{directory}/import.mbt"), indent(&src).as_bytes());
        files.push(
            &format!("{directory}/ffi_import.mbt"),
            indent(&ffi).as_bytes(),
        );
        generate_pkg_definition(&name, files);

        // Export world fragments
        let mut src = Source::default();
        let mut stub = Source::default();
        wit_bindgen_core::generated_preamble(&mut src, version);
        generated_preamble(&mut stub, version);
        self.export_world_fragments.iter().for_each(|f| {
            uwriteln!(src, "{}", f.src);
            uwriteln!(stub, "{}", f.stub);
        });

        files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());
        if !self.opts.ignore_stub {
            files.push(
                &format!("{}/{directory}/stub.mbt", self.opts.gen_dir),
                indent(&stub).as_bytes(),
            );
            generate_pkg_definition(&format!("{}.{}", self.opts.gen_dir, name), files);
        }

        let generate_ffi =
            |directory: String, fragments: &[InterfaceFragment], files: &mut Files| {
                let b = fragments
                    .iter()
                    .map(|f| f.ffi.deref())
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut body = Source::default();
                wit_bindgen_core::generated_preamble(&mut body, version);
                uwriteln!(&mut body, "{b}");

                files.push(
                    &format!(
                        "{}/{}_export.mbt",
                        self.opts.gen_dir,
                        directory.to_snake_case()
                    ),
                    indent(&body).as_bytes(),
                );
            };

        generate_ffi(directory, &self.export_world_fragments, files);

        // Import interface fragments
        for (name, fragments) in &self.import_interface_fragments {
            let mut src = Source::default();
            let mut ffi = Source::default();
            wit_bindgen_core::generated_preamble(&mut src, version);
            wit_bindgen_core::generated_preamble(&mut ffi, version);
            fragments.iter().for_each(|f| {
                uwriteln!(src, "{}", f.src);
                uwriteln!(ffi, "{}", f.ffi);
                assert!(f.stub.is_empty());
            });

            let directory = name.replace('.', "/");
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());
            files.push(&format!("{directory}/ffi.mbt"), indent(&ffi).as_bytes());
            generate_pkg_definition(&name, files);
        }

        // Export interface fragments
        for (name, fragments) in &self.export_interface_fragments {
            let mut src = Source::default();
            let mut stub = Source::default();
            wit_bindgen_core::generated_preamble(&mut src, version);
            generated_preamble(&mut stub, version);
            fragments.iter().for_each(|f| {
                uwriteln!(src, "{}", f.src);
                uwriteln!(stub, "{}", f.stub);
            });

            let directory = name.replace('.', "/");
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());
            if !self.opts.ignore_stub {
                files.push(&format!("{directory}/stub.mbt"), indent(&stub).as_bytes());
                generate_pkg_definition(&name, files);
            }
            generate_ffi(directory, fragments, files);
        }

        // Export FFI Utils
        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, version);
        body.push_str(FFI);

        // Export Async utils
        // If async is used, export async utils
        if self.is_async || self.futures.len() > 0 {
            ASYNC_UTILS.iter().for_each(|s| {
                body.push_str("\n");
                body.push_str(s);
            });
        }

        files.push(&format!("{FFI_DIR}/top.mbt"), indent(&body).as_bytes());
        files.push(
            &format!("{FFI_DIR}/moon.pkg.json"),
            "{ \"warn-list\": \"-44\", \"supported-targets\": [\"wasm\"] }".as_bytes(),
        );

        // Export project files
        if !self.opts.ignore_stub && !self.opts.ignore_module_file {
            let mut body = Source::default();
            uwriteln!(
                &mut body,
                "{{ \"name\": \"{project_name}\", \"preferred-target\": \"wasm\" }}"
            );
            files.push(&format!("moon.mod.json"), body.as_bytes());
        }

        let export_dir = self.opts.gen_dir.clone();

        // Export project entry point
        let mut gen = self.interface(resolve, &export_dir.as_str(), "", Direction::Export);
        let ffi_qualifier = gen.qualify_package(FFI_DIR);

        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, version);
        uwriteln!(
            &mut body,
            "
            pub fn cabi_realloc(
                src_offset : Int,
                src_size : Int,
                dst_alignment : Int,
                dst_size : Int
            ) -> Int {{
                {ffi_qualifier}cabi_realloc(src_offset, src_size, dst_alignment, dst_size)
            }}
            "
        );
        if !self.return_area_size.is_empty() {
            uwriteln!(
                &mut body,
                "
                let return_area : Int = {ffi_qualifier}malloc({})
                ",
                self.return_area_size.size_wasm32(),
            );
        }
        files.push(
            &format!("{}/ffi.mbt", self.opts.gen_dir),
            indent(&body).as_bytes(),
        );
        self.export
            .insert("cabi_realloc".into(), "cabi_realloc".into());

        let mut body = Source::default();
        let mut exports = self
            .export
            .iter()
            .map(|(k, v)| format!("\"{k}:{v}\""))
            .collect::<Vec<_>>();
        exports.sort();

        uwrite!(
            &mut body,
            r#"
            {{
                "link": {{
                    "wasm": {{
                        "exports": [{}],
                        "export-memory-name": "memory",
                        "heap-start-address": 16
                    }}
                }}
            "#,
            exports.join(", ")
        );
        if let Some(imports) = self.package_import.get(&self.opts.gen_dir) {
            let mut deps = imports
                .packages
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{{ \"path\" : \"{project_name}/{}\", \"alias\" : \"{}\" }}",
                        k.replace(".", "/"),
                        v
                    )
                })
                .collect::<Vec<_>>();
            deps.sort();

            uwrite!(&mut body, "    ,\"import\": [{}]", deps.join(", "));
        }
        uwrite!(
            &mut body,
            "
            }}
            ",
        );
        files.push(
            &format!("{}/moon.pkg.json", self.opts.gen_dir,),
            indent(&body).as_bytes(),
        );

        Ok(())
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    stub: String,
    ffi: String,
    gen: &'a mut MoonBit,
    resolve: &'a Resolve,
    // The current interface getting generated
    name: &'a str,
    module: &'a str,
    direction: Direction,
}

impl InterfaceGenerator<'_> {
    fn qualify_package(&mut self, name: &str) -> String {
        if name != self.name {
            let imports = self
                .gen
                .package_import
                .entry(self.name.to_string())
                .or_default();
            if let Some(alias) = imports.packages.get(name) {
                return format!("@{}.", alias);
            } else {
                let alias = imports
                    .ns
                    .tmp(&name.split(".").last().unwrap().to_lower_camel_case());
                imports
                    .packages
                    .entry(name.to_string())
                    .or_insert(alias.clone());
                return format!("@{}.", alias);
            }
        } else {
            "".into()
        }
    }
    fn qualifier(&mut self, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.gen.export_interface_names.get(id) {
                if name != self.name {
                    return self.qualify_package(&name.clone());
                }
            } else if let Some(name) = self.gen.import_interface_names.get(id) {
                if name != self.name {
                    return self.qualify_package(&name.clone());
                }
            }
        } else if let TypeOwner::World(id) = &ty.owner {
            let name = world_name(self.resolve, *id);
            if name != self.name {
                return self.qualify_package(&name.clone());
            }
        }

        String::new()
    }

    fn add_interface_fragment(self) {
        match self.direction {
            Direction::Import => {
                self.gen
                    .import_interface_fragments
                    .entry(self.name.to_owned())
                    .or_default()
                    .push(InterfaceFragment {
                        src: self.src,
                        stub: self.stub,
                        ffi: self.ffi,
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
                        ffi: self.ffi,
                    });
            }
        }
    }

    fn add_world_fragment(self) {
        match self.direction {
            Direction::Import => {
                self.gen.import_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                    ffi: self.ffi,
                });
            }
            Direction::Export => {
                self.gen.export_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                    ffi: self.ffi,
                });
            }
        }
    }

    fn import(&mut self, module: Option<&WorldKey>, func: &Function) {
        let async_ = self
            .gen
            .opts
            .async_
            .is_async(self.resolve, module, func, false);
        if async_ {
            self.gen.is_async = true;
        }

        let interface_name = match module {
            Some(key) => &self.resolve.name_world_key(key),
            None => "$root",
        };
        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            self.name,
            func.params
                .iter()
                .map(|(name, _)| name.to_moonbit_ident())
                .collect(),
        );

        let (variant, async_prefix) = if async_ {
            (AbiVariant::GuestImportAsync, "[async-lower]")
        } else {
            (AbiVariant::GuestImport, "")
        };

        abi::call(
            bindgen.gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
            false,
        );

        let mut src = bindgen.src.clone();

        let cleanup_list = if bindgen.needs_cleanup_list {
            self.gen.needs_cleanup = true;

            let ffi_qualifier = self.qualify_package(FFI_DIR);

            format!(
                r#"let cleanupList : Array[{ffi_qualifier}Cleanup] = []
                   let ignoreList : Array[&{ffi_qualifier}Any] = []"#
            )
        } else {
            String::new()
        };

        let name = &func.name;

        let wasm_sig = self.resolve.wasm_signature(variant, func);

        let result_type = match &wasm_sig.results[..] {
            [] => "".into(),
            [result] => format!("-> {}", wasm_type(*result)),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = wasm_sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("p{i} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mbt_sig = self.mbt_sig(func, false);
        let sig = self.sig_string(&mbt_sig, async_);

        let module = match module {
            Some(key) => self.resolve.name_world_key(key),
            None => "$root".into(),
        };

        self.generation_futures_and_streams_import("", func, interface_name);

        uwriteln!(
            self.ffi,
            r#"fn wasmImport{camel_name}({params}) {result_type} = "{module}" "{async_prefix}{name}""#
        );

        print_docs(&mut self.src, &func.docs);

        if async_ {
            src = self.generate_async_import_function(func, mbt_sig, &wasm_sig);
        }

        uwrite!(
            self.src,
            r#"
            {sig} {{
            {cleanup_list}
            {src}
            }}
            "#
        );
    }

    fn generate_async_import_function(
        &mut self,
        func: &Function,
        mbt_sig: MoonbitSignature,
        sig: &WasmSignature,
    ) -> String {
        let mut lower_params = Vec::new();

        if sig.indirect_params {
            todo!("Unsupported indirect params");
        } else {
            let mut f = FunctionBindgen::new(self, "INVALID", self.name, Box::new([]));
            for (name, ty) in mbt_sig.params.iter() {
                lower_params.extend(abi::lower_flat(f.gen.resolve, &mut f, name.clone(), ty));
            }
        }

        let mut body = String::default();

        let func_name = func.name.to_upper_camel_case();

        let call_import = |params: &Vec<String>| {
            format!(
                r#"
                let subtask_status = @ffi.SubtaskStatus::decode(wasmImport{func_name}({}))
                match subtask_status {{
                    Returned(_) => ()
                    _ => {{
                        let task_group = @ffi.get_or_create_waitable_set()
                        let subtask = @ffi.Subtask::from_handle(subtask_status.handle())
                        task_group.wait(
                            async fn() -> Unit raise {{
                                for {{
                                    if subtask.is_done() {{
                                        break
                                    }} else {{
                                        @ffi.suspend()
                                    }}
                                }}
                            }},
                            subtask
                        )
                    }}
                }}
                "#,
                params.join(", ")
            )
        };
        match &func.result {
            Some(ty) => {
                lower_params.push("result_ptr".into());
                let call_import = call_import(&lower_params);
                let (lift, lift_result) = &self.lift_from_memory("result_ptr", ty, self.name);
                body.push_str(&format!(
                    r#"
                    {}
                    {call_import}
                    {lift}
                    {lift_result}
                    "#,
                    &self.malloc_memory("result_ptr", ty)
                ));
            }
            None => {
                let call_import = call_import(&lower_params);
                body.push_str(&call_import);
            }
        }

        body.to_string()
    }

    fn export(&mut self, interface: Option<&WorldKey>, func: &Function, _: Option<String>) {
        let async_ = self
            .gen
            .opts
            .async_
            .is_async(&self.resolve, interface, func, false);
        if async_ {
            self.gen.is_async = true;
        }

        let variant = if async_ {
            AbiVariant::GuestExportAsync
        } else {
            AbiVariant::GuestExport
        };

        let sig = self.resolve.wasm_signature(variant, func);
        let mbt_sig = self.mbt_sig(func, false);

        let func_sig = self.sig_string(&mbt_sig, async_);
        let export_dir = self.gen.opts.gen_dir.clone();

        let mut toplevel_generator = self.gen.interface(
            self.resolve,
            export_dir.as_str(),
            self.module,
            Direction::Export,
        );

        let mut bindgen = FunctionBindgen::new(
            &mut toplevel_generator,
            &func.name,
            self.name,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        );

        abi::call(
            bindgen.gen.resolve,
            variant,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
            async_,
        );

        assert!(!bindgen.needs_cleanup_list);

        // Async functions deferred task return
        let deferred_task_return = bindgen.deferred_task_return.clone();

        let src = bindgen.src;
        assert!(toplevel_generator.src.is_empty());
        assert!(toplevel_generator.ffi.is_empty());

        let result_type = match &sig.results[..] {
            [] => "Unit",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let func_name = self.gen.export_ns.tmp(&format!("wasmExport{camel_name}"));

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

        // Async export prefix for FFI
        let async_export_prefix = if async_ { "[async-lift]" } else { "" };
        // Async functions return type
        let interface_name = match interface {
            Some(key) => Some(self.resolve.name_world_key(key)),
            None => None,
        };

        let export_name = func.legacy_core_export_name(interface_name.as_deref());
        let module_name = interface_name.as_deref().unwrap_or("$root");
        self.generation_futures_and_streams_import("[export]", func, module_name);

        uwrite!(
            self.ffi,
            r#"
            pub fn {func_name}({params}) -> {result_type} {{
                {src}
            }}
            "#,
        );

        self.gen
            .export
            .insert(func_name, format!("{async_export_prefix}{export_name}"));

        if async_ {
            let snake = self.gen.name.to_lower_camel_case();
            let export_func_name = self
                .gen
                .export_ns
                .tmp(&format!("wasmExport{snake}Async{camel_name}"));
            let DeferredTaskReturn::Emitted {
                body: task_return_body,
                params: task_return_params,
                return_param,
            } = deferred_task_return
            else {
                unreachable!()
            };
            let func_name = func.name.clone();
            let import_module = self.resolve.name_world_key(interface.unwrap());
            self.gen.export.insert(
                export_func_name.clone(),
                format!("[callback]{async_export_prefix}{export_name}"),
            );
            let task_return_param_tys = task_return_params
                .iter()
                .enumerate()
                .map(|(idx, (ty, _expr))| format!("p{}: {}", idx, wasm_type(*ty)))
                .collect::<Vec<_>>()
                .join(", ");
            let task_return_param_exprs = task_return_params
                .iter()
                .map(|(_ty, expr)| expr.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let return_ty = match &func.result {
                Some(result) => {
                    format!("{}", self.type_name(result, false))
                }
                None => "Unit".into(),
            };
            let return_expr = match return_ty.as_str() {
                "Unit" => "".into(),
                _ => format!("{return_param}: {return_ty}",),
            };
            let snake_func_name = format!("{}", func.name.to_snake_case());
            let ffi = self.qualify_package(FFI_DIR);

            uwriteln!(
                self.src,
                r#"
                fn {export_func_name}TaskReturn({task_return_param_tys}) = "[export]{import_module}" "[task-return]{func_name}"
                
                pub fn {snake_func_name}_task_return({return_expr}) -> Unit {{ 
                    {task_return_body}
                    {export_func_name}TaskReturn({task_return_param_exprs})
                }}
                "#
            );

            uwriteln!(
                self.ffi,
                r#"
                pub fn {export_func_name}(event_raw: Int, waitable: Int, code: Int) -> Int {{
                    {ffi}callback(event_raw, waitable, code)
                }}
                "#
            );
        } else if abi::guest_export_needs_post_return(self.resolve, func) {
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
                self.name,
                (0..sig.results.len()).map(|i| format!("p{i}")).collect(),
            );

            abi::post_return(bindgen.gen.resolve, func, &mut bindgen);

            let src = bindgen.src;

            let func_name = self
                .gen
                .export_ns
                .tmp(&format!("wasmExport{camel_name}PostReturn"));

            uwrite!(
                self.ffi,
                r#"
                pub fn {func_name}({params}) -> Unit {{
                    {src}
                }}
                "#
            );
            self.gen
                .export
                .insert(func_name, format!("cabi_post_{export_name}"));
        }

        print_docs(&mut self.stub, &func.docs);
        uwrite!(
            self.stub,
            r#"
            {func_sig} {{
                ...
            }}
            "#
        );
    }

    fn mbt_sig(&mut self, func: &Function, ignore_param: bool) -> MoonbitSignature {
        let name = match func.kind {
            FunctionKind::Freestanding => func.name.to_moonbit_ident(),
            FunctionKind::Constructor(_) => {
                func.name.replace("[constructor]", "").to_moonbit_ident()
            }
            _ => func.name.split(".").last().unwrap().to_moonbit_ident(),
        };
        let type_name = match func.kind.resource() {
            Some(ty) => {
                format!("{}::", self.type_name(&Type::Id(ty), true))
            }
            None => "".into(),
        };

        let result_type = match &func.result {
            None => "Unit".into(),
            Some(ty) => self.type_name(ty, true),
        };
        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let name = if ignore_param {
                    format!("_{}", name.to_moonbit_ident())
                } else {
                    name.to_moonbit_ident()
                };
                (name, ty.clone())
            })
            .collect::<Vec<_>>();

        MoonbitSignature {
            name: format!("{type_name}{name}"),
            params: params,
            result_type: result_type,
        }
    }

    fn type_name(&mut self, ty: &Type, type_variable: bool) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "Byte".into(),
            Type::S32 | Type::S8 | Type::S16 => "Int".into(),
            Type::U16 | Type::U32 => "UInt".into(),
            Type::Char => "Char".into(),
            Type::U64 => "UInt64".into(),
            Type::S64 => "Int64".into(),
            Type::F32 => "Float".into(),
            Type::F64 => "Double".into(),
            Type::String => "String".into(),
            Type::ErrorContext => todo!("moonbit error context type name"),
            Type::Id(id) => {
                let ty = &self.resolve.types[dealias(self.resolve, *id)];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.type_name(ty, type_variable),
                    TypeDefKind::List(ty) => {
                        if type_variable {
                            match ty {
                                Type::U8
                                | Type::U32
                                | Type::U64
                                | Type::S32
                                | Type::S64
                                | Type::F32
                                | Type::F64 => {
                                    format!("FixedArray[{}]", self.type_name(ty, type_variable))
                                }
                                _ => format!("Array[{}]", self.type_name(ty, type_variable)),
                            }
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
                        let ty = &self.resolve.types[dealias(self.resolve, *ty)];
                        if let Some(name) = &ty.name {
                            format!("{}{}", self.qualifier(ty), name.to_moonbit_type_ident())
                        } else {
                            unreachable!()
                        }
                    }

                    TypeDefKind::Future(ty) => {
                        let qualifier = self.qualify_package(FFI_DIR);
                        format!(
                            "{}Future[{}]",
                            qualifier,
                            ty.as_ref()
                                .map(|t| self.type_name(t, type_variable))
                                .unwrap_or_else(|| "Unit".into())
                        )
                    }

                    TypeDefKind::Stream(ty) => {
                        let qualifier = self.qualify_package(FFI_DIR);
                        format!(
                            "{}Stream[{}]",
                            qualifier,
                            ty.as_ref()
                                .map(|t| self.type_name(t, type_variable))
                                .unwrap_or_else(|| "Unit".into())
                        )
                    }

                    _ => {
                        if let Some(name) = &ty.name {
                            format!("{}{}", self.qualifier(ty), name.to_moonbit_type_ident())
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
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

    fn deallocate_lists(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        module: &str,
    ) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::deallocate_lists_in_types(f.r#gen.resolve, types, operands, indirect, &mut f);
        String::from(f.src)
    }

    fn lift_from_memory(&mut self, address: &str, ty: &Type, module: &str) -> (String, String) {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        let result = abi::lift_from_memory(f.gen.resolve, &mut f, address.into(), ty);
        (String::from(f.src), result)
    }

    fn lower_to_memory(&mut self, address: &str, value: &str, ty: &Type, module: &str) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::lower_to_memory(f.r#gen.resolve, &mut f, address.into(), value.into(), ty);
        String::from(f.src)
    }

    fn malloc_memory(&mut self, address: &str, ty: &Type) -> String {
        let size = self.gen.sizes.size(ty).size_wasm32();
        let ffi = self.qualify_package(FFI_DIR);
        format!("let {address} = {ffi}malloc({size});")
    }

    fn generation_futures_and_streams_import(
        &mut self,
        prefix: &str,
        func: &Function,
        module: &str,
    ) {
        let module = format!("{prefix}{module}");
        for (index, ty) in func
            .find_futures_and_streams(self.resolve)
            .into_iter()
            .enumerate()
        {
            let func_name = &func.name;

            match &self.resolve.types[ty].kind {
                TypeDefKind::Future(payload_type) => {
                    self.generate_async_future_or_stream_import(
                        PayloadFor::Future,
                        &module,
                        index,
                        func_name,
                        ty,
                        payload_type.as_ref(),
                    );
                }
                TypeDefKind::Stream(payload_type) => {
                    self.generate_async_future_or_stream_import(
                        PayloadFor::Stream,
                        &module,
                        index,
                        func_name,
                        ty,
                        payload_type.as_ref(),
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    fn generate_async_future_or_stream_import(
        &mut self,
        payload_for: PayloadFor,
        module: &str,
        index: usize,
        func_name: &str,
        ty: TypeId,
        result_type: Option<&Type>,
    ) {
        if self.gen.futures.contains(&ty) {
            return;
        }
        self.gen.futures.push(ty);

        let result = match result_type {
            Some(ty) => self.type_name(ty, true),
            None => "Unit".into(),
        };

        let type_name = self.type_name(&Type::Id(ty), true);
        let name = result.to_upper_camel_case();
        let kind = match payload_for {
            PayloadFor::Future => "future",
            PayloadFor::Stream => "stream",
        };
        let table_name = format!("{}_{}_table", type_name.to_snake_case(), kind);
        let camel_kind = kind.to_upper_camel_case();
        let payload_len_arg = match payload_for {
            PayloadFor::Future => "",
            PayloadFor::Stream => " ,length : Int",
        };
        let ffi = self.qualify_package(FFI_DIR);

        let lift;
        let lower;
        let dealloc_list;
        let malloc;
        let lift_result;
        if let Some(result_type) = result_type {
            (lift, lift_result) = self.lift_from_memory("ptr", &result_type, module);
            lower = self.lower_to_memory("ptr", "value", &*result_type, module);
            dealloc_list = self.deallocate_lists(
                std::slice::from_ref(result_type),
                &[String::from("ptr")],
                true,
                module,
            );
            malloc = self.malloc_memory("ptr", result_type);
        } else {
            lift = format!("let _ = ptr");
            lower = format!("let _ = (ptr, value)");
            dealloc_list = format!("let _ = ptr");
            malloc = "let ptr = 0;".into();
            lift_result = "".into();
        }
        uwriteln!(
            self.src,
            r#"
fn wasmImport{name}New() -> UInt64 = "{module}" "[{kind}-new-{index}]{func_name}"
fn wasmImport{name}Read(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-read-{index}]{func_name}"
fn wasmImport{name}Write(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-write-{index}]{func_name}"
fn wasmImport{name}CancelRead(handle : Int) -> Int = "{module}" "[{kind}-cancel-read-{index}]{func_name}"
fn wasmImport{name}CancelWrite(handle : Int) -> Int = "{module}" "[{kind}-cancel-write-{index}]{func_name}"
fn wasmImport{name}DropReadable(handle : Int) = "{module}" "[{kind}-drop-readable-{index}]{func_name}"
fn wasmImport{name}DropWritable(handle : Int) = "{module}" "[{kind}-drop-writable-{index}]{func_name}"
fn wasm{name}Lift(ptr: Int) -> {result} {{
    {lift}
    {lift_result}
}}
fn wasm{name}Lower(value: {result}, ptr: Int) -> Unit {{
    {lower}
}}
fn wasm{name}Deallocate(ptr: Int) -> Unit {{
    {dealloc_list}
}}
fn wasm{name}Malloc() -> Int {{
    {malloc}
    ptr
}}
fn {table_name}() -> {ffi}{camel_kind}VTable[{result}] {{
    {ffi}{camel_kind}VTable::new(
        wasmImport{name}New,
        wasmImport{name}Read,
        wasmImport{name}Write,
        wasmImport{name}CancelRead,
        wasmImport{name}CancelWrite,
        wasmImport{name}DropReadable,
        wasmImport{name}DropWritable,
        wasm{name}Malloc,
        wasm{name}Deallocate,
        wasm{name}Lift,
        wasm{name}Lower
    )
}}

pub let static_{table_name}: {ffi}{camel_kind}VTable[{result}]  = {table_name}();
"#
        );
    }

    fn sig_string(&mut self, sig: &MoonbitSignature, async_: bool) -> String {
        let params = sig
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name(ty, true);
                format!("{name} : {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let (async_prefix, async_suffix) = if async_ {
            ("async ", " raise")
        } else {
            ("", "")
        };
        format!(
            "pub {async_prefix}fn {}({params}) -> {}{async_suffix}",
            sig.name, sig.result_type
        )
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        print_docs(&mut self.src, docs);

        let name = name.to_moonbit_type_ident();

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

        let mut deriviation: Vec<_> = Vec::new();
        if self.gen.opts.derive_show {
            deriviation.push("Show")
        }
        if self.gen.opts.derive_eq {
            deriviation.push("Eq")
        }

        uwrite!(
            self.src,
            "
            pub(all) struct {name} {{
                {parameters}
            }} derive({})
            ",
            deriviation.join(", ")
        );
    }

    fn type_resource(&mut self, _id: TypeId, name: &str, docs: &Docs) {
        print_docs(&mut self.src, docs);
        let type_name = name;
        let name = name.to_moonbit_type_ident();

        let mut deriviation: Vec<_> = Vec::new();
        if self.gen.opts.derive_show {
            deriviation.push("Show")
        }
        if self.gen.opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.gen.opts.derive_error && name.contains("Error") {
            "suberror"
        } else {
            "struct"
        };

        uwrite!(
            self.src,
            r#"
            pub(all) {declaration} {name}(Int) derive({})
            "#,
            deriviation.join(", "),
        );

        let module = self.module;

        if self.direction == Direction::Import {
            uwrite!(
                &mut self.src,
                r#"
                /// Drops a resource handle.
                pub fn {name}::drop(self : {name}) -> Unit {{
                    let {name}(resource) = self
                    wasmImportResourceDrop{name}(resource)
                }}
                "#,
            );

            uwrite!(
                &mut self.ffi,
                r#"
                fn wasmImportResourceDrop{name}(resource : Int) = "{module}" "[resource-drop]{type_name}"
                "#,
            )
        } else {
            uwrite!(
                &mut self.src,
                r#"
                /// Creates a new resource with the given `rep` as its representation and returning the handle to this resource.
                pub fn {name}::new(rep : Int) -> {name} {{
                    {name}::{name}(wasmExportResourceNew{name}(rep))
                }}
                fn wasmExportResourceNew{name}(rep : Int) -> Int = "[export]{module}" "[resource-new]{type_name}"

                /// Drops a resource handle.
                pub fn {name}::drop(self : Self) -> Unit {{
                    let {name}(resource) = self
                    wasmExportResourceDrop{name}(resource)
                }}
                fn wasmExportResourceDrop{name}(resource : Int) = "[export]{module}" "[resource-drop]{type_name}"

                /// Gets the `Int` representation of the resource pointed to the given handle.
                pub fn {name}::rep(self : Self) -> Int {{
                    let {name}(resource) = self
                    wasmExportResourceRep{name}(resource)
                }}
                fn wasmExportResourceRep{name}(resource : Int) -> Int = "[export]{module}" "[resource-rep]{type_name}"
                "#,
            );

            uwrite!(
                &mut self.stub,
                r#"
                /// Destructor of the resource.
                pub fn {name}::dtor(_self : {name}) -> Unit {{
                  ...
                }}
                "#
            );

            let func_name = self.gen.export_ns.tmp(&format!("wasmExport{name}Dtor"));

            let export_dir = self.gen.opts.gen_dir.clone();

            let mut gen =
                self.gen
                    .interface(self.resolve, export_dir.as_str(), "", Direction::Export);

            uwrite!(
                self.ffi,
                r#"
                pub fn {func_name}(handle : Int) -> Unit {{
                    {}{name}::dtor(handle)
                }}
                "#,
                gen.qualify_package(&self.name.to_string())
            );

            self.gen
                .export
                .insert(func_name, format!("{module}#[dtor]{type_name}"));
        }
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        print_docs(&mut self.src, docs);

        let name = name.to_moonbit_type_ident();

        let ty = match flags.repr() {
            FlagsRepr::U8 => "Byte",
            FlagsRepr::U16 | FlagsRepr::U32(1) => "UInt",
            FlagsRepr::U32(2) => "UInt64",
            _ => unreachable!(), // https://github.com/WebAssembly/component-model/issues/370
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

        let mut deriviation: Vec<_> = Vec::new();
        if self.gen.opts.derive_show {
            deriviation.push("Show")
        }
        if self.gen.opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.gen.opts.derive_error && name.contains("Error") {
            "suberror"
        } else {
            "struct"
        };

        uwrite!(
            self.src,
            "
            pub(all) {declaration} {name}({ty}) derive({})
            pub fn {name}::default() -> {name} {{
                {}
            }}
            pub(all) enum {name}Flag {{
                {cases}
            }}
            fn {name}Flag::value(self : {name}Flag) -> {ty} {{
              match self {{
                {map_to_int}
              }}
            }}
            pub fn {name}::set(self : Self, other: {name}Flag) -> {name} {{
              let {name}(flag) = self
              flag.lor(other.value())
            }}
            pub fn {name}::unset(self : Self, other: {name}Flag) -> {name} {{
              let {name}(flag) = self
              flag.land(other.value().lnot())
            }}
            pub fn {name}::is_set(self : Self, other: {name}Flag) -> Bool {{
              let {name}(flag) = self
              (flag.land(other.value()) == other.value())
            }}
            ",
            deriviation.join(", "),
            match ty {
                "Byte" => "b'\\x00'",
                "UInt" => "0U",
                "UInt64" => "0UL",
                _ => unreachable!(),
            }
        );
    }

    fn type_tuple(&mut self, _id: TypeId, _name: &str, _tuple: &Tuple, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        print_docs(&mut self.src, docs);

        let name = name.to_moonbit_type_ident();

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

        let mut deriviation: Vec<_> = Vec::new();
        if self.gen.opts.derive_show {
            deriviation.push("Show")
        }
        if self.gen.opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.gen.opts.derive_error && name.contains("Error") {
            "suberror"
        } else {
            "enum"
        };

        uwrite!(
            self.src,
            "
            pub(all) {declaration} {name} {{
              {cases}
            }} derive({})
            ",
            deriviation.join(", ")
        );
    }

    fn type_option(&mut self, _id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_result(&mut self, _id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        print_docs(&mut self.src, docs);

        let name = name.to_moonbit_type_ident();

        // Type definition
        let cases = enum_
            .cases
            .iter()
            .map(|case| case.name.to_shouty_snake_case())
            .collect::<Vec<_>>()
            .join("; ");

        let mut deriviation: Vec<_> = Vec::new();
        if self.gen.opts.derive_show {
            deriviation.push("Show")
        }
        if self.gen.opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.gen.opts.derive_error && name.contains("Error") {
            "suberror"
        } else {
            "enum"
        };

        uwrite!(
            self.src,
            "
            pub(all) {declaration} {name} {{
                {cases}
            }} derive({})
            ",
            deriviation.join(", ")
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
            pub fn {name}::ordinal(self : {name}) -> Int {{
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

    fn type_alias(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_list(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_future(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        unreachable!() // Not needed
    }

    fn type_stream(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        unreachable!() // Not needed
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

#[derive(Clone, Debug)]
enum DeferredTaskReturn {
    None,
    Generating {
        prev_src: String,
        return_param: String,
    },
    Emitted {
        params: Vec<(WasmType, String)>,
        body: String,
        return_param: String,
    },
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    func_name: &'b str,
    func_interface: &'b str,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    payloads: Vec<String>,
    cleanup: Vec<Cleanup>,
    needs_cleanup_list: bool,
    deferred_task_return: DeferredTaskReturn,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'b mut InterfaceGenerator<'a>,
        func_name: &'b str,
        func_interface: &'b str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        let mut locals = Ns::default();
        params.iter().for_each(|str| {
            locals.tmp(str);
        });
        Self {
            gen,
            func_name,
            func_interface,
            params,
            src: String::new(),
            locals,
            block_storage: Vec::new(),
            blocks: Vec::new(),
            payloads: Vec::new(),
            cleanup: Vec::new(),
            needs_cleanup_list: false,
            deferred_task_return: DeferredTaskReturn::None,
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

                let payload = if self.gen.non_empty_type(ty.as_ref()).is_some() {
                    payload
                } else if is_result {
                    format!("_{payload}")
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
                }}
                "#
            );
        } else {
            uwrite!(
                self.src,
                r#"
                let ({declarations}) = match {op} {{
                    {cases}
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
                    WasmType::F32 => "(0.0 : Float)",
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

            Instruction::I32FromS32
            | Instruction::I64FromS64
            | Instruction::S32FromI32
            | Instruction::S64FromI64
            | Instruction::CoreF64FromF64
            | Instruction::F64FromCoreF64 => results.push(operands[0].clone()),

            Instruction::F32FromCoreF32 => results.push(operands[0].clone()),
            Instruction::CoreF32FromF32 => results.push(operands[0].clone()),

            Instruction::CharFromI32 => {
                results.push(format!("Int::unsafe_to_char({})", operands[0]))
            }
            Instruction::I32FromChar => results.push(format!("({}).to_int()", operands[0])),

            Instruction::I32FromU8 => results.push(format!("({}).to_int()", operands[0])),
            Instruction::I32FromU16 => {
                results.push(format!("({}).reinterpret_as_int()", operands[0]))
            }
            Instruction::U8FromI32 => results.push(format!("({}).to_byte()", operands[0])),

            Instruction::I32FromS8 => results.push(format!(
                "{}extend8({})",
                self.gen.qualify_package(FFI_DIR),
                operands[0]
            )),
            Instruction::S8FromI32 => results.push(format!("({} - 0x100)", operands[0])),
            Instruction::S16FromI32 => results.push(format!("({} - 0x10000)", operands[0])),
            Instruction::I32FromS16 => results.push(format!(
                "{}extend16({})",
                self.gen.qualify_package(FFI_DIR),
                operands[0]
            )),
            Instruction::U16FromI32 => results.push(format!(
                "({}.land(0xFFFF).reinterpret_as_uint())",
                operands[0]
            )),
            Instruction::U32FromI32 => {
                results.push(format!("({}).reinterpret_as_uint()", operands[0]))
            }
            Instruction::I32FromU32 => {
                results.push(format!("({}).reinterpret_as_int()", operands[0]))
            }

            Instruction::U64FromI64 => {
                results.push(format!("({}).reinterpret_as_uint64()", operands[0]))
            }
            Instruction::I64FromU64 => {
                results.push(format!("({}).reinterpret_as_int64()", operands[0]))
            }

            Instruction::I32FromBool => {
                results.push(format!("(if {} {{ 1 }} else {{ 0 }})", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            Instruction::FlagsLower { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 => {
                    let op = &operands[0];
                    let flag = self.locals.tmp("flag");
                    let ty = self.gen.type_name(&Type::Id(*ty), false);
                    uwriteln!(
                        self.src,
                        r#"
                        let {ty}({flag}) = {op}
                        "#
                    );
                    results.push(format!("{flag}.to_int()"));
                }
                Int::U16 | Int::U32 => {
                    let op = &operands[0];
                    let flag = self.locals.tmp("flag");
                    let ty = self.gen.type_name(&Type::Id(*ty), false);
                    uwriteln!(
                        self.src,
                        r#"
                        let {ty}({flag}) = {op}
                        "#
                    );
                    results.push(format!("{flag}.reinterpret_as_int()"));
                }
                Int::U64 => {
                    let op = &operands[0];
                    let flag = self.locals.tmp("flag");
                    let ty = self.gen.type_name(&Type::Id(*ty), false);
                    uwriteln!(
                        self.src,
                        r#"
                        let {ty}({flag}) = {op}
                        "#
                    );
                    results.push(format!("({flag}.to_int())"));
                    results.push(format!("({flag}.lsr(32)).to_int())"));
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
                        "{}({}.reinterpret_as_uint())",
                        self.gen.type_name(&Type::Id(*ty), true),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "{}(({}).reinterpret_as_uint().to_uint64() | (({}).reinterpret_as_uint().to_uint64() << 32))",
                        self.gen.type_name(&Type::Id(*ty), true),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::HandleLower { ty, .. } => {
                let op = &operands[0];
                let handle = self.locals.tmp("handle");
                let ty = self.gen.type_name(&Type::Id(*ty), false);
                uwrite!(
                    self.src,
                    r#"
                    let {ty}({handle}) = {op}
                    "#
                );
                results.push(handle);
            }
            Instruction::HandleLift { ty, .. } => {
                let op = &operands[0];
                let ty = self.gen.type_name(&Type::Id(*ty), false);

                results.push(format!(
                    "{}::{}({})",
                    ty,
                    if ty.starts_with("@") {
                        ty.split('.').last().unwrap()
                    } else {
                        &ty
                    },
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
                    let {lifted} : {ty} = match {op} {{
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

            Instruction::ListCanonLower { element, realloc } => match element {
                Type::U8 => {
                    let op = &operands[0];

                    results.push(format!(
                        "{}bytes2ptr({op})",
                        self.gen.qualify_package(FFI_DIR)
                    ));
                    results.push(format!("{op}.length()"));
                    if realloc.is_none() {
                        self.cleanup.push(Cleanup::Object(op.clone()));
                    }
                }
                Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64 => {
                    let op = &operands[0];

                    let ty = match element {
                        Type::U32 => "uint",
                        Type::U64 => "uint64",
                        Type::S32 => "int",
                        Type::S64 => "int64",
                        Type::F32 => "float",
                        Type::F64 => "double",
                        _ => unreachable!(),
                    };

                    results.push(format!(
                        "{}{ty}_array2ptr({op})",
                        self.gen.qualify_package(FFI_DIR)
                    ));
                    results.push(format!("{op}.length()"));
                    if realloc.is_none() {
                        self.cleanup.push(Cleanup::Object(op.clone()));
                    }
                }
                _ => unreachable!("unsupported list element type"),
            },

            Instruction::ListCanonLift { element, .. } => match element {
                Type::U8 => {
                    let result = self.locals.tmp("result");
                    let address = &operands[0];
                    let length = &operands[1];

                    uwrite!(
                        self.src,
                        "
                        let {result} = {}ptr2bytes({address}, {length})
                        ",
                        self.gen.qualify_package(FFI_DIR)
                    );

                    results.push(result);
                }
                Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64 => {
                    let ty = match element {
                        Type::U32 => "uint",
                        Type::U64 => "uint64",
                        Type::S32 => "int",
                        Type::S64 => "int64",
                        Type::F32 => "float",
                        Type::F64 => "double",
                        _ => unreachable!(),
                    };

                    let result = self.locals.tmp("result");
                    let address = &operands[0];
                    let length = &operands[1];

                    uwrite!(
                        self.src,
                        "
                        let {result} = {}ptr2{ty}_array({address}, {length})
                        ",
                        self.gen.qualify_package(FFI_DIR)
                    );

                    results.push(result);
                }
                _ => unreachable!("unsupported list element type"),
            },

            Instruction::StringLower { realloc } => {
                let op = &operands[0];

                results.push(format!(
                    "{}str2ptr({op})",
                    self.gen.qualify_package(FFI_DIR)
                ));
                results.push(format!("{op}.length()"));
                if realloc.is_none() {
                    self.cleanup.push(Cleanup::Object(op.clone()));
                }
            }

            Instruction::StringLift { .. } => {
                let result = self.locals.tmp("result");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    let {result} = {}ptr2str({address}, {length})
                    ",
                    self.gen.qualify_package(FFI_DIR)
                );

                results.push(result);
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
                let size = self.gen.gen.sizes.size(element).size_wasm32();
                let align = self.gen.gen.sizes.align(element).align_wasm32();
                let address = self.locals.tmp("address");
                let ty = self.gen.type_name(element, true);
                let index = self.locals.tmp("index");

                uwrite!(
                    self.src,
                    "
                    let {address} = {}malloc(({op}).length() * {size});
                    for {index} = 0; {index} < ({op}).length(); {index} = {index} + 1 {{
                        let {block_element} : {ty} = ({op})[({index})]
                        let {base} = {address} + ({index} * {size});
                        {body}
                    }}
                    ",
                    self.gen.qualify_package(FFI_DIR)
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
                let size = self.gen.gen.sizes.size(element).size_wasm32();
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
                    {}free({address})
                    ",
                    self.gen.qualify_package(FFI_DIR)
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
                // TODO: handle this to support async functions
                uwriteln!(self.src, "{assignment} wasmImport{func_name}({operands});");
            }

            Instruction::CallInterface { func, async_ } => {
                let name = match func.kind {
                    FunctionKind::Freestanding => {
                        format!(
                            "{}{}",
                            self.r#gen.qualify_package(&self.func_interface.to_string()),
                            func.name.to_moonbit_ident()
                        )
                    }
                    FunctionKind::AsyncFreestanding => {
                        format!(
                            "{}{}",
                            self.r#gen.qualify_package(&self.func_interface.to_string()),
                            func.name.to_moonbit_ident()
                        )
                    }
                    FunctionKind::Constructor(ty) => {
                        let name = self.gen.type_name(&Type::Id(ty), false);
                        format!(
                            "{}::{}",
                            name,
                            func.name.replace("[constructor]", "").to_moonbit_ident()
                        )
                    }
                    FunctionKind::Method(ty)
                    | FunctionKind::Static(ty)
                    | FunctionKind::AsyncMethod(ty)
                    | FunctionKind::AsyncStatic(ty) => {
                        let name = self.gen.type_name(&Type::Id(ty), false);
                        format!(
                            "{}::{}",
                            name,
                            func.name.split(".").last().unwrap().to_moonbit_ident()
                        )
                    }
                };

                let args = operands.join(", ");

                if *async_ {
                    let (async_func_result, task_return_result) = if func.result.is_some() {
                        let res = self.locals.tmp("result");
                        (res.clone(), res)
                    } else {
                        ("_".into(), "".into())
                    };

                    if func.result.is_some() {
                        results.push(async_func_result.clone());
                    }
                    uwrite!(
                        self.src,
                        r#"
                        let task = @ffi.get_or_create_waitable_set();
                        task.with_waitable_set(task => {{
                            let {async_func_result} = {name}(task, {args});
                            {name}_task_return({task_return_result});
                        }})
                        return @ffi.CallbackCode::Wait(task.id).encode()
                        "#,
                    );
                    assert!(matches!(
                        self.deferred_task_return,
                        DeferredTaskReturn::None
                    ));
                    self.deferred_task_return = DeferredTaskReturn::Generating {
                        prev_src: mem::take(&mut self.src),
                        return_param: async_func_result.to_string(),
                    };
                    return;
                }

                let assignment = match func.result {
                    None => "let _ = ".into(),
                    Some(ty) => {
                        let ty = format!("({})", self.gen.type_name(&ty, true));
                        let result = self.locals.tmp("result");
                        if func.result.is_some() {
                            results.push(result.clone());
                        }
                        let assignment = format!("let ({result}) : {ty} = ");
                        assignment
                    }
                };

                uwrite!(
                    self.src,
                    "
                    {assignment}{name}({args});
                    ",
                );
            }

            Instruction::Return { amt, .. } => {
                for clean in &self.cleanup {
                    match clean {
                        Cleanup::Memory {
                            address,
                            size: _,
                            align: _,
                        } => uwriteln!(
                            self.src,
                            "{}free({address})",
                            self.gen.qualify_package(FFI_DIR)
                        ),
                        Cleanup::Object(obj) => uwriteln!(self.src, "ignore({obj})"),
                    }
                }

                if self.needs_cleanup_list {
                    uwrite!(
                        self.src,
                        "
                        cleanupList.each(fn(cleanup) {{
                            {}free(cleanup.address);
                        }})
                    ignore(ignoreList)
                        ",
                        self.gen.qualify_package(FFI_DIR)
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
            | Instruction::LengthLoad { offset } => results.push(format!(
                "{}load32(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I32Load8U { offset } => results.push(format!(
                "{}load8_u(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I32Load8S { offset } => results.push(format!(
                "{}load8(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I32Load16U { offset } => results.push(format!(
                "{}load16_u(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I32Load16S { offset } => results.push(format!(
                "{}load16(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I64Load { offset } => results.push(format!(
                "{}load64(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::F32Load { offset } => results.push(format!(
                "{}loadf32(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::F64Load { offset } => results.push(format!(
                "{}loadf64(({}) + {offset})",
                self.gen.qualify_package(FFI_DIR),
                operands[0],
                offset = offset.size_wasm32()
            )),

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => uwriteln!(
                self.src,
                "{}store32(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),

            Instruction::I32Store8 { offset } => uwriteln!(
                self.src,
                "{}store8(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),

            Instruction::I32Store16 { offset } => uwriteln!(
                self.src,
                "{}store16(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),

            Instruction::I64Store { offset } => uwriteln!(
                self.src,
                "{}store64(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),

            Instruction::F32Store { offset } => uwriteln!(
                self.src,
                "{}storef32(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),

            Instruction::F64Store { offset } => uwriteln!(
                self.src,
                "{}storef64(({}) + {offset}, {})",
                self.gen.qualify_package(FFI_DIR),
                operands[1],
                operands[0],
                offset = offset.size_wasm32()
            ),
            // TODO: see what we can do with align
            Instruction::Malloc { size, .. } => {
                uwriteln!(
                    self.src,
                    "{}malloc({})",
                    self.gen.qualify_package(FFI_DIR),
                    size.size_wasm32()
                )
            }

            Instruction::GuestDeallocate { .. } => {
                uwriteln!(
                    self.src,
                    "{}free({})",
                    self.gen.qualify_package(FFI_DIR),
                    operands[0]
                )
            }

            Instruction::GuestDeallocateString => {
                uwriteln!(
                    self.src,
                    "{}free({})",
                    self.gen.qualify_package(FFI_DIR),
                    operands[0]
                )
            }

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
                        _ => panic()
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

                let size = self.gen.gen.sizes.size(element).size_wasm32();
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

                uwriteln!(
                    self.src,
                    "{}free({address})",
                    self.gen.qualify_package(FFI_DIR)
                );
            }

            Instruction::Flush { amt } => {
                results.extend(operands.iter().take(*amt).map(|v| v.clone()));
            }

            Instruction::FutureLift { ty, .. } => {
                let result = self.locals.tmp("result");
                let op = &operands[0];
                let qualifier = self.r#gen.qualify_package(&self.func_interface.to_string());
                let ty = self.gen.type_name(&Type::Id(*ty), true);
                let ffi = self.gen.qualify_package(FFI_DIR);
                let snake_name = format!(
                    "{}static_{}_future_table",
                    qualifier,
                    ty.replace(&qualifier, "").to_snake_case(),
                );

                uwriteln!(
                    self.src,
                    r#"let {result} = {ffi}Future::new({op}, {snake_name});"#,
                );

                results.push(result);
            }

            Instruction::FutureLower { .. } => {
                let op = &operands[0];
                results.push(format!("{}.handle", op));
            }

            Instruction::AsyncTaskReturn { params, .. } => {
                let (body, return_param) = match &mut self.deferred_task_return {
                    DeferredTaskReturn::Generating {
                        prev_src,
                        return_param,
                    } => {
                        mem::swap(&mut self.src, prev_src);
                        (mem::take(prev_src), return_param.clone())
                    }
                    _ => unreachable!(),
                };
                assert_eq!(params.len(), operands.len());
                self.deferred_task_return = DeferredTaskReturn::Emitted {
                    body,
                    params: params
                        .iter()
                        .zip(operands)
                        .map(|(a, b)| (a.clone(), b.clone()))
                        .collect(),
                    return_param,
                };
            }

            Instruction::StreamLower { .. } => {
                let op = &operands[0];
                results.push(format!("{}.handle", op));
            }

            Instruction::StreamLift { ty, .. } => {
                let result = self.locals.tmp("result");
                let op = &operands[0];
                let qualifier = self.r#gen.qualify_package(&self.func_interface.to_string());
                let ty = self.gen.type_name(&Type::Id(*ty), true);
                let ffi = self.gen.qualify_package(FFI_DIR);
                let snake_name = format!(
                    "{}static_{}_stream_table",
                    qualifier,
                    ty.replace(&qualifier, "").to_snake_case(),
                );
                uwrite!(
                    self.src,
                    r#"let {result} = {ffi}Stream::new({op}, {qualifier}{snake_name});"#,
                );

                results.push(result);
            }
            Instruction::ErrorContextLower { .. }
            | Instruction::ErrorContextLift { .. }
            | Instruction::DropHandle { .. } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: ArchitectureSize, align: Alignment) -> String {
        if self.gen.direction == Direction::Import {
            let ffi_qualifier = self.gen.qualify_package(FFI_DIR);
            let address = self.locals.tmp("return_area");
            uwriteln!(
                self.src,
                "let {address} = {ffi_qualifier}malloc({})",
                size.size_wasm32(),
            );
            self.cleanup.push(Cleanup::Memory {
                address: address.clone(),
                size: size.size_wasm32().to_string(),
                align: align.align_wasm32(),
            });
            address
        } else {
            self.gen.gen.return_area_size = self.gen.gen.return_area_size.max(size);
            self.gen.gen.return_area_align = self.gen.gen.return_area_align.max(align);
            "return_area".into()
        }
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
        matches!(
            element,
            Type::U8 | Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64
        )
    }
}

fn perform_cast(op: &str, cast: &Bitcast) -> String {
    match cast {
        Bitcast::I32ToF32 => {
            format!("({op}).reinterpret_as_float()")
        }
        Bitcast::I64ToF32 => format!("({op}).to_int().reinterpret_as_float()"),
        Bitcast::F32ToI32 => {
            format!("({op}).reinterpret_as_int()")
        }
        Bitcast::F32ToI64 => format!("({op}).reinterpret_as_int().to_int64()"),
        Bitcast::I64ToF64 => {
            format!("({op}).reinterpret_as_double()")
        }
        Bitcast::F64ToI64 => {
            format!("({op}).reinterpret_as_int64()")
        }
        Bitcast::LToI64 | Bitcast::PToP64 | Bitcast::I32ToI64 => format!("Int::to_int64({op})"),
        Bitcast::I64ToL | Bitcast::P64ToP | Bitcast::I64ToI32 => format!("Int64::to_int({op})"),
        Bitcast::I64ToP64
        | Bitcast::P64ToI64
        | Bitcast::I32ToP
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

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "Int",
        WasmType::I64 => "Int64",
        WasmType::F32 => "Float",
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

fn indent(code: &str) -> Source {
    let mut indented = Source::default();
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
            indented.deindent(2)
        }
        indented.push_str(trimmed);
        if trimmed.ends_with('{') && !trimmed.starts_with("///") {
            indented.indent(2)
        }
        indented.push_str("\n");
    }
    indented
}

fn world_name(resolve: &Resolve, world: WorldId) -> String {
    format!("world.{}", resolve.worlds[world].name.to_lower_camel_case())
}

fn interface_name(resolve: &Resolve, name: &WorldKey) -> String {
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
    .to_lower_camel_case();

    format!(
        "interface.{}{name}",
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

trait ToMoonBitIdent: ToOwned {
    fn to_moonbit_ident(&self) -> Self::Owned;
}

impl ToMoonBitIdent for str {
    fn to_moonbit_ident(&self) -> String {
        // Escape MoonBit keywords and reserved keywords
        match self {
            // Keywords
            "as" | "else" | "extern" | "fn" | "fnalias" | "if" | "let" | "const" | "match" | "using"
            | "mut" | "type" | "typealias" | "struct" | "enum" | "trait" | "traitalias" | "derive"
            | "while" | "break" | "continue" | "import" | "return" | "throw" | "raise" | "try" | "catch"
            | "pub" | "priv" | "readonly" | "true" | "false" | "_" | "test" | "loop" | "for" | "in" | "impl"
            | "with" | "guard" | "async" | "is" | "suberror" | "and" | "letrec" | "enumview" | "noraise" 
            | "defer" | "init" | "main"
            // Reserved keywords
            | "module" | "move" | "ref" | "static" | "super" | "unsafe" | "use" | "where" | "await"
            | "dyn" | "abstract" | "do" | "final" | "macro" | "override" | "typeof" | "virtual" | "yield"
            | "local" | "method" | "alias" | "assert" | "package" | "recur" | "isnot" | "define" | "downcast"
            | "inherit" | "member" | "namespace" | "upcast" | "void" | "lazy" | "include" | "mixin"
            | "protected" | "sealed" | "constructor" | "atomic" | "volatile" | "anyframe" | "anytype"
            | "asm" | "comptime" | "errdefer" | "export" | "opaque" | "orelse" | "resume" | "threadlocal"
            | "unreachable" | "dynclass" | "dynobj" | "dynrec" | "var" | "finally" | "noasync" => {
                format!("{self}_")
            }
            _ => self.to_snake_case(),
        }
    }
}

trait ToMoonBitTypeIdent: ToOwned {
    fn to_moonbit_type_ident(&self) -> Self::Owned;
}

impl ToMoonBitTypeIdent for str {
    fn to_moonbit_type_ident(&self) -> String {
        // Escape MoonBit builtin types
        match self.to_upper_camel_case().as_str() {
            type_name @ ("Bool" | "Byte" | "Int" | "Int64" | "UInt" | "UInt64" | "Float"
            | "Double" | "Error" | "Buffer" | "Bytes" | "Array" | "FixedArray"
            | "Map" | "String" | "Option" | "Result" | "Char" | "Json") => {
                format!("{type_name}_")
            }
            type_name => type_name.to_owned(),
        }
    }
}

fn generated_preamble(src: &mut Source, version: &str) {
    uwriteln!(src, "// Generated by `wit-bindgen` {version}.")
}

fn print_docs(src: &mut String, docs: &Docs) {
    if let Some(docs) = &docs.contents {
        let lines = docs
            .trim()
            .lines()
            .map(|line| format!("/// {line}"))
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(src, "{}", lines)
    }
}
