use anyhow::Result;
use core::panic;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    mem,
    ops::Deref,
};
use wit_bindgen_core::{
    AsyncFilterSet, Direction, Files, InterfaceGenerator as CoreInterfaceGenerator, Ns, Source,
    WorldGenerator,
    abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Alignment, ArchitectureSize, Docs, Enum, Flags, FlagsRepr, Function, Int, InterfaceId,
        LiftLowerAbi, LiveTypes, ManglingAndAbi, Param, Record, Resolve, ResourceIntrinsic,
        Result_, SizeAlign, Tuple, Type, TypeDefKind, TypeId, Variant, WasmExport, WasmExportKind,
        WasmImport, WorldId, WorldKey,
    },
};

use crate::async_support::{AsyncFunctionState, AsyncSupport};
use crate::pkg::{Imports, MoonbitSignature, PkgResolver, ToMoonBitIdent, ToMoonBitTypeIdent};

mod async_support;
mod ffi;
mod pkg;

// Assumptions:
// - Data: u8 -> Byte, s8 | s16 | s32 -> Int, u16 | u32 -> UInt, s64 -> Int64, u64 -> UInt64, f32 | f64 -> Double, address -> Int
// - Encoding: UTF16
// - Lift/Lower list<T>: T == Int/UInt/Int64/UInt64/Float/Double -> FixedArray[T], T == Byte -> Bytes, T == Char -> String
// Organization:
// - one package per interface (export and import are treated as different interfaces)
// - ffi utils are under `./ffi`, and the project entrance (package as link target) is under `./gen`

// We use Legacy mangling for MoonBit (no specific reason, just because we haven't switched yet)
// We use AsyncCallback ABI for async functions

// TODO: Export will share the type signatures with the import by using a newtype alias
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    #[cfg_attr(feature = "clap", clap(flatten))]
    pub derive: DeriveOpts,

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

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct DeriveOpts {
    /// Whether or not to derive Debug for all types
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_debug: bool,

    /// Whether or not to derive Show for all types
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_show: bool,

    /// Whether or not to derive Eq for all types
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_eq: bool,

    /// Whether or not to declare as Error type for types ".*error"
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub derive_error: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(MoonBit {
            opts: self.clone(),
            ..MoonBit::default()
        })
    }
}

#[derive(Default)]
struct InterfaceFragment {
    src: String,
    ffi: String,
    builtins: HashSet<&'static str>,
}

impl InterfaceFragment {
    fn concat(&mut self, other: Self) {
        self.src.push_str(&other.src);
        self.ffi.push_str(&other.ffi);
        self.builtins.extend(other.builtins);
    }
}

#[derive(Default)]
pub struct MoonBit {
    opts: Opts,
    project_name: String,
    import_world_fragment: InterfaceFragment,
    sizes: SizeAlign,

    // Collision may happen when a package is imported with multiple versions.
    // see multiverison
    interface_ns: Ns,
    // dependencies between packages
    pkg_resolver: PkgResolver,
    // Wasm export name -> (exported function name, func)
    export: HashMap<String, (String, String)>,

    export_ns: Ns,

    async_support: AsyncSupport,
}

impl MoonBit {
    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a str,
        direction: Direction,
        interface: Option<&'a WorldKey>,
    ) -> InterfaceGenerator<'a> {
        let derive_opts = self.opts.derive.clone();
        InterfaceGenerator {
            src: String::new(),
            ffi: String::new(),
            world_gen: self,
            resolve,
            name,
            direction,
            ffi_imports: HashSet::new(),
            derive_opts,
            interface,
        }
    }

    fn write_moon_pkg(&self, moon_pkg: &mut Source, imports: Option<&Imports>, link: bool) {
        // Disable warning for invalid inline wasm
        moon_pkg.push_str("{\n\"warn-list\": \"-44\"");
        // Dependencies
        if let Some(imports) = imports {
            moon_pkg.push_str(",\n\"import\": [\n");
            moon_pkg.indent(1);
            let mut deps = imports
                .packages
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{{ \"path\" : \"{}/{}\", \"alias\" : \"{}\" }}",
                        self.project_name,
                        k.replace(".", "/"),
                        v
                    )
                })
                .collect::<Vec<_>>();
            deps.sort();
            uwrite!(moon_pkg, "{}", deps.join(",\n"));
            moon_pkg.deindent(1);
            moon_pkg.push_str("\n]");
        }
        // Link target
        if link {
            let memory_name = self.pkg_resolver.resolve.wasm_export_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmExport::Memory,
            );
            moon_pkg.push_str(",\n\"link\": {\n\"wasm\": {\n");
            moon_pkg.push_str(&format!("\"export-memory-name\": \"{memory_name}\",\n"));
            moon_pkg.push_str("\"heap-start-address\": 16,\n");
            moon_pkg.push_str("\"exports\": [\n");
            moon_pkg.indent(1);
            let mut exports = self
                .export
                .iter()
                .map(|(export_name, (func_name, _))| format!("\"{func_name}:{export_name}\""))
                .collect::<Vec<_>>();
            exports.push(format!(
                "\"mbt_ffi_cabi_realloc:{}\"",
                self.pkg_resolver.resolve.wasm_export_name(
                    ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                    WasmExport::Realloc,
                ),
            ));
            exports.sort();
            uwrite!(moon_pkg, "{}", exports.join(",\n"));
            moon_pkg.deindent(1);
            moon_pkg.push_str("\n]\n}\n}\n");
        }
        moon_pkg.push_str("\n}\n");
    }
}

/// World generator implementation for MoonBit.
///
/// This implementation connects the generic `wit-bindgen` world generation
/// workflow with MoonBit-specific codegen details. It consumes the parsed
/// WIT `Resolve` structure and emits MoonBit source (`*.mbt`) and package
/// metadata files into the provided `Files` collection.
///
/// Responsibilities and behavior:
/// - `preprocess`: Initialize generator-wide state (package resolver,
///   project name, and size/align information) for the current world.
/// - `import_interface` / `export_interface`: Generate per-interface
///   sources, FFI glue, README documentation and `moon.pkg.json` metadata.
/// - `import_funcs` / `export_funcs` / `import_types`: Collect and accumulate
///   world-level functions and types (the `$root` module) into fragments
///   that are later written out by `finish_imports` or `finish`.
/// - `finish_imports` / `finish`: Emit aggregated import artifacts and the
///   final project entrypoints such as the combined FFI module and package
///   descriptor files.
///
/// Implementation notes:
/// - Namespacing and collision avoidance are handled using `PkgResolver` and
///   an internal `Ns` to make import/export package names stable even when
///   multiple package versions are present.
/// - Inline FFI helpers and builtins are collected and written once into the
///   final export FFI module. Async helpers are emitted when required.
impl WorldGenerator for MoonBit {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) -> Result<()> {
        if world_contains_endpoint_fixed_length_list_combination(resolve, world) {
            anyhow::bail!(
                "MoonBit async bindings do not yet support combining future or stream types with fixed-length lists"
            );
        }
        if world_contains_future_or_stream(resolve, world) {
            self.async_support.require_runtime();
        }

        self.pkg_resolver.resolve = resolve.clone();
        self.project_name = self
            .opts
            .project_name
            .clone()
            .or(resolve.worlds[world].package.map(|id| {
                let package = &resolve.packages[id].name;
                format!("{}/{}", package.namespace, package.name)
            }))
            .unwrap_or("generated".into());
        self.sizes.fill(resolve);
        Ok(())
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        files: &mut Files,
    ) -> Result<()> {
        let name = PkgResolver::interface_name(resolve, key);
        let name = self.interface_ns.tmp(&name);
        self.pkg_resolver
            .import_interface_names
            .insert(id, name.clone());

        let mut r#gen = self.interface(resolve, &name, Direction::Import, Some(key));
        r#gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            r#gen.import(func);
        }

        let fragment = r#gen.finish();
        // Write files
        {
            let directory = name.replace('.', "/");

            // README
            if let Some(content) = &resolve.interfaces[id].docs.contents
                && !content.is_empty()
            {
                files.push(&format!("{directory}/README.md"), content.as_bytes());
            }

            // Source
            let mut src = Source::default();
            wit_bindgen_core::generated_preamble(&mut src, VERSION);
            uwriteln!(src, "{}", fragment.src);
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());

            // FFI
            let mut ffi = Source::default();
            wit_bindgen_core::generated_preamble(&mut ffi, VERSION);
            uwriteln!(ffi, "{}", fragment.ffi);
            for builtin in fragment.builtins {
                uwriteln!(ffi, "{}", builtin);
            }
            files.push(&format!("{directory}/ffi.mbt"), indent(&ffi).as_bytes());

            // moon.pkg.json
            let mut moon_pkg = Source::default();
            self.write_moon_pkg(
                &mut moon_pkg,
                self.pkg_resolver.package_import.get(&name),
                false,
            );
            files.push(&format!("{directory}/moon.pkg.json"), moon_pkg.as_bytes());
        }

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = PkgResolver::world_name(resolve, world);
        let mut r#gen = self.interface(resolve, &name, Direction::Import, None);

        for (_, func) in funcs {
            r#gen.import(func);
        }

        let result = r#gen.finish();
        self.import_world_fragment.concat(result);
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = PkgResolver::world_name(resolve, world);
        let mut r#gen = self.interface(resolve, &name, Direction::Import, None);

        for (ty_name, ty) in types {
            r#gen.define_type(ty_name, *ty);
        }

        let result = r#gen.finish();
        self.import_world_fragment.concat(result);
    }

    fn finish_imports(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) {
        let name = PkgResolver::world_name(resolve, world);
        let directory = name.replace('.', "/");

        // README
        if let Some(content) = &resolve.worlds[world].docs.contents
            && !content.is_empty()
        {
            files.push(&format!("{directory}/README.md"), content.as_bytes());
        }
        // Source
        let mut src = Source::default();
        wit_bindgen_core::generated_preamble(&mut src, VERSION);
        uwriteln!(src, "{}", self.import_world_fragment.src);
        files.push(&format!("{directory}/import.mbt"), indent(&src).as_bytes());
        // FFI
        let mut ffi = Source::default();
        let mut builtins: HashSet<&'static str> = HashSet::new();
        wit_bindgen_core::generated_preamble(&mut ffi, VERSION);
        uwriteln!(ffi, "{}", self.import_world_fragment.ffi);
        builtins.extend(self.import_world_fragment.builtins.iter());
        for b in builtins.iter() {
            uwriteln!(ffi, "{}", b);
        }
        files.push(
            &format!("{directory}/ffi_import.mbt"),
            indent(&ffi).as_bytes(),
        );
        // moon.pkg.json
        let mut moon_pkg = Source::default();
        self.write_moon_pkg(
            &mut moon_pkg,
            self.pkg_resolver.package_import.get(&name),
            false,
        );
        files.push(&format!("{directory}/moon.pkg.json"), moon_pkg.as_bytes());
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        files: &mut Files,
    ) -> Result<()> {
        let name = format!(
            "{}.{}",
            self.opts.r#gen_dir,
            PkgResolver::interface_name(resolve, key)
        );
        let name = self.interface_ns.tmp(&name);
        self.pkg_resolver
            .export_interface_names
            .insert(id, name.clone());

        let mut r#gen = self.interface(resolve, &name, Direction::Export, Some(key));
        r#gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            r#gen.export(func);
        }

        let fragment = r#gen.finish();

        // Write files
        {
            let directory = name.replace('.', "/");

            // README
            if let Some(content) = &resolve.interfaces[id].docs.contents
                && !content.is_empty()
            {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
            // Source
            let mut src = Source::default();
            wit_bindgen_core::generated_preamble(&mut src, VERSION);
            uwriteln!(src, "{}", fragment.src);
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());

            if !self.opts.ignore_stub {
                // moon.pkg.json
                let mut moon_pkg = Source::default();
                self.write_moon_pkg(
                    &mut moon_pkg,
                    self.pkg_resolver.package_import.get(&name),
                    false,
                );
                files.push(&format!("{directory}/moon.pkg.json"), moon_pkg.as_bytes());
            }

            // FFI
            let mut ffi = Source::default();
            wit_bindgen_core::generated_preamble(&mut ffi, VERSION);

            uwriteln!(&mut ffi, "{}", fragment.ffi);
            for b in fragment.builtins.iter() {
                uwriteln!(ffi, "{}", b);
            }
            files.push(&format!("{directory}/ffi.mbt",), indent(&ffi).as_bytes());
        }

        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        files: &mut Files,
    ) -> Result<()> {
        let name = format!(
            "{}.{}",
            self.opts.r#gen_dir,
            PkgResolver::world_name(resolve, world)
        );
        let mut r#gen = self.interface(resolve, &name, Direction::Export, None);

        for (_, func) in funcs {
            r#gen.export(func);
        }

        let fragment = r#gen.finish();

        // Write files
        {
            let directory = name.replace('.', "/");
            // Source
            let mut src = Source::default();
            wit_bindgen_core::generated_preamble(&mut src, VERSION);
            uwriteln!(src, "{}", fragment.src);
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());

            if !self.opts.ignore_stub {
                // moon.pkg.json
                let mut moon_pkg = Source::default();
                self.write_moon_pkg(
                    &mut moon_pkg,
                    self.pkg_resolver.package_import.get(&name),
                    false,
                );
                files.push(&format!("{directory}/moon.pkg.json"), moon_pkg.as_bytes());
            }

            // FFI
            let mut export = Source::default();
            wit_bindgen_core::generated_preamble(&mut export, VERSION);
            uwriteln!(&mut export, "{}", fragment.ffi);
            for b in fragment.builtins.iter() {
                uwriteln!(&mut export, "{}", b);
            }
            files.push(&format!("{directory}/ffi.mbt",), indent(&export).as_bytes());
        }

        Ok(())
    }

    fn finish(&mut self, _resolve: &Resolve, _id: WorldId, files: &mut Files) -> Result<()> {
        // If async is used, export async utils
        self.async_support.emit_runtime_files(files, VERSION);

        // Export project files
        if !self.opts.ignore_stub && !self.opts.ignore_module_file {
            let mut body = Source::default();
            uwriteln!(
                &mut body,
                "{{ \"name\": \"{}\", \"preferred-target\": \"wasm\" }}",
                self.project_name
            );
            files.push("moon.mod.json", body.as_bytes());
        }

        // Export project entry point
        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, VERSION);
        // CABI Realloc
        for builtin in [ffi::CABI_REALLOC, ffi::MALLOC, ffi::FREE] {
            uwriteln!(&mut body, "{}", builtin);
        }
        // Import all exported interfaces
        for (_, (_, impl_)) in self.export.iter() {
            uwriteln!(&mut body, "{impl_}");
        }

        files.push(
            &format!("{}/ffi.mbt", self.opts.r#gen_dir),
            indent(&body).as_bytes(),
        );

        let mut moon_pkg = Source::default();
        self.write_moon_pkg(
            &mut moon_pkg,
            self.pkg_resolver.package_import.get(&self.opts.r#gen_dir),
            true,
        );
        files.push(
            &format!("{}/moon.pkg.json", self.opts.r#gen_dir),
            indent(&moon_pkg).as_bytes(),
        );

        Ok(())
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    ffi: String,
    // Collect of FFI imports used in this interface
    ffi_imports: HashSet<&'static str>,

    world_gen: &'a mut MoonBit,
    resolve: &'a Resolve,
    // The current interface getting generated
    name: &'a str,
    direction: Direction,
    interface: Option<&'a WorldKey>,

    // Options for deriving traits
    derive_opts: DeriveOpts,
}

impl InterfaceGenerator<'_> {
    fn finish(self) -> InterfaceFragment {
        InterfaceFragment {
            src: self.src,
            ffi: self.ffi,
            builtins: self.ffi_imports,
        }
    }

    fn import(&mut self, func: &Function) {
        let async_plan = self.world_gen.async_support.import_plan(
            &mut self.world_gen.opts.async_,
            self.resolve,
            self.interface,
            func,
        );
        let variant = async_plan.abi_variant();
        let endpoint_plan = self.import_async_function_plan(self.interface, func);
        let wasm_sig = self.resolve.wasm_signature(variant, func);
        let mbt_sig = self.world_gen.pkg_resolver.mbt_sig(self.name, func, false);
        let (src, needs_cleanup_list, endpoint_state) = if async_plan.is_async() {
            let body = self.generate_async_import_body(&endpoint_plan, func, &mbt_sig, &wasm_sig);
            (body.src, body.needs_cleanup_list, body.state)
        } else {
            let mut bindgen = FunctionBindgen::new(
                self,
                func.params
                    .iter()
                    .map(|Param { name, .. }| name.to_moonbit_ident())
                    .collect(),
            )
            .with_async_state(endpoint_plan.state());
            if endpoint_plan.has_endpoints() {
                bindgen = bindgen.with_sync_import_commit(
                    func.params.iter().map(|Param { ty, .. }| *ty).collect(),
                );
            }

            abi::call(
                bindgen.interface_gen.resolve,
                AbiVariant::GuestImport,
                LiftLower::LowerArgsLiftResults,
                func,
                &mut bindgen,
                false,
            );
            (bindgen.src, bindgen.needs_cleanup_list, bindgen.async_state)
        };

        let cleanup_list = if needs_cleanup_list {
            "let cleanup_list : Array[Int] = []"
        } else {
            ""
        };

        let (import_module, import_name) = self.resolve.wasm_import_name(
            async_plan.mangling_and_abi(),
            WasmImport::Func {
                interface: self.interface,
                func,
            },
        );
        let result_type = match &wasm_sig.results[..] {
            [] => "".into(),
            [result] => format!("-> {}", wasm_type(*result)),
            _ => unimplemented!("multi-value results are not supported yet"),
        };
        let params = wasm_sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| format!("p{i} : {}", wasm_type(*param)))
            .collect::<Vec<_>>()
            .join(", ");
        let ffi_import_name = format!("wasmImport{}", func.name.to_upper_camel_case());
        uwriteln!(
            self.ffi,
            r#"
            fn {ffi_import_name}({params}) {result_type} = "{import_module}" "{import_name}"
            "#
        );

        self.emit_future_stream_helpers(&endpoint_plan, &endpoint_state);

        print_docs(&mut self.src, &func.docs);
        let sig = self.sig_string(func, &mbt_sig, async_plan.signature_is_async());
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

    fn export(&mut self, func: &Function) {
        let async_plan = self.world_gen.async_support.export_plan(
            &mut self.world_gen.opts.async_,
            self.resolve,
            self.interface,
            func,
        );
        let variant = async_plan.abi_variant();
        let sig = self.resolve.wasm_signature(variant, func);
        let mbt_sig = self.world_gen.pkg_resolver.mbt_sig(self.name, func, false);
        let func_sig = self.sig_string(func, &mbt_sig, async_plan.signature_is_async());

        print_docs(&mut self.src, &func.docs);
        uwrite!(
            self.src,
            r#"
            declare {func_sig}
            "#
        );

        let endpoint_plan = self.export_async_function_plan(self.interface, func);
        let mut bindgen = FunctionBindgen::new(
            self,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        )
        .with_async_state(endpoint_plan.state());

        abi::call(
            bindgen.interface_gen.resolve,
            variant,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
            async_plan.is_async(),
        );

        let cleanup_list = if bindgen.needs_cleanup_list {
            "let cleanup_list : Array[Int] = []"
        } else {
            ""
        };
        let async_state = bindgen.async_state.clone();
        let src = bindgen.src;

        let result_type = match &sig.results[..] {
            [] => "Unit",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let func_name = self
            .world_gen
            .export_ns
            .tmp(&format!("wasmExport{camel_name}"));

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

        let export_name = self.resolve.wasm_export_name(
            async_plan.mangling_and_abi(),
            WasmExport::Func {
                interface: self.interface,
                func,
                kind: WasmExportKind::Normal,
            },
        );
        self.emit_future_stream_helpers(&endpoint_plan, &async_state);

        if !self.emit_async_export_wrapper(
            &async_plan,
            func,
            &func_name,
            &params,
            result_type,
            cleanup_list,
            &src,
        ) {
            uwrite!(
                self.ffi,
                r#"
                #doc(hidden)
                pub fn {func_name}({params}) -> {result_type} {{
                    {cleanup_list}
                    {src}
                }}
                "#,
            );
        }

        let export = format!(
            r#"
            #doc(hidden)
            pub fn {func_name}({params}) -> {result_type} {{
                {}{func_name}({})
            }}
            "#,
            self.world_gen
                .pkg_resolver
                .qualify_package(self.world_gen.opts.gen_dir.as_str(), self.name),
            (0..sig.params.len())
                .map(|i| format!("p{i}"))
                .collect::<Vec<_>>()
                .join(", "),
        );

        self.world_gen
            .export
            .insert(export_name, (func_name, export));

        if !self.emit_async_export_callback(
            &async_plan,
            self.interface,
            func,
            &camel_name,
            async_state,
        ) && abi::guest_export_needs_post_return(self.resolve, func)
        {
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
                (0..sig.results.len()).map(|i| format!("p{i}")).collect(),
            );

            abi::post_return(bindgen.interface_gen.resolve, func, &mut bindgen);

            let src = bindgen.src;

            let func_name = self
                .world_gen
                .export_ns
                .tmp(&format!("wasmExport{camel_name}PostReturn"));

            uwrite!(
                self.ffi,
                r#"
                #doc(hidden)
                pub fn {func_name}({params}) -> Unit {{
                    {src}
                }}
                "#
            );
            let export_name = self.resolve.wasm_export_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmExport::Func {
                    interface: self.interface,
                    func,
                    kind: WasmExportKind::PostReturn,
                },
            );
            let export = format!(
                r#"
                #doc(hidden)
                pub fn {func_name}({params}) -> Unit {{
                    {}{func_name}({})
                }}
                "#,
                self.world_gen
                    .pkg_resolver
                    .qualify_package(self.world_gen.opts.gen_dir.as_str(), self.name),
                (0..sig.results.len())
                    .map(|i| format!("p{i}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            self.world_gen
                .export
                .insert(export_name, (func_name, export));
        }
    }

    fn sig_string(&mut self, func: &Function, sig: &MoonbitSignature, async_: bool) -> String {
        let mut params = sig
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.world_gen.pkg_resolver.type_name(self.name, ty);
                format!("{name} : {ty}")
            })
            .collect::<Vec<_>>();

        self.add_async_export_stub_parameter(func, async_, &mut params);

        let params = params.join(", ");
        let result_type = match &sig.result_type {
            None => "Unit".into(),
            Some(ty) => self.world_gen.pkg_resolver.type_name(self.name, ty),
        };
        format!(
            "pub {}fn {}({params}) -> {}",
            if async_ { "async " } else { "" },
            sig.name,
            result_type
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
                    self.world_gen.pkg_resolver.type_name(self.name, &field.ty),
                )
            })
            .collect::<Vec<_>>()
            .join("; ");

        let contains_endpoint = record
            .fields
            .iter()
            .any(|field| type_contains_future_or_stream(self.resolve, &field.ty));
        let mut deriviation: Vec<_> = Vec::new();
        if self.derive_opts.derive_debug && !contains_endpoint {
            deriviation.push("Debug")
        }
        if self.derive_opts.derive_show && !contains_endpoint {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq && !contains_endpoint {
            deriviation.push("Eq")
        }

        let derivation = if deriviation.is_empty() {
            String::new()
        } else {
            format!(" derive({})", deriviation.join(", "))
        };

        uwrite!(
            self.src,
            "
            pub(all) struct {name} {{
                {parameters}
            }}{derivation}
            "
        );
    }

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        print_docs(&mut self.src, docs);
        let name = name.to_moonbit_type_ident();

        let mut deriviation: Vec<_> = Vec::new();
        if self.derive_opts.derive_debug {
            deriviation.push("Debug")
        }
        if self.derive_opts.derive_show {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.derive_opts.derive_error && name.contains("Error") {
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

        if self.direction == Direction::Import {
            let (drop_module, drop_name) = self.resolve.wasm_import_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmImport::ResourceIntrinsic {
                    resource: id,
                    interface: self.interface,
                    intrinsic: ResourceIntrinsic::ImportedDrop,
                },
            );
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
                fn wasmImportResourceDrop{name}(resource : Int) = "{drop_module}" "{drop_name}"
                "#,
            )
        } else {
            let (drop_module, drop_name) = self.resolve.wasm_import_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmImport::ResourceIntrinsic {
                    resource: id,
                    interface: self.interface,
                    intrinsic: ResourceIntrinsic::ExportedDrop,
                },
            );
            let (new_module, new_name) = self.resolve.wasm_import_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmImport::ResourceIntrinsic {
                    resource: id,
                    interface: self.interface,
                    intrinsic: ResourceIntrinsic::ExportedNew,
                },
            );
            let (rep_module, rep_name) = self.resolve.wasm_import_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmImport::ResourceIntrinsic {
                    resource: id,
                    interface: self.interface,
                    intrinsic: ResourceIntrinsic::ExportedRep,
                },
            );
            uwrite!(
                &mut self.src,
                r#"
                /// Creates a new resource with the given `rep` as its representation and returning the handle to this resource.
                pub fn {name}::new(rep : Int) -> {name} {{
                    {name}::{name}(wasmExportResourceNew{name}(rep))
                }}
                fn wasmExportResourceNew{name}(rep : Int) -> Int = "{new_module}" "{new_name}"

                /// Drops a resource handle.
                pub fn {name}::drop(self : Self) -> Unit {{
                    let {name}(resource) = self
                    wasmExportResourceDrop{name}(resource)
                }}
                fn wasmExportResourceDrop{name}(resource : Int) = "{drop_module}" "{drop_name}"

                /// Gets the `Int` representation of the resource pointed to the given handle.
                pub fn {name}::rep(self : Self) -> Int {{
                    let {name}(resource) = self
                    wasmExportResourceRep{name}(resource)
                }}
                fn wasmExportResourceRep{name}(resource : Int) -> Int = "{rep_module}" "{rep_name}"
                "#,
            );

            uwrite!(
                &mut self.src,
                r#"
                /// Destructor of the resource.
                declare pub fn {name}::dtor(_self : {name}) -> Unit
                "#
            );

            let func_name = self
                .world_gen
                .export_ns
                .tmp(&format!("wasmExport{name}Dtor"));

            uwrite!(
                self.ffi,
                r#"
                #doc(hidden)
                pub fn {func_name}(handle : Int) -> Unit {{
                    {name}::dtor(handle)
                }}
                "#,
            );

            let export_name = self.resolve.wasm_export_name(
                ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
                WasmExport::ResourceDtor {
                    interface: self.interface.unwrap(),
                    resource: id,
                },
            );

            let export = format!(
                r#"
                #doc(hidden)
                pub fn {func_name}(handle : Int) -> Unit {{
                    {}{func_name}(handle)
                }}
                "#,
                self.world_gen
                    .pkg_resolver
                    .qualify_package(self.world_gen.opts.gen_dir.as_str(), self.name),
            );
            self.world_gen
                .export
                .insert(export_name, (func_name, export));
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
        if self.derive_opts.derive_debug {
            deriviation.push("Debug")
        }
        if self.derive_opts.derive_show {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.derive_opts.derive_error && name.contains("Error") {
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
        // Not needed. They will become `(T1, T2, ...)` in Moonbit
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
                    let ty = self.world_gen.pkg_resolver.type_name(self.name, &ty);
                    format!("{name}({ty})")
                } else {
                    name.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n  ");

        let contains_endpoint = variant
            .cases
            .iter()
            .filter_map(|case| case.ty.as_ref())
            .any(|ty| type_contains_future_or_stream(self.resolve, ty));
        let mut deriviation: Vec<_> = Vec::new();
        if self.derive_opts.derive_debug && !contains_endpoint {
            deriviation.push("Debug")
        }
        if self.derive_opts.derive_show && !contains_endpoint {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq && !contains_endpoint {
            deriviation.push("Eq")
        }
        let declaration = if self.derive_opts.derive_error && name.contains("Error") {
            "suberror"
        } else {
            "enum"
        };

        let derivation = if deriviation.is_empty() {
            String::new()
        } else {
            format!(" derive({})", deriviation.join(", "))
        };

        uwrite!(
            self.src,
            "
            pub(all) {declaration} {name} {{
              {cases}
            }}{derivation}
            "
        );
    }

    fn type_option(&mut self, _id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        // Not needed. They will become `Option[T]` in Moonbit
    }

    fn type_result(&mut self, _id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        // Not needed. They will become `Result[Ok, Err]` in Moonbit
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
        if self.derive_opts.derive_debug {
            deriviation.push("Debug")
        }
        if self.derive_opts.derive_show {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq {
            deriviation.push("Eq")
        }
        let declaration = if self.derive_opts.derive_error && name.contains("Error") {
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

    fn type_alias(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {}

    fn type_list(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        // Not needed. They will become `Array[T]` or `FixedArray[T]` in Moonbit
    }

    fn type_fixed_length_list(
        &mut self,
        _id: TypeId,
        _name: &str,
        _ty: &Type,
        _size: u32,
        _docs: &Docs,
    ) {
        // Not needed. They will become `FixedArray[T]` in Moonbit
    }

    fn type_map(&mut self, _id: TypeId, _name: &str, _key: &Type, _value: &Type, _docs: &Docs) {
        // Not needed. Maps become `Map[K, V]` inline in MoonBit
    }

    fn type_future(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        // Rendered inline by `PkgResolver::type_name`.
    }

    fn type_stream(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        // Rendered inline by `PkgResolver::type_name`.
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unimplemented!();
    }
}

struct Block {
    body: String,
    results: Vec<String>,
}

struct Cleanup {
    address: String,
}

struct BlockStorage {
    body: String,
    cleanup: Vec<Cleanup>,
}

struct FunctionBindgen<'a, 'b> {
    interface_gen: &'b mut InterfaceGenerator<'a>,
    type_context: String,
    func_interface: String,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    payloads: Vec<String>,
    cleanup: Vec<Cleanup>,
    needs_cleanup_list: bool,
    suppress_block_cleanup: bool,
    preserve_guest_allocations: bool,
    sync_endpoint_drop: bool,
    commit_endpoints: bool,
    sync_import_argument_types: Option<Vec<Type>>,
    async_state: AsyncFunctionState,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        r#gen: &'b mut InterfaceGenerator<'a>,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        let mut locals = Ns::default();
        params.iter().for_each(|str| {
            locals.tmp(str);
        });
        let type_context = r#gen.name.to_string();
        Self {
            interface_gen: r#gen,
            func_interface: type_context.clone(),
            type_context,
            params,
            src: String::new(),
            locals,
            block_storage: Vec::new(),
            blocks: Vec::new(),
            payloads: Vec::new(),
            cleanup: Vec::new(),
            needs_cleanup_list: false,
            suppress_block_cleanup: false,
            preserve_guest_allocations: false,
            sync_endpoint_drop: false,
            commit_endpoints: false,
            sync_import_argument_types: None,
            async_state: AsyncFunctionState::default(),
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
                    .map(|result| result.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let payload = if self
                    .interface_gen
                    .world_gen
                    .pkg_resolver
                    .non_empty_type(ty.as_ref())
                    .is_some()
                {
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
        let ty = self.resolve_constructor(ty);
        let lifted = self.locals.tmp("lifted");

        let cases = cases
            .iter()
            .zip(blocks)
            .enumerate()
            .map(|(i, ((case_name, case_ty), Block { body, results, .. }))| {
                let payload = if self
                    .interface_gen
                    .world_gen
                    .pkg_resolver
                    .non_empty_type(case_ty.as_ref())
                    .is_some()
                {
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

    // Utilities
    fn resolve_constructor(&mut self, ty: &Type) -> String {
        self.interface_gen
            .world_gen
            .pkg_resolver
            .type_constructor(&self.type_context, ty)
    }

    fn resolve_type_name(&mut self, ty: &Type) -> String {
        self.interface_gen
            .world_gen
            .pkg_resolver
            .type_name(&self.type_context, ty)
    }

    fn use_ffi(&mut self, str: &'static str) {
        self.interface_gen.ffi_imports.insert(str);
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
            Instruction::I32Const { val } => results.push(format!("({val})")),
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
            | Instruction::F64FromCoreF64
            | Instruction::F32FromCoreF32
            | Instruction::CoreF32FromF32 => results.push(operands[0].clone()),

            Instruction::CharFromI32 => {
                results.push(format!("Int::unsafe_to_char({})", operands[0]))
            }
            Instruction::I32FromChar => results.push(format!("({}).to_int()", operands[0])),

            Instruction::I32FromU8 => results.push(format!("({}).to_int()", operands[0])),
            Instruction::I32FromU16 => {
                results.push(format!("({}).reinterpret_as_int()", operands[0]))
            }
            Instruction::U8FromI32 => results.push(format!("({}).to_byte()", operands[0])),

            Instruction::I32FromS8 => {
                self.use_ffi(ffi::EXTEND8);
                results.push(format!("mbt_ffi_extend8({})", operands[0]))
            }
            Instruction::S8FromI32 => results.push(format!("({} - 0x100)", operands[0])),
            Instruction::S16FromI32 => results.push(format!("({} - 0x10000)", operands[0])),
            Instruction::I32FromS16 => {
                self.use_ffi(ffi::EXTEND16);
                results.push(format!("mbt_ffi_extend16({})", operands[0]))
            }
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
                    let ty = self.resolve_constructor(&Type::Id(*ty));
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
                    let ty = self.resolve_constructor(&Type::Id(*ty));
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
                    let ty = self.resolve_constructor(&Type::Id(*ty));
                    uwriteln!(
                        self.src,
                        r#"
                        let {ty}({flag}) = {op}
                        "#
                    );
                    results.push(format!("({flag}.to_int())"));
                    results.push(format!("({flag} >> 32).to_int())"));
                }
            },

            Instruction::FlagsLift { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 => {
                    results.push(format!(
                        "{}({}.to_byte())",
                        self.resolve_type_name(&Type::Id(*ty)),
                        operands[0]
                    ));
                }
                Int::U16 | Int::U32 => {
                    results.push(format!(
                        "{}({}.reinterpret_as_uint())",
                        self.resolve_type_name(&Type::Id(*ty)),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "{}(({}).reinterpret_as_uint().to_uint64() | (({}).reinterpret_as_uint().to_uint64() << 32))",
                        self.resolve_type_name(&Type::Id(*ty)),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::HandleLower { ty, .. } => {
                let op = &operands[0];
                let handle = self.locals.tmp("handle");
                let ty = self.resolve_constructor(&Type::Id(*ty));
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
                let ty = self.resolve_constructor(&Type::Id(*ty));
                results.push(format!(
                    "{}::{}({})",
                    ty,
                    if ty.starts_with("@") {
                        ty.split('.').next_back().unwrap()
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
                    .map(|(i, op)| format!("{} : {}", record.fields[i].name.to_moonbit_ident(), op))
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!(
                    "{}::{{{ops}}}",
                    self.resolve_type_name(&Type::Id(*ty))
                ));
            }

            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                // Empty tuple is Unit
                // (T) is T
                if tuple.types.is_empty() {
                    results.push("()".into());
                } else if tuple.types.len() == 1 {
                    results.push(operands[0].to_string());
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
                    .map(|lowered| lowered.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let op = &operands[0];

                let block = |Block { body, results, .. }| {
                    let assignments = results
                        .iter()
                        .map(|result| result.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");

                    format!(
                        "{body}
                         ({assignments})"
                    )
                };

                let none = block(none);
                let some = block(some);
                let assignment = if declarations.is_empty() {
                    "".into()
                } else {
                    format!("let ({declarations}) = ")
                };
                uwrite!(
                    self.src,
                    r#"
                    {assignment}match ({op}) {{
                        None => {{
                            {none}
                        }}
                        Some({some_payload}) => {{
                            {some}
                        }}
                    }}
                    "#,
                );
            }

            Instruction::OptionLift { ty, .. } => {
                let some = self.blocks.pop().unwrap();
                let _none = self.blocks.pop().unwrap();

                let ty = self.resolve_type_name(&Type::Id(*ty));
                let lifted = self.locals.tmp("lifted");
                let op = &operands[0];

                let assignment = some.results.first().unwrap();

                let some = some.body;

                uwrite!(
                    self.src,
                    r#"
                    let {lifted} : {ty} = match {op} {{
                        0 => Option::None
                        1 => {{
                            {some}
                            Option::Some({assignment})
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
                self.resolve_type_name(&Type::Id(*ty)),
                operands[0]
            )),

            Instruction::ListCanonLower { element, realloc } => match element {
                Type::U8 => {
                    let op = &operands[0];
                    let ptr = self.locals.tmp("ptr");
                    self.use_ffi(ffi::BYTES2PTR);
                    uwriteln!(
                        self.src,
                        "
                        let {ptr} = mbt_ffi_bytes2ptr({op})
                        ",
                    );
                    results.push(ptr.clone());
                    results.push(format!("{op}.length()"));
                    if realloc.is_none() {
                        self.cleanup.push(Cleanup { address: ptr });
                    }
                }
                Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64 => {
                    let op = &operands[0];
                    let ptr = self.locals.tmp("ptr");
                    let ty = match element {
                        Type::U32 => {
                            self.use_ffi(ffi::UINT_ARRAY2PTR);
                            "uint"
                        }
                        Type::U64 => {
                            self.use_ffi(ffi::UINT64_ARRAY2PTR);
                            "uint64"
                        }
                        Type::S32 => {
                            self.use_ffi(ffi::INT_ARRAY2PTR);
                            "int"
                        }
                        Type::S64 => {
                            self.use_ffi(ffi::INT64_ARRAY2PTR);
                            "int64"
                        }
                        Type::F32 => {
                            self.use_ffi(ffi::FLOAT_ARRAY2PTR);
                            "float"
                        }
                        Type::F64 => {
                            self.use_ffi(ffi::DOUBLE_ARRAY2PTR);
                            "double"
                        }
                        _ => unreachable!(),
                    };

                    uwriteln!(
                        self.src,
                        "
                        let {ptr} = mbt_ffi_{ty}_array2ptr({op})
                        ",
                    );
                    results.push(ptr.clone());
                    results.push(format!("{op}.length()"));
                    if realloc.is_none() {
                        self.cleanup.push(Cleanup { address: ptr });
                    }
                }
                _ => unreachable!("unsupported list element type"),
            },

            Instruction::ListCanonLift { element, .. } => match element {
                Type::U8 => {
                    let result = self.locals.tmp("result");
                    let address = &operands[0];
                    let length = &operands[1];
                    self.use_ffi(ffi::PTR2BYTES);
                    uwrite!(
                        self.src,
                        "
                        let {result} = mbt_ffi_ptr2bytes({address}, {length})
                        ",
                    );

                    results.push(result);
                }
                Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64 => {
                    let ty = match element {
                        Type::U32 => {
                            self.use_ffi(ffi::PTR2UINT_ARRAY);
                            "uint"
                        }
                        Type::U64 => {
                            self.use_ffi(ffi::PTR2UINT64_ARRAY);
                            "uint64"
                        }
                        Type::S32 => {
                            self.use_ffi(ffi::PTR2INT_ARRAY);
                            "int"
                        }
                        Type::S64 => {
                            self.use_ffi(ffi::PTR2INT64_ARRAY);
                            "int64"
                        }
                        Type::F32 => {
                            self.use_ffi(ffi::PTR2FLOAT_ARRAY);
                            "float"
                        }
                        Type::F64 => {
                            self.use_ffi(ffi::PTR2DOUBLE_ARRAY);
                            "double"
                        }
                        _ => unreachable!(),
                    };

                    let result = self.locals.tmp("result");
                    let address = &operands[0];
                    let length = &operands[1];

                    uwrite!(
                        self.src,
                        "
                        let {result} = mbt_ffi_ptr2{ty}_array({address}, {length})
                        ",
                    );

                    results.push(result);
                }
                _ => unreachable!("unsupported list element type"),
            },

            Instruction::StringLower { realloc } => {
                let op = &operands[0];
                let ptr = self.locals.tmp("ptr");

                self.use_ffi(ffi::STR2PTR);
                uwrite!(
                    self.src,
                    "
                    let {ptr} = mbt_ffi_str2ptr({op})
                    ",
                );

                results.push(ptr.clone());
                results.push(format!("{op}.length()"));
                if realloc.is_none() {
                    self.cleanup.push(Cleanup { address: ptr });
                }
            }

            Instruction::StringLift { .. } => {
                let result = self.locals.tmp("result");
                let address = &operands[0];
                let length = &operands[1];

                self.use_ffi(ffi::PTR2STR);
                uwrite!(
                    self.src,
                    "
                    let {result} = mbt_ffi_ptr2str({address}, {length})
                    ",
                );

                results.push(result);
            }

            Instruction::ListLower { element, realloc } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                assert!(block_results.is_empty());

                let op = &operands[0];
                let size = self
                    .interface_gen
                    .world_gen
                    .sizes
                    .size(element)
                    .size_wasm32();
                let _align = self
                    .interface_gen
                    .world_gen
                    .sizes
                    .align(element)
                    .align_wasm32();
                let address = self.locals.tmp("address");
                let ty = self.resolve_type_name(element);
                let index = self.locals.tmp("index");

                self.use_ffi(ffi::MALLOC);
                uwrite!(
                    self.src,
                    "
                    let {address} = mbt_ffi_malloc(({op}).length() * {size});
                    for {index} = 0; {index} < ({op}).length(); {index} = {index} + 1 {{
                        let iter_elem : {ty} = ({op})[({index})]
                        let iter_base = {address} + ({index} * {size});
                        {body}
                    }}
                    ",
                );

                results.push(address.clone());
                results.push(format!("({op}).length()"));

                if realloc.is_none() {
                    self.cleanup.push(Cleanup { address });
                }
            }

            Instruction::ListLift { element, .. } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                let address = &operands[0];
                let length = &operands[1];
                let array = self.locals.tmp("array");
                let ty = self.resolve_type_name(element);
                let size = self
                    .interface_gen
                    .world_gen
                    .sizes
                    .size(element)
                    .size_wasm32();
                // let align = self.r#gen.r#gen.sizes.align(element);
                let index = self.locals.tmp("index");

                let result = match &block_results[..] {
                    [result] => result,
                    _ => todo!("result count == {}", results.len()),
                };

                self.use_ffi(ffi::FREE);
                uwrite!(
                    self.src,
                    "
                    let {array} : Array[{ty}] = [];
                    for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                        let iter_base = ({address}) + ({index} * {size})
                        {body}
                        {array}.push({result})
                    }}
                    mbt_ffi_free({address})
                    ",
                );

                results.push(array);
            }

            Instruction::IterElem { .. } => results.push("iter_elem".into()),

            Instruction::IterBasePointer => results.push("iter_base".into()),

            Instruction::CallWasm { sig, name } => {
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

                let func_name = name.to_upper_camel_case();
                let call_operands = if self.sync_import_argument_types.is_some() {
                    operands
                        .iter()
                        .map(|operand| {
                            let stable = self.locals.tmp("lower_arg");
                            uwriteln!(self.src, "let {stable} = {operand}");
                            stable
                        })
                        .collect::<Vec<_>>()
                } else {
                    operands.clone()
                };
                let arguments = call_operands.join(", ");
                // TODO: handle this to support async functions
                uwriteln!(self.src, "{assignment} wasmImport{func_name}({arguments});");
                self.commit_sync_import_arguments(sig, &call_operands);
            }

            Instruction::CallInterface { func, async_ } => {
                if *async_ {
                    self.emit_async_call_interface(func, operands, results);
                    return;
                }

                let name = self.interface_gen.world_gen.pkg_resolver.func_call(
                    &self.type_context,
                    func,
                    &self.func_interface,
                );

                let args = operands.join(", ");

                let assignment = match func.result {
                    None => "let _ = ".into(),
                    Some(ty) => {
                        let ty = format!("({})", self.resolve_type_name(&ty));
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
                // Bind return operands to locals BEFORE cleanup to avoid
                // use-after-free when operands contain inline loads from
                // return_area or other freed memory.
                let return_locals: Vec<String> = if *amt > 0 {
                    operands
                        .iter()
                        .map(|op| {
                            let local = self.locals.tmp("ret");
                            uwriteln!(self.src, "let {local} = {op}");
                            local
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                if !self.cleanup.is_empty() || self.needs_cleanup_list {
                    self.use_ffi(ffi::FREE);
                }
                for clean in &self.cleanup {
                    let address = &clean.address;
                    uwriteln!(self.src, "mbt_ffi_free({address})",);
                }

                if self.needs_cleanup_list {
                    uwrite!(
                        self.src,
                        "
                        cleanup_list.each(mbt_ffi_free)
                        ",
                    );
                }

                match *amt {
                    0 => (),
                    1 => uwriteln!(self.src, "return {}", return_locals[0]),
                    _ => {
                        let results = return_locals.join(", ");
                        uwriteln!(self.src, "return ({results})");
                    }
                }
            }

            Instruction::I32Load { offset }
            | Instruction::PointerLoad { offset }
            | Instruction::LengthLoad { offset } => {
                self.use_ffi(ffi::LOAD32);
                results.push(format!(
                    "mbt_ffi_load32(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load8U { offset } => {
                self.use_ffi(ffi::LOAD8_U);
                results.push(format!(
                    "mbt_ffi_load8_u(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load8S { offset } => {
                self.use_ffi(ffi::LOAD8);
                results.push(format!(
                    "mbt_ffi_load8(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load16U { offset } => {
                self.use_ffi(ffi::LOAD16_U);
                results.push(format!(
                    "mbt_ffi_load16_u(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load16S { offset } => {
                self.use_ffi(ffi::LOAD16);
                results.push(format!(
                    "mbt_ffi_load16(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I64Load { offset } => {
                self.use_ffi(ffi::LOAD64);
                results.push(format!(
                    "mbt_ffi_load64(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::F32Load { offset } => {
                self.use_ffi(ffi::LOADF32);
                results.push(format!(
                    "mbt_ffi_loadf32(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::F64Load { offset } => {
                self.use_ffi(ffi::LOADF64);
                results.push(format!(
                    "mbt_ffi_loadf64(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => {
                self.use_ffi(ffi::STORE32);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store32(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I32Store8 { offset } => {
                self.use_ffi(ffi::STORE8);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store8(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I32Store16 { offset } => {
                self.use_ffi(ffi::STORE16);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store16(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I64Store { offset } => {
                self.use_ffi(ffi::STORE64);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store64(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::F32Store { offset } => {
                self.use_ffi(ffi::STOREF32);
                uwriteln!(
                    self.src,
                    "mbt_ffi_storef32(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::F64Store { offset } => {
                self.use_ffi(ffi::STOREF64);
                uwriteln!(
                    self.src,
                    "mbt_ffi_storef64(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }
            // TODO: see what we can do with align
            Instruction::Malloc { size, .. } => {
                self.use_ffi(ffi::MALLOC);
                uwriteln!(self.src, "mbt_ffi_malloc({})", size.size_wasm32())
            }

            Instruction::GuestDeallocate { .. } => {
                if !self.preserve_guest_allocations {
                    self.use_ffi(ffi::FREE);
                    uwriteln!(self.src, "mbt_ffi_free({})", operands[0])
                }
            }

            Instruction::GuestDeallocateString => {
                if !self.preserve_guest_allocations {
                    self.use_ffi(ffi::FREE);
                    uwriteln!(self.src, "mbt_ffi_free({})", operands[0])
                }
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
                let Block { body, results, .. } = self.blocks.pop().unwrap();
                assert!(results.is_empty());

                let address = &operands[0];
                let length = &operands[1];

                let size = self
                    .interface_gen
                    .world_gen
                    .sizes
                    .size(element)
                    .size_wasm32();
                // let align = self.r#gen.r#gen.sizes.align(element);

                if !body.trim().is_empty() {
                    let index = self.locals.tmp("index");

                    uwrite!(
                        self.src,
                        "
                        for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                            let iter_base = ({address}) + ({index} * {size})
                            {body}
                        }}
                        "
                    );
                }

                if !self.preserve_guest_allocations {
                    self.use_ffi(ffi::FREE);
                    uwriteln!(self.src, "mbt_ffi_free({address})",);
                }
            }

            Instruction::Flush { amt } => {
                results.extend(operands.iter().take(*amt).cloned());
            }

            Instruction::FutureLift { ty, .. } => {
                self.emit_future_lift(*ty, operands, results);
            }

            Instruction::FutureLower { ty, .. } => {
                self.emit_future_lower(*ty, operands, results);
            }

            Instruction::AsyncTaskReturn { params, .. } => {
                self.capture_task_return(params, operands);
            }

            Instruction::StreamLower { ty, .. } => {
                self.emit_stream_lower(*ty, operands, results);
            }

            Instruction::StreamLift { ty, .. } => {
                self.emit_stream_lift(*ty, operands, results);
            }
            Instruction::DropHandle { ty } => {
                let is_endpoint = match ty {
                    Type::Id(id) => matches!(
                        &self.interface_gen.resolve.types[*id].kind,
                        TypeDefKind::Future(_) | TypeDefKind::Stream(_)
                    ),
                    _ => false,
                };
                if !self.commit_endpoints {
                    let method = if self.sync_endpoint_drop && is_endpoint {
                        "drop_sync"
                    } else {
                        "drop"
                    };
                    uwriteln!(self.src, "{}.{method}()", operands[0]);
                }
            }
            Instruction::ErrorContextLower { .. } | Instruction::ErrorContextLift { .. } => todo!(),
            Instruction::FixedLengthListLift {
                element: _,
                size,
                id: _,
            } => {
                let array = self.locals.tmp("array");
                let mut elements = String::new();
                for a in operands.drain(0..(*size as usize)) {
                    elements.push_str(&a);
                    elements.push_str(", ");
                }
                uwriteln!(self.src, "let {array} : FixedArray[_] = [{elements}]");
                results.push(array);
            }
            Instruction::FixedLengthListLower {
                element: _,
                size,
                id: _,
            } => {
                uwriteln!(
                    self.src,
                    "if ({}).length() != {size} {{ panic() }}",
                    operands[0]
                );
                for i in 0..(*size as usize) {
                    results.push(format!("({})[{i}]", operands[0]));
                }
            }
            Instruction::FixedLengthListLowerToMemory {
                element,
                size: fixed_length,
                id: _,
            } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                assert!(block_results.is_empty());

                let vec = operands[0].clone();
                let target = operands[1].clone();
                let size = self.sizes().size(element).size_wasm32();
                let index = self.locals.tmp("index");

                uwrite!(
                    self.src,
                    "
                    if ({vec}).length() != {fixed_length} {{ panic() }}
                    for {index} = 0; {index} < {fixed_length}; {index} = {index} + 1 {{
                        let iter_elem = ({vec})[{index}]
                        let iter_base = ({target}) + ({index} * {size})
                        {body}
                    }}
                    ",
                );
            }
            Instruction::FixedLengthListLiftFromMemory {
                element,
                size: fll_size,
                id: _,
            } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                let address = &operands[0];
                let array = self.locals.tmp("array");
                let ty = self.resolve_type_name(element);
                let elem_size = self.sizes().size(element).size_wasm32();
                let index = self.locals.tmp("index");

                let result = match &block_results[..] {
                    [result] => result,
                    _ => todo!("result count == {}", block_results.len()),
                };

                uwrite!(
                    self.src,
                    "
                    let {array} : Array[{ty}] = []
                    for {index} = 0; {index} < {fll_size}; {index} = {index} + 1 {{
                        let iter_base = ({address}) + ({index} * {elem_size})
                        {body}
                        {array}.push({result})
                    }}
                    ",
                );

                results.push(format!("FixedArray::from_array({array}[:])"));
            }

            Instruction::MapLower {
                key,
                value,
                realloc,
            } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                assert!(block_results.is_empty());

                let op = &operands[0];
                let entry = self.interface_gen.world_gen.sizes.record([*key, *value]);
                let size = entry.size.size_wasm32();
                let address = self.locals.tmp("address");
                let index = self.locals.tmp("index");
                let iter_map_key = self.locals.tmp("iter_map_key");
                let iter_map_value = self.locals.tmp("iter_map_value");

                self.use_ffi(ffi::MALLOC);
                uwrite!(
                    self.src,
                    "
                    let {address} = mbt_ffi_malloc(({op}).length() * {size});
                    let mut {index} = 0
                    ({op}).each(fn({iter_map_key}, {iter_map_value}) {{
                        let iter_map_key = {iter_map_key}
                        let iter_map_value = {iter_map_value}
                        let iter_base = {address} + ({index} * {size})
                        {body}
                        {index} = {index} + 1
                    }})
                    ",
                );

                results.push(address.clone());
                results.push(format!("({op}).length()"));

                if realloc.is_none() {
                    self.cleanup.push(Cleanup { address });
                }
            }

            Instruction::MapLift { key, value, .. } => {
                let Block {
                    body,
                    results: block_results,
                } = self.blocks.pop().unwrap();
                let address = &operands[0];
                let length = &operands[1];
                let map = self.locals.tmp("map");
                let key_ty = self.resolve_type_name(key);
                let value_ty = self.resolve_type_name(value);
                let entry = self.interface_gen.world_gen.sizes.record([*key, *value]);
                let size = entry.size.size_wasm32();
                let index = self.locals.tmp("index");

                let (body_key, body_value) = match &block_results[..] {
                    [k, v] => (k, v),
                    _ => todo!(
                        "expected 2 results from map lift block, got {}",
                        block_results.len()
                    ),
                };

                self.use_ffi(ffi::FREE);
                uwrite!(
                    self.src,
                    "
                    let {map} : Map[{key_ty}, {value_ty}] = {{}}
                    for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                        let iter_base = ({address}) + ({index} * {size})
                        {body}
                        {map}[{body_key}] = {body_value}
                    }}
                    mbt_ffi_free({address})
                    ",
                );

                results.push(map);
            }

            Instruction::IterMapKey { .. } => results.push("iter_map_key".into()),

            Instruction::IterMapValue { .. } => results.push("iter_map_value".into()),

            Instruction::GuestDeallocateMap { key, value } => {
                let Block { body, results, .. } = self.blocks.pop().unwrap();
                assert!(results.is_empty());

                let address = &operands[0];
                let length = &operands[1];

                let entry = self.interface_gen.world_gen.sizes.record([*key, *value]);
                let size = entry.size.size_wasm32();

                if !body.trim().is_empty() {
                    let index = self.locals.tmp("index");

                    uwrite!(
                        self.src,
                        "
                        for {index} = 0; {index} < ({length}); {index} = {index} + 1 {{
                            let iter_base = ({address}) + ({index} * {size})
                            {body}
                        }}
                        "
                    );
                }

                if !self.preserve_guest_allocations {
                    self.use_ffi(ffi::FREE);
                    uwriteln!(self.src, "mbt_ffi_free({address})",);
                }
            }
        }
    }

    fn return_pointer(&mut self, size: ArchitectureSize, _align: Alignment) -> String {
        self.use_ffi(ffi::MALLOC);
        let address = self.locals.tmp("return_area");
        uwriteln!(
            self.src,
            "let {address} = mbt_ffi_malloc({})",
            size.size_wasm32(),
        );
        // If the interface is an import, we need to track this for cleanup
        // Otherwise, the caller is responsible for cleaning up in post_return
        if self.interface_gen.direction == Direction::Import {
            self.cleanup.push(Cleanup {
                address: address.clone(),
            });
        }
        address
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            cleanup: mem::take(&mut self.cleanup),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage { body, cleanup } = self.block_storage.pop().unwrap();

        if !self.cleanup.is_empty() && !self.suppress_block_cleanup {
            self.needs_cleanup_list = true;
            self.use_ffi(ffi::FREE);

            for cleanup in &self.cleanup {
                let address = &cleanup.address;
                uwriteln!(self.src, "cleanup_list.push({address})",);
            }
        }

        self.cleanup = cleanup;

        self.blocks.push(Block {
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.interface_gen.world_gen.sizes
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

fn type_contains_future_or_stream(resolve: &Resolve, ty: &Type) -> bool {
    let mut live = LiveTypes::default();
    live.add_type(resolve, ty);
    live.iter().any(|id| {
        matches!(
            resolve.types[id].kind,
            TypeDefKind::Future(_) | TypeDefKind::Stream(_)
        )
    })
}

fn type_contains_endpoint_fixed_length_list_combination(
    resolve: &Resolve,
    ty: &Type,
    inside_fixed_length_list: bool,
    inside_endpoint: bool,
) -> bool {
    let Type::Id(id) = ty else {
        return false;
    };

    match &resolve.types[*id].kind {
        TypeDefKind::Future(payload) | TypeDefKind::Stream(payload) => {
            inside_fixed_length_list
                || payload.as_ref().is_some_and(|ty| {
                    type_contains_endpoint_fixed_length_list_combination(resolve, ty, false, true)
                })
        }
        TypeDefKind::FixedLengthList(ty, _) => {
            inside_endpoint
                || type_contains_endpoint_fixed_length_list_combination(resolve, ty, true, false)
        }
        TypeDefKind::Record(record) => record.fields.iter().any(|field| {
            type_contains_endpoint_fixed_length_list_combination(
                resolve,
                &field.ty,
                inside_fixed_length_list,
                inside_endpoint,
            )
        }),
        TypeDefKind::Tuple(tuple) => tuple.types.iter().any(|ty| {
            type_contains_endpoint_fixed_length_list_combination(
                resolve,
                ty,
                inside_fixed_length_list,
                inside_endpoint,
            )
        }),
        TypeDefKind::Variant(variant) => variant
            .cases
            .iter()
            .filter_map(|case| case.ty.as_ref())
            .any(|ty| {
                type_contains_endpoint_fixed_length_list_combination(
                    resolve,
                    ty,
                    inside_fixed_length_list,
                    inside_endpoint,
                )
            }),
        TypeDefKind::Option(ty) | TypeDefKind::List(ty) | TypeDefKind::Type(ty) => {
            type_contains_endpoint_fixed_length_list_combination(
                resolve,
                ty,
                inside_fixed_length_list,
                inside_endpoint,
            )
        }
        TypeDefKind::Map(key, value) => {
            type_contains_endpoint_fixed_length_list_combination(
                resolve,
                key,
                inside_fixed_length_list,
                inside_endpoint,
            ) || type_contains_endpoint_fixed_length_list_combination(
                resolve,
                value,
                inside_fixed_length_list,
                inside_endpoint,
            )
        }
        TypeDefKind::Result(result) => result.ok.iter().chain(result.err.iter()).any(|ty| {
            type_contains_endpoint_fixed_length_list_combination(
                resolve,
                ty,
                inside_fixed_length_list,
                inside_endpoint,
            )
        }),
        TypeDefKind::Resource
        | TypeDefKind::Handle(_)
        | TypeDefKind::Flags(_)
        | TypeDefKind::Enum(_) => false,
        TypeDefKind::Unknown => unreachable!(),
    }
}

fn world_contains_endpoint_fixed_length_list_combination(
    resolve: &Resolve,
    world: WorldId,
) -> bool {
    let mut live = LiveTypes::default();
    live.add_world(resolve, world);
    live.iter().any(|id| {
        type_contains_endpoint_fixed_length_list_combination(resolve, &Type::Id(id), false, false)
    })
}

fn world_contains_future_or_stream(resolve: &Resolve, world: WorldId) -> bool {
    let mut live = LiveTypes::default();
    live.add_world(resolve, world);
    live.iter().any(|id| {
        matches!(
            resolve.types[id].kind,
            TypeDefKind::Future(_) | TypeDefKind::Stream(_)
        )
    })
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

fn print_docs(src: &mut String, docs: &Docs) {
    uwrite!(src, "///|");
    if let Some(docs) = &docs.contents {
        for line in docs.trim().lines() {
            uwrite!(src, "\n/// {line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn try_generate_with_opts(wit: &str, world: &str, opts: Opts) -> Result<Files> {
        let mut resolve = Resolve::default();
        let pkg = resolve.push_str("test.wit", wit).unwrap();
        let world = resolve.select_world(&[pkg], Some(world)).unwrap();
        let mut files = Files::default();
        let mut generator = MoonBit {
            opts,
            ..MoonBit::default()
        };
        generator.generate(&mut resolve, world, &mut files)?;
        Ok(files)
    }

    fn try_generate(wit: &str, world: &str) -> Result<Files> {
        try_generate_with_opts(
            wit,
            world,
            Opts {
                gen_dir: "gen".into(),
                ..Opts::default()
            },
        )
    }

    fn generate(wit: &str, world: &str) -> Files {
        try_generate(wit, world).unwrap()
    }

    fn file<'a>(files: &'a Files, path: &str) -> &'a str {
        let contents = files
            .iter()
            .find_map(|(name, contents)| (name == path).then_some(contents))
            .unwrap_or_else(|| {
                let names = files
                    .iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>()
                    .join(", ");
                std::panic!("missing generated file `{path}`; generated: {names}")
            });
        std::str::from_utf8(contents).unwrap()
    }

    #[test]
    fn endpoint_free_sync_generation_matches_golden() {
        let files = generate(
            r#"
            package a:b;

            world runner {
                import add: func(a: u32, b: u32) -> u32;
                export echo: func(value: u32) -> u32;
            }
            "#,
            "runner",
        );

        // This exact-output fingerprint runs with both async adapters. Ignore only
        // the release-version preamble, which is unrelated to generated ABI.
        let mut entries = files.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(name, _)| *name);
        let fingerprints = entries
            .into_iter()
            .map(|(name, contents)| {
                let contents = if contents.starts_with(b"// Generated by") {
                    let preamble_end = contents.iter().position(|byte| *byte == b'\n').unwrap() + 1;
                    &contents[preamble_end..]
                } else {
                    contents
                };
                let hash = contents
                    .iter()
                    .fold(0xcbf29ce484222325_u64, |mut hash, byte| {
                        hash ^= u64::from(*byte);
                        hash.wrapping_mul(0x100000001b3)
                    });
                (name.to_string(), hash)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            fingerprints,
            vec![
                ("gen/ffi.mbt".into(), 10220319382745692950),
                ("gen/moon.pkg.json".into(), 15894505084782869543),
                ("gen/world/runner/ffi.mbt".into(), 14715999128234894449),
                ("gen/world/runner/moon.pkg.json".into(), 6361049410124596525,),
                ("gen/world/runner/top.mbt".into(), 12192865914091673515,),
                ("moon.mod.json".into(), 14111159726816684443),
                ("world/runner/ffi_import.mbt".into(), 17812050158059242657,),
                ("world/runner/import.mbt".into(), 5430383198437179961),
                ("world/runner/moon.pkg.json".into(), 6361049410124596525,),
            ]
        );
    }

    #[test]
    fn sync_world_does_not_emit_async_runtime_or_wrappers() {
        let files = generate(
            r#"
            package a:b;

            world runner {
                import add: func(a: u32, b: u32) -> u32;
            }
            "#,
            "runner",
        );

        let import = file(&files, "world/runner/import.mbt");
        assert!(import.contains("pub fn add("));
        for async_name in [
            "pub async fn",
            "with_waitableset",
            "TaskGroup",
            "background_group",
            "CMFuture",
            "CMStream",
        ] {
            assert!(!import.contains(async_name));
        }
        assert!(
            files
                .iter()
                .all(|(name, _)| !name.starts_with("async-core/async_"))
        );
    }

    #[test]
    fn async_filters_respect_import_export_direction() {
        let wit = r#"
            package a:b;
            world runner { import run: func(); }
        "#;

        let mut import_opts = Opts {
            gen_dir: "gen".into(),
            ..Opts::default()
        };
        import_opts.async_.push("import:run");
        let import_files = try_generate_with_opts(wit, "runner", import_opts).unwrap();
        let import = file(&import_files, "world/runner/import.mbt");
        let import_ffi = file(&import_files, "world/runner/ffi_import.mbt");
        assert!(import.contains("pub async fn run("), "{import}");
        assert!(import_ffi.contains("[async-lower]run"), "{import_ffi}");

        let mut export_opts = Opts {
            gen_dir: "gen".into(),
            ..Opts::default()
        };
        export_opts.async_.push("export:run");
        let export_files = try_generate_with_opts(wit, "runner", export_opts).unwrap();
        let import = file(&export_files, "world/runner/import.mbt");
        let import_ffi = file(&export_files, "world/runner/ffi_import.mbt");
        assert!(import.contains("pub fn run("), "{import}");
        assert!(!import.contains("pub async fn run("), "{import}");
        assert!(!import_ffi.contains("[async-lower]run"), "{import_ffi}");
    }

    #[test]
    fn async_import_indirect_params_emit_malloc_builtin() {
        let files = generate(
            r#"
            package a:b;

            interface types {
                resource descriptor {
                    run: async func(first: string, second: list<s32>);
                }
            }

            world bindings { import types; }
            "#,
            "bindings",
        );

        let ffi = file(&files, "interface/a/b/types/ffi.mbt");
        assert!(ffi.contains("extern \"wasm\" fn mbt_ffi_malloc"), "{ffi}");
    }

    #[test]
    fn type_only_endpoints_emit_async_runtime() {
        let files = generate(
            r#"
            package a:b;

            interface types {
                record payload { ready: future<u32> }
            }

            world runner { import types; }
            "#,
            "runner",
        );

        let source = file(&files, "interface/a/b/types/top.mbt");
        assert!(source.contains("@async-core.Future[UInt]"), "{source}");
        file(&files, "async-core/moon.pkg.json");
        file(&files, "async-core/async_trait.mbt");
    }

    #[test]
    fn endpoint_container_types_omit_value_derives() {
        let mut resolve = Resolve::default();
        let pkg = resolve
            .push_str(
                "test.wit",
                r#"
                package a:b;

                interface types {
                    record plain { value: u32 }
                    record nested { value: option<future<u32>> }
                    variant streamed { none, value(list<stream<string>>) }
                    use-types: func(a: plain, b: nested, c: streamed);
                }

                world runner { import types; }
                "#,
            )
            .unwrap();
        let world = resolve.select_world(&[pkg], Some("runner")).unwrap();
        let mut files = Files::default();
        let mut generator = MoonBit {
            opts: Opts {
                derive: DeriveOpts {
                    derive_debug: true,
                    derive_show: true,
                    derive_eq: true,
                    derive_error: false,
                },
                gen_dir: "gen".into(),
                ..Opts::default()
            },
            ..MoonBit::default()
        };
        generator.generate(&mut resolve, world, &mut files).unwrap();

        let source = file(&files, "interface/a/b/types/top.mbt");
        assert!(
            source.contains("struct Plain {\n      value : UInt\n} derive(Debug, Show, Eq)"),
            "{source}"
        );
        assert!(
            source.contains("struct Nested {\n      value : @async-core.Future[UInt]?\n}"),
            "{source}"
        );
        assert!(
            !source.contains("struct Nested {\n      value : @async-core.Future[UInt]?\n} derive(")
        );
        assert!(
            source.contains(
                "Streamed {\n      None\n      Value(Array[@async-core.Stream[String]])\n}"
            ),
            "{source}"
        );
        assert!(!source.contains(
            "Streamed {\n      None\n      Value(Array[@async-core.Stream[String]])\n} derive("
        ));
    }

    #[test]
    fn sync_functions_with_endpoints_remain_sync() {
        let files = generate(
            r#"
            package a:b;

            world runner {
                import exchange: func(
                    input: stream<u8>,
                    ready: future<u32>,
                ) -> tuple<stream<u8>, future<u32>>;
            }
            "#,
            "runner",
        );

        let import = file(&files, "world/runner/import.mbt");
        assert!(import.contains("pub fn exchange("));
        assert!(import.contains("@async-core.Stream[Byte]"));
        assert!(import.contains("@async-core.Future[UInt]"));
        assert!(!import.contains("pub async fn exchange"));
        assert!(!import.contains("background_group"));
        let call = import.find("wasmImportExchange").unwrap();
        let after_call = &import[call..];
        assert!(after_call.contains("StreamCommit"), "{import}");
        assert!(after_call.contains("FutureCommit"), "{import}");
        assert_eq!(import.matches("StreamLower(input)").count(), 1, "{import}");
        assert_eq!(import.matches("FutureLower(ready)").count(), 1, "{import}");
        assert!(
            files
                .iter()
                .any(|(name, _)| name == "async-core/async_trait.mbt")
        );
    }

    #[test]
    fn async_export_background_group_name_is_deconflicted() {
        let files = generate(
            r#"
            package a:b;
            world service {
                export handle: async func(background-group: u32);
            }
            "#,
            "service",
        );
        let public = file(&files, "gen/world/service/top.mbt");
        assert!(
            public.contains(
                "background_group : UInt, background_group0 : @async-core.TaskGroup[Unit]"
            ),
            "{public}"
        );
        let wrapper = file(&files, "gen/world/service/ffi.mbt");
        assert!(
            wrapper.contains("with_task_group(async fn(background_group0)"),
            "{wrapper}"
        );
        assert!(
            wrapper.contains("handle((p0).reinterpret_as_uint(), background_group0)"),
            "{wrapper}"
        );
    }

    #[test]
    fn async_export_surface_hides_component_model_bridge_types() {
        let files = generate(
            r#"
            package a:b;

            interface handler {
                handle: async func(
                    input: stream<u8>,
                    ready: future<u32>,
                ) -> tuple<stream<u8>, future<u32>>;
            }

            world service {
                export handler;
            }
            "#,
            "service",
        );

        let public = file(&files, "gen/interface/a/b/handler/top.mbt");
        assert!(public.contains("input : @async-core.Stream[Byte]"));
        assert!(public.contains("ready : @async-core.Future[UInt]"));
        assert!(public.contains("background_group : @async-core.TaskGroup[Unit]"));
        for internal_name in [
            "CMFuture",
            "CMStream",
            "VTable",
            "take_cm_handle",
            "take_producer",
            "from_callbacks",
        ] {
            assert!(!public.contains(internal_name));
        }

        let wrapper = file(&files, "gen/interface/a/b/handler/ffi.mbt");
        assert!(wrapper.contains("with_task_group(async fn(background_group)"));
        let root_wrapper = file(&files, "gen/ffi.mbt");
        for ffi in [wrapper, root_wrapper] {
            let lines = ffi.lines().collect::<Vec<_>>();
            for (index, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("pub fn wasmExport") {
                    assert_eq!(lines[index - 1].trim(), "#doc(hidden)", "{ffi}");
                }
            }
        }

        let coroutine = file(&files, "async-core/async_coroutine.mbt");
        assert!(!coroutine.contains("fn pause()"));
        assert!(!coroutine.contains("wait_until"));
        let task = file(&files, "async-core/async_task.mbt");
        assert!(task.contains("pub struct Task[X] {\n  priv value : Ref[X?]"));
        let promise = file(&files, "async-core/async_promise.mbt");
        assert!(promise.contains("pub struct Promise[X]"));
        assert!(promise.contains("pub fn[X] Future::new()"));
        assert!(promise.contains("pub fn[X] Promise::complete"));
        let semaphore = file(&files, "async-core/async_semaphore.mbt");
        assert!(semaphore.contains("pub struct Semaphore"));
        assert!(semaphore.contains("pub async fn Semaphore::acquire"));
        let cond_var = file(&files, "async-core/async_cond_var.mbt");
        assert!(cond_var.contains("pub struct CondVar"));
        assert!(cond_var.contains("pub async fn CondVar::wait"));
        let mutex = file(&files, "async-core/async_mutex.mbt");
        assert!(mutex.contains("pub struct Mutex"));
        assert!(mutex.contains("pub async fn Mutex::acquire"));

        let async_core = files
            .iter()
            .filter(|(name, _)| name.starts_with("async-core/"))
            .map(|(_, contents)| String::from_utf8_lossy(contents))
            .collect::<Vec<_>>()
            .join("\n");
        for hidden in [
            "#doc(hidden)\npub fn with_waitableset",
            "#doc(hidden)\npub fn cb",
            "#doc(hidden)\npub fn spawn_component_task_current",
            "#doc(hidden)\npub fn has_component_task_scope",
            "#doc(hidden)\npub async fn suspend_for_subtask",
            "#doc(hidden)\npub async fn suspend_for_future_read",
            "#doc(hidden)\npub async fn suspend_for_future_write_terminal",
            "#doc(hidden)\npub async fn suspend_for_stream_read",
            "#doc(hidden)\npub async fn suspend_for_stream_write",
        ] {
            assert!(
                async_core.contains(hidden),
                "runtime helper is public: {hidden}"
            );
        }
        for raw in [
            "pub extern \"wasm\" fn malloc",
            "pub extern \"wasm\" fn load32",
            "pub fn context_set",
            "pub fn context_get",
            "pub fn task_cancel",
            "pub fn backpressure_inc",
            "pub fn backpressure_dec",
            "[backpressure-inc]",
            "[backpressure-dec]",
            "pub fn current_coroutine",
            "pub fn detach_waitable",
            "pub fn has_immediately_ready_task",
            "pub fn no_more_work",
            "pub fn reschedule",
            "pub fn spawn(",
            "pub fn spawn_bg_current",
            "pub async fn suspend()",
        ] {
            assert!(
                !async_core.contains(raw),
                "raw async-core API leaked: {raw}"
            );
        }
    }

    #[test]
    fn nested_endpoints_use_static_boundary_helpers() {
        let files = generate(
            r#"
            package test:moonbit-nested;

            interface nested {
                relay: async func(
                    value: future<future<stream<u8>>>,
                ) -> future<future<stream<u8>>>;
                relay-stream: async func(
                    value: stream<future<u8>>,
                ) -> stream<future<u8>>;
            }

            world service { export nested; }
            "#,
            "service",
        );

        let ffi = file(&files, "gen/interface/test/moonbit-nested/nested/ffi.mbt");
        for site in [
            "RelayStream0StreamSource",
            "RelayFuture1FutureSource",
            "RelayFuture2FutureSource",
            "RelayStream3StreamLower",
            "RelayFuture4FutureLower",
            "RelayFuture5FutureLower",
        ] {
            assert!(ffi.contains(site), "missing static endpoint site {site}");
        }
        assert!(ffi.contains("[async-lower][future-read-2]relay"));
        assert!(ffi.contains("[async-lower][future-write-2]relay"));
        assert!(ffi.contains("suspend_for_future_write_terminal"));
        assert!(ffi.contains("let data_len = if data.length() < 1"));
        assert!(ffi.contains("let writer_lock = @async-core.Mutex()"));
        assert!(ffi.contains("writer_lock.acquire()"));
        assert!(ffi.contains("defer writer_lock.release()"));
        assert!(ffi.contains("read_cleanup : @async-core.CondVar"));
        assert!(ffi.contains("self.read_cleanup.broadcast()"));
        assert!(ffi.contains("@async-core.cancel_future_read("));
        assert!(ffi.contains("@async-core.cancel_stream_read("));
        assert!(ffi.contains("@async-core.cancel_stream_write("));
        assert!(ffi.contains("[stream-cancel-write-"));
        assert!(!ffi.contains("suspend_for_future_cancel_read"));
        assert!(!ffi.contains("wait_until"));
        assert!(ffi.contains("RelayFuture5FutureLowerCommitted"));
        assert!(ffi.contains("RelayFuture4FutureRejectPrepared"));
        assert!(ffi.contains("RelayStream3StreamRejectPrepared"));
        assert!(!ffi.contains("RelayFuture5FutureRejectPrepared"));

        let generated = files
            .iter()
            .map(|(name, contents)| format!("{name}\n{}", String::from_utf8_lossy(contents)))
            .collect::<Vec<_>>()
            .join("\n");
        for legacy in [
            "async_cm.mbt",
            "CMFutureVTable",
            "CMStreamVTable",
            "take_cm_handle",
            "new_cm_future",
            "new_cm_stream",
        ] {
            assert!(
                !generated.contains(legacy),
                "legacy bridge leaked: {legacy}"
            );
        }

        let imports = generate(
            r#"
            package test:moonbit-nested;

            interface nested {
                relay: async func(
                    value: future<future<stream<u8>>>,
                ) -> future<future<stream<u8>>>;
                relay-stream: async func(
                    value: stream<future<u8>>,
                ) -> stream<future<u8>>;
            }

            world client { import nested; }
            "#,
            "client",
        );
        let import = file(&imports, "interface/test/moonbit-nested/nested/top.mbt");
        assert!(import.contains("if before_started"));
        assert!(import.contains(".drop_sync()"));
        assert!(import.contains("defer mbt_ffi_free(_result_ptr)"));
        assert_eq!(import.matches("FutureLower(value)").count(), 1, "{import}");
        assert_eq!(import.matches("StreamLower(value)").count(), 1, "{import}");
        assert!(!import.contains("cleanup_list"));
    }

    #[test]
    fn fixed_length_lists_use_checked_fixed_arrays_but_reject_nested_endpoints() {
        let files = generate(
            r#"
            package test:fixed-array;

            interface api {
                type pair = list<u32, 2>;
                type block = list<u32, 20>;
                accept: func(value: pair) -> pair;
                accept-block: func(value: block);
            }

            world client { import api; }
            "#,
            "client",
        );
        let top = file(&files, "interface/test/fixed-array/api/top.mbt");
        assert!(top.contains("FixedArray[UInt]"), "{top}");
        assert!(top.contains(".length() != 2"), "{top}");
        assert!(top.contains(".length() != 20"), "{top}");

        for unsupported in [
            "type unsupported = list<future<u32>, 1>;",
            "type unsupported = future<list<u32, 1>>;",
        ] {
            let wit = format!(
                r#"
                package test:fixed-endpoints;

                interface api {{
                    {unsupported}
                    accept: func(value: unsupported);
                }}

                world client {{ import api; }}
                "#
            );
            let error = match try_generate(&wit, "client") {
                Ok(_) => std::panic!(
                    "fixed-length list and async endpoint combinations must be rejected"
                ),
                Err(error) => error,
            };
            assert!(
                error
                    .to_string()
                    .contains("combining future or stream types with fixed-length lists"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn cancelled_returned_async_import_recursively_cleans_result() {
        let files = generate(
            r#"
            package test:cancelled-result;

            interface api {
                resource leaf;
                record payload {
                    label: string,
                    leaves: list<leaf>,
                    ready: future<leaf>,
                }
                load: async func() -> payload;
            }

            world client { import api; }
            "#,
            "client",
        );

        let import = file(&files, "interface/test/cancelled-result/api/top.mbt");
        assert!(
            import.contains("suspend_for_subtask")
                && import.contains("fn() {")
                && import.contains(".drop_sync()")
                && import.contains(".drop()"),
            "{import}"
        );
        assert!(
            import.contains("mbt_ffi_free(mbt_ffi_load32((_result_ptr) + 0))")
                && import.contains("mbt_ffi_free(mbt_ffi_load32((_result_ptr) + 8))")
                && import.contains("defer mbt_ffi_free(_result_ptr)"),
            "cancelled returned result must free its strings, lists, and result area: {import}"
        );
    }

    #[test]
    fn async_runtime_traps_unhandled_export_failure() {
        let files = generate(
            r#"
            package test:failing-export;
            world service { export run: async func(); }
            "#,
            "service",
        );

        let event_loop = file(&files, "async-core/async_ev.mbt");
        assert!(
            event_loop.contains("abort(\"async export failed before task return\")")
                && event_loop.contains("if !(ev.resolved.get(waitable_set) is Some(true))"),
            "{event_loop}"
        );
    }

    #[test]
    fn async_runtime_borrows_waitable_poll_payload() {
        let files = generate(
            r#"
            package test:poll-payload;
            world service { export run: async func(); }
            "#,
            "service",
        );

        let abi = file(&files, "async-core/async_abi.mbt");
        assert!(
            abi.contains("#borrow(array)\nextern \"wasm\" fn int_array2ptr"),
            "{abi}"
        );
        assert!(!abi.contains("#owned(array)"), "{abi}");
    }

    #[test]
    fn async_runtime_uses_baseline_subtask_cancel() {
        let files = generate(
            r#"
            package test:subtask-cancel;
            world service { export run: async func(); }
            "#,
            "service",
        );

        let abi = file(&files, "async-core/async_abi.mbt");
        assert!(
            abi.contains(r#""$root" "[subtask-cancel]""#)
                && !abi.contains("[async-lower][subtask-cancel]"),
            "{abi}"
        );
    }

    #[test]
    fn unit_future_intrinsics_use_wit_parser_unit_name() {
        let files = generate(
            r#"
            package a:b;

            world runner {
                import exchange: func(value: future) -> future;
            }
            "#,
            "runner",
        );

        let ffi = file(&files, "world/runner/ffi_import.mbt");
        assert!(ffi.contains(r#""$root" "[future-new-unit]exchange""#));
        assert!(ffi.contains(r#""$root" "[async-lower][future-read-unit]exchange""#));
        assert!(ffi.contains(r#""$root" "[future-cancel-read-unit]exchange""#));
    }
}
