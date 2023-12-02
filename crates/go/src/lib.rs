use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::mem;
use std::process::Stdio;

use anyhow::Result;
use heck::{ToKebabCase, ToSnakeCase};
use wit_bindgen_c::imported_types_used_by_exported_interfaces;
use wit_bindgen_core::wit_parser::{
    Function, InterfaceId, LiveTypes, Resolve, SizeAlign, Type, TypeId, WorldId, WorldKey,
};
use wit_bindgen_core::{Direction, Files, Source, WorldGenerator};

mod bindgen;
mod imports;
mod interface;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `gofmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub gofmt: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self { gofmt: true } // Set the default value of gofmt to true
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
    opts: Opts,
    src: Source,

    // the parts immediately precede the import of "C"
    preamble: Source,

    world: String,

    // import requirements for the generated code
    import_requirements: imports::ImportRequirements,

    sizes: SizeAlign,

    // mapping from interface ID to the name of the interface
    interface_names: HashMap<InterfaceId, WorldKey>,

    // C type names
    c_type_names: HashMap<TypeId, String>,

    // C type namespaces
    c_type_namespaces: HashMap<TypeId, String>,

    // Go type names
    type_names: HashMap<TypeId, String>,

    // tracking all the exported resources used in generating the
    // resource interface and the resource destructors
    exported_resources: HashSet<TypeId>,

    // the world ID
    world_id: Option<WorldId>,
}

impl TinyGo {
    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        direction: Direction,
    ) -> interface::InterfaceGenerator<'a> {
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
        let name = &resolve.worlds[world].name;
        self.world = name.to_string();
        self.sizes.fill(resolve);
        self.world_id = Some(world);
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
        self.interface_names.insert(id, name.clone());

        let mut gen = self.interface(resolve, Direction::Import);
        gen.interface = Some((id, name));
        gen.define_interface_types(id);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.import(resolve, func);
        }

        let src = mem::take(&mut gen.src);
        let preamble = mem::take(&mut gen.preamble);
        self.src.push_str(&src);
        self.preamble.append_src(&preamble);
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
        let src = mem::take(&mut gen.src);
        let preamble = mem::take(&mut gen.preamble);
        self.src.push_str(&src);
        self.preamble.append_src(&preamble);
    }

    fn pre_export_interface(&mut self, resolve: &Resolve, _files: &mut Files) -> Result<()> {
        let world = self.world_id.unwrap();
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
        self.interface_names.insert(id, name.clone());
        let name_raw = &resolve.name_world_key(name);
        self.src
            .push_str(&format!("// Export functions from {name_raw}\n"));

        let mut gen = self.interface(resolve, Direction::Export);
        gen.interface = Some((id, name));
        gen.define_interface_types(id);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.export(resolve, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        let preamble = mem::take(&mut gen.preamble);
        self.src.push_str(&src);
        self.preamble.append_src(&preamble);
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

        gen.finish();

        let src = mem::take(&mut gen.src);
        let preamble = mem::take(&mut gen.preamble);
        self.src.push_str(&src);
        self.preamble.append_src(&preamble);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(resolve, Direction::Import);
        let mut live = LiveTypes::default();
        for (_, id) in types {
            live.add_type_id(resolve, *id);
        }
        gen.define_live_types(&live);
        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        // make sure all types are defined on top of the file
        let src = mem::take(&mut self.src);
        self.src.push_str(&src);

        // prepend package and imports header
        let src = mem::take(&mut self.src);
        wit_bindgen_core::generated_preamble(&mut self.src, env!("CARGO_PKG_VERSION"));
        let snake = self.world.to_snake_case();
        // add package
        self.src.push_str("package ");
        self.src.push_str(&snake);
        self.src.push_str("\n\n");

        // import C
        self.src.push_str("// #include \"");
        self.src.push_str(self.world.to_snake_case().as_str());
        self.src.push_str(".h\"\n");
        if self.preamble.len() > 0 {
            self.src.append_src(&self.preamble);
        }
        self.src.push_str("import \"C\"\n");
        let world = &resolve.worlds[id];

        self.import_requirements.generate(
            snake,
            files,
            format!("{}_types.go", world.name.to_kebab_case()),
        );
        self.src.push_str(&self.import_requirements.src);

        self.src.push_str(&src);

        if self.opts.gofmt {
            let mut child = std::process::Command::new("gofmt")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn gofmt");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(self.src.as_bytes())
                .expect("failed to write to gofmt");
            self.src.as_mut_string().truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(self.src.as_mut_string())
                .expect("failed to read from gofmt");
            let status = child.wait().expect("failed to wait on gofmt");
            assert!(status.success());
        }
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            self.src.as_bytes(),
        );

        let mut opts = wit_bindgen_c::Opts::default();
        opts.no_sig_flattening = true;
        opts.no_object_file = true;
        opts.build()
            .generate(resolve, id, files)
            .expect("C generator should be infallible")
    }
}

fn avoid_keyword(s: &str) -> String {
    if GOKEYWORDS.contains(&s) {
        format!("{s}_")
    } else {
        s.into()
    }
}

// a list of Go keywords
const GOKEYWORDS: [&str; 26] = [
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
    // not a Go keyword but needs to escape due to
    // it's used as a variable name that passes to C
    "ret",
];
