use anyhow::Result;
use core::panic;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    mem,
    ops::Deref,
};
use wit_bindgen_core::{
    AsyncFilterSet, Direction, Files, InterfaceGenerator as CoreInterfaceGenerator, Ns, Source,
    WorldGenerator,
    abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmSignature, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Alignment, ArchitectureSize, Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Int,
        InterfaceId, Record, Resolve, Result_, SizeAlign, Tuple, Type, TypeDefKind, TypeId,
        Variant, WorldId, WorldKey,
    },
};

use crate::pkg::{Imports, MoonbitSignature, PkgResolver, ToMoonBitIdent, ToMoonBitTypeIdent};

mod ffi;
mod pkg;

// Assumptions:
// - Data: u8 -> Byte, s8 | s16 | s32 -> Int, u16 | u32 -> UInt, s64 -> Int64, u64 -> UInt64, f32 | f64 -> Double, address -> Int
// - Encoding: UTF16
// - Lift/Lower list<T>: T == Int/UInt/Int64/UInt64/Float/Double -> FixedArray[T], T == Byte -> Bytes, T == Char -> String
// Organization:
// - one package per interface (export and import are treated as different interfaces)
// - ffi utils are under `./ffi`, and the project entrance (package as link target) is under `./gen`
// TODO: Export will share the type signatures with the import by using a newtype alias
pub(crate) const FFI_DIR: &str = "ffi";

pub(crate) const FFI: &str = include_str!("./ffi/ffi.mbt");

pub(crate) const ASYNC_PRIMITIVE: &str = include_str!("./ffi/async_primitive.mbt");
pub(crate) const ASYNC_FUTURE: &str = include_str!("./ffi/future.mbt");
pub(crate) const ASYNC_WASM_PRIMITIVE: &str = include_str!("./ffi/wasm_primitive.mbt");
pub(crate) const ASYNC_WAITABLE_SET: &str = include_str!("./ffi/waitable_task.mbt");
pub(crate) const ASYNC_SUBTASK: &str = include_str!("./ffi/subtask.mbt");
struct Segment<'a> {
    name: &'a str,
    src: &'a str,
}

pub(crate) const ASYNC_UTILS: [&Segment; 5] = [
    &Segment {
        name: "async_primitive",
        src: ASYNC_PRIMITIVE,
    },
    &Segment {
        name: "async_future",
        src: ASYNC_FUTURE,
    },
    &Segment {
        name: "async_wasm_primitive",
        src: ASYNC_WASM_PRIMITIVE,
    },
    &Segment {
        name: "async_waitable_set",
        src: ASYNC_WAITABLE_SET,
    },
    &Segment {
        name: "async_subtask",
        src: ASYNC_SUBTASK,
    },
];

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

struct InterfaceFragment {
    src: String,
    ffi: String,
    stub: String,
    builtins: HashSet<&'static str>,
}

enum PayloadFor {
    Future,
    Stream,
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

    interface_ns: Ns,
    // dependencies between packages
    pkg_resolver: PkgResolver,
    export: HashMap<String, String>,
    export_ns: Ns,
    // return area allocation
    return_area_size: ArchitectureSize,
    return_area_align: Alignment,

    futures: HashMap<String, HashSet<TypeId>>,
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
        let derive_opts = self.opts.derive.clone();
        InterfaceGenerator {
            src: String::new(),
            stub: String::new(),
            ffi: String::new(),
            r#gen: self,
            resolve,
            name,
            module,
            direction,
            ffi_imports: HashSet::new(),
            derive_opts,
        }
    }
}

impl WorldGenerator for MoonBit {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.pkg_resolver.resolve = resolve.clone();
        self.name = PkgResolver::world_name(resolve, world);
        self.sizes.fill(resolve);
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

        if let Some(content) = &resolve.interfaces[id].docs.contents {
            if !content.is_empty() {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
        }

        let module = &resolve.name_world_key(key);
        let mut r#gen = self.interface(resolve, &name, module, Direction::Import);
        r#gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            r#gen.import(Some(key), func);
        }

        r#gen.add_interface_fragment();

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
        let mut r#gen = self.interface(resolve, &name, "$root", Direction::Import);

        for (_, func) in funcs {
            r#gen.import(None, func); // None is "$root"
        }

        r#gen.add_world_fragment();
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

        if let Some(content) = &resolve.interfaces[id].docs.contents {
            if !content.is_empty() {
                files.push(
                    &format!("{}/README.md", name.replace(".", "/")),
                    content.as_bytes(),
                );
            }
        }

        let module = &resolve.name_world_key(key);
        let mut r#gen = self.interface(resolve, &name, module, Direction::Export);
        r#gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            r#gen.export(Some(key), func);
        }

        r#gen.add_interface_fragment();
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = format!(
            "{}.{}",
            self.opts.r#gen_dir,
            PkgResolver::world_name(resolve, world)
        );
        let mut r#gen = self.interface(resolve, &name, "$root", Direction::Export);

        for (_, func) in funcs {
            r#gen.export(None, func);
        }

        r#gen.add_world_fragment();
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = PkgResolver::world_name(resolve, world);
        let mut r#gen = self.interface(resolve, &name, "$root", Direction::Import);

        for (ty_name, ty) in types {
            r#gen.define_type(ty_name, *ty);
        }

        r#gen.add_world_fragment();
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
        let name = PkgResolver::world_name(resolve, id);

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
            let imports: Option<&Imports> = self.pkg_resolver.package_import.get(name);
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
                    format!(
                        "{{ \"import\": [{}], \"warn-list\": \"-44\" }}",
                        deps.join(", ")
                    )
                    .as_bytes(),
                );
            } else {
                files.push(
                    &format!("{directory}/moon.pkg.json"),
                    "{ \"warn-list\": \"-44\" }".to_string().as_bytes(),
                );
            }
        };

        // Import world fragments
        let mut src = Source::default();
        let mut ffi = Source::default();
        let mut builtins: HashSet<&'static str> = HashSet::new();
        wit_bindgen_core::generated_preamble(&mut src, version);
        wit_bindgen_core::generated_preamble(&mut ffi, version);
        self.import_world_fragments.iter().for_each(|f| {
            uwriteln!(src, "{}", f.src);
            uwriteln!(ffi, "{}", f.ffi);
            builtins.extend(f.builtins.iter());
            assert!(f.stub.is_empty());
        });
        for b in builtins.iter() {
            uwriteln!(ffi, "{}", b);
        }

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
                &format!("{}/{directory}/stub.mbt", self.opts.r#gen_dir),
                indent(&stub).as_bytes(),
            );
            generate_pkg_definition(&format!("{}.{}", self.opts.r#gen_dir, name), files);
        }

        let mut builtins: HashSet<&'static str> = HashSet::new();
        builtins.insert(ffi::MALLOC);
        builtins.insert(ffi::FREE);
        let mut generate_ffi =
            |directory: String, fragments: &[InterfaceFragment], files: &mut Files| {
                // For cabi_realloc

                let mut body = Source::default();
                wit_bindgen_core::generated_preamble(&mut body, version);
                for fragment in fragments.iter() {
                    uwriteln!(&mut body, "{}", fragment.ffi);
                    builtins.extend(fragment.builtins.iter());
                }

                files.push(
                    &format!(
                        "{}/{}_export.mbt",
                        self.opts.r#gen_dir,
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
            let mut builtins: HashSet<&'static str> = HashSet::new();
            fragments.iter().for_each(|f| {
                uwriteln!(src, "{}", f.src);
                uwriteln!(ffi, "{}", f.ffi);
                builtins.extend(f.builtins.iter());
                assert!(f.stub.is_empty());
            });
            for builtin in builtins {
                uwriteln!(ffi, "{}", builtin);
            }

            let directory = name.replace('.', "/");
            files.push(&format!("{directory}/top.mbt"), indent(&src).as_bytes());
            files.push(&format!("{directory}/ffi.mbt"), indent(&ffi).as_bytes());
            generate_pkg_definition(name, files);
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
                generate_pkg_definition(name, files);
            }
            generate_ffi(directory, fragments, files);
        }

        // Export FFI Utils
        // Export Async utils

        // If async is used, export async utils
        if self.is_async || !self.futures.is_empty() {
            let mut body = Source::default();
            wit_bindgen_core::generated_preamble(&mut body, version);
            body.push_str(FFI);
            files.push(&format!("{FFI_DIR}/top.mbt"), indent(&body).as_bytes());
            ASYNC_UTILS.iter().for_each(|s| {
                files.push(
                    &format!("{FFI_DIR}/{}.mbt", s.name),
                    indent(s.src).as_bytes(),
                );
            });
            files.push(
                &format!("{FFI_DIR}/moon.pkg.json"),
                "{ \"warn-list\": \"-44\", \"supported-targets\": [\"wasm\"] }".as_bytes(),
            );
        }

        // Export project files
        if !self.opts.ignore_stub && !self.opts.ignore_module_file {
            let mut body = Source::default();
            uwriteln!(
                &mut body,
                "{{ \"name\": \"{project_name}\", \"preferred-target\": \"wasm\" }}"
            );
            files.push("moon.mod.json", body.as_bytes());
        }

        // Export project entry point
        let mut body = Source::default();
        wit_bindgen_core::generated_preamble(&mut body, version);
        uwriteln!(&mut body, "{}", ffi::CABI_REALLOC);

        if !self.return_area_size.is_empty() {
            uwriteln!(
                &mut body,
                "
                let return_area : Int = mbt_ffi_malloc({})
                ",
                self.return_area_size.size_wasm32(),
            );
        }
        for builtin in builtins {
            uwriteln!(&mut body, "{}", builtin);
        }
        files.push(
            &format!("{}/ffi.mbt", self.opts.r#gen_dir),
            indent(&body).as_bytes(),
        );

        self.export
            .insert("mbt_ffi_cabi_realloc".into(), "cabi_realloc".into());

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
        if let Some(imports) = self.pkg_resolver.package_import.get(&self.opts.r#gen_dir) {
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
              , \"warn-list\": \"-44\"
            }}
            ",
        );
        files.push(
            &format!("{}/moon.pkg.json", self.opts.r#gen_dir,),
            indent(&body).as_bytes(),
        );

        Ok(())
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    stub: String,
    ffi: String,
    r#gen: &'a mut MoonBit,
    resolve: &'a Resolve,
    // The current interface getting generated
    name: &'a str,
    module: &'a str,
    direction: Direction,
    // Collect of FFI imports used in this interface
    ffi_imports: HashSet<&'static str>,
    // Options for deriving traits
    derive_opts: DeriveOpts,
}

impl InterfaceGenerator<'_> {
    fn add_interface_fragment(self) {
        match self.direction {
            Direction::Import => {
                self.r#gen
                    .import_interface_fragments
                    .entry(self.name.to_owned())
                    .or_default()
                    .push(InterfaceFragment {
                        src: self.src,
                        stub: self.stub,
                        ffi: self.ffi,
                        builtins: self.ffi_imports,
                    });
            }
            Direction::Export => {
                self.r#gen
                    .export_interface_fragments
                    .entry(self.name.to_owned())
                    .or_default()
                    .push(InterfaceFragment {
                        src: self.src,
                        stub: self.stub,
                        ffi: self.ffi,
                        builtins: self.ffi_imports,
                    });
            }
        }
    }

    fn add_world_fragment(self) {
        match self.direction {
            Direction::Import => {
                self.r#gen.import_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                    ffi: self.ffi,
                    builtins: self.ffi_imports,
                });
            }
            Direction::Export => {
                self.r#gen.export_world_fragments.push(InterfaceFragment {
                    src: self.src,
                    stub: self.stub,
                    ffi: self.ffi,
                    builtins: self.ffi_imports,
                });
            }
        }
    }

    fn import(&mut self, module: Option<&WorldKey>, func: &Function) {
        let async_ = self
            .r#gen
            .opts
            .async_
            .is_async(self.resolve, module, func, false);
        if async_ {
            self.r#gen.is_async = true;
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
            bindgen.r#gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
            false,
        );

        let mut src = bindgen.src.clone();

        let cleanup_list = if bindgen.needs_cleanup_list {
            self.r#gen.needs_cleanup = true;

            "
            let cleanup_list : Array[Int] = []
            "
            .into()
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

        let mbt_sig = self.r#gen.pkg_resolver.mbt_sig(self.name, func, false);
        let sig = self.sig_string(&mbt_sig, async_);

        let module = match module {
            Some(key) => self.resolve.name_world_key(key),
            None => "$root".into(),
        };

        self.r#generation_futures_and_streams_import("", func, interface_name);

        uwriteln!(
            self.ffi,
            r#"fn wasmImport{camel_name}({params}) {result_type} = "{module}" "{async_prefix}{name}""#
        );

        print_docs(&mut self.src, &func.docs);

        if async_ {
            src = self.r#generate_async_import_function(func, mbt_sig, &wasm_sig);
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
        let mut body = String::default();
        let mut lower_params = Vec::new();
        let mut lower_results = Vec::new();

        if sig.indirect_params {
            match &func.params[..] {
                [] => {}
                [(_, _)] => {
                    lower_params.push("_lower_ptr".into());
                }
                multiple_params => {
                    let params = multiple_params.iter().map(|(_, ty)| ty);
                    let offsets = self.r#gen.sizes.field_offsets(params.clone());
                    let elem_info = self.r#gen.sizes.params(params);
                    body.push_str(&format!(
                        r#"
                        let _lower_ptr : Int = {ffi}malloc({})
                        "#,
                        elem_info.size.size_wasm32(),
                        ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR)
                    ));

                    for ((offset, ty), name) in offsets.iter().zip(
                        multiple_params
                            .iter()
                            .map(|(name, _)| name.to_moonbit_ident()),
                    ) {
                        let result = self.lower_to_memory(
                            &format!("_lower_ptr + {}", offset.size_wasm32()),
                            &name,
                            ty,
                            self.name,
                        );
                        body.push_str(&result);
                    }

                    lower_params.push("_lower_ptr".into());
                }
            }
        } else {
            let mut f = FunctionBindgen::new(self, "INVALID", self.name, Box::new([]));
            for (name, ty) in mbt_sig.params.iter() {
                lower_params.extend(abi::lower_flat(f.r#gen.resolve, &mut f, name.clone(), ty));
            }
            lower_results.push(f.src.clone());
        }

        let func_name = func.name.to_upper_camel_case();

        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);

        let call_import = |params: &Vec<String>| {
            format!(
                r#"
                let _subtask_code = wasmImport{func_name}({})
                let _subtask_status = {ffi}SubtaskStatus::decode(_subtask_code)
                let _subtask = @ffi.Subtask::from_handle(_subtask_status.handle(), code=_subtask_code)

                let task = @ffi.current_task()
                task.add_waitable(_subtask, @ffi.current_coroutine())
                defer task.remove_waitable(_subtask)

                for {{
                        if _subtask.done() || _subtask_status is Returned(_) {{
                            break
                        }} else {{
                            @ffi.suspend()
                        }}
                    }}

                "#,
                params.join(", ")
            )
        };
        match &func.result {
            Some(ty) => {
                lower_params.push("_result_ptr".into());
                let call_import = call_import(&lower_params);
                let (lift, lift_result) = &self.lift_from_memory("_result_ptr", ty, self.name);
                body.push_str(&format!(
                    r#"
                    {}
                    {}
                    {call_import}
                    {lift}
                    {lift_result}
                    "#,
                    lower_results.join("\n"),
                    &self.malloc_memory("_result_ptr", "1", ty)
                ));
            }
            None => {
                let call_import = call_import(&lower_params);
                body.push_str(&call_import);
            }
        }

        body.to_string()
    }

    fn export(&mut self, interface: Option<&WorldKey>, func: &Function) {
        let async_ = self
            .r#gen
            .opts
            .async_
            .is_async(self.resolve, interface, func, false);
        if async_ {
            self.r#gen.is_async = true;
        }

        let variant = if async_ {
            AbiVariant::GuestExportAsync
        } else {
            AbiVariant::GuestExport
        };

        let sig = self.resolve.wasm_signature(variant, func);
        let mbt_sig = self.r#gen.pkg_resolver.mbt_sig(self.name, func, false);

        let func_sig = self.sig_string(&mbt_sig, async_);
        let export_dir = self.r#gen.opts.r#gen_dir.clone();

        let mut toplevel_generator = self.r#gen.interface(
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
            bindgen.r#gen.resolve,
            variant,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
            async_,
        );

        // TODO: adapt async cleanup
        assert!(!bindgen.needs_cleanup_list);

        // Async functions deferred task return
        let deferred_task_return = bindgen.deferred_task_return.clone();

        let src = bindgen.src;
        assert!(toplevel_generator.src.is_empty());
        assert!(toplevel_generator.ffi.is_empty());

        // Transfer ffi_imports from toplevel_generator to self
        self.ffi_imports
            .extend(toplevel_generator.ffi_imports.iter());

        let result_type = match &sig.results[..] {
            [] => "Unit",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let camel_name = func.name.to_upper_camel_case();

        let func_name = self.r#gen.export_ns.tmp(&format!("wasmExport{camel_name}"));

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
        self.r#generation_futures_and_streams_import("[export]", func, module_name);

        uwrite!(
            self.ffi,
            r#"
            pub fn {func_name}({params}) -> {result_type} {{
                {src}
            }}
            "#,
        );

        self.r#gen
            .export
            .insert(func_name, format!("{async_export_prefix}{export_name}"));

        if async_ {
            let snake = self.r#gen.name.to_lower_camel_case();
            let export_func_name = self
                .r#gen
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
            self.r#gen.export.insert(
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
                Some(result) => self
                    .r#gen
                    .pkg_resolver
                    .type_name(self.name, result, true)
                    .to_string(),
                None => "Unit".into(),
            };
            let return_expr = match return_ty.as_str() {
                "Unit" => "".into(),
                _ => format!("{return_param}: {return_ty}",),
            };
            let snake_func_name = func.name.to_moonbit_ident().to_string();
            let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);

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

            abi::post_return(bindgen.r#gen.resolve, func, &mut bindgen);

            let src = bindgen.src;

            let func_name = self
                .r#gen
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
            self.r#gen
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

    fn deallocate_lists(
        &mut self,
        types: &[Type],
        operands: &[String],
        indirect: bool,
        module: &str,
    ) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::deallocate_lists_in_types(f.r#gen.resolve, types, operands, indirect, &mut f);
        f.src
    }

    fn lift_from_memory(&mut self, address: &str, ty: &Type, module: &str) -> (String, String) {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));

        let result = abi::lift_from_memory(f.r#gen.resolve, &mut f, address.into(), ty);
        (f.src, result)
    }

    fn lower_to_memory(&mut self, address: &str, value: &str, ty: &Type, module: &str) -> String {
        let mut f = FunctionBindgen::new(self, "INVALID", module, Box::new([]));
        abi::lower_to_memory(f.r#gen.resolve, &mut f, address.into(), value.into(), ty);
        f.src
    }

    fn malloc_memory(&mut self, address: &str, length: &str, ty: &Type) -> String {
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        format!("let {address} = {ffi}malloc({size} * {length});")
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        matches!(
            element,
            Type::U8 | Type::U32 | Type::U64 | Type::S32 | Type::S64 | Type::F32 | Type::F64
        )
    }

    fn list_lift_from_memory(
        &mut self,
        address: &str,
        length: &str,
        lift_func: &str,
        ty: &Type,
    ) -> String {
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        if self.is_list_canonical(self.resolve, ty) {
            if ty == &Type::U8 {
                return format!("{ffi}ptr2bytes({address}, {length})");
            }
            let ty = match ty {
                Type::U32 => "uint",
                Type::U64 => "uint64",
                Type::S32 => "int",
                Type::S64 => "int64",
                Type::F32 => "float",
                Type::F64 => "double",
                _ => unreachable!(),
            };

            return format!("{ffi}ptr2{ty}_array({address}, {length})");
        }
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        format!(
            r#"
            FixedArray::makei(
                {length},
                (index) => {{ 
                    let ptr = ({address}) + (index * {size});
                    {lift_func}(ptr)
                }}
            )
            "#
        )
    }

    fn list_lower_to_memory(&mut self, lower_func: &str, value: &str, ty: &Type) -> String {
        // Align the address, moonbit only supports wasm32 for now
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);
        if self.is_list_canonical(self.resolve, ty) {
            if ty == &Type::U8 {
                return format!("{ffi}bytes2ptr({value})");
            }

            let ty = match ty {
                Type::U32 => "uint",
                Type::U64 => "uint64",
                Type::S32 => "int",
                Type::S64 => "int64",
                Type::F32 => "float",
                Type::F64 => "double",
                _ => unreachable!(),
            };
            return format!("{ffi}{ty}_array2ptr({value})");
        }
        let size = self.r#gen.sizes.size(ty).size_wasm32();
        format!(
            r#"
            let address = {ffi}malloc(({value}).length() * {size});
            for index = 0; index < ({value}).length(); index = index + 1 {{
                let ptr = (address) + (index * {size});
                let value = {value}[index];
                {lower_func}(value, ptr);
            }}
            address 
            "#
        )
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
                    self.r#generate_async_future_or_stream_import(
                        PayloadFor::Future,
                        &module,
                        index,
                        func_name,
                        ty,
                        payload_type.as_ref(),
                    );
                }
                TypeDefKind::Stream(payload_type) => {
                    self.r#generate_async_future_or_stream_import(
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
        if let Some(set) = self.r#gen.futures.get(module) {
            if set.contains(&ty) {
                return;
            }
        }

        self.r#gen
            .futures
            .entry(module.to_string())
            .or_default()
            .insert(ty);
        let result = match result_type {
            Some(ty) => self.r#gen.pkg_resolver.type_name(self.name, ty, true),
            None => "Unit".into(),
        };

        let type_name = self
            .r#gen
            .pkg_resolver
            .type_name(self.name, &Type::Id(ty), true);
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

        let payload_lift_func = match payload_for {
            PayloadFor::Future => "",
            PayloadFor::Stream => "List",
        };
        let ffi = self.r#gen.pkg_resolver.qualify_package(self.name, FFI_DIR);

        let mut dealloc_list;
        let malloc;
        let lift;
        let lower;
        let lift_result;
        let lift_list: String;
        let lower_list: String;
        if let Some(result_type) = result_type {
            (lift, lift_result) = self.lift_from_memory("ptr", result_type, module);
            lower = self.lower_to_memory("ptr", "value", result_type, module);
            dealloc_list = self.deallocate_lists(
                std::slice::from_ref(result_type),
                &[String::from("ptr")],
                true,
                module,
            );
            lift_list = self.list_lift_from_memory(
                "ptr",
                "length",
                &format!("wasm{name}{kind}Lift"),
                result_type,
            );
            lower_list =
                self.list_lower_to_memory(&format!("wasm{name}{kind}Lower"), "value", result_type);

            malloc = self.malloc_memory("ptr", "length", result_type);

            if dealloc_list.is_empty() {
                dealloc_list = "let _ = ptr".to_string();
            }
        } else {
            lift = "let _ = ptr".to_string();
            lower = "let _ = (ptr, value)".to_string();
            dealloc_list = "let _ = ptr".to_string();
            malloc = "let ptr = 0;".into();
            lift_result = "".into();
            lift_list = "FixedArray::make(length, Unit::default())".into();
            lower_list = "0".into();
        }

        let (mut lift_func, mut lower_func) = if result_type
            .is_some_and(|ty| self.is_list_canonical(self.resolve, ty))
            && matches!(payload_for, PayloadFor::Stream)
        {
            ("".into(), "".into())
        } else {
            (
                format!(
                    r#"
                fn wasm{name}{kind}Lift(ptr: Int) -> {result} {{
                    {lift}
                    {lift_result}
                }}
                "#
                ),
                format!(
                    r#"
                fn wasm{name}{kind}Lower(value: {result}, ptr: Int) -> Unit {{
                    {lower}
                }}
                "#
                ),
            )
        };

        if matches!(payload_for, PayloadFor::Stream) {
            lift_func.push_str(&format!(
                r#"
                fn wasm{name}{kind}ListLift(ptr: Int, length: Int) -> FixedArray[{result}] {{ 
                    {lift_list}
                }}
                "#
            ));

            lower_func.push_str(&format!(
                r#"
                fn wasm{name}{kind}ListLower(value: FixedArray[{result}]) -> Int {{ 
                    {lower_list}
                }}
                "#
            ));
        };

        uwriteln!(
            self.ffi,
            r#"
fn wasmImport{name}{kind}New() -> UInt64 = "{module}" "[{kind}-new-{index}]{func_name}"
fn wasmImport{name}{kind}Read(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-read-{index}]{func_name}"
fn wasmImport{name}{kind}Write(handle : Int, buffer_ptr : Int{payload_len_arg}) -> Int = "{module}" "[async-lower][{kind}-write-{index}]{func_name}"
fn wasmImport{name}{kind}CancelRead(handle : Int) -> Int = "{module}" "[{kind}-cancel-read-{index}]{func_name}"
fn wasmImport{name}{kind}CancelWrite(handle : Int) -> Int = "{module}" "[{kind}-cancel-write-{index}]{func_name}"
fn wasmImport{name}{kind}DropReadable(handle : Int) = "{module}" "[{kind}-drop-readable-{index}]{func_name}"
fn wasmImport{name}{kind}DropWritable(handle : Int) = "{module}" "[{kind}-drop-writable-{index}]{func_name}"
fn wasm{name}{kind}Deallocate(ptr: Int) -> Unit {{
    {dealloc_list}
}}
fn wasm{name}{kind}Malloc(length: Int) -> Int {{
    {malloc}
    ptr
}}

fn {table_name}() -> {ffi}{camel_kind}VTable[{result}] {{
    {ffi}{camel_kind}VTable::new(
        wasmImport{name}{kind}New,
        wasmImport{name}{kind}Read,
        wasmImport{name}{kind}Write,
        wasmImport{name}{kind}CancelRead,
        wasmImport{name}{kind}CancelWrite,
        wasmImport{name}{kind}DropReadable,
        wasmImport{name}{kind}DropWritable,
        wasm{name}{kind}Malloc,
        wasm{name}{kind}Deallocate,
        wasm{name}{kind}{payload_lift_func}Lift,
        wasm{name}{kind}{payload_lift_func}Lower,
    )
}}
{lift_func}
{lower_func}
pub let static_{table_name}: {ffi}{camel_kind}VTable[{result}]  = {table_name}();
"#
        );
    }

    fn sig_string(&mut self, sig: &MoonbitSignature, async_: bool) -> String {
        let params = sig
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.r#gen.pkg_resolver.type_name(self.name, ty, true);
                format!("{name} : {ty}")
            })
            .collect::<Vec<_>>();

        let params = params.join(", ");
        let (async_prefix, async_suffix) = if async_ { ("async ", "") } else { ("", "") };
        let result_type = match &sig.result_type {
            None => "Unit".into(),
            Some(ty) => self.r#gen.pkg_resolver.type_name(self.name, ty, true),
        };
        format!(
            "pub {async_prefix}fn {}({params}) -> {}{async_suffix}",
            sig.name, result_type
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
                    self.r#gen.pkg_resolver.type_name(self.name, &field.ty, true),
                )
            })
            .collect::<Vec<_>>()
            .join("; ");

        let mut deriviation: Vec<_> = Vec::new();
        if self.derive_opts.derive_show {
            deriviation.push("Show")
        }
        if self.derive_opts.derive_eq {
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

            let func_name = self.r#gen.export_ns.tmp(&format!("wasmExport{name}Dtor"));

            let export_dir = self.r#gen.opts.r#gen_dir.clone();

            let r#gen = self
                .r#gen
                .interface(self.resolve, export_dir.as_str(), "", Direction::Export);

            uwrite!(
                self.ffi,
                r#"
                pub fn {func_name}(handle : Int) -> Unit {{
                    {}{name}::dtor(handle)
                }}
                "#,
                r#gen.r#gen.pkg_resolver.qualify_package(r#gen.name, self.name)
            );

            self.r#gen
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
                    let ty = self.r#gen.pkg_resolver.type_name(self.name, &ty, true);
                    format!("{name}({ty})")
                } else {
                    name.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n  ");

        let mut deriviation: Vec<_> = Vec::new();
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

    fn type_future(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        unimplemented!() // Not needed
    }

    fn type_stream(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        unimplemented!() // Not needed
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
    r#gen: &'b mut InterfaceGenerator<'a>,
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
        r#gen: &'b mut InterfaceGenerator<'a>,
        func_name: &'b str,
        func_interface: &'b str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        let mut locals = Ns::default();
        params.iter().for_each(|str| {
            locals.tmp(str);
        });
        Self {
            r#gen,
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
                    .map(|result| result.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let payload = if self
                    .r#gen
                    .r#gen
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
        let ty = self
            .r#gen
            .r#gen
            .pkg_resolver
            .type_name(self.r#gen.name, ty, false);
        let lifted = self.locals.tmp("lifted");

        let cases = cases
            .iter()
            .zip(blocks)
            .enumerate()
            .map(|(i, ((case_name, case_ty), Block { body, results, .. }))| {
                let payload = if self
                    .r#gen
                    .r#gen
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
                self.r#gen.ffi_imports.insert(ffi::EXTEND8);
                results.push(format!("mbt_ffi_extend8({})", operands[0]))
            }
            Instruction::S8FromI32 => results.push(format!("({} - 0x100)", operands[0])),
            Instruction::S16FromI32 => results.push(format!("({} - 0x10000)", operands[0])),
            Instruction::I32FromS16 => {
                self.r#gen.ffi_imports.insert(ffi::EXTEND16);
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
                    let ty =
                        self.r#gen
                            .r#gen
                            .pkg_resolver
                            .type_name(self.r#gen.name, &Type::Id(*ty), false);
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
                    let ty =
                        self.r#gen
                            .r#gen
                            .pkg_resolver
                            .type_name(self.r#gen.name, &Type::Id(*ty), false);
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
                    let ty =
                        self.r#gen
                            .r#gen
                            .pkg_resolver
                            .type_name(self.r#gen.name, &Type::Id(*ty), false);
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
                        self.r#gen
                            .r#gen
                            .pkg_resolver
                            .type_name(self.r#gen.name, &Type::Id(*ty), true),
                        operands[0]
                    ));
                }
                Int::U16 | Int::U32 => {
                    results.push(format!(
                        "{}({}.reinterpret_as_uint())",
                        self.r#gen
                            .r#gen
                            .pkg_resolver
                            .type_name(self.r#gen.name, &Type::Id(*ty), true),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "{}(({}).reinterpret_as_uint().to_uint64() | (({}).reinterpret_as_uint().to_uint64() << 32))",
                        self.r#gen.r#gen.pkg_resolver.type_name(self.r#gen.name, &Type::Id(*ty), true),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::HandleLower { ty, .. } => {
                let op = &operands[0];
                let handle = self.locals.tmp("handle");
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), false);
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
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), false);

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
                    self.r#gen
                        .r#gen
                        .pkg_resolver
                        .type_name(self.r#gen.name, &Type::Id(*ty), true)
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

            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap();
                let _none = self.blocks.pop().unwrap();

                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), true);
                let lifted = self.locals.tmp("lifted");
                let op = &operands[0];

                let payload = if self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .non_empty_type(Some(*payload))
                    .is_some()
                {
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
                self.r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), true),
                operands[0]
            )),

            Instruction::ListCanonLower { element, realloc } => match element {
                Type::U8 => {
                    let op = &operands[0];
                    let ptr = self.locals.tmp("ptr");
                    self.r#gen.ffi_imports.insert(ffi::BYTES2PTR);
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
                            self.r#gen.ffi_imports.insert(ffi::UINT_ARRAY2PTR);
                            "uint"
                        }
                        Type::U64 => {
                            self.r#gen.ffi_imports.insert(ffi::UINT64_ARRAY2PTR);
                            "uint64"
                        }
                        Type::S32 => {
                            self.r#gen.ffi_imports.insert(ffi::INT_ARRAY2PTR);
                            "int"
                        }
                        Type::S64 => {
                            self.r#gen.ffi_imports.insert(ffi::INT64_ARRAY2PTR);
                            "int64"
                        }
                        Type::F32 => {
                            self.r#gen.ffi_imports.insert(ffi::FLOAT_ARRAY2PTR);
                            "float"
                        }
                        Type::F64 => {
                            self.r#gen.ffi_imports.insert(ffi::DOUBLE_ARRAY2PTR);
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
                    self.r#gen.ffi_imports.insert(ffi::PTR2BYTES);
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
                            self.r#gen.ffi_imports.insert(ffi::PTR2UINT_ARRAY);
                            "uint"
                        }
                        Type::U64 => {
                            self.r#gen.ffi_imports.insert(ffi::PTR2UINT64_ARRAY);
                            "uint64"
                        }
                        Type::S32 => {
                            self.r#gen.ffi_imports.insert(ffi::PTR2INT_ARRAY);
                            "int"
                        }
                        Type::S64 => {
                            self.r#gen.ffi_imports.insert(ffi::PTR2INT64_ARRAY);
                            "int64"
                        }
                        Type::F32 => {
                            self.r#gen.ffi_imports.insert(ffi::PTR2FLOAT_ARRAY);
                            "float"
                        }
                        Type::F64 => {
                            self.r#gen.ffi_imports.insert(ffi::PTR2DOUBLE_ARRAY);
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

                self.r#gen.ffi_imports.insert(ffi::STR2PTR);
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

                self.r#gen.ffi_imports.insert(ffi::PTR2STR);
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
                let size = self.r#gen.r#gen.sizes.size(element).size_wasm32();
                let _align = self.r#gen.r#gen.sizes.align(element).align_wasm32();
                let address = self.locals.tmp("address");
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, element, true);
                let index = self.locals.tmp("index");

                self.r#gen.ffi_imports.insert(ffi::MALLOC);
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
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, element, true);
                let size = self.r#gen.r#gen.sizes.size(element).size_wasm32();
                // let align = self.r#gen.r#gen.sizes.align(element);
                let index = self.locals.tmp("index");

                let result = match &block_results[..] {
                    [result] => result,
                    _ => todo!("result count == {}", results.len()),
                };

                self.r#gen.ffi_imports.insert(ffi::FREE);
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
                            self.r#gen
                                .r#gen
                                .pkg_resolver
                                .qualify_package(self.r#gen.name, self.func_interface),
                            func.name.to_moonbit_ident()
                        )
                    }
                    FunctionKind::AsyncFreestanding => {
                        format!(
                            "{}{}",
                            self.r#gen
                                .r#gen
                                .pkg_resolver
                                .qualify_package(self.r#gen.name, self.func_interface),
                            func.name.to_moonbit_ident()
                        )
                    }
                    FunctionKind::Constructor(ty) => {
                        let name = self.r#gen.r#gen.pkg_resolver.type_name(
                            self.r#gen.name,
                            &Type::Id(ty),
                            false,
                        );
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
                        let name = self.r#gen.r#gen.pkg_resolver.type_name(
                            self.r#gen.name,
                            &Type::Id(ty),
                            false,
                        );
                        format!(
                            "{}::{}",
                            name,
                            func.name.split(".").last().unwrap().to_moonbit_ident()
                        )
                    }
                };

                let args = operands.join(", ");

                if *async_ {
                    let (async_func_result, task_return_result, task_return_type) =
                        match func.result {
                            Some(ty) => {
                                let res = self.locals.tmp("return_result");
                                (
                                    res.clone(),
                                    res,
                                    self.r#gen
                                        .r#gen
                                        .pkg_resolver
                                        .type_name(self.r#gen.name, &ty, true),
                                )
                            }
                            None => ("_ignore".into(), "".into(), "Unit".into()),
                        };

                    if func.result.is_some() {
                        results.push(async_func_result.clone());
                    }
                    let ffi = self
                        .r#gen
                        .r#gen
                        .pkg_resolver
                        .qualify_package(self.r#gen.name, FFI_DIR);
                    uwrite!(
                        self.src,
                        r#"
                        let task = {ffi}current_task();
                        let _ = task.with_waitable_set(fn(task) {{
                            let {async_func_result}: Ref[{task_return_type}?] = Ref::new(None)
                            task.wait(fn() {{
                                {async_func_result}.val = Some({name}({args}));
                            }})
                            for {{
                                if task.no_wait() && {async_func_result}.val is Some({async_func_result}){{
                                   {name}_task_return({task_return_result});
                                   break;
                                }} else {{
                                   {ffi}suspend() catch {{ 
                                        _ => {{
                                            {ffi}task_cancel();
                                        }}
                                   }}
                                }}
                            }}
                        }})
                        if task.is_fail() is Some({ffi}Cancelled::Cancelled) {{
                                {ffi}task_cancel();
                                return {ffi}CallbackCode::Exit.encode()
                        }}
                        if task.is_done() {{
                            return {ffi}CallbackCode::Exit.encode()
                        }}
                        return {ffi}CallbackCode::Wait(task.handle()).encode()
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
                        let ty = format!(
                            "({})",
                            self.r#gen
                                .r#gen
                                .pkg_resolver
                                .type_name(self.r#gen.name, &ty, true)
                        );
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
                    let address = &clean.address;
                    self.r#gen.ffi_imports.insert(ffi::FREE);
                    uwriteln!(self.src, "mbt_ffi_free({address})",);
                }

                if self.needs_cleanup_list {
                    self.r#gen.ffi_imports.insert(ffi::FREE);
                    uwrite!(
                        self.src,
                        "
                        cleanup_list.each(mbt_ffi_free)
                        ",
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
                self.r#gen.ffi_imports.insert(ffi::LOAD32);
                results.push(format!(
                    "mbt_ffi_load32(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load8U { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOAD8_U);
                results.push(format!(
                    "mbt_ffi_load8_u(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load8S { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOAD8);
                results.push(format!(
                    "mbt_ffi_load8(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load16U { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOAD16_U);
                results.push(format!(
                    "mbt_ffi_load16_u(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Load16S { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOAD16);
                results.push(format!(
                    "mbt_ffi_load16(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I64Load { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOAD64);
                results.push(format!(
                    "mbt_ffi_load64(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::F32Load { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOADF32);
                results.push(format!(
                    "mbt_ffi_loadf32(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::F64Load { offset } => {
                self.r#gen.ffi_imports.insert(ffi::LOADF64);
                results.push(format!(
                    "mbt_ffi_loadf64(({}) + {offset})",
                    operands[0],
                    offset = offset.size_wasm32()
                ))
            }

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STORE32);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store32(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I32Store8 { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STORE8);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store8(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I32Store16 { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STORE16);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store16(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::I64Store { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STORE64);
                uwriteln!(
                    self.src,
                    "mbt_ffi_store64(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::F32Store { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STOREF32);
                uwriteln!(
                    self.src,
                    "mbt_ffi_storef32(({}) + {offset}, {})",
                    operands[1],
                    operands[0],
                    offset = offset.size_wasm32()
                )
            }

            Instruction::F64Store { offset } => {
                self.r#gen.ffi_imports.insert(ffi::STOREF64);
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
                self.r#gen.ffi_imports.insert(ffi::MALLOC);
                uwriteln!(self.src, "mbt_ffi_malloc({})", size.size_wasm32())
            }

            Instruction::GuestDeallocate { .. } => {
                self.r#gen.ffi_imports.insert(ffi::FREE);
                uwriteln!(self.src, "mbt_ffi_free({})", operands[0])
            }

            Instruction::GuestDeallocateString => {
                self.r#gen.ffi_imports.insert(ffi::FREE);
                uwriteln!(self.src, "mbt_ffi_free({})", operands[0])
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

                let size = self.r#gen.r#gen.sizes.size(element).size_wasm32();
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

                self.r#gen.ffi_imports.insert(ffi::FREE);
                uwriteln!(self.src, "mbt_ffi_free({address})",);
            }

            Instruction::Flush { amt } => {
                results.extend(operands.iter().take(*amt).cloned());
            }

            Instruction::FutureLift { ty, .. } => {
                let result = self.locals.tmp("result");
                let op = &operands[0];
                // let qualifier = self.r#gen.qualify_package(self.func_interface);
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), true);
                let ffi = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .qualify_package(self.r#gen.name, FFI_DIR);

                let snake_name = format!("static_{}_future_table", ty.to_snake_case(),);

                uwriteln!(
                    self.src,
                    r#"let {result} = {ffi}FutureReader::new({op}, {snake_name});"#,
                );

                results.push(result);
            }

            Instruction::FutureLower { .. } => {
                let op = &operands[0];
                results.push(format!("{op}.handle"));
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
                        .map(|(a, b)| (*a, b.clone()))
                        .collect(),
                    return_param,
                };
            }

            Instruction::StreamLower { .. } => {
                let op = &operands[0];
                results.push(format!("{op}.handle"));
            }

            Instruction::StreamLift { ty, .. } => {
                let result = self.locals.tmp("result");
                let op = &operands[0];
                let qualifier = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .qualify_package(self.r#gen.name, self.func_interface);
                let ty = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .type_name(self.r#gen.name, &Type::Id(*ty), true);
                let ffi = self
                    .r#gen
                    .r#gen
                    .pkg_resolver
                    .qualify_package(self.r#gen.name, FFI_DIR);
                let snake_name = format!(
                    "static_{}_stream_table",
                    ty.replace(&qualifier, "").to_snake_case(),
                );

                uwriteln!(
                    self.src,
                    r#"let {result} = {ffi}StreamReader::new({op}, {snake_name});"#,
                );

                results.push(result);
            }
            Instruction::ErrorContextLower { .. }
            | Instruction::ErrorContextLift { .. }
            | Instruction::DropHandle { .. } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: ArchitectureSize, align: Alignment) -> String {
        if self.r#gen.direction == Direction::Import {
            self.r#gen.ffi_imports.insert(ffi::MALLOC);
            let address = self.locals.tmp("return_area");
            uwriteln!(
                self.src,
                "let {address} = mbt_ffi_malloc({})",
                size.size_wasm32(),
            );
            self.cleanup.push(Cleanup {
                address: address.clone(),
            });
            address
        } else {
            self.r#gen.r#gen.return_area_size = self.r#gen.r#gen.return_area_size.max(size);
            self.r#gen.r#gen.return_area_align = self.r#gen.r#gen.return_area_align.max(align);
            "return_area".into()
        }
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            cleanup: mem::take(&mut self.cleanup),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage { body, cleanup } = self.block_storage.pop().unwrap();

        if !self.cleanup.is_empty() {
            self.needs_cleanup_list = true;
            self.r#gen.ffi_imports.insert(ffi::FREE);

            for cleanup in &self.cleanup {
                let address = &cleanup.address;
                uwriteln!(self.src, "mbt_ffi_free({address})",);
            }
        }

        self.cleanup = cleanup;

        self.blocks.push(Block {
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.r#gen.r#gen.sizes
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

fn generated_preamble(src: &mut Source, version: &str) {
    uwriteln!(src, "// Generated by `wit-bindgen` {version}.")
}

fn print_docs(src: &mut String, docs: &Docs) {
    uwrite!(src, "///|");
    if let Some(docs) = &docs.contents {
        for line in docs.trim().lines() {
            uwrite!(src, "\n/// {line}");
        }
    }
}
