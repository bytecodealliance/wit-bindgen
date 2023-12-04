use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::io::{Read, Write as _};
use std::mem;
use std::process::Stdio;

use anyhow::Result;
use heck::ToKebabCase;
use wit_bindgen_c::imported_types_used_by_exported_interfaces;
use wit_bindgen_core::wit_parser::{
    Function, InterfaceId, LiveTypes, Resolve, Type, TypeId, WorldId, WorldKey,
};
use wit_bindgen_core::{generated_preamble, uwriteln, Direction, Files, Source, WorldGenerator};
use world::{Packages, TinyGoWorld};

mod bindgen;
mod imports;
mod interface;
mod path;
mod world;

static C_GEN_FILES_PATH: &'static str = "c_files_";

#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `gofmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub gofmt: bool,

    /// The optional package name to use for the generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub package_name: Option<String>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            gofmt: true,        // Set the default value of gofmt to true
            package_name: None, // Set the default value of package_name to None
        }
    }
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(TinyGo {
            opts: self.clone(),
            ..TinyGo::default()
        })
    }
}

#[derive(Default)]
pub struct TinyGo {
    // the options for the generator provided by the user
    opts: Opts,

    // the generated code
    src: Source,

    // the parts immediately precede the import of "C"
    preamble: Source,

    // the name of the world being generated
    world: TinyGoWorld,

    // import requirements for the generated code
    import_requirements: imports::ImportRequirements,

    // C type names
    c_type_names: HashMap<TypeId, String>,

    // C type namespaces
    c_type_namespaces: HashMap<TypeId, String>,

    // Go type names
    type_names: HashMap<TypeId, String>,

    // tracking all the exported resources used in generating the
    // resource interface and the resource destructors
    exported_resources: HashSet<TypeId>,

    /// tracking all the pending Go packages to be generated
    go_import_packages: Packages,
    go_export_packages: Packages,
}

impl TinyGo {
    fn interface<'b>(
        &'b mut self,
        resolve: &'b Resolve,
        direction: Direction,
    ) -> interface::InterfaceGenerator {
        interface::InterfaceGenerator {
            src: Source::default(),
            preamble: Source::default(),
            gen: self,
            resolve,
            interface: None,
            direction,
            export_funcs: Default::default(),
            exported_resources: Default::default(),
            methods: Default::default(),
        }
    }

    fn get_c_ty(&self, ty: &Type) -> String {
        let res = match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "uint8_t".into(),
            Type::U16 => "uint16_t".into(),
            Type::U32 => "uint32_t".into(),
            Type::U64 => "uint64_t".into(),
            Type::S8 => "int8_t".into(),
            Type::S16 => "int16_t".into(),
            Type::S32 => "int32_t".into(),
            Type::S64 => "int64_t".into(),
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::Char => "uint32_t".into(),
            Type::String => {
                format!(
                    "{namespace}_string_t",
                    namespace = self.world.to_snake_case()
                )
            }
            Type::Id(id) => {
                if let Some(name) = self.c_type_names.get(id) {
                    name.to_owned()
                } else {
                    panic!("failed to find type name for {id:?}");
                }
            }
        };
        if res == "bool" {
            return res;
        }
        format!("C.{res}")
    }

    fn with_result_option(&mut self, needs_result_option: bool) {
        self.import_requirements.needs_result_option = needs_result_option;
    }

    fn with_import_unsafe(&mut self, needs_import_unsafe: bool) {
        self.import_requirements.needs_import_unsafe = needs_import_unsafe;
    }

    fn with_fmt_import(&mut self, needs_fmt_import: bool) {
        self.import_requirements.needs_fmt_import = needs_fmt_import;
    }

    pub fn with_sync_import(&mut self, needs_sync_import: bool) {
        self.import_requirements.needs_sync_import = needs_sync_import;
    }
}

impl WorldGenerator for TinyGo {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.world = TinyGoWorld::from_world_id(world, resolve);

        self.go_import_packages.prefix_name = self.opts.package_name.clone();
        self.go_export_packages.prefix_name = self.opts.package_name.clone();
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let name_raw = &resolve.name_world_key(name);
        self.src
            .push_str(&format!("// Import functions from {name_raw}\n"));

        let mut gen = self.interface(resolve, Direction::Import);
        gen.interface = Some((id, name));
        let (snake, module_path) = gen.start_append_submodule(name);

        gen.define_interface_types(id);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.import(resolve, func);
        }

        gen.finish_append_submodule(&snake, module_path);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        self.src
            .push_str(&format!("// Import functions from {name}\n"));

        let mut gen = self.interface(resolve, Direction::Import);

        gen.define_function_types(funcs);

        for (_name, func) in funcs.iter() {
            gen.import(resolve, func);
        }

        gen.finish_append_submodule(name, vec!["imports".to_string(), name.to_owned()]);
    }

    fn pre_export_interface(&mut self, resolve: &Resolve, _files: &mut Files) -> Result<()> {
        let world = self.world.unwrap_id();
        let live_import_types = imported_types_used_by_exported_interfaces(resolve, world);
        self.c_type_namespaces
            .retain(|k, _| live_import_types.contains(k));
        self.c_type_names
            .retain(|k, _| live_import_types.contains(k));
        self.type_names.retain(|k, _| live_import_types.contains(k));
        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name_raw = &resolve.name_world_key(name);
        self.src
            .push_str(&format!("// Export functions from {name_raw}\n"));

        let mut gen = self.interface(resolve, Direction::Export);
        let (snake, module_path) = gen.start_append_submodule(name);
        gen.interface = Some((id, name));
        gen.define_interface_types(id);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.export(resolve, func);
        }

        gen.finish_append_submodule(&snake, module_path);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = &resolve.worlds[world].name;
        self.src
            .push_str(&format!("// Export functions from {name}\n"));

        let mut gen = self.interface(resolve, Direction::Export);
        gen.define_function_types(funcs);

        for (_name, func) in funcs.iter() {
            gen.export(resolve, func);
        }
        gen.finish_append_submodule(name, vec!["exports".to_string(), name.to_owned()]);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        let mut gen = self.interface(resolve, Direction::Import);
        let mut live = LiveTypes::default();
        for (_, id) in types {
            live.add_type_id(resolve, *id);
        }
        gen.define_live_types(&live);
        gen.finish_append_submodule(name, vec!["imports".to_string(), name.to_owned()]);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        self.go_import_packages.finish(resolve, id, files);
        self.go_export_packages.finish(resolve, id, files);

        // TODO:
        // 1. import_requirements
        // 2. opts (gofmt)

        let mut opts = wit_bindgen_c::Opts::default();
        opts.no_sig_flattening = true;
        opts.no_object_file = true;
        opts.c_out_dir = Some(format!("{C_GEN_FILES_PATH}/"));
        opts.build()
            .generate(resolve, id, files)
            .expect("C generator should be infallible");

        let mut c_lib: Source = Source::default();
        let version = env!("CARGO_PKG_VERSION");
        generated_preamble(&mut c_lib, version);
        c_lib.push_str(&format!("package {C_GEN_FILES_PATH}\n\n"));
        uwriteln!(
            c_lib,
            "
        // Go will only compile the C code if there is a C file in the same directory
        // as the Go file. This file is a dummy Go package that it's only purpose is
        // to compile the C code and expose it to the Go code. In order to use this Go 
        // package, you need to import it in your Go code
        // and then import the C header file using CGo.
        //
        // This package is only used internally by wit-bindgen, and it's not meant to
        // be used by the user. The user should not import this package.
        "
        );
        c_lib.push_str("import \"C\"\n\n");
        files.push(&format!("{C_GEN_FILES_PATH}/lib.go"), c_lib.as_bytes());
    }
    // fn finish_(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
    //     // make sure all types are defined on top of the file
    //     let src = mem::take(&mut self.src);
    //     self.src.push_str(&src);

    //     // prepend package and imports header
    //     let src = mem::take(&mut self.src);
    //     wit_bindgen_core::generated_preamble(&mut self.src, env!("CARGO_PKG_VERSION"));
    //     let snake = self.world.to_snake_case();
    //     // add package
    //     self.src.push_str("package ");
    //     self.src.push_str(&snake);
    //     self.src.push_str("\n\n");

    //     // import C
    //     self.src.push_str("// #include \"");
    //     self.src.push_str(self.world.to_snake_case().as_str());
    //     self.src.push_str(".h\"\n");
    //     if self.preamble.len() > 0 {
    //         self.src.append_src(&self.preamble);
    //     }
    //     self.src.push_str("import \"C\"\n");
    //     let world = &resolve.worlds[id];

    //     self.import_requirements.generate(
    //         snake,
    //         files,
    //         format!("{}_types.go", world.name.to_kebab_case()),
    //     );
    //     self.src.push_str(&self.import_requirements.src);

    //     self.src.push_str(&src);

    //     if self.opts.gofmt {
    //         let mut child = std::process::Command::new("gofmt")
    //             .stdin(Stdio::piped())
    //             .stdout(Stdio::piped())
    //             .spawn()
    //             .expect("failed to spawn gofmt");
    //         child
    //             .stdin
    //             .take()
    //             .unwrap()
    //             .write_all(self.src.as_bytes())
    //             .expect("failed to write to gofmt");
    //         self.src.as_mut_string().truncate(0);
    //         child
    //             .stdout
    //             .take()
    //             .unwrap()
    //             .read_to_string(self.src.as_mut_string())
    //             .expect("failed to read from gofmt");
    //         let status = child.wait().expect("failed to wait on gofmt");
    //         assert!(status.success());
    //     }
    //     files.push(
    //         &format!("{}.go", world.name.to_kebab_case()),
    //         self.src.as_bytes(),
    //     );

    //     let mut opts = wit_bindgen_c::Opts::default();
    //     opts.no_sig_flattening = true;
    //     opts.no_object_file = true;
    //     opts.build()
    //         .generate(resolve, id, files)
    //         .expect("C generator should be infallible")
    // }
}

fn avoid_keyword(s: &str) -> String {
    if GOKEYWORDS.contains(&s) {
        format!("{s}_")
    } else {
        s.into()
    }
}

// a list of Go keywords
const GOKEYWORDS: [&str; 25] = [
    "break",
    "default",
    "func",
    "interface",
    "select",
    "case",
    "defer",
    "go",
    "map",
    "struct",
    "chan",
    "else",
    "goto",
    "package",
    "switch",
    "const",
    "fallthrough",
    "if",
    "range",
    "type",
    "continue",
    "for",
    "import",
    "return",
    "var",
];
