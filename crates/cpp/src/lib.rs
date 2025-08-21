use anyhow::bail;
use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display, Write as FmtWrite},
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};
use symbol_name::{make_external_component, make_external_symbol};
use wit_bindgen_c::to_c_ident;
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Bitcast, LiftLower, WasmSignature, WasmType},
    symmetric, uwrite, uwriteln,
    wit_parser::{
        Alignment, ArchitectureSize, Docs, Function, FunctionKind, Handle, Int, InterfaceId,
        Resolve, SizeAlign, Stability, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, WorldId,
        WorldKey,
    },
    Files, InterfaceGenerator, Source, Types, WorldGenerator,
};

mod symbol_name;
mod wamr;

pub const RESOURCE_IMPORT_BASE_CLASS_NAME: &str = "ResourceImportBase";
pub const RESOURCE_EXPORT_BASE_CLASS_NAME: &str = "ResourceExportBase";
pub const RESOURCE_TABLE_NAME: &str = "ResourceTable";
pub const OWNED_CLASS_NAME: &str = "Owned";
pub const POINTER_SIZE_EXPRESSION: &str = "sizeof(void*)";

type CppType = String;

#[derive(Clone, Copy, Debug)]
enum Flavor {
    Argument(AbiVariant),
    Result(AbiVariant),
    InStruct,
    BorrowedArgument,
}

#[derive(Default)]
struct HighlevelSignature {
    /// this is a constructor or destructor without a written type
    // implicit_result: bool, -> empty result
    const_member: bool,
    static_member: bool,
    result: CppType,
    arguments: Vec<(String, CppType)>,
    name: String,
    namespace: Vec<String>,
    implicit_self: bool,
    post_return: bool,
}

// follows https://google.github.io/styleguide/cppguide.html

#[derive(Default)]
struct Includes {
    needs_vector: bool,
    needs_expected: bool,
    needs_string: bool,
    needs_string_view: bool,
    needs_optional: bool,
    needs_cstring: bool,
    needs_guest_alloc: bool,
    needs_imported_resources: bool,
    needs_exported_resources: bool,
    needs_variant: bool,
    needs_tuple: bool,
    needs_assert: bool,
    needs_bit: bool,
    needs_span: bool,
    // needs wit types
    needs_wit: bool,
    needs_memory: bool,
    needs_future: bool,
    needs_stream: bool,
    needs_array: bool,
}

#[derive(Clone)]
struct HostFunction {
    wasm_name: String,
    wamr_signature: String,
    host_name: String,
}

#[derive(Default)]
struct SourceWithState {
    src: Source,
    namespace: Vec<String>,
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
enum Direction {
    Import,
    Export,
}

#[derive(Default)]
struct Cpp {
    opts: Opts,
    c_src: SourceWithState,
    h_src: SourceWithState,
    c_src_head: Source,
    extern_c_decls: Source,
    dependencies: Includes,
    includes: Vec<String>,
    host_functions: HashMap<String, Vec<HostFunction>>,
    world: String,
    world_id: Option<WorldId>,
    imported_interfaces: HashSet<InterfaceId>,
    user_class_files: HashMap<String, String>,
    defined_types: HashSet<(Vec<String>, String)>,
    types: Types,

    // needed for symmetric disambiguation
    interface_prefixes: HashMap<(Direction, WorldKey), String>,
    import_prefix: Option<String>,

    temp: usize,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Generate host bindings
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub host: bool,
    /// Generate code for directly linking to guest code (WIP)
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default(), alias = "direct"))]
    pub short_cut: bool,
    /// Call clang-format on the generated code
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub format: bool,
    /// 64bit guest
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub wasm64: bool,

    /// Place each interface in its own file,
    /// this enables sharing bindings across projects
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub split_interfaces: bool,

    /// Optionally prefix any export names with the specified value.
    ///
    /// This is useful to avoid name conflicts when testing.
    #[cfg_attr(feature = "clap", arg(long))]
    pub export_prefix: Option<String>,

    /// Wrap all C++ classes inside a custom namespace.
    ///
    /// This avoids identical names across components, useful for native
    #[cfg_attr(feature = "clap", arg(long))]
    pub internal_prefix: Option<String>,

    /// Symmetric ABI, this enables to directly link components to each
    /// other and removes the primary distinction between host and guest.
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub symmetric: bool,

    /// Set API style to symmetric or asymmetric
    ///
    /// Symmetric API is the same API for imported and exported functions.
    /// Reduces the allocation overhead for symmetric ABI.
    #[cfg_attr(
        feature = "clap", 
        arg(
            long,
            default_value_t = APIStyle::default(),
            value_name = "STYLE",
        ),
    )]
    pub api_style: APIStyle,

    /// Whether to generate owning or borrowing type definitions for `record` arguments to imported functions.
    ///
    /// Valid values include:
    ///
    /// - `owning`: Generated types will be composed entirely of owning fields,
    /// regardless of whether they are used as parameters to imports or not.
    ///
    /// - `coarse-borrowing`: Generated types used as parameters to imports will be
    /// "deeply borrowing", i.e. contain references rather than owned values,
    /// so long as they don't contain resources, in which case they will be
    /// owning.
    ///
    /// - `fine-borrowing": Generated types used as parameters to imports will be
    /// "deeply borrowing", i.e. contain references rather than owned values
    /// for all fields that are not resources, which will be owning.
    #[cfg_attr(feature = "clap", arg(long, default_value_t = Ownership::Owning))]
    pub ownership: Ownership,

    /// Where to place output files
    #[cfg_attr(feature = "clap", arg(skip))]
    out_dir: Option<PathBuf>,
}

/// Supported API styles for the generated bindings.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum APIStyle {
    /// Imported functions borrow arguments, while exported functions receive owned arguments. Reduces the allocation overhead for the canonical ABI.
    #[default]
    Asymmetric,
    /// Same API for imported and exported functions. Reduces the allocation overhead for symmetric ABI.
    Symmetric,
}

impl Display for APIStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            APIStyle::Asymmetric => write!(f, "asymmetric"),
            APIStyle::Symmetric => write!(f, "symmetric"),
        }
    }
}

impl FromStr for APIStyle {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asymmetric" => Ok(APIStyle::Asymmetric),
            "symmetric" => Ok(APIStyle::Symmetric),
            _ => bail!(
                "unrecognized API style: `{}`; expected `asymmetric` or `symmetric`",
                s
            ),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Ownership {
    /// Generated types will be composed entirely of owning fields, regardless
    /// of whether they are used as parameters to imports or not.
    #[default]
    Owning,

    /// Generated types used as parameters to imports will be "deeply
    /// borrowing", i.e. contain references rather than owned values when
    /// applicable.
    CoarseBorrowing,

    /// Generated types used as parameters to imports will be "deeply
    /// borrowing", i.e. contain references rather than owned values
    /// for all fields that are not resources, which will be owning.
    FineBorrowing,
}

impl FromStr for Ownership {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owning" => Ok(Self::Owning),
            "coarse-borrowing" => Ok(Self::CoarseBorrowing),
            "fine-borrowing" => Ok(Self::FineBorrowing),
            _ => Err(format!(
                "unrecognized ownership: `{s}`; \
                 expected `owning`, `coarse-borrowing`, or `fine-borrowing`"
            )),
        }
    }
}

impl fmt::Display for Ownership {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Ownership::Owning => "owning",
            Ownership::CoarseBorrowing => "coarse-borrowing",
            Ownership::FineBorrowing => "fine-borrowing",
        })
    }
}

impl Opts {
    pub fn build(mut self, out_dir: Option<&PathBuf>) -> Box<dyn WorldGenerator> {
        let mut r = Cpp::new();
        self.out_dir = out_dir.cloned();
        r.opts = self;
        Box::new(r)
    }

    fn host_side(&self) -> bool {
        self.short_cut || self.host
    }

    fn is_only_handle(&self, variant: AbiVariant) -> bool {
        self.host_side() == matches!(variant, AbiVariant::GuestExport)
    }

    fn ptr_type(&self) -> &'static str {
        if !self.host {
            "uint8_t*"
        } else if self.wasm64 {
            "int64_t"
        } else {
            "int32_t"
        }
    }

    // we need to map pointers depending on context
    fn wasm_type(&self, ty: WasmType) -> &'static str {
        match ty {
            WasmType::Pointer => self.ptr_type(),
            _ => wit_bindgen_c::wasm_type(ty),
        }
    }
}

impl Cpp {
    fn new() -> Cpp {
        Cpp::default()
    }

    pub fn is_first_definition(&mut self, ns: &Vec<String>, name: &str) -> bool {
        let owned = (ns.to_owned(), name.to_owned());
        if !self.defined_types.contains(&owned) {
            self.defined_types.insert(owned);
            true
        } else {
            false
        }
    }

    fn include(&mut self, s: &str) {
        self.includes.push(s.to_string());
    }

    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: Option<&'a WorldKey>,
        in_guest_import: bool,
        wasm_import_module: Option<String>,
    ) -> CppInterfaceGenerator<'a> {
        let mut sizes = if self.opts.symmetric {
            SizeAlign::new_symmetric()
        } else {
            SizeAlign::default()
        };
        sizes.fill(resolve);

        CppInterfaceGenerator {
            _src: Source::default(),
            gen: self,
            resolve,
            interface: None,
            _name: name,
            sizes,
            in_guest_import,
            wasm_import_module,
        }
    }

    fn clang_format(code: &mut String) {
        let mut child = Command::new("clang-format")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn `clang-format`");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(code.as_bytes())
            .unwrap();
        code.truncate(0);
        child.stdout.take().unwrap().read_to_string(code).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());
    }

    fn perform_cast(&mut self, op: &str, cast: &Bitcast) -> String {
        match cast {
            Bitcast::I32ToF32 | Bitcast::I64ToF32 => {
                self.dependencies.needs_bit = true;
                format!("std::bit_cast<float, int32_t>({})", op)
            }
            Bitcast::F32ToI32 | Bitcast::F32ToI64 => {
                self.dependencies.needs_bit = true;
                format!("std::bit_cast<int32_t, float>({})", op)
            }
            Bitcast::I64ToF64 => {
                self.dependencies.needs_bit = true;
                format!("std::bit_cast<double, int64_t>({})", op)
            }
            Bitcast::F64ToI64 => {
                self.dependencies.needs_bit = true;
                format!("std::bit_cast<int64_t, double>({})", op)
            }
            Bitcast::I32ToI64 | Bitcast::LToI64 | Bitcast::PToP64 => {
                format!("static_cast<int64_t>({})", op)
            }
            Bitcast::I64ToI32 | Bitcast::PToI32 | Bitcast::LToI32 => {
                format!("static_cast<int32_t>({})", op)
            }
            Bitcast::P64ToI64 | Bitcast::None | Bitcast::I64ToP64 => op.to_string(),
            Bitcast::P64ToP | Bitcast::I32ToP | Bitcast::LToP => {
                format!("static_cast<uint8_t*>({})", op)
            }
            Bitcast::PToL | Bitcast::I32ToL | Bitcast::I64ToL => {
                format!("static_cast<size_t>({})", op)
            }
            Bitcast::Sequence(sequence) => {
                let [first, second] = &**sequence;
                let inner = self.perform_cast(op, first);
                self.perform_cast(&inner, second)
            }
        }
    }

    fn finish_includes(&mut self) {
        self.include("<cstdint>");
        self.include("<utility>"); // for std::move
        if self.dependencies.needs_string {
            self.include("<string>");
        }
        if self.dependencies.needs_string_view {
            self.include("<string_view>");
        }
        if self.dependencies.needs_vector {
            self.include("<vector>");
        }
        if self.dependencies.needs_expected {
            self.include("<expected>");
        }
        if self.dependencies.needs_future {
            self.include("<future>");
        }
        if self.dependencies.needs_stream {
            self.include("<stream_support.h>");
        }
        if self.dependencies.needs_optional {
            self.include("<optional>");
        }
        if self.dependencies.needs_cstring {
            self.include("<cstring>");
        }
        if self.dependencies.needs_imported_resources {
            self.include("<cassert>");
        }
        if self.dependencies.needs_exported_resources {
            self.include("<map>");
        }
        if self.dependencies.needs_variant {
            self.include("<variant>");
        }
        if self.dependencies.needs_tuple {
            self.include("<tuple>");
        }
        if self.dependencies.needs_wit {
            if self.opts.host_side() {
                self.include("<wit-host.h>");
            } else {
                self.include("\"wit.h\"");
            }
        }
        if self.dependencies.needs_memory {
            self.include("<memory>");
        }
        if self.dependencies.needs_array {
            self.include("<array>");
        }
        if self.dependencies.needs_bit {
            self.include("<bit>");
        }
    }

    fn start_new_file(&mut self, condition: Option<bool>) -> FileContext {
        if condition == Some(true) || self.opts.split_interfaces {
            FileContext {
                includes: std::mem::take(&mut self.includes),
                src: std::mem::take(&mut self.h_src),
                dependencies: std::mem::take(&mut self.dependencies),
            }
        } else {
            Default::default()
        }
    }

    fn finish_file(&mut self, namespace: &[String], store: FileContext) {
        if !store.src.src.is_empty() {
            let mut header = String::default();
            self.finish_includes();
            self.h_src.change_namespace(&[]);
            uwriteln!(header, "#pragma once");
            if self.opts.symmetric {
                uwriteln!(header, "#define WIT_SYMMETRIC");
            }
            for include in self.includes.iter() {
                uwriteln!(header, "#include {include}");
            }
            header.push_str(&self.h_src.src);
            let mut filename = namespace.join("-");
            filename.push_str(".h");
            if self.opts.format {
                Self::clang_format(&mut header);
            }
            self.user_class_files.insert(filename.clone(), header);

            let _ = std::mem::replace(&mut self.includes, store.includes);
            let _ = std::mem::replace(&mut self.h_src, store.src);
            let _ = std::mem::replace(&mut self.dependencies, store.dependencies);
            self.includes.push(String::from("\"") + &filename + "\"");
        }
    }

    fn tmp(&mut self) -> usize {
        self.temp += 1;
        self.temp
    }
}

#[derive(Default)]
struct FileContext {
    includes: Vec<String>,
    src: SourceWithState,
    dependencies: Includes,
}

impl WorldGenerator for Cpp {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.world = name.to_string();
        self.types.analyze(resolve);
        self.world_id = Some(world);
        //        self.sizes.fill(resolve);
        if !self.opts.host_side() {
            let export_name = if self.opts.symmetric {
                ""
            } else {
                ", __export_name__(\"cabi_realloc\")"
            };
            uwriteln!(
                self.c_src_head,
                r#"#include "{}_cpp.h"
            #include <cstdlib> // realloc

            extern "C" void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size);

            __attribute__((__weak__{export_name}))
            void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {{
                (void) old_size;
                void *ret = reinterpret_cast<void*>(align);
                if (new_size != 0) {{
                    ret = realloc(ptr, new_size);
                    if (!ret) {{ abort(); }}
                }}
                return ret;
            }}

            "#,
                self.world.to_snake_case(),
            );
        }
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        if let Some(prefix) = self
            .interface_prefixes
            .get(&(Direction::Import, name.clone()))
        {
            self.import_prefix = Some(prefix.clone());
        }

        let store = self.start_new_file(None);
        self.imported_interfaces.insert(id);
        let wasm_import_module = resolve.name_world_key(name);
        let binding = Some(name);
        let mut gen = self.interface(resolve, binding, true, Some(wasm_import_module));
        gen.interface = Some(id);
        gen.types(id);
        let namespace = namespace(resolve, &TypeOwner::Interface(id), false, &gen.gen.opts);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            if matches!(func.kind, FunctionKind::Freestanding) {
                gen.gen.h_src.change_namespace(&namespace);
                gen.generate_function(func, &TypeOwner::Interface(id), AbiVariant::GuestImport);
            }
        }
        self.finish_file(&namespace, store);
        let _ = self.import_prefix.take();
        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let old_prefix = self.opts.export_prefix.clone();
        if let Some(prefix) = self
            .interface_prefixes
            .get(&(Direction::Export, name.clone()))
        {
            self.opts.export_prefix =
                Some(prefix.clone() + old_prefix.as_ref().unwrap_or(&String::new()));
        }
        let store = self.start_new_file(None);
        self.h_src
            .src
            .push_str(&format!("// export_interface {name:?}\n"));
        self.imported_interfaces.remove(&id);
        let wasm_import_module = resolve.name_world_key(name);
        let binding = Some(name);
        let mut gen = self.interface(resolve, binding, false, Some(wasm_import_module));
        gen.interface = Some(id);
        gen.types(id);
        let namespace = namespace(resolve, &TypeOwner::Interface(id), true, &gen.gen.opts);

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            if matches!(func.kind, FunctionKind::Freestanding) {
                gen.gen.h_src.change_namespace(&namespace);
                gen.generate_function(func, &TypeOwner::Interface(id), AbiVariant::GuestExport);
            }
        }
        self.finish_file(&namespace, store);
        self.opts.export_prefix = old_prefix;
        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = WorldKey::Name("$root".to_string()); //WorldKey::Name(resolve.worlds[world].name.clone());
        let wasm_import_module = resolve.name_world_key(&name);
        let binding = Some(name);
        let mut gen = self.interface(resolve, binding.as_ref(), true, Some(wasm_import_module));
        let namespace = namespace(resolve, &TypeOwner::World(world), false, &gen.gen.opts);

        for (_name, func) in funcs.iter() {
            if matches!(func.kind, FunctionKind::Freestanding) {
                gen.gen.h_src.change_namespace(&namespace);
                gen.generate_function(func, &TypeOwner::World(world), AbiVariant::GuestImport);
            }
        }
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let name = WorldKey::Name(resolve.worlds[world].name.clone());
        let binding = Some(name);
        let mut gen = self.interface(resolve, binding.as_ref(), false, None);
        let namespace = namespace(resolve, &TypeOwner::World(world), true, &gen.gen.opts);

        for (_name, func) in funcs.iter() {
            if matches!(func.kind, FunctionKind::Freestanding) {
                gen.gen.h_src.change_namespace(&namespace);
                gen.generate_function(func, &TypeOwner::World(world), AbiVariant::GuestExport);
            }
        }
        Ok(())
    }

    fn import_types(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        for i in types.iter() {
            uwriteln!(self.h_src.src, "// import_type {}", i.0);
        }
    }

    fn finish(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        files: &mut Files,
    ) -> std::result::Result<(), anyhow::Error> {
        let world = &resolve.worlds[world_id];
        let snake = world.name.to_snake_case();
        let linking_symbol = wit_bindgen_c::component_type_object::linking_symbol(&world.name);

        let mut h_str = SourceWithState::default();
        let mut c_str = SourceWithState::default();

        let version = env!("CARGO_PKG_VERSION");
        uwriteln!(
            h_str.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );

        if self.opts.short_cut {
            uwrite!(
                h_str.src,
                "#ifndef __CPP_NATIVE_BINDINGS_{0}_H
                #define __CPP_NATIVE_BINDINGS_{0}_H\n",
                world.name.to_shouty_snake_case(),
            );
        } else if !self.opts.host {
            uwrite!(
                h_str.src,
                "#ifndef __CPP_GUEST_BINDINGS_{0}_H
                #define __CPP_GUEST_BINDINGS_{0}_H\n",
                world.name.to_shouty_snake_case(),
            );
        } else {
            uwrite!(
                h_str.src,
                "#ifndef __CPP_HOST_BINDINGS_{0}_H
                #define __CPP_HOST_BINDINGS_{0}_H
                struct WASMExecEnv; // WAMR execution environment\n",
                world.name.to_shouty_snake_case(),
            );
        }
        if self.dependencies.needs_future || self.dependencies.needs_stream {
            uwriteln!(self.c_src_head, "#include \"async_support.h\"")
        }
        self.finish_includes();

        if self.opts.short_cut {
            uwriteln!(h_str.src, "#define WIT_HOST_DIRECT");
        } else if self.opts.symmetric {
            uwriteln!(h_str.src, "#define WIT_SYMMETRIC");
        }
        for include in self.includes.iter() {
            uwriteln!(h_str.src, "#include {include}");
        }

        uwriteln!(
            c_str.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );
        if self.opts.short_cut {
            uwriteln!(c_str.src, "#include \"{snake}_cpp_native.h\"");
        } else if !self.opts.host {
            uwriteln!(
                c_str.src,
                "\n// Ensure that the *_component_type.o object is linked in"
            );
            uwrite!(
                c_str.src,
                "#ifdef __wasm32__
                   extern \"C\" void {linking_symbol}(void);
                   __attribute__((used))
                   void {linking_symbol}_public_use_in_this_compilation_unit(void) {{
                       {linking_symbol}();
                   }}
                   #endif
               ",
            );
        } else {
            uwriteln!(c_str.src, "#include \"{snake}_cpp_host.h\"");
            uwriteln!(
                c_str.src,
                "#include <wasm_export.h> // wasm-micro-runtime header\n\
                 #include <wasm_c_api.h>\n\
                 #include <assert.h>"
            );

            if c_str.src.len() > 0 {
                c_str.src.push_str("\n");
            }
            if self.dependencies.needs_guest_alloc {
                uwriteln!(
                    c_str.src,
                    "int32_t guest_alloc(wasm_exec_env_t exec_env, uint32_t size);"
                );
            }
        }
        if self.opts.host_side() && self.dependencies.needs_exported_resources {
            uwriteln!(
                c_str.src,
                "template <class R> std::map<int32_t, R> wit::{RESOURCE_TABLE_NAME}<R>::resources;"
            );
        }
        if self.dependencies.needs_assert {
            uwriteln!(c_str.src, "#include <assert.h>");
        }
        if self.dependencies.needs_span {
            uwriteln!(c_str.src, "#include <span>");
        }

        h_str.change_namespace(&Vec::default());

        self.c_src.change_namespace(&Vec::default());
        c_str.src.push_str(&self.c_src_head);
        c_str.src.push_str(&self.extern_c_decls);
        c_str.src.push_str(&self.c_src.src);
        self.h_src.change_namespace(&Vec::default());
        h_str.src.push_str(&self.h_src.src);

        uwriteln!(c_str.src, "\n// Component Adapters");

        if !self.opts.short_cut && self.opts.host {
            uwriteln!(
                h_str.src,
                "extern \"C\" void register_{}();",
                world.name.to_snake_case()
            );
            uwriteln!(
                c_str.src,
                "void register_{}() {{",
                world.name.to_snake_case()
            );
            for i in self.host_functions.iter() {
                uwriteln!(
                    c_str.src,
                    "  static NativeSymbol {}_funs[] = {{",
                    i.0.replace(&[':', '.', '-', '+'], "_").to_snake_case()
                );
                for f in i.1.iter() {
                    uwriteln!(
                        c_str.src,
                        "    {{ \"{}\", (void*){}, \"{}\", nullptr }},",
                        f.wasm_name,
                        f.host_name,
                        f.wamr_signature
                    );
                }
                uwriteln!(c_str.src, "  }};");
            }
            for i in self.host_functions.iter() {
                uwriteln!(c_str.src, "  wasm_runtime_register_natives(\"{}\", {1}_funs, sizeof({1}_funs)/sizeof(NativeSymbol));", i.0, i.0.replace(&[':','.','-','+'], "_").to_snake_case());
            }
            uwriteln!(c_str.src, "}}");
        }

        uwriteln!(
            h_str.src,
            "
            #endif"
        );

        if self.opts.format {
            Self::clang_format(&mut c_str.src.as_mut_string());
            Self::clang_format(&mut h_str.src.as_mut_string());
        }

        if self.opts.short_cut {
            files.push(&format!("{snake}_native.cpp"), c_str.src.as_bytes());
            files.push(&format!("{snake}_cpp_native.h"), h_str.src.as_bytes());
        } else if !self.opts.host {
            files.push(&format!("{snake}.cpp"), c_str.src.as_bytes());
            files.push(&format!("{snake}_cpp.h"), h_str.src.as_bytes());
        } else {
            files.push(&format!("{snake}_host.cpp"), c_str.src.as_bytes());
            files.push(&format!("{snake}_cpp_host.h"), h_str.src.as_bytes());
        }
        for (name, content) in self.user_class_files.iter() {
            // if the user class file exists create an updated .template
            let dst = match &self.opts.out_dir {
                Some(path) => path.join(name),
                None => name.into(),
            };
            if std::path::Path::exists(&dst) {
                files.push(&(String::from(name) + ".template"), content.as_bytes());
            } else {
                files.push(name, content.as_bytes());
            }
        }
        files.push(
            &format!("{snake}_component_type.o",),
            wit_bindgen_c::component_type_object::object(
                resolve,
                world_id,
                &world.name,
                wit_component::StringEncoding::UTF8,
                None,
            )
            .unwrap()
            .as_slice(),
        );
        if self.opts.symmetric {
            // this keeps the symbols down for shared objects, could be more specific
            files.push(
                &format!("{}.verscr", world.name),
                b"{\n  global:\n    *X00*;\n  local: *;\n};\n",
            );
        }

        if self.dependencies.needs_wit {
            files.push(&format!("wit.h"), include_bytes!("../helper-types/wit.h"));
        }
        Ok(())
    }

    fn apply_resolve_options(&mut self, resolve: &mut Resolve, world: &mut WorldId) {
        if self.opts.symmetric {
            let world = &resolve.worlds[*world];
            let exports: HashMap<&WorldKey, &wit_bindgen_core::wit_parser::WorldItem> =
                world.exports.iter().collect();
            for (key, _item) in world.imports.iter() {
                // duplicate found
                if exports.contains_key(key)
                    && !self
                        .interface_prefixes
                        .contains_key(&(Direction::Import, key.clone()))
                    && !self
                        .interface_prefixes
                        .contains_key(&(Direction::Export, key.clone()))
                {
                    self.interface_prefixes
                        .insert((Direction::Import, key.clone()), "imp_".into());
                    self.interface_prefixes
                        .insert((Direction::Export, key.clone()), "exp_".into());
                }
            }
        }
    }
}

// determine namespace (for the lifted C++ function)
fn namespace(resolve: &Resolve, owner: &TypeOwner, guest_export: bool, opts: &Opts) -> Vec<String> {
    let mut result = Vec::default();
    if let Some(prefix) = &opts.internal_prefix {
        result.push(prefix.clone());
    }
    if guest_export {
        result.push(String::from("exports"));
    }
    match owner {
        TypeOwner::World(w) => result.push(resolve.worlds[*w].name.to_snake_case()),
        TypeOwner::Interface(i) => {
            let iface = &resolve.interfaces[*i];
            let pkg = &resolve.packages[iface.package.unwrap()];
            result.push(pkg.name.namespace.to_snake_case());
            result.push(pkg.name.name.to_snake_case());
            if let Some(name) = &iface.name {
                result.push(name.to_snake_case());
            }
        }
        TypeOwner::None => (),
    }
    result
}

impl SourceWithState {
    fn change_namespace(&mut self, target: &[String]) {
        let mut same = 0;
        // itertools::fold_while?
        for (a, b) in self.namespace.iter().zip(target.iter()) {
            if a == b {
                same += 1;
            } else {
                break;
            }
        }
        for _i in same..self.namespace.len() {
            uwrite!(self.src, "}}\n");
        }
        self.namespace.truncate(same);
        for i in target.iter().skip(same) {
            uwrite!(self.src, "namespace {} {{\n", i);
            self.namespace.push(i.clone());
        }
    }

    fn qualify(&mut self, target: &[String]) {
        let mut same = 0;
        // itertools::fold_while?
        for (a, b) in self.namespace.iter().zip(target.iter()) {
            if a == b {
                same += 1;
            } else {
                break;
            }
        }
        if same == 0 && !target.is_empty() {
            // if the root namespace exists below the current namespace we need to start at root
            if self.namespace.contains(target.first().unwrap()) {
                self.src.push_str("::");
            }
        }
        if same == target.len() && self.namespace.len() != target.len() && same > 0 {
            // namespace is parent, qualify at least one namespace (and cross fingers)
            uwrite!(self.src, "{}::", target[same - 1]);
        } else {
            for i in target.iter().skip(same) {
                uwrite!(self.src, "{i}::");
            }
        }
    }
}

struct CppInterfaceGenerator<'a> {
    _src: Source,
    gen: &'a mut Cpp,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
    _name: Option<&'a WorldKey>,
    sizes: SizeAlign,
    in_guest_import: bool,
    pub wasm_import_module: Option<String>,
}

impl CppInterfaceGenerator<'_> {
    fn types(&mut self, iface: InterfaceId) {
        let iface = &self.resolve().interfaces[iface];
        for (name, id) in iface.types.iter() {
            self.define_type(name, *id);
        }
    }

    fn define_type(&mut self, name: &str, id: TypeId) {
        let ty = &self.resolve().types[id];
        match &ty.kind {
            TypeDefKind::Record(record) => self.type_record(id, name, record, &ty.docs),
            TypeDefKind::Resource => self.type_resource(id, name, &ty.docs),
            TypeDefKind::Flags(flags) => self.type_flags(id, name, flags, &ty.docs),
            TypeDefKind::Tuple(tuple) => self.type_tuple(id, name, tuple, &ty.docs),
            TypeDefKind::Enum(enum_) => self.type_enum(id, name, enum_, &ty.docs),
            TypeDefKind::Variant(variant) => self.type_variant(id, name, variant, &ty.docs),
            TypeDefKind::Option(t) => self.type_option(id, name, t, &ty.docs),
            TypeDefKind::Result(r) => self.type_result(id, name, r, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(_) => todo!("generate for future"),
            TypeDefKind::Stream(_) => todo!("generate for stream"),
            TypeDefKind::Handle(_) => todo!("generate for handle"),
            TypeDefKind::FixedSizeList(_, _) => todo!(),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    /// This describes the C++ side name
    fn func_namespace_name(
        &self,
        func: &Function,
        guest_export: bool,
        cpp_file: bool,
    ) -> (Vec<String>, String) {
        let (object, owner) = match &func.kind {
            FunctionKind::Freestanding => None,
            FunctionKind::Method(i) => Some(i),
            FunctionKind::Static(i) => Some(i),
            FunctionKind::Constructor(i) => Some(i),
            FunctionKind::AsyncFreestanding => todo!(),
            FunctionKind::AsyncMethod(_id) => todo!(),
            FunctionKind::AsyncStatic(_id) => todo!(),
        }
        .map(|i| {
            let ty = &self.resolve.types[*i];
            (ty.name.as_ref().unwrap().to_pascal_case(), ty.owner)
        })
        .unwrap_or((
            Default::default(),
            self.interface
                .map(TypeOwner::Interface)
                .unwrap_or(TypeOwner::World(self.gen.world_id.unwrap())),
        ));
        let mut namespace = namespace(self.resolve, &owner, guest_export, &self.gen.opts);
        let is_drop = is_special_method(func);
        let func_name_h = if !matches!(&func.kind, FunctionKind::Freestanding) {
            namespace.push(object.clone());
            if let FunctionKind::Constructor(_i) = &func.kind {
                if guest_export && cpp_file {
                    String::from("New")
                } else {
                    object.clone()
                }
            } else {
                match is_drop {
                    SpecialMethod::ResourceDrop => {
                        if self.gen.opts.host_side() && !guest_export {
                            "Dtor".to_string()
                        } else if guest_export {
                            "ResourceDrop".to_string()
                        } else {
                            "~".to_string() + &object
                        }
                    }
                    SpecialMethod::Dtor => {
                        if self.gen.opts.host_side() && guest_export {
                            "~".to_string() + &object
                        } else {
                            "Dtor".to_string()
                        }
                    }
                    SpecialMethod::ResourceNew => "ResourceNew".to_string(),
                    SpecialMethod::ResourceRep => "ResourceRep".to_string(),
                    SpecialMethod::Allocate => "New".to_string(),
                    SpecialMethod::None => func.item_name().to_pascal_case(),
                }
            }
        } else {
            func.name.to_pascal_case()
        };
        (namespace, func_name_h)
    }

    // local patching of borrows function needs more complex solution
    fn patched_wasm_signature(&self, variant: AbiVariant, func: &Function) -> WasmSignature {
        self.resolve
            .wasm_signature_symmetric(variant, func, self.gen.opts.symmetric)
    }

    // print the signature of the guest export (lowered (wasm) function calling into highlevel)
    fn print_export_signature(&mut self, func: &Function, variant: AbiVariant) -> Vec<String> {
        let is_drop = is_special_method(func);
        let id_type = if self.gen.opts.symmetric {
            WasmType::Pointer
        } else {
            WasmType::I32
        };
        let signature = match is_drop {
            SpecialMethod::ResourceDrop => WasmSignature {
                params: vec![id_type],
                results: Vec::new(),
                indirect_params: false,
                retptr: false,
            },
            SpecialMethod::ResourceRep => WasmSignature {
                params: vec![id_type],
                results: vec![WasmType::Pointer],
                indirect_params: false,
                retptr: false,
            },
            SpecialMethod::Dtor => WasmSignature {
                params: vec![WasmType::Pointer],
                results: Vec::new(),
                indirect_params: false,
                retptr: false,
            },
            SpecialMethod::ResourceNew => WasmSignature {
                params: vec![WasmType::Pointer],
                results: vec![id_type],
                indirect_params: false,
                retptr: false,
            },
            SpecialMethod::None => {
                // TODO perhaps remember better names for the arguments
                self.patched_wasm_signature(variant, func)
            }
            SpecialMethod::Allocate => WasmSignature {
                params: vec![],
                results: vec![],
                indirect_params: false,
                retptr: false,
            },
        };
        let mut module_name = self.wasm_import_module.clone();
        let mut symbol_variant = variant;
        if self.gen.opts.symmetric && matches!(variant, AbiVariant::GuestExport) {
            // symmetric doesn't distinguish
            symbol_variant = AbiVariant::GuestImport;
        }
        if matches!(variant, AbiVariant::GuestExport)
            && matches!(
                is_drop,
                SpecialMethod::ResourceNew
                    | SpecialMethod::ResourceDrop
                    | SpecialMethod::ResourceRep
            )
        {
            module_name = Some(String::from("[export]") + &module_name.unwrap());
            if self.gen.opts.host_side() {
                symbol_variant = AbiVariant::GuestImport;
            }
        }
        let func_name = if self.gen.opts.symmetric && matches!(is_drop, SpecialMethod::Dtor) {
            // replace [dtor] with [resource_drop]
            format!("[resource_drop]{}", &func.name[6..])
        } else {
            func.name.clone()
        };
        if self.gen.opts.short_cut {
            uwrite!(self.gen.c_src.src, "extern \"C\" ");
        } else if self.gen.opts.host {
            self.gen.c_src.src.push_str("static ");
        } else {
            let module_prefix = module_name.as_ref().map_or(String::default(), |name| {
                let mut res = name.clone();
                res.push('#');
                res
            });
            if self.gen.opts.symmetric {
                uwriteln!(self.gen.c_src.src, r#"extern "C" "#);
            } else {
                uwriteln!(
                    self.gen.c_src.src,
                    r#"extern "C" __attribute__((__export_name__("{module_prefix}{func_name}")))"#
                );
            }
        }
        let return_via_pointer = signature.retptr && self.gen.opts.host_side();
        self.gen
            .c_src
            .src
            .push_str(if signature.results.is_empty() || return_via_pointer {
                "void"
            } else {
                self.gen.opts.wasm_type(signature.results[0])
            });
        self.gen.c_src.src.push_str(" ");
        let export_name = match module_name {
            Some(ref module_name) => make_external_symbol(&module_name, &func_name, symbol_variant),
            None => make_external_component(&func_name),
        };
        if let Some(prefix) = self.gen.opts.export_prefix.as_ref() {
            self.gen.c_src.src.push_str(prefix);
        }
        self.gen.c_src.src.push_str(&export_name);
        self.gen.c_src.src.push_str("(");
        let mut first_arg = true;
        if self.gen.opts.host {
            self.gen.c_src.src.push_str("wasm_exec_env_t exec_env");
            first_arg = false;
        }
        let mut params = Vec::new();
        for (n, ty) in signature.params.iter().enumerate() {
            let name = format!("arg{n}");
            if !first_arg {
                self.gen.c_src.src.push_str(", ");
            } else {
                first_arg = false;
            }
            self.gen.c_src.src.push_str(self.gen.opts.wasm_type(*ty));
            self.gen.c_src.src.push_str(" ");
            self.gen.c_src.src.push_str(&name);
            params.push(name);
        }
        if return_via_pointer {
            if !first_arg {
                self.gen.c_src.src.push_str(", ");
            }
            self.gen.c_src.src.push_str(self.gen.opts.ptr_type());
            self.gen.c_src.src.push_str(" resultptr");
            params.push("resultptr".into());
        }
        self.gen.c_src.src.push_str(")\n");
        if self.gen.opts.host_side() {
            let signature = wamr::wamr_signature(self.resolve, func);
            let remember = HostFunction {
                wasm_name: func_name.clone(),
                wamr_signature: signature.to_string(),
                host_name: export_name.clone(),
            };
            self.gen
                .host_functions
                .entry(module_name.unwrap_or(self.gen.world.clone()))
                .and_modify(|v| v.push(remember.clone()))
                .or_insert(vec![remember]);
        }
        params
    }

    fn high_level_signature(
        &mut self,
        func: &Function,
        abi_variant: AbiVariant,
        outer_namespace: &[String],
    ) -> HighlevelSignature {
        let mut res = HighlevelSignature::default();

        let (namespace, func_name_h) =
            self.func_namespace_name(func, matches!(abi_variant, AbiVariant::GuestExport), false);
        res.name = func_name_h;
        res.namespace = namespace;
        let is_drop = is_special_method(func);
        // we might want to separate c_sig and h_sig
        // let mut sig = String::new();
        if self.gen.opts.symmetric && matches!(is_drop, SpecialMethod::ResourceNew) {
            res.result = "uint8_t*".into();
        } else
        // not for ctor nor imported dtor on guest
        if !matches!(&func.kind, FunctionKind::Constructor(_))
            && !(matches!(is_drop, SpecialMethod::ResourceDrop)
                && matches!(abi_variant, AbiVariant::GuestImport)
                && !self.gen.opts.host_side())
            && !(matches!(is_drop, SpecialMethod::Dtor)
                && matches!(abi_variant, AbiVariant::GuestExport)
                && self.gen.opts.host_side())
        {
            if matches!(is_drop, SpecialMethod::Allocate) {
                res.result.push_str("Owned");
            }
            if let Some(ty) = &func.result {
                res.result.push_str(
                    &(self.type_name(ty, outer_namespace, Flavor::Result(abi_variant))
                        + if matches!(is_drop, SpecialMethod::ResourceRep) {
                            "*"
                        } else {
                            ""
                        }),
                );
            } else {
                res.result = "void".into();
            }
            if matches!(abi_variant, AbiVariant::GuestExport)
                && abi::guest_export_needs_post_return(self.resolve, func)
            {
                res.post_return = true;
            }
        }
        if matches!(func.kind, FunctionKind::Static(_))
            && !(matches!(&is_drop, SpecialMethod::ResourceDrop)
                && matches!(abi_variant, AbiVariant::GuestImport)
                && !self.gen.opts.host_side())
            && !(matches!(&is_drop, SpecialMethod::Dtor)
                && matches!(abi_variant, AbiVariant::GuestExport)
                && self.gen.opts.host_side())
        {
            res.static_member = true;
        }
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i == 0
                && name == "self"
                && (matches!(&func.kind, FunctionKind::Method(_))
                    || (matches!(&is_drop, SpecialMethod::ResourceDrop)
                        && matches!(abi_variant, AbiVariant::GuestImport)
                        && !self.gen.opts.host_side())
                    || (matches!(&is_drop, SpecialMethod::Dtor)
                        && matches!(abi_variant, AbiVariant::GuestExport)
                        && self.gen.opts.host_side()))
            {
                res.implicit_self = true;
                continue;
            }
            if self.gen.opts.symmetric
                && matches!(
                    &is_drop,
                    SpecialMethod::ResourceRep | SpecialMethod::ResourceDrop
                )
            {
                res.arguments
                    .push((name.to_snake_case(), "uint8_t*".into()));
            } else {
                let is_pointer = if matches!(
                    (&is_drop, self.gen.opts.host_side()),
                    (SpecialMethod::Dtor, _)
                        | (SpecialMethod::ResourceNew, _)
                        | (SpecialMethod::ResourceDrop, true)
                ) {
                    "*"
                } else {
                    ""
                };
                res.arguments.push((
                    name.to_snake_case(),
                    self.type_name(param, &res.namespace, Flavor::Argument(abi_variant))
                        + is_pointer,
                ));
            }
        }
        // default to non-const when exporting a method
        let import = matches!(abi_variant, AbiVariant::GuestImport) ^ self.gen.opts.host_side();
        if matches!(func.kind, FunctionKind::Method(_)) && import {
            res.const_member = true;
        }
        res
    }

    fn print_signature(
        &mut self,
        func: &Function,
        variant: AbiVariant,
        import: bool,
    ) -> Vec<String> {
        let is_special = is_special_method(func);
        let from_namespace = self.gen.h_src.namespace.clone();
        let cpp_sig = self.high_level_signature(func, variant, &from_namespace);
        if cpp_sig.static_member {
            self.gen.h_src.src.push_str("static ");
        }
        self.gen.h_src.src.push_str(&cpp_sig.result);
        if !cpp_sig.result.is_empty() {
            self.gen.h_src.src.push_str(" ");
        }
        self.gen.h_src.src.push_str(&cpp_sig.name);
        self.gen.h_src.src.push_str("(");
        for (num, (arg, typ)) in cpp_sig.arguments.iter().enumerate() {
            if num > 0 {
                self.gen.h_src.src.push_str(", ");
            }
            self.gen.h_src.src.push_str(typ);
            self.gen.h_src.src.push_str(" ");
            self.gen.h_src.src.push_str(arg);
        }
        self.gen.h_src.src.push_str(")");
        if cpp_sig.const_member {
            self.gen.h_src.src.push_str(" const");
        }
        match (&is_special, false, &variant) {
            (SpecialMethod::Allocate, _, _) => {
                uwriteln!(
                    self.gen.h_src.src,
                    "{{\
                        return {OWNED_CLASS_NAME}(new {}({}));\
                    }}",
                    cpp_sig.namespace.last().unwrap(), //join("::"),
                    cpp_sig
                        .arguments
                        .iter()
                        .map(|(arg, _)| format!("std::move({})", arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                // body is inside the header
                return Vec::default();
            }
            (SpecialMethod::Dtor, _, _ /*AbiVariant::GuestImport*/)
            | (SpecialMethod::ResourceDrop, true, _) => {
                uwriteln!(
                    self.gen.h_src.src,
                    "{{\
                        delete {};\
                    }}",
                    cpp_sig.arguments.first().unwrap().0
                );
            }
            _ => self.gen.h_src.src.push_str(";\n"),
        }

        // we want to separate the lowered signature (wasm) and the high level signature
        if !(import == true
            && self.gen.opts.host_side()
            && matches!(
                &is_special,
                SpecialMethod::ResourceDrop
                    | SpecialMethod::ResourceNew
                    | SpecialMethod::ResourceRep
            ))
        {
            let from_namespace = self.gen.h_src.namespace.clone();
            let cpp_sig = self.high_level_signature(func, variant, &from_namespace);
            if cpp_sig.static_member {
                self.gen.h_src.src.push_str("static ");
            }
            if cpp_sig.post_return && self.gen.opts.host_side() {
                self.gen.h_src.src.push_str("wit::guest_owned<");
            }
            self.gen.h_src.src.push_str(&cpp_sig.result);
            if cpp_sig.post_return && self.gen.opts.host_side() {
                self.gen.h_src.src.push_str(">");
            }
            if !cpp_sig.result.is_empty() {
                self.gen.h_src.src.push_str(" ");
            }
            self.gen.h_src.src.push_str(&cpp_sig.name);
            self.gen.h_src.src.push_str("(");
            if
            /*import &&*/
            self.gen.opts.host && !matches!(func.kind, FunctionKind::Method(_)) {
                self.gen.h_src.src.push_str("WASMExecEnv* exec_env");
                if !cpp_sig.arguments.is_empty() {
                    self.gen.h_src.src.push_str(", ");
                }
            }
            for (num, (arg, typ)) in cpp_sig.arguments.iter().enumerate() {
                if num > 0 {
                    self.gen.h_src.src.push_str(", ");
                }
                self.gen.h_src.src.push_str(typ);
                self.gen.h_src.src.push_str(" ");
                self.gen.h_src.src.push_str(arg);
            }
            self.gen.h_src.src.push_str(")");
            if cpp_sig.const_member {
                self.gen.h_src.src.push_str(" const");
            }
            match (&is_special, self.gen.opts.host_side(), &variant) {
                (SpecialMethod::Allocate, _, _) => {
                    uwrite!(
                        self.gen.h_src.src,
                        "{{\
                        return {OWNED_CLASS_NAME}(new {}({}));\
                    }}",
                        cpp_sig.namespace.last().unwrap(),
                        cpp_sig
                            .arguments
                            .iter()
                            .map(|(arg, _)| arg.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    // body is inside the header
                    return Vec::default();
                }
                (SpecialMethod::Dtor, _, AbiVariant::GuestImport)
                | (SpecialMethod::ResourceDrop, true, _) => {
                    uwrite!(
                        self.gen.h_src.src,
                        "{{\
                        delete {};\
                    }}",
                        cpp_sig.arguments.get(0).unwrap().0
                    );
                }
                _ => self.gen.h_src.src.push_str(";\n"),
            }
        }
        //        drop(cpp_sig);

        // we want to separate the lowered signature (wasm) and the high level signature
        if (!import
            && (self.gen.opts.host_side()
                || !matches!(
                    &is_special,
                    SpecialMethod::ResourceDrop
                        | SpecialMethod::ResourceNew
                        | SpecialMethod::ResourceRep
                )))
            || (import
                && self.gen.opts.host_side()
                && matches!(
                    &is_special,
                    SpecialMethod::ResourceDrop
                        | SpecialMethod::ResourceNew
                        | SpecialMethod::ResourceRep
                ))
        {
            self.print_export_signature(func, variant)
        } else {
            // recalulate with c file namespace
            let c_namespace = self.gen.c_src.namespace.clone();
            let cpp_sig = self.high_level_signature(func, variant, &c_namespace);
            let mut params = Vec::new();
            if cpp_sig.post_return && self.gen.opts.host_side() {
                self.gen.c_src.src.push_str("wit::guest_owned<");
            }
            self.gen.c_src.src.push_str(&cpp_sig.result);
            if cpp_sig.post_return && self.gen.opts.host_side() {
                self.gen.c_src.src.push_str(">");
            }
            if !cpp_sig.result.is_empty() {
                self.gen.c_src.src.push_str(" ");
            }
            self.gen.c_src.qualify(&cpp_sig.namespace);
            self.gen.c_src.src.push_str(&cpp_sig.name);
            self.gen.c_src.src.push_str("(");
            if import && self.gen.opts.host && !matches!(func.kind, FunctionKind::Method(_)) {
                self.gen.c_src.src.push_str("wasm_exec_env_t exec_env");
                if !cpp_sig.arguments.is_empty() || cpp_sig.implicit_self {
                    self.gen.c_src.src.push_str(", ");
                }
            }
            if cpp_sig.implicit_self {
                params.push("(*this)".into());
            }
            for (num, (arg, typ)) in cpp_sig.arguments.iter().enumerate() {
                if num > 0 {
                    self.gen.c_src.src.push_str(", ");
                }
                self.gen.c_src.src.push_str(typ);
                self.gen.c_src.src.push_str(" ");
                self.gen.c_src.src.push_str(arg);
                params.push(arg.clone());
            }
            self.gen.c_src.src.push_str(")");
            if cpp_sig.const_member {
                self.gen.c_src.src.push_str(" const");
            }
            self.gen.c_src.src.push_str("\n");
            params
        }
    }

    fn generate_function(&mut self, func: &Function, owner: &TypeOwner, variant: AbiVariant) {
        fn class_namespace(
            cifg: &CppInterfaceGenerator,
            func: &Function,
            variant: AbiVariant,
        ) -> Vec<String> {
            let owner = &cifg.resolve.types[match &func.kind {
                FunctionKind::Static(id) => *id,
                _ => panic!("special func should be static"),
            }];
            let mut namespace = namespace(
                cifg.resolve,
                &owner.owner,
                matches!(variant, AbiVariant::GuestExport),
                &cifg.gen.opts,
            );
            namespace.push(owner.name.as_ref().unwrap().to_upper_camel_case());
            namespace
        }

        let export = match variant {
            AbiVariant::GuestImport => self.gen.opts.host_side(),
            AbiVariant::GuestExport => !self.gen.opts.host_side(),
            AbiVariant::GuestImportAsync => todo!(),
            AbiVariant::GuestExportAsync => todo!(),
            AbiVariant::GuestExportAsyncStackful => todo!(),
        };
        let params = self.print_signature(func, variant, !export);
        let special = is_special_method(func);
        if !matches!(special, SpecialMethod::Allocate) {
            self.gen.c_src.src.push_str("{\n");
            let needs_dealloc = if self.gen.opts.api_style == APIStyle::Symmetric
                && matches!(variant, AbiVariant::GuestExport)
                && ((!self.gen.opts.symmetric
                    && symmetric::needs_dealloc(self.resolve, &func.params))
                    || (self.gen.opts.symmetric
                        && symmetric::has_non_canonical_list(self.resolve, &func.params)))
            {
                self.gen
                    .c_src
                    .src
                    .push_str("std::vector<void*> _deallocate;\n");
                self.gen.dependencies.needs_vector = true;
                true
            } else {
                false
            };
            let lift_lower = if self.gen.opts.symmetric {
                LiftLower::Symmetric
            } else if export {
                LiftLower::LiftArgsLowerResults
            } else {
                LiftLower::LowerArgsLiftResults
            };
            match is_special_method(func) {
                SpecialMethod::ResourceDrop => match lift_lower {
                    LiftLower::LiftArgsLowerResults => {
                        if self.gen.opts.host_side() {
                            let namespace = class_namespace(self, func, variant);
                            uwrite!(self.gen.c_src.src, "  auto ptr = ");
                            self.gen.c_src.qualify(&namespace);
                            uwriteln!(
                                self.gen.c_src.src,
                                "remove_resource({});
                                assert(ptr.has_value());",
                                params[0]
                            );
                            self.gen.dependencies.needs_assert = true;
                            self.gen.c_src.qualify(&namespace);
                            uwriteln!(self.gen.c_src.src, "Dtor(*ptr);")
                        } else {
                            let module_name = String::from("[export]")
                                + &self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                            let wasm_sig = self.declare_import(
                                &module_name,
                                &func.name,
                                &[WasmType::I32],
                                &[],
                            );
                            uwriteln!(
                                self.gen.c_src.src,
                                "{wasm_sig}({});",
                                func.params.get(0).unwrap().0
                            );
                        }
                    }
                    LiftLower::LowerArgsLiftResults => {
                        if self.gen.opts.host_side() {
                            let namespace = class_namespace(self, func, variant);
                            self.gen.c_src.qualify(&namespace);
                            uwriteln!(self.gen.c_src.src, "remove_resource(arg0);");
                        } else {
                            let module_name =
                                self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                            let name = self.declare_import(
                                &module_name,
                                &func.name,
                                &[WasmType::I32],
                                &[],
                            );
                            uwriteln!(
                                self.gen.c_src.src,
                                "   if (handle>=0) {{
                                {name}(handle);
                            }}"
                            );
                        }
                    }
                    LiftLower::Symmetric => {
                        let module_name =
                            self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                        if matches!(variant, AbiVariant::GuestExport) {
                            let mut namespace = class_namespace(self, func, variant);
                            self.gen.c_src.qualify(&namespace);
                            self.gen.c_src.src.push_str("Dtor((");
                            let classname = namespace.pop().unwrap_or_default();
                            self.gen.c_src.qualify(&namespace);
                            uwriteln!(
                                self.gen.c_src.src,
                                "{classname}*){});",
                                func.params.get(0).unwrap().0
                            );
                        } else {
                            let name = self.declare_import(
                                &module_name,
                                &func.name,
                                &[WasmType::Pointer],
                                &[],
                            );
                            uwriteln!(
                                self.gen.c_src.src,
                                "   if (handle!=nullptr) {{
                                {name}(handle);
                            }}"
                            );
                        }
                    }
                },
                SpecialMethod::Dtor => {
                    if self.gen.opts.host_side() {
                        let module_name =
                            self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                        let name = self.declare_import(
                            &module_name,
                            &func.name,
                            &[WasmType::Pointer],
                            &[],
                        );
                        uwriteln!(
                            self.gen.c_src.src,
                            "if (this->rep) {{ {name}(this->rep); }}"
                        );
                    } else {
                        let classname = class_namespace(self, func, variant).join("::");
                        if self.gen.opts.symmetric {
                            uwriteln!(
                                self.gen.c_src.src,
                                "{}::ResourceDrop(({})arg0);",
                                classname,
                                self.gen.opts.ptr_type()
                            );
                        } else {
                            uwriteln!(
                                self.gen.c_src.src,
                                "static_cast<{classname}*>(arg0)->handle=-1;"
                            );
                            uwriteln!(
                                self.gen.c_src.src,
                                "{0}::Dtor(static_cast<{0}*>(arg0));",
                                classname
                            );
                        }
                    }
                }
                SpecialMethod::ResourceNew => {
                    if self.gen.opts.symmetric {
                        uwriteln!(
                            self.gen.c_src.src,
                            "return static_cast<{}>({});",
                            self.gen.opts.ptr_type(),
                            func.params.get(0).unwrap().0
                        );
                    } else if !self.gen.opts.host_side() {
                        let module_name = String::from("[export]")
                            + &self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                        let wasm_sig = self.declare_import(
                            &module_name,
                            &func.name,
                            &[WasmType::Pointer],
                            &[WasmType::I32],
                        );
                        uwriteln!(
                            self.gen.c_src.src,
                            "return {wasm_sig}(({}){});",
                            self.gen.opts.ptr_type(),
                            func.params.get(0).unwrap().0
                        );
                    } else {
                        uwriteln!(self.gen.c_src.src, "return ");
                        let namespace = class_namespace(self, func, variant);
                        self.gen.c_src.qualify(&namespace);
                        uwriteln!(self.gen.c_src.src, "store_resource(std::move(arg0));");
                    }
                }
                SpecialMethod::ResourceRep => {
                    if self.gen.opts.symmetric {
                        let classname = class_namespace(self, func, variant).join("::");
                        uwriteln!(
                            self.gen.c_src.src,
                            "return static_cast<{}*>({});",
                            classname,
                            func.params.get(0).unwrap().0
                        );
                    } else if !self.gen.opts.host_side() {
                        let module_name = String::from("[export]")
                            + &self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                        let wasm_sig = self.declare_import(
                            &module_name,
                            &func.name,
                            &[WasmType::I32],
                            &[WasmType::Pointer],
                        );
                        let classname = class_namespace(self, func, variant).join("::");
                        uwriteln!(
                            self.gen.c_src.src,
                            "return ({}*){wasm_sig}({});",
                            classname,
                            func.params.get(0).unwrap().0
                        );
                    } else {
                        uwriteln!(self.gen.c_src.src, "return *");
                        let namespace = class_namespace(self, func, variant);
                        self.gen.c_src.qualify(&namespace);
                        uwriteln!(self.gen.c_src.src, "lookup_resource(arg0);",);
                    }
                }
                SpecialMethod::Allocate => unreachable!(),
                SpecialMethod::None => {
                    // normal methods
                    let namespace = if matches!(func.kind, FunctionKind::Freestanding) {
                        namespace(
                            self.resolve,
                            owner,
                            matches!(variant, AbiVariant::GuestExport),
                            &self.gen.opts,
                        )
                    } else {
                        let owner = &self.resolve.types[match &func.kind {
                            FunctionKind::Static(id) => *id,
                            FunctionKind::Constructor(id) => *id,
                            FunctionKind::Method(id) => *id,
                            FunctionKind::Freestanding => unreachable!(),
                            FunctionKind::AsyncFreestanding => todo!(),
                            FunctionKind::AsyncMethod(_id) => todo!(),
                            FunctionKind::AsyncStatic(_id) => todo!(),
                        }]
                        .clone();
                        let mut namespace = namespace(
                            self.resolve,
                            &owner.owner,
                            matches!(variant, AbiVariant::GuestExport),
                            &self.gen.opts,
                        );
                        namespace.push(owner.name.as_ref().unwrap().to_upper_camel_case());
                        namespace
                    };
                    let mut f = FunctionBindgen::new(self, params);
                    if !export {
                        f.namespace = namespace.clone();
                        f.wamr_signature = Some(wamr::wamr_signature(&f.gen.resolve, func));
                    }
                    f.variant = variant;
                    f.needs_dealloc = needs_dealloc;
                    f.cabi_post = if matches!(variant, AbiVariant::GuestExport)
                        && f.gen.gen.opts.host_side()
                        && abi::guest_export_needs_post_return(f.gen.resolve, func)
                    {
                        let module_name = f
                            .gen
                            .wasm_import_module
                            .as_ref()
                            .map(|e| e.clone())
                            .unwrap();
                        let cpp_sig = f.gen.high_level_signature(func, variant, &namespace);
                        Some(CabiPostInformation {
                            module: module_name,
                            name: func.name.clone(),
                            ret_type: cpp_sig.result,
                        })
                    } else {
                        None
                    };
                    abi::call(f.gen.resolve, variant, lift_lower, func, &mut f, false);
                    let code = String::from(f.src);
                    self.gen.c_src.src.push_str(&code);
                }
            }
            self.gen.c_src.src.push_str("}\n");
            // cabi_post
            if !self.gen.opts.host_side()
                && !matches!(lift_lower, LiftLower::Symmetric)
                && matches!(variant, AbiVariant::GuestExport)
                && abi::guest_export_needs_post_return(self.resolve, func)
            {
                let sig = self.patched_wasm_signature(variant, func);
                let module_name = self.wasm_import_module.as_ref().map(|e| e.clone());
                let export_name = match module_name {
                    Some(ref module_name) => {
                        format!("{module_name}#{}", func.name)
                    }
                    None => make_external_component(&func.name),
                };
                let import_name = match module_name {
                    Some(ref module_name) => make_external_symbol(
                        module_name,
                        &func.name,
                        if self.gen.opts.symmetric {
                            AbiVariant::GuestImport
                        } else {
                            AbiVariant::GuestExport
                        },
                    ),
                    None => make_external_component(&func.name),
                };
                uwriteln!(
                    self.gen.c_src.src,
                    "extern \"C\" __attribute__((__weak__, __export_name__(\"cabi_post_{export_name}\")))"
                );
                uwrite!(self.gen.c_src.src, "void cabi_post_{import_name}(");

                let mut params = Vec::new();
                for (i, result) in sig.results.iter().enumerate() {
                    let name = format!("arg{i}");
                    uwrite!(
                        self.gen.c_src.src,
                        "{} {name}",
                        self.gen.opts.wasm_type(*result)
                    );
                    params.push(name);
                }
                if sig.retptr && self.gen.opts.symmetric {
                    let name = "retptr";
                    uwrite!(
                        self.gen.c_src.src,
                        "{} {name}",
                        self.gen.opts.wasm_type(WasmType::Pointer)
                    );
                    params.push(name.into());
                }
                self.gen.c_src.src.push_str(") {\n");

                let mut f = FunctionBindgen::new(self, params.clone());
                f.params = params;
                abi::post_return(f.gen.resolve, func, &mut f);
                let FunctionBindgen { src, .. } = f;
                self.gen.c_src.src.push_str(&src);
                self.gen.c_src.src.push_str("}\n");
            }
        }
    }

    // pub fn type_path(&self, id: TypeId, owned: bool) -> String {
    //     self.type_path_with_name(
    //         id,
    //         if owned {
    //             self.result_name(id)
    //         } else {
    //             self.param_name(id)
    //         },
    //     )
    // }

    // fn type_path_with_name(&self, id: TypeId, name: String) -> String {
    //     if let TypeOwner::Interface(id) = self.resolve.types[id].owner {
    //         if let Some(path) = self.path_to_interface(id) {
    //             return format!("{path}::{name}");
    //         }
    //     }
    //     name
    // }

    // fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
    //     let iface = &self.resolve.interfaces[interface];
    //     let name = iface.name.as_ref().unwrap();
    //     let mut full_path = String::new();
    //     full_path.push_str(name);
    //     Some(full_path)
    // }

    // fn param_name(&self, ty: TypeId) -> String {
    //     self.resolve.types[ty]
    //         .name
    //         .as_ref()
    //         .unwrap()
    //         .to_upper_camel_case()
    // }

    // fn result_name(&self, ty: TypeId) -> String {
    //     self.resolve.types[ty]
    //         .name
    //         .as_ref()
    //         .unwrap()
    //         .to_upper_camel_case()
    // }

    // in C this is print_optional_ty
    fn optional_type_name(
        &mut self,
        ty: Option<&Type>,
        from_namespace: &[String],
        flavor: Flavor,
    ) -> String {
        match ty {
            Some(ty) => self.type_name(ty, from_namespace, flavor),
            None => "void".into(),
        }
    }

    fn scoped_record_name(
        &self,
        id: TypeId,
        from_namespace: &[String],
        guest_export: bool,
        flavor: Flavor,
    ) -> String {
        let name = self.scoped_type_name(id, from_namespace, guest_export);

        if let Flavor::Argument(AbiVariant::GuestImport) = flavor {
            match self.gen.opts.ownership {
                Ownership::Owning => {
                    format!("{}", name)
                }
                Ownership::CoarseBorrowing => {
                    if self.gen.types.get(id).has_own_handle {
                        format!("{}", name)
                    } else {
                        format!("{}Param", name)
                    }
                }
                Ownership::FineBorrowing => {
                    format!("{}Param", name)
                }
            }
        } else {
            name
        }
    }

    fn scoped_type_name(
        &self,
        id: TypeId,
        from_namespace: &[String],
        guest_export: bool,
    ) -> String {
        let ty = &self.resolve.types[id];
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);
        let mut relative = SourceWithState::default();
        relative.namespace = Vec::from(from_namespace);
        relative.qualify(&namespc);
        format!(
            "{}{}",
            relative.src.to_string(),
            ty.name.as_ref().unwrap().to_pascal_case()
        )
    }

    fn type_name(&mut self, ty: &Type, from_namespace: &[String], flavor: Flavor) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::Char => "uint32_t".into(),
            Type::U8 => "uint8_t".into(),
            Type::S8 => "int8_t".into(),
            Type::U16 => "uint16_t".into(),
            Type::S16 => "int16_t".into(),
            Type::U32 => "uint32_t".into(),
            Type::S32 => "int32_t".into(),
            Type::U64 => "uint64_t".into(),
            Type::S64 => "int64_t".into(),
            Type::F32 => "float".into(),
            Type::F64 => "double".into(),
            Type::String => match flavor {
                Flavor::BorrowedArgument => {
                    self.gen.dependencies.needs_string_view = true;
                    "std::string_view".into()
                }
                Flavor::Argument(var)
                    if matches!(var, AbiVariant::GuestImport)
                        || self.gen.opts.api_style == APIStyle::Symmetric =>
                {
                    self.gen.dependencies.needs_string_view = true;
                    "std::string_view".into()
                }
                Flavor::Argument(AbiVariant::GuestExport) if !self.gen.opts.host_side() => {
                    self.gen.dependencies.needs_wit = true;
                    "wit::string".into()
                }
                Flavor::Result(AbiVariant::GuestExport) if self.gen.opts.host_side() => {
                    self.gen.dependencies.needs_string_view = true;
                    "std::string_view".into()
                }
                _ => {
                    self.gen.dependencies.needs_wit = true;
                    "wit::string".into()
                }
            },
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Record(_) => {
                    let guest_export = self.is_exported_type(&self.resolve.types[*id]);
                    self.scoped_record_name(*id, from_namespace, guest_export, flavor)
                }
                TypeDefKind::Resource => {
                    let guest_export = self.is_exported_type(&self.resolve.types[*id]);
                    self.scoped_type_name(*id, from_namespace, guest_export)
                }
                TypeDefKind::Handle(Handle::Own(id)) => {
                    let mut typename = self.type_name(&Type::Id(*id), from_namespace, flavor);
                    match (self.gen.opts.host_side(), flavor) {
                        (false, Flavor::Argument(AbiVariant::GuestImport))
                        | (true, Flavor::Argument(AbiVariant::GuestExport)) => {
                            typename.push_str("&&")
                        }
                        (false, Flavor::Argument(AbiVariant::GuestExport))
                        | (false, Flavor::Result(AbiVariant::GuestExport))
                        | (true, Flavor::Argument(AbiVariant::GuestImport))
                        | (true, Flavor::Result(AbiVariant::GuestImport)) => {
                            typename.push_str(&format!("::{OWNED_CLASS_NAME}"))
                        }
                        (false, Flavor::Result(AbiVariant::GuestImport))
                        | (true, Flavor::Result(AbiVariant::GuestExport)) => (),
                        (_, Flavor::InStruct) => (),
                        (false, Flavor::BorrowedArgument) => (),
                        (_, _) => todo!(),
                    }
                    let ty = &self.resolve.types[*id];
                    if matches!(flavor, Flavor::InStruct) && self.is_exported_type(ty) {
                        typename.push_str(&format!("::{OWNED_CLASS_NAME}"))
                    }
                    typename
                }
                TypeDefKind::Handle(Handle::Borrow(id)) => {
                    "std::reference_wrapper<const ".to_string()
                        + &self.type_name(&Type::Id(*id), from_namespace, flavor)
                        + ">"
                }
                TypeDefKind::Flags(_f) => {
                    let ty = &self.resolve.types[*id];
                    let guest_export = self.is_exported_type(ty);
                    self.scoped_type_name(*id, from_namespace, guest_export)
                }
                TypeDefKind::Tuple(t) => {
                    let types = t.types.iter().fold(String::new(), |mut a, b| {
                        if !a.is_empty() {
                            a += ", ";
                        }
                        a + &self.type_name(b, from_namespace, flavor)
                    });
                    self.gen.dependencies.needs_tuple = true;
                    String::from("std::tuple<") + &types + ">"
                }
                TypeDefKind::Variant(_v) => {
                    let ty = &self.resolve.types[*id];
                    let guest_export = self.is_exported_type(ty);
                    self.scoped_type_name(*id, from_namespace, guest_export)
                }
                TypeDefKind::Enum(_e) => {
                    let ty = &self.resolve.types[*id];
                    let guest_export = self.is_exported_type(ty);
                    self.scoped_type_name(*id, from_namespace, guest_export)
                }
                TypeDefKind::Option(o) => {
                    self.gen.dependencies.needs_optional = true;
                    "std::optional<".to_string() + &self.type_name(o, from_namespace, flavor) + ">"
                }
                TypeDefKind::Result(r) => {
                    let err_type = r.err.as_ref().map_or(String::from("wit::Void"), |ty| {
                        self.type_name(ty, from_namespace, flavor)
                    });
                    self.gen.dependencies.needs_expected = true;
                    "std::expected<".to_string()
                        + &self.optional_type_name(r.ok.as_ref(), from_namespace, flavor)
                        + ", "
                        + &err_type
                        + ">"
                }
                TypeDefKind::List(ty) => {
                    let inner = self.type_name(ty, from_namespace, flavor);
                    match flavor {
                        Flavor::BorrowedArgument => {
                            self.gen.dependencies.needs_span = true;
                            format!("std::span<{inner} const>")
                        }
                        Flavor::Argument(var)
                            if matches!(var, AbiVariant::GuestImport)
                                || self.gen.opts.api_style == APIStyle::Symmetric =>
                        {
                            self.gen.dependencies.needs_span = true;
                            // If the list has an owning handle, it must support moving, so can't be const
                            let constness = if self.gen.types.get(*id).has_own_handle {
                                ""
                            } else {
                                " const"
                            };
                            format!("std::span<{inner}{constness}>")
                        }
                        Flavor::Argument(AbiVariant::GuestExport) if !self.gen.opts.host => {
                            self.gen.dependencies.needs_wit = true;
                            format!("wit::vector<{inner}>")
                        }
                        Flavor::Result(AbiVariant::GuestExport) if self.gen.opts.host => {
                            self.gen.dependencies.needs_wit = true;
                            format!("wit::span<{inner} const>")
                        }
                        _ => {
                            self.gen.dependencies.needs_wit = true;
                            format!("wit::vector<{inner}>")
                        }
                    }
                }
                TypeDefKind::Future(ty) => {
                    self.gen.dependencies.needs_future = true;
                    "std::future<".to_string()
                        + &self.optional_type_name(ty.as_ref(), from_namespace, flavor)
                        + ">"
                }
                TypeDefKind::Stream(ty) => {
                    self.gen.dependencies.needs_stream = true;
                    "wit::stream<".to_string()
                        + &self.optional_type_name(ty.as_ref(), from_namespace, flavor)
                        + ">"
                }
                TypeDefKind::Type(ty) => self.type_name(ty, from_namespace, flavor),
                TypeDefKind::FixedSizeList(ty, size) => {
                    self.r#gen.dependencies.needs_array = true;
                    format!(
                        "std::array<{}, {size}>",
                        self.type_name(ty, from_namespace, flavor)
                    )
                }
                TypeDefKind::Unknown => todo!(),
            },
            Type::ErrorContext => todo!(),
        }
    }

    fn declare_import2(
        &self,
        module_name: &str,
        name: &str,
        args: &str,
        result: &str,
        variant: AbiVariant,
    ) -> (String, String) {
        let extern_name = make_external_symbol(module_name, name, variant);
        let import = if self.gen.opts.symmetric {
            format!("extern \"C\" {result} {extern_name}({args});\n")
        } else {
            format!("extern \"C\" __attribute__((import_module(\"{module_name}\")))\n __attribute__((import_name(\"{name}\")))\n {result} {extern_name}({args});\n")
        };
        (extern_name, import)
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
            args.push_str(self.gen.opts.wasm_type(*param));
            if n + 1 != params.len() {
                args.push_str(", ");
            }
        }
        let result = if results.is_empty() {
            "void"
        } else {
            self.gen.opts.wasm_type(results[0])
        };
        let variant = if self.gen.opts.short_cut {
            AbiVariant::GuestExport
        } else {
            AbiVariant::GuestImport
        };
        let (name, code) = self.declare_import2(module_name, name, &args, result, variant);
        self.gen.extern_c_decls.push_str(&code);
        name
    }

    fn docs(src: &mut Source, docs: &Docs) {
        if let Some(docs) = docs.contents.as_ref() {
            for line in docs.trim().lines() {
                src.push_str("/// ");
                src.push_str(line);
                src.push_str("\n");
            }
        }
    }

    fn type_record_param(
        &mut self,
        id: TypeId,
        name: &str,
        record: &wit_bindgen_core::wit_parser::Record,
        namespc: &[String],
    ) {
        let (flavor, needs_param_type) = {
            match self.gen.opts.ownership {
                Ownership::Owning => (Flavor::InStruct, false),
                Ownership::CoarseBorrowing => {
                    if self.gen.types.get(id).has_own_handle {
                        (Flavor::InStruct, false)
                    } else {
                        (Flavor::BorrowedArgument, true)
                    }
                }
                Ownership::FineBorrowing => (Flavor::BorrowedArgument, true),
            }
        };

        if needs_param_type {
            let pascal = format!("{name}-param").to_pascal_case();

            uwriteln!(self.gen.h_src.src, "struct {pascal} {{");
            for field in record.fields.iter() {
                let typename = self.type_name(&field.ty, namespc, flavor);
                let fname = field.name.to_snake_case();
                uwriteln!(self.gen.h_src.src, "{typename} {fname};");
            }
            uwriteln!(self.gen.h_src.src, "}};");
        }
    }

    fn is_exported_type(&self, ty: &TypeDef) -> bool {
        if let TypeOwner::Interface(intf) = ty.owner {
            !self.gen.imported_interfaces.contains(&intf)
        } else {
            true
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for CppInterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(
        &mut self,
        id: TypeId,
        name: &str,
        record: &wit_bindgen_core::wit_parser::Record,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let guest_export = self.is_exported_type(ty);
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);

        if self.gen.is_first_definition(&namespc, &name) {
            self.gen.h_src.change_namespace(&namespc);
            Self::docs(&mut self.gen.h_src.src, docs);
            let pascal = name.to_pascal_case();

            uwriteln!(self.gen.h_src.src, "struct {pascal} {{");
            for field in record.fields.iter() {
                Self::docs(&mut self.gen.h_src.src, &field.docs);
                let typename = self.type_name(&field.ty, &namespc, Flavor::InStruct);
                let fname = field.name.to_snake_case();
                uwriteln!(self.gen.h_src.src, "{typename} {fname};");
            }
            uwriteln!(self.gen.h_src.src, "}};");
            self.type_record_param(id, name, record, namespc.as_slice());
        }
    }

    fn type_resource(
        &mut self,
        id: TypeId,
        name: &str,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let type_ = &self.resolve.types[id];
        if let TypeOwner::Interface(intf) = type_.owner {
            let guest_import = self.gen.imported_interfaces.contains(&intf);
            let definition = !(guest_import ^ self.gen.opts.host_side());
            let store = self.gen.start_new_file(Some(definition));
            let mut world_name = self.gen.world.to_snake_case();
            world_name.push_str("::");
            let namespc = namespace(self.resolve, &type_.owner, !guest_import, &self.gen.opts);
            let pascal = name.to_upper_camel_case();
            let mut user_filename = namespc.clone();
            user_filename.push(pascal.clone());
            if definition {
                uwriteln!(
                    self.gen.h_src.src,
                    r#"/* User class definition file, autogenerated once, then user modified
                    * Updated versions of this file are generated into {pascal}.template.
                    */"#
                );
            }
            self.gen.h_src.change_namespace(&namespc);

            if !definition {
                self.gen.dependencies.needs_imported_resources = true;
            } else {
                self.gen.dependencies.needs_exported_resources = true;
            }
            self.gen.dependencies.needs_wit = true;

            let base_type = match (definition, self.gen.opts.host_side()) {
                (true, false) => format!("wit::{RESOURCE_EXPORT_BASE_CLASS_NAME}<{pascal}>"),
                (false, false) => {
                    String::from_str("wit::").unwrap() + RESOURCE_IMPORT_BASE_CLASS_NAME
                }
                (false, true) => {
                    String::from_str("wit::").unwrap() + RESOURCE_EXPORT_BASE_CLASS_NAME
                }
                (true, true) => format!("wit::{RESOURCE_IMPORT_BASE_CLASS_NAME}<{pascal}>"),
            };
            let derive = format!(" : public {base_type}");
            uwriteln!(self.gen.h_src.src, "class {pascal}{derive} {{\n");
            uwriteln!(self.gen.h_src.src, "public:\n");
            let variant = if guest_import {
                AbiVariant::GuestImport
            } else {
                AbiVariant::GuestExport
            };
            {
                // destructor
                let name = match variant {
                    AbiVariant::GuestImport => "[resource-drop]",
                    AbiVariant::GuestExport => "[dtor]",
                    AbiVariant::GuestImportAsync => todo!(),
                    AbiVariant::GuestExportAsync => todo!(),
                    AbiVariant::GuestExportAsyncStackful => todo!(),
                }
                .to_string()
                    + name;
                let func = Function {
                    name,
                    kind: FunctionKind::Static(id),
                    params: vec![("self".into(), Type::Id(id))],
                    result: None,
                    docs: Docs::default(),
                    stability: Stability::Unknown,
                };
                self.generate_function(&func, &TypeOwner::Interface(intf), variant);
            }
            let funcs = self.resolve.interfaces[intf].functions.values();
            for func in funcs {
                if match &func.kind {
                    FunctionKind::Freestanding => false,
                    FunctionKind::Method(mid) => *mid == id,
                    FunctionKind::Static(mid) => *mid == id,
                    FunctionKind::Constructor(mid) => *mid == id,
                    FunctionKind::AsyncFreestanding => todo!(),
                    FunctionKind::AsyncMethod(_id) => todo!(),
                    FunctionKind::AsyncStatic(_id) => todo!(),
                } {
                    self.generate_function(func, &TypeOwner::Interface(intf), variant);
                    if matches!(func.kind, FunctionKind::Constructor(_))
                        && matches!(variant, AbiVariant::GuestExport) != self.gen.opts.host_side()
                    {
                        // functional safety requires the option to use a different allocator, so move new into the implementation
                        let func2 = Function {
                            name: "$alloc".to_string(),
                            kind: FunctionKind::Static(id),
                            // same params as constructor
                            params: func.params.clone(),
                            result: Some(Type::Id(id)),
                            docs: Docs::default(),
                            stability: Stability::Unknown,
                        };
                        self.generate_function(&func2, &TypeOwner::Interface(intf), variant);
                    }
                }
            }

            if !definition {
                // consuming constructor from handle (bindings)
                uwriteln!(self.gen.h_src.src, "{pascal}({base_type} &&);",);
                uwriteln!(self.gen.h_src.src, "{pascal}({pascal}&&) = default;");
                uwriteln!(
                    self.gen.h_src.src,
                    "{pascal}& operator=({pascal}&&) = default;"
                );
                uwriteln!(self.gen.h_src.src, "{pascal}(const {pascal}&) = delete;");
                uwriteln!(
                    self.gen.h_src.src,
                    "{pascal}& operator=(const {pascal}&) = delete;"
                );
                self.gen.c_src.qualify(&namespc);
                uwriteln!(
                    self.gen.c_src.src,
                    "{pascal}::{pascal}({base_type}&&b) : {base_type}(std::move(b)) {{}}"
                );
            }
            if matches!(variant, AbiVariant::GuestExport) {
                let id_type = if self.gen.opts.symmetric {
                    Type::Id(id)
                } else {
                    Type::S32
                };
                let func = Function {
                    name: "[resource-new]".to_string() + name,
                    kind: FunctionKind::Static(id),
                    params: vec![("self".into(), Type::Id(id))],
                    result: Some(id_type),
                    docs: Docs::default(),
                    stability: Stability::Unknown,
                };
                self.generate_function(&func, &TypeOwner::Interface(intf), variant);

                let func1 = Function {
                    name: "[resource-rep]".to_string() + name,
                    kind: FunctionKind::Static(id),
                    params: vec![("id".into(), id_type)],
                    result: Some(Type::Id(id)),
                    docs: Docs::default(),
                    stability: Stability::Unknown,
                };
                self.generate_function(&func1, &TypeOwner::Interface(intf), variant);

                let func2 = Function {
                    name: "[resource-drop]".to_string() + name,
                    kind: FunctionKind::Static(id),
                    params: vec![("id".into(), id_type)],
                    result: None,
                    docs: Docs::default(),
                    stability: Stability::Unknown,
                };
                self.generate_function(&func2, &TypeOwner::Interface(intf), variant);
            }
            uwriteln!(self.gen.h_src.src, "}};\n");
            self.gen.finish_file(&user_filename, store);
        }
    }

    fn type_flags(
        &mut self,
        id: TypeId,
        name: &str,
        flags: &wit_bindgen_core::wit_parser::Flags,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let guest_export = self.is_exported_type(ty);
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);
        if self.gen.is_first_definition(&namespc, name) {
            self.gen.h_src.change_namespace(&namespc);
            Self::docs(&mut self.gen.h_src.src, docs);
            let pascal = name.to_pascal_case();
            let int_repr = wit_bindgen_c::int_repr(wit_bindgen_c::flags_repr(flags));
            uwriteln!(self.gen.h_src.src, "enum class {pascal} : {int_repr} {{");
            uwriteln!(self.gen.h_src.src, "k_None = 0,");
            for (n, field) in flags.flags.iter().enumerate() {
                Self::docs(&mut self.gen.h_src.src, &field.docs);
                let fname = field.name.to_pascal_case();
                uwriteln!(self.gen.h_src.src, "k{fname} = (1ULL<<{n}),");
            }
            uwriteln!(self.gen.h_src.src, "}};");
            uwriteln!(
                self.gen.h_src.src,
                r#"static inline {pascal} operator|({pascal} a, {pascal} b) {{ return {pascal}({int_repr}(a)|{int_repr}(b)); }}
        static inline {pascal} operator&({pascal} a, {pascal} b) {{ return {pascal}({int_repr}(a)&{int_repr}(b)); }}"#
            );
        }
    }

    fn type_tuple(
        &mut self,
        _id: TypeId,
        _name: &str,
        _flags: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        // I assume I don't need to do anything ...
    }

    fn type_variant(
        &mut self,
        id: TypeId,
        name: &str,
        variant: &wit_bindgen_core::wit_parser::Variant,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let guest_export = self.is_exported_type(ty);
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);
        if self.gen.is_first_definition(&namespc, name) {
            self.gen.h_src.change_namespace(&namespc);
            Self::docs(&mut self.gen.h_src.src, docs);
            let pascal = name.to_pascal_case();
            uwriteln!(self.gen.h_src.src, "struct {pascal} {{");
            let mut inner_namespace = namespc.clone();
            inner_namespace.push(pascal.clone());
            let mut all_types = String::new();
            for case in variant.cases.iter() {
                Self::docs(&mut self.gen.h_src.src, &case.docs);
                let case_pascal = case.name.to_pascal_case();
                if !all_types.is_empty() {
                    all_types += ", ";
                }
                all_types += &case_pascal;
                uwrite!(self.gen.h_src.src, "struct {case_pascal} {{");
                if let Some(ty) = case.ty.as_ref() {
                    let typestr = self.type_name(ty, &inner_namespace, Flavor::InStruct);
                    uwrite!(self.gen.h_src.src, " {typestr} value; ")
                }
                uwriteln!(self.gen.h_src.src, "}};");
            }
            uwriteln!(self.gen.h_src.src, "  std::variant<{all_types}> variants;");
            uwriteln!(self.gen.h_src.src, "}};");
            self.gen.dependencies.needs_variant = true;
        }
    }

    fn type_option(
        &mut self,
        _id: TypeId,
        _name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        // nothing to do here
    }

    fn type_result(
        &mut self,
        _id: TypeId,
        _name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        // nothing to do here
    }

    fn type_enum(
        &mut self,
        id: TypeId,
        name: &str,
        enum_: &wit_bindgen_core::wit_parser::Enum,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let guest_export = self.is_exported_type(ty);
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);
        if self.gen.is_first_definition(&namespc, name) {
            self.gen.h_src.change_namespace(&namespc);
            let pascal = name.to_pascal_case();
            Self::docs(&mut self.gen.h_src.src, docs);
            let int_t = wit_bindgen_c::int_repr(enum_.tag());
            uwriteln!(self.gen.h_src.src, "enum class {pascal} : {int_t} {{");
            for (i, case) in enum_.cases.iter().enumerate() {
                Self::docs(&mut self.gen.h_src.src, &case.docs);
                uwriteln!(
                    self.gen.h_src.src,
                    " k{} = {i},",
                    case.name.to_pascal_case(),
                );
            }
            uwriteln!(self.gen.h_src.src, "}};\n");
        }
    }

    fn type_alias(
        &mut self,
        id: TypeId,
        name: &str,
        alias_type: &wit_bindgen_core::wit_parser::Type,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let guest_export = self.is_exported_type(ty);
        let namespc = namespace(self.resolve, &ty.owner, guest_export, &self.gen.opts);
        self.gen.h_src.change_namespace(&namespc);
        let pascal = name.to_pascal_case();
        Self::docs(&mut self.gen.h_src.src, docs);
        let typename = self.type_name(alias_type, &namespc, Flavor::InStruct);
        uwriteln!(self.gen.h_src.src, "using {pascal} = {typename};");
    }

    fn type_list(
        &mut self,
        _id: TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        // nothing to do here
    }

    fn type_builtin(
        &mut self,
        _id: TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_future(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        todo!()
    }

    fn type_stream(&mut self, _id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        todo!()
    }
}

struct CabiPostInformation {
    module: String,
    name: String,
    ret_type: String,
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut CppInterfaceGenerator<'a>,
    params: Vec<String>,
    tmp: usize,
    namespace: Vec<String>,
    src: Source,
    block_storage: Vec<wit_bindgen_core::Source>,
    /// intermediate calculations for contained objects
    blocks: Vec<(String, Vec<String>)>,
    payloads: Vec<String>,
    // caching for wasm
    wamr_signature: Option<wamr::WamrSig>,
    variant: AbiVariant,
    cabi_post: Option<CabiPostInformation>,
    needs_dealloc: bool,
    leak_on_insertion: Option<String>,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(gen: &'b mut CppInterfaceGenerator<'a>, params: Vec<String>) -> Self {
        Self {
            gen,
            params,
            tmp: 0,
            namespace: Default::default(),
            src: Default::default(),
            block_storage: Default::default(),
            blocks: Default::default(),
            payloads: Default::default(),
            wamr_signature: None,
            variant: AbiVariant::GuestImport,
            cabi_post: None,
            needs_dealloc: false,
            leak_on_insertion: None,
        }
    }

    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn tempname(&self, base: &str, idx: usize) -> String {
        format!("{base}{idx}")
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    // fn typename_lift(&self, id: TypeId) -> String {
    //     self.gen.type_path(id, true)
    // }

    fn let_results(&mut self, amt: usize, results: &mut Vec<String>) {
        if amt > 0 {
            let tmp = self.tmp();
            let res = format!("result{}", tmp);
            self.push_str("auto ");
            self.push_str(&res);
            self.push_str(" = ");
            if amt == 1 {
                results.push(res);
            } else {
                for i in 0..amt {
                    results.push(format!("std::get<{i}>({res})"));
                }
            }
        }
    }

    fn load(
        &mut self,
        ty: &str,
        offset: ArchitectureSize,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        if self.gen.gen.opts.host {
            results.push(format!("*(({}*) wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), ({} + {})))", ty, operands[0], offset.format(POINTER_SIZE_EXPRESSION)));
        } else {
            results.push(format!(
                "*static_cast<{ty}*>({} + {})",
                operands[0],
                offset.format(POINTER_SIZE_EXPRESSION)
            ));
        }
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
        results.push(format!("static_cast<int32_t>({})", result));
    }

    fn store(&mut self, ty: &str, offset: ArchitectureSize, operands: &[String]) {
        if self.gen.gen.opts.host {
            uwriteln!(
                self.src,
                "*(({ty}*)wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), ({} + {}))) = {};",
                operands[1],
                offset.format(POINTER_SIZE_EXPRESSION),
                operands[0]
            );
        } else {
            uwriteln!(
                self.src,
                "*static_cast<{ty}*>({} + {}) = {};",
                operands[1],
                offset.format(POINTER_SIZE_EXPRESSION),
                operands[0]
            );
        }
    }

    fn lower_lift(&mut self, payload: Option<&Type>) -> String {
        let typestr = self
            .gen
            .optional_type_name(payload, &Vec::new(), Flavor::InStruct);
        let tmpnr = self.r#gen.r#gen.tmp();
        uwriteln!(self.r#gen.r#gen.c_src_head, "struct Lift{tmpnr} {{");
        let symmetric = self.r#gen.r#gen.opts.symmetric;
        let mut bindgen = FunctionBindgen::new(self.gen, Vec::new());
        let lift = if let Some(ty) = payload {
            let res = wit_bindgen_core::abi::lift_from_memory(
                bindgen.r#gen.resolve,
                &mut bindgen,
                "ptr".into(),
                ty,
                symmetric,
            );
            format!("{} return {res};", String::from(bindgen.src))
        } else {
            String::new()
        };
        let mut bindgen = FunctionBindgen::new(self.gen, Vec::new());
        // GuestImport doesn't leak the objects (string)
        bindgen.variant = AbiVariant::GuestExport;
        let lower = if let Some(ty) = payload {
            wit_bindgen_core::abi::lower_to_memory(
                bindgen.r#gen.resolve,
                &mut bindgen,
                "ptr".into(),
                "value".into(),
                ty,
                symmetric,
            );
            String::from(bindgen.src)
        } else {
            String::new()
        };
        let lowered_size = if let Some(ty) = payload {
            self.r#gen
                .sizes
                .size(ty)
                .format_term(POINTER_SIZE_EXPRESSION, true)
        } else {
            String::from("1")
        };

        uwriteln!(
            self.r#gen.r#gen.c_src_head,
            "static {typestr} lift(uint8_t const* ptr) {{ {lift} }}
                    static void lower({typestr} && value, uint8_t *ptr) {{ {lower} }}
                    static constexpr size_t SIZE = {lowered_size};"
        );
        uwriteln!(self.r#gen.r#gen.c_src_head, "}};");
        format!("Lift{tmpnr}")
    }

    fn lower_lift_stream(&mut self, payload: Option<&Type>) {
        let typestr = self
            .gen
            .optional_type_name(payload, &Vec::new(), Flavor::InStruct);
        let symmetric = self.r#gen.r#gen.opts.symmetric;
        let mut bindgen = FunctionBindgen::new(self.gen, Vec::new());
        let lift = if let Some(ty) = payload {
            let res = wit_bindgen_core::abi::lift_from_memory(
                bindgen.r#gen.resolve,
                &mut bindgen,
                "ptr".into(),
                ty,
                symmetric,
            );
            format!("{} return {res};", String::from(bindgen.src))
        } else {
            String::new()
        };
        let mut bindgen = FunctionBindgen::new(self.gen, Vec::new());
        // GuestImport doesn't leak the objects (string)
        bindgen.variant = AbiVariant::GuestExport;
        let lower = if let Some(ty) = payload {
            wit_bindgen_core::abi::lower_to_memory(
                bindgen.r#gen.resolve,
                &mut bindgen,
                "ptr".into(),
                "value".into(),
                ty,
                symmetric,
            );
            String::from(bindgen.src)
        } else {
            String::new()
        };
        let lowered_size = if let Some(ty) = payload {
            self.r#gen
                .sizes
                .size(ty)
                .format_term(POINTER_SIZE_EXPRESSION, true)
        } else {
            String::from("1")
        };

        uwriteln!(self.r#gen.r#gen.c_src_head, "template <>
            const uint32_t wit::StreamProperties<{typestr}>::lowered_size = {lowered_size};
            template <>
            {typestr} wit::StreamProperties<{typestr}>::lift(uint8_t const*ptr) {{ {lift} }}
            template <>
            void wit::StreamProperties<{typestr}>::lower({typestr} && value, uint8_t *ptr) {{ {lower} }}"
        );
    }
}

fn move_if_necessary(arg: &str) -> String {
    // if it is a name of a variable move it
    if !arg.is_empty() && arg.chars().all(char::is_alphanumeric) {
        format!("std::move({arg})")
    } else {
        arg.into()
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
            results.push(format!("static_cast<{cvt}>({})", operands.pop().unwrap()));
        };

        match inst {
            abi::Instruction::GetArg { nth } => {
                if *nth == 0 && self.params[0].as_str() == "self" {
                    if self.gen.in_guest_import ^ self.gen.gen.opts.host {
                        results.push("(*this)".to_string());
                    } else {
                        results.push("(*lookup_resource(self))".to_string());
                    }
                } else {
                    results.push(self.params[*nth].clone());
                }
            }
            abi::Instruction::I32Const { val } => {
                results.push(format!("static_cast<int32_t>({val})"))
            }
            abi::Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    // let op = op;
                    results.push(self.gen.gen.perform_cast(op, cast));
                }
            }
            abi::Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("static_cast<int32_t>(0)".to_string()),
                        WasmType::I64 => results.push("static_cast<int64_t>(0)".to_string()),
                        WasmType::F32 => results.push("0.0f".to_string()),
                        WasmType::F64 => results.push("0.0".to_string()),
                        WasmType::Length => results.push("static_cast<size_t>(0)".to_string()),
                        WasmType::Pointer => results.push("nullptr".to_string()),
                        WasmType::PointerOrI64 => {
                            results.push("static_cast<int64_t>(0)".to_string())
                        }
                    }
                }
            }
            abi::Instruction::I32Load { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "int32_t l{tmp} = *static_cast<int32_t const*>({} + {offset});",
                    operands[0],
                    offset = offset.format(POINTER_SIZE_EXPRESSION)
                );
                results.push(format!("l{tmp}"));
            }
            abi::Instruction::I32Load8U { offset } => {
                self.load_ext("uint8_t", *offset, operands, results)
            }
            abi::Instruction::I32Load8S { offset } => {
                self.load_ext("int8_t", *offset, operands, results)
            }
            abi::Instruction::I32Load16U { offset } => {
                self.load_ext("uint16_t", *offset, operands, results)
            }
            abi::Instruction::I32Load16S { offset } => {
                self.load_ext("int16_t", *offset, operands, results)
            }
            abi::Instruction::I64Load { offset } => {
                self.load("int64_t", *offset, operands, results)
            }
            abi::Instruction::F32Load { offset } => self.load("float", *offset, operands, results),
            abi::Instruction::F64Load { offset } => self.load("double", *offset, operands, results),
            abi::Instruction::I32Store { offset } => self.store("int32_t", *offset, operands),
            abi::Instruction::I32Store8 { offset } => self.store("int8_t", *offset, operands),
            abi::Instruction::I32Store16 { offset } => self.store("int16_t", *offset, operands),
            abi::Instruction::I64Store { offset } => self.store("int64_t", *offset, operands),
            abi::Instruction::F32Store { offset } => self.store("float", *offset, operands),
            abi::Instruction::F64Store { offset } => self.store("double", *offset, operands),
            abi::Instruction::I32FromChar
            | abi::Instruction::I32FromBool
            | abi::Instruction::I32FromU8
            | abi::Instruction::I32FromS8
            | abi::Instruction::I32FromU16
            | abi::Instruction::I32FromS16
            | abi::Instruction::I32FromU32
            | abi::Instruction::I32FromS32 => top_as("int32_t"),
            abi::Instruction::I64FromU64 | abi::Instruction::I64FromS64 => top_as("int64_t"),
            abi::Instruction::F32FromCoreF32 => top_as("float"),
            abi::Instruction::F64FromCoreF64 => top_as("double"),
            abi::Instruction::S8FromI32 => top_as("int8_t"),
            abi::Instruction::U8FromI32 => top_as("uint8_t"),
            abi::Instruction::S16FromI32 => top_as("int16_t"),
            abi::Instruction::U16FromI32 => top_as("uint16_t"),
            abi::Instruction::S32FromI32 => top_as("int32_t"),
            abi::Instruction::U32FromI32 => top_as("uint32_t"),
            abi::Instruction::S64FromI64 => top_as("int64_t"),
            abi::Instruction::U64FromI64 => top_as("uint64_t"),
            abi::Instruction::CharFromI32 => top_as("uint32_t"),
            abi::Instruction::CoreF32FromF32 => top_as("float"),
            abi::Instruction::CoreF64FromF64 => top_as("double"),
            abi::Instruction::BoolFromI32 => top_as("bool"),
            abi::Instruction::ListCanonLower { realloc, .. } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                // let result = format!("result{}", tmp);
                self.push_str(&format!("auto&& {} = {};\n", val, operands[0]));
                if self.gen.gen.opts.host_side() {
                    self.push_str(&format!("auto {} = {}.data();\n", ptr, val));
                    self.push_str(&format!("auto {} = {}.size();\n", len, val));
                } else {
                    self.push_str(&format!(
                        "auto {ptr} = static_cast<{}>({val}.data());\n",
                        self.gen.gen.opts.ptr_type(),
                    ));
                    self.push_str(&format!(
                        "auto {} = static_cast<size_t>({}.size());\n",
                        len, val
                    ));
                }
                if realloc.is_none() {
                    results.push(ptr);
                } else {
                    if !self.gen.gen.opts.host_side()
                        && !(self.gen.gen.opts.symmetric
                            && matches!(self.variant, AbiVariant::GuestImport))
                    {
                        uwriteln!(self.src, "{}.leak();\n", operands[0]);
                    }
                    results.push(ptr);
                }
                results.push(len);
            }
            abi::Instruction::StringLower { realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                // let result = format!("result{}", tmp);
                self.push_str(&format!("auto&& {} = {};\n", val, operands[0]));
                if self.gen.gen.opts.host_side() {
                    self.push_str(&format!("auto {} = {}.data();\n", ptr, val));
                    self.push_str(&format!("auto {} = {}.size();\n", len, val));
                } else {
                    self.push_str(&format!(
                        "auto {} = static_cast<{}>({}.data());\n",
                        ptr,
                        self.gen.gen.opts.ptr_type(),
                        val
                    ));
                    self.push_str(&format!(
                        "auto {} = static_cast<size_t>({}.size());\n",
                        len, val
                    ));
                }
                if realloc.is_none() {
                    results.push(ptr);
                } else {
                    if !self.gen.gen.opts.host_side()
                        && !(self.gen.gen.opts.symmetric
                            && matches!(self.variant, AbiVariant::GuestImport))
                    {
                        uwriteln!(self.src, "{}.leak();\n", operands[0]);
                    }
                    results.push(ptr);
                }
                results.push(len);
            }
            abi::Instruction::ListLower { element, realloc } => {
                let tmp = self.tmp();
                let body = self.blocks.pop().unwrap();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                self.push_str(&format!("auto&& {} = {};\n", val, operands[0]));
                //self.push_str(&format!("auto const&{} = {};\n", val, operands[0]));
                if self.gen.gen.opts.host_side() {
                    self.push_str(&format!("auto {} = {}.data();\n", ptr, val));
                    self.push_str(&format!("auto {} = {}.size();\n", len, val));
                } else {
                    self.push_str(&format!(
                        "auto {} = static_cast<{}>({}.data());\n",
                        ptr,
                        self.gen.gen.opts.ptr_type(),
                        val
                    ));
                    self.push_str(&format!(
                        "auto {} = static_cast<size_t>({}.size());\n",
                        len, val
                    ));
                }

                let size = self.gen.sizes.size(element);
                self.push_str(&format!("for (size_t i = 0; i < {len}; ++i) {{\n"));
                self.push_str(&format!(
                    "auto base = {ptr} + i * {size};\n",
                    size = size.format(POINTER_SIZE_EXPRESSION)
                ));
                self.push_str(&format!("auto&& iter_elem = {val}[i];\n"));
                self.push_str(&format!("{}\n", body.0));
                self.push_str("}\n");
                if realloc.is_none() {
                    results.push(ptr);
                } else {
                    if !self.gen.gen.opts.host_side()
                        && !(self.gen.gen.opts.symmetric
                            && matches!(self.variant, AbiVariant::GuestImport))
                    {
                        uwriteln!(self.src, "{}.leak();\n", operands[0]);
                    }
                    results.push(ptr);
                }
                results.push(len);
            }
            abi::Instruction::ListCanonLift { element, .. } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                let inner = self
                    .gen
                    .type_name(element, &self.namespace, Flavor::InStruct);
                self.push_str(&format!("auto {} = {};\n", len, operands[1]));
                let result = if self.gen.gen.opts.host {
                    uwriteln!(self.src, "{inner} const* ptr{tmp} = ({inner} const*)wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), {});\n", operands[0]);
                    format!("wit::span<{inner} const>(ptr{}, (size_t){len})", tmp)
                } else if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                {
                    if self.gen.gen.opts.symmetric {
                        format!(
                            "wit::span<{inner} const>(static_cast<{inner}*>({}), {len})",
                            operands[0]
                        )
                    } else {
                        format!(
                            "wit::vector<{inner} const>(static_cast<{inner}*>({}), {len}).get_view()",
                            operands[0]
                        )
                    }
                } else {
                    format!(
                        "wit::vector<{inner}>(static_cast<{inner}*>({}), {len})",
                        operands[0]
                    )
                };
                results.push(result);
            }
            abi::Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                uwriteln!(self.src, "auto {} = {};\n", len, operands[1]);
                let result = if self.gen.gen.opts.symmetric
                    && self.gen.gen.opts.api_style == APIStyle::Asymmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                {
                    uwriteln!(self.src, "auto string{tmp} = wit::string::from_view(std::string_view((char const *)({}), {len}));\n", operands[0]);
                    format!("std::move(string{tmp})")
                } else if self.gen.gen.opts.host {
                    uwriteln!(self.src, "char const* ptr{} = static_cast<char const*>(wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), {}));\n", tmp, operands[0]);
                    format!("std::string_view(ptr{}, {len})", tmp)
                } else if self.gen.gen.opts.short_cut
                    || (self.gen.gen.opts.api_style == APIStyle::Symmetric
                        && matches!(self.variant, AbiVariant::GuestExport))
                {
                    if self.gen.gen.opts.api_style == APIStyle::Symmetric
                        && matches!(self.variant, AbiVariant::GuestExport)
                        && !self.gen.gen.opts.symmetric
                    {
                        assert!(self.needs_dealloc);
                        uwriteln!(
                            self.src,
                            "if ({len}>0) _deallocate.push_back({});\n",
                            operands[0]
                        );
                    }
                    format!(
                        "std::string_view(static_cast<char const*>({}), {len})",
                        operands[0]
                    )
                } else {
                    format!(
                        "wit::string(static_cast<char const*>({}), {len})",
                        operands[0]
                    )
                };
                results.push(result);
            }
            abi::Instruction::ListLift { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let _align = self.gen.sizes.align(element);
                let flavor = if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                {
                    Flavor::BorrowedArgument
                } else {
                    Flavor::InStruct
                };
                let vtype = self.gen.type_name(element, &self.namespace, flavor);
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
                self.push_str(&format!(
                    r#"auto {result} = wit::vector<{vtype}>::allocate({len});
                    "#,
                ));

                if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                    && !self.gen.gen.opts.symmetric
                {
                    assert!(self.needs_dealloc);
                    self.push_str(&format!("if ({len}>0) _deallocate.push_back({base});\n"));
                }

                uwriteln!(self.src, "for (unsigned i=0; i<{len}; ++i) {{");
                uwriteln!(
                    self.src,
                    "auto base = {base} + i * {size};",
                    size = size.format(POINTER_SIZE_EXPRESSION)
                );
                uwrite!(self.src, "{}", body.0);
                uwriteln!(self.src, "auto e{tmp} = {};", body.1[0]);
                if let Some(code) = self.leak_on_insertion.take() {
                    assert!(self.needs_dealloc);
                    uwriteln!(self.src, "{code}");
                }
                // inplace construct
                uwriteln!(self.src, "{result}.initialize(i, std::move(e{tmp}));");
                uwriteln!(self.src, "}}");

                if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestImport)
                    && self.gen.gen.opts.symmetric
                {
                    // we converted the result, free the returned vector
                    uwriteln!(self.src, "free({base});");
                }
                if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                {
                    results.push(format!("{result}.get_const_view()"));
                    if !self.gen.gen.opts.symmetric
                        || (self.gen.gen.opts.api_style == APIStyle::Symmetric
                            && matches!(self.variant, AbiVariant::GuestExport))
                    {
                        self.leak_on_insertion.replace(format!(
                            "if ({len}>0) _deallocate.push_back(static_cast<void*>({result}.leak()));\n"
                        ));
                    }
                } else {
                    results.push(move_if_necessary(&result));
                }
            }
            abi::Instruction::FixedSizeListLift {
                element,
                size,
                id: _,
            } => {
                let tmp = self.tmp();
                let result = format!("result{tmp}");
                let typename = self
                    .gen
                    .type_name(element, &self.namespace, Flavor::InStruct);
                self.push_str(&format!("std::array<{typename}, {size}> {result} = {{",));
                for a in operands.drain(0..(*size as usize)) {
                    self.push_str(&a);
                    self.push_str(", ");
                }
                self.push_str("};\n");
                results.push(result);
            }
            abi::Instruction::FixedSizeListLower { .. } => todo!(),
            abi::Instruction::FixedSizeListLowerMemory {
                element,
                size: elemsize,
                id: _,
            } => {
                let body = self.blocks.pop().unwrap();
                let vec = operands[0].clone();
                let target = operands[1].clone();
                let size = self.r#gen.sizes.size(element);
                let size_str = size.format(POINTER_SIZE_EXPRESSION);
                let typename = self
                    .r#gen
                    .type_name(element, &self.namespace, Flavor::InStruct);
                let ptr_type = self.r#gen.r#gen.opts.ptr_type();
                self.push_str(&format!(
                    "{{
                    {ptr_type} outer_base = {target};\n"
                ));
                let target: String = "outer_base".into();
                self.push_str(&format!(
                    "std::array<{typename}, {elemsize}>& outer_vec = {vec};\n"
                ));
                let vec: String = "outer_vec".into();
                self.push_str(&format!("for (unsigned i = 0; i<{vec}.size(); ++i) {{\n",));
                self.push_str(&format!(
                    "{ptr_type} base = {target} + i * {size_str};
                     {typename}& iter_elem = {vec}[i];\n"
                ));
                self.push_str(&body.0);
                self.push_str("\n}\n}\n");
            }
            abi::Instruction::IterElem { .. } => results.push("iter_elem".to_string()),
            abi::Instruction::IterBasePointer => results.push("base".to_string()),
            abi::Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for f in record.fields.iter() {
                    results.push(format!("({}).{}", op, to_c_ident(&f.name)));
                }
            }
            abi::Instruction::RecordLift { record, ty, .. } => {
                let mut result =
                    self.gen
                        .type_name(&Type::Id(*ty), &self.namespace, Flavor::InStruct);
                result.push('{');
                for (_field, val) in record.fields.iter().zip(operands) {
                    result.push_str(&(move_if_necessary(val) + ", "));
                }
                result.push('}');
                results.push(result);
            }
            abi::Instruction::HandleLower {
                handle: Handle::Own(_ty),
                ..
            } => {
                let op = &operands[0];
                if self.gen.gen.opts.host_side() {
                    if matches!(self.variant, AbiVariant::GuestImport) {
                        results.push(format!("{op}.release()->get_handle()"));
                    } else {
                        let tmp = self.tmp();
                        let var = self.tempname("rep", tmp);
                        uwriteln!(self.src, "auto {var} = {op}.take_rep();");
                        results.push(format!("{op}.get_handle()"));
                    }
                } else {
                    if matches!(self.variant, AbiVariant::GuestImport) {
                        results.push(format!("{op}.into_handle()"));
                    } else {
                        results.push(format!("{op}.release()->handle"));
                    }
                }
            }
            abi::Instruction::HandleLower {
                handle: Handle::Borrow(_),
                ..
            } => {
                let op = &operands[0];
                if self.gen.gen.opts.host_side() {
                    if op == "(*this)" {
                        results.push(format!("{op}.get_rep()"));
                    } else {
                        results.push(format!("{op}.get().get_rep()"));
                    }
                } else if op == "(*this)" {
                    // TODO is there a better way to decide?
                    results.push(format!("{op}.get_handle()"));
                } else {
                    results.push(format!("{op}.get().get_handle()"));
                }
            }
            abi::Instruction::HandleLift { handle, .. } => {
                let op = &operands[0];
                match (handle, self.gen.gen.opts.host_side()) {
                    (Handle::Own(ty), true) => match self.variant {
                        AbiVariant::GuestExport => {
                            results.push(format!("wit::{RESOURCE_EXPORT_BASE_CLASS_NAME}{{{op}}}"))
                        }
                        AbiVariant::GuestImport => {
                            let tmp = self.tmp();
                            let var = self.tempname("obj", tmp);
                            let tname = self.gen.type_name(
                                &Type::Id(*ty),
                                &self.namespace,
                                Flavor::Argument(self.variant),
                            );
                            uwriteln!(
                                self.src,
                                "auto {var} = {tname}::remove_resource({op});
                                assert({var}.has_value());"
                            );
                            results.push(format!("{tname}::Owned(*{var})"));
                        }
                        AbiVariant::GuestImportAsync => todo!(),
                        AbiVariant::GuestExportAsync => todo!(),
                        AbiVariant::GuestExportAsyncStackful => todo!(),
                    },
                    (Handle::Own(ty), false) => match self.variant {
                        AbiVariant::GuestImport => {
                            results.push(format!("wit::{RESOURCE_IMPORT_BASE_CLASS_NAME}{{{op}}}"))
                        }
                        AbiVariant::GuestExport => {
                            let tmp = self.tmp();
                            let var = self.tempname("obj", tmp);
                            let tname = self.gen.type_name(
                                &Type::Id(*ty),
                                &self.namespace,
                                Flavor::Argument(self.variant),
                            );
                            uwriteln!(
                                self.src,
                                "auto {var} = {tname}::Owned({tname}::ResourceRep({op}));"
                            );
                            // if !self.gen.gen.opts.symmetric {
                            //     uwriteln!(self.src, "{var}->into_handle();");
                            // }
                            results.push(format!("std::move({var})"))
                        }
                        AbiVariant::GuestImportAsync => todo!(),
                        AbiVariant::GuestExportAsync => todo!(),
                        AbiVariant::GuestExportAsyncStackful => todo!(),
                    },
                    (Handle::Borrow(ty), true) => {
                        let tname = self.gen.type_name(
                            &Type::Id(*ty),
                            &self.namespace,
                            Flavor::Argument(self.variant),
                        );
                        results.push(format!("**{tname}::lookup_resource({op})"));
                    }
                    (Handle::Borrow(ty), false) => match self.variant {
                        AbiVariant::GuestImport => results.push(op.clone()),
                        AbiVariant::GuestExport => {
                            let tname = self.gen.type_name(
                                &Type::Id(*ty),
                                &self.namespace,
                                Flavor::Argument(self.variant),
                            );
                            results.push(format!("std::ref(*({tname} *){op})"));
                        }
                        AbiVariant::GuestImportAsync => todo!(),
                        AbiVariant::GuestExportAsync => todo!(),
                        AbiVariant::GuestExportAsyncStackful => todo!(),
                    },
                }
            }
            abi::Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                for n in 0..tuple.types.len() {
                    results.push(format!("std::get<{n}>({op})"));
                }
            }
            abi::Instruction::TupleLift { tuple, .. } => {
                let name = format!("tuple{}", self.tmp());
                uwrite!(self.src, "auto {name} = std::tuple<");
                self.src.push_str(
                    &(tuple
                        .types
                        .iter()
                        .map(|t| self.gen.type_name(t, &self.namespace, Flavor::InStruct)))
                    .collect::<Vec<_>>()
                    .join(", "),
                );
                self.src.push_str(">(");
                self.src.push_str(&operands.join(", "));
                self.src.push_str(");\n");
                results.push(format!("std::move({name})"));
            }
            abi::Instruction::FlagsLower { flags, ty, .. } => {
                match wit_bindgen_c::flags_repr(flags) {
                    Int::U8 | Int::U16 | Int::U32 => {
                        results.push(format!("static_cast<int32_t>({})", operands.pop().unwrap()));
                    }
                    Int::U64 => {
                        let name =
                            self.gen
                                .type_name(&Type::Id(*ty), &self.namespace, Flavor::InStruct);
                        let tmp = self.tmp();
                        let tempname = self.tempname("flags", tmp);
                        uwriteln!(self.src, "{name} {tempname} = {};", operands[0]);
                        results.push(format!(
                            "static_cast<int32_t>(static_cast<uint64_t>({tempname}) & 0xffffffff)"
                        ));
                        results.push(format!(
                            "static_cast<int32_t>((static_cast<uint64_t>({tempname}) >> 32) & 0xffffffff)"
                        ));
                    }
                }
            }
            abi::Instruction::FlagsLift { flags, ty, .. } => {
                let typename =
                    self.gen
                        .type_name(&Type::Id(*ty), &self.namespace, Flavor::InStruct);
                match wit_bindgen_c::flags_repr(flags) {
                    Int::U8 | Int::U16 | Int::U32 => {
                        results.push(format!("(({typename}){})", operands.pop().unwrap()));
                    }
                    Int::U64 => {
                        let op0 = &operands[0];
                        let op1 = &operands[1];
                        results.push(format!(
                            "(static_cast<{typename}>(({op0}) | (static_cast<uint64_t>({op1}) << 32)))"
                        ));
                    }
                }
            }
            abi::Instruction::VariantPayloadName => {
                let name = format!("payload{}", self.tmp());
                results.push(name.clone());
                self.payloads.push(name);
            }
            abi::Instruction::VariantLower {
                variant,
                results: result_types,
                ty: var_ty,
                name: _var_name,
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
                for ty in result_types.iter() {
                    let name = format!("variant{}", self.tmp());
                    results.push(name.clone());
                    self.src.push_str(self.gen.gen.opts.wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    variant_results.push(name);
                }

                let expr_to_match = format!("({}).variants.index()", operands[0]);
                let elem_ns =
                    self.gen
                        .type_name(&Type::Id(*var_ty), &self.namespace, Flavor::InStruct);

                uwriteln!(
                    self.src,
                    "switch (static_cast<int32_t>({})) {{",
                    expr_to_match
                );
                for (i, ((case, (block, block_results)), payload)) in
                    variant.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    uwriteln!(self.src, "case {}: {{", i);
                    if let Some(ty) = case.ty.as_ref() {
                        let ty = self.gen.type_name(ty, &self.namespace, Flavor::InStruct);
                        let case = format!("{elem_ns}::{}", case.name.to_pascal_case());
                        uwriteln!(
                            self.src,
                            "{} &{} = std::get<{case}>({}.variants).value;",
                            ty,
                            payload,
                            operands[0],
                        );
                    }

                    self.src.push_str(&block);

                    for (name, result) in variant_results.iter().zip(&block_results) {
                        uwriteln!(self.src, "{} = {};", name, result);
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

                let ty = self
                    .gen
                    .type_name(&Type::Id(*ty), &self.namespace, Flavor::InStruct);
                let resultno = self.tmp();
                let result = format!("variant{resultno}");

                uwriteln!(self.src, "{ty} {result};");

                let op0 = &operands[0];

                uwriteln!(self.src, "switch ({op0}) {{");
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    let tp = case.name.clone().to_pascal_case();
                    uwriteln!(self.src, "case {i}: {{ {block}");
                    uwriteln!(
                        self.src,
                        "{result}.variants = {ty}::{tp}{{{}}};",
                        move_if_necessary(&block_results.first().cloned().unwrap_or_default())
                    );
                    uwriteln!(self.src, "}} break;");
                }
                uwriteln!(self.src, "}}");

                results.push(result);
            }
            abi::Instruction::EnumLower { .. } => {
                results.push(format!("static_cast<int32_t>({})", operands[0]))
            }
            abi::Instruction::EnumLift { ty, .. } => {
                let typename =
                    self.gen
                        .type_name(&Type::Id(*ty), &self.namespace, Flavor::InStruct);
                results.push(format!("static_cast<{typename}>({})", &operands[0]));
            }
            abi::Instruction::OptionLower {
                payload,
                results: result_types,
                ..
            } => {
                let (mut some, some_results) = self.blocks.pop().unwrap();
                let (mut none, none_results) = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let _none_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let tmp = self.tmp();
                    let name = self.tempname("option", tmp);
                    results.push(name.clone());
                    self.src.push_str(self.gen.gen.opts.wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    let some_result = &some_results[i];
                    uwriteln!(some, "{name} = {some_result};");
                    let none_result = &none_results[i];
                    uwriteln!(none, "{name} = {none_result};");
                }

                let op0 = &operands[0];
                let flavor = if matches!(self.variant, AbiVariant::GuestImport) {
                    Flavor::BorrowedArgument
                } else {
                    Flavor::InStruct
                };
                let ty = self.gen.type_name(payload, &self.namespace, flavor);
                let bind_some = format!("{ty} {some_payload} = (std::move({op0})).value();");

                uwrite!(
                    self.src,
                    "\
                    if (({op0}).has_value()) {{
                        {bind_some}
                        {some}}} else {{
                        {none}}}
                    "
                );
            }
            abi::Instruction::OptionLift { payload, .. } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (_none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.is_empty());
                assert!(some_results.len() == 1);
                let flavor = if self.gen.gen.opts.api_style == APIStyle::Symmetric
                    && matches!(self.variant, AbiVariant::GuestExport)
                {
                    Flavor::BorrowedArgument
                } else {
                    Flavor::InStruct
                };
                let type_name = self.gen.type_name(payload, &self.namespace, flavor);
                let full_type = format!("std::optional<{type_name}>");
                let op0 = &operands[0];

                let tmp = self.tmp();
                let resultname = self.tempname("option", tmp);
                uwriteln!(
                    self.src,
                    "{full_type} {resultname};
                    if ({op0}) {{
                        {some}
                        {resultname}.emplace({});
                    }}",
                    some_results[0]
                );
                results.push(format!("std::move({resultname})"));
            }
            abi::Instruction::ResultLower {
                results: result_types,
                result,
                ..
            } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                let (mut ok, ok_results) = self.blocks.pop().unwrap();
                let err_payload = self.payloads.pop().unwrap();
                let ok_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let tmp = self.tmp();
                    let name = self.tempname("result", tmp);
                    results.push(name.clone());
                    self.src.push_str(self.gen.gen.opts.wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    let ok_result = &ok_results[i];
                    uwriteln!(ok, "{name} = {ok_result};");
                    let err_result = &err_results[i];
                    uwriteln!(err, "{name} = {err_result};");
                }

                let op0 = &operands[0];
                let ok_ty = self.gen.optional_type_name(
                    result.ok.as_ref(),
                    &self.namespace,
                    Flavor::InStruct,
                );
                let err_ty = self.gen.optional_type_name(
                    result.err.as_ref(),
                    &self.namespace,
                    Flavor::InStruct,
                );
                let bind_ok = if let Some(_ok) = result.ok.as_ref() {
                    format!("{ok_ty} {ok_payload} = std::move({op0}).value();")
                } else {
                    String::new()
                };
                let bind_err = if let Some(_err) = result.err.as_ref() {
                    format!("{err_ty} {err_payload} = std::move({op0}).error();")
                } else {
                    String::new()
                };

                uwrite!(
                    self.src,
                    "\
                    if (({op0}).has_value()) {{
                        {bind_ok}
                        {ok}}} else {{
                        {bind_err}
                        {err}}}
                    "
                );
            }
            abi::Instruction::ResultLift { result, .. } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                let (mut ok, ok_results) = self.blocks.pop().unwrap();
                let mut ok_result = String::new();
                let err_result;
                if result.ok.is_none() {
                    ok.clear();
                } else {
                    ok_result = move_if_necessary(&ok_results[0]);
                }
                if result.err.is_none() {
                    err.clear();
                    self.gen.gen.dependencies.needs_wit = true;
                    err_result = String::from("wit::Void{}");
                } else {
                    err_result = move_if_necessary(&err_results[0]);
                }
                let ok_type = self.gen.optional_type_name(
                    result.ok.as_ref(),
                    &self.namespace,
                    Flavor::InStruct,
                );
                let err_type = result.err.as_ref().map_or(String::from("wit::Void"), |ty| {
                    self.gen.type_name(ty, &self.namespace, Flavor::InStruct)
                });
                let full_type = format!("std::expected<{ok_type}, {err_type}>",);
                let err_type = "std::unexpected";
                let operand = &operands[0];

                let tmp = self.tmp();
                let resultname = self.tempname("result", tmp);
                let ok_assign = if result.ok.is_some() {
                    format!("{resultname}.emplace({ok_result});")
                } else {
                    String::new()
                };
                uwriteln!(
                    self.src,
                    "{full_type} {resultname};
                    if ({operand}==0) {{
                        {ok}
                        {ok_assign}
                    }} else {{
                        {err}
                        {resultname}={err_type}{{{err_result}}};
                    }}"
                );
                results.push(resultname);
            }
            abi::Instruction::CallWasm {
                name,
                sig,
                module_prefix,
            } => {
                let module_name = self
                    .gen
                    .wasm_import_module
                    .as_ref()
                    .map(|e| {
                        self.gen
                            .gen
                            .import_prefix
                            .as_ref()
                            .cloned()
                            .unwrap_or_default()
                            + *module_prefix
                            + e
                    })
                    .unwrap();
                if self.gen.gen.opts.host {
                    uwriteln!(self.src, "wasm_function_inst_t wasm_func = wasm_runtime_lookup_function(wasm_runtime_get_module_inst(exec_env), \n\
                            \"{}#{}\", \"{}\");", module_name, name, self.wamr_signature.as_ref().unwrap().to_string());
                    if !sig.results.is_empty() {
                        uwriteln!(
                            self.src,
                            "wasm_val_t wasm_results[{}] = {{ WASM_INIT_VAL }};",
                            sig.results.len()
                        );
                    } else {
                        uwriteln!(self.src, "wasm_val_t *wasm_results = nullptr;");
                    }
                    if !sig.params.is_empty() {
                        uwrite!(self.src, "wasm_val_t wasm_args[{}] = {{", sig.params.len());
                        for (typ, value) in sig.params.iter().zip(operands.iter()) {
                            match typ {
                                WasmType::I32 => uwrite!(self.src, "WASM_I32_VAL({}),", value),
                                WasmType::I64 => uwrite!(self.src, "WASM_I64_VAL({}),", value),
                                WasmType::F32 => uwrite!(self.src, "WASM_F32_VAL({}),", value),
                                WasmType::F64 => uwrite!(self.src, "WASM_F64_VAL({}),", value),
                                WasmType::Length => {
                                    if self.gen.gen.opts.wasm64 {
                                        uwrite!(self.src, "WASM_I64_VAL({}),", value)
                                    } else {
                                        uwrite!(self.src, "WASM_I32_VAL((int32_t){}),", value)
                                    }
                                }
                                WasmType::Pointer => {
                                    if self.gen.gen.opts.wasm64 {
                                        uwrite!(self.src, "WASM_I64_VAL({}),", value)
                                    } else {
                                        uwrite!(self.src, "WASM_I32_VAL((int32_t){}),", value)
                                    }
                                }
                                WasmType::PointerOrI64 => {
                                    uwrite!(self.src, "WASM_I64_VAL({}),", value)
                                }
                            }
                        }
                        self.src.push_str("};\n");
                    } else {
                        uwriteln!(self.src, "wasm_val_t *wasm_args = nullptr;");
                    }
                    uwriteln!(self.src, "bool wasm_ok = wasm_runtime_call_wasm_a(exec_env, wasm_func, {}, wasm_results, {}, wasm_args);", sig.results.len(), sig.params.len());
                    uwriteln!(self.src, "assert(wasm_ok);");
                    if sig.results.len() > 0 {
                        let (kind, elem) = match sig.results.first() {
                            Some(WasmType::I32) => (String::from("WASM_I32"), String::from("i32")),
                            Some(WasmType::I64) => (String::from("WASM_I64"), String::from("i64")),
                            Some(WasmType::F32) => (String::from("WASM_F32"), String::from("f32")),
                            Some(WasmType::F64) => (String::from("WASM_F64"), String::from("f64")),
                            Some(WasmType::Pointer) => {
                                if self.gen.gen.opts.wasm64 {
                                    (String::from("WASM_I64"), String::from("i64"))
                                } else {
                                    (String::from("WASM_I32"), String::from("i32"))
                                }
                            }
                            Some(WasmType::Length) => {
                                if self.gen.gen.opts.wasm64 {
                                    (String::from("WASM_I64"), String::from("i64"))
                                } else {
                                    (String::from("WASM_I32"), String::from("i32"))
                                }
                            }
                            Some(WasmType::PointerOrI64) => {
                                (String::from("WASM_I64"), String::from("i64"))
                            }
                            None => todo!(),
                        };
                        uwriteln!(self.src, "assert(wasm_results[0].kind=={kind});");
                        uwriteln!(self.src, "auto ret = wasm_results[0].of.{elem};");
                        results.push("ret".to_string());
                    }
                } else {
                    let func =
                        self.gen
                            .declare_import(&module_name, name, &sig.params, &sig.results);

                    // ... then call the function with all our operands
                    if sig.results.len() > 0 {
                        self.src.push_str("auto ret = ");
                        results.push("ret".to_string());
                    }
                    self.src.push_str(&func);
                    self.src.push_str("(");
                    self.src.push_str(&operands.join(", "));
                    self.src.push_str(");\n");
                }
            }
            abi::Instruction::CallInterface { func, .. } => {
                // dbg!(func);
                self.let_results(if func.result.is_some() { 1 } else { 0 }, results);
                let (mut namespace, func_name_h) =
                    self.gen
                        .func_namespace_name(func, !self.gen.gen.opts.host_side(), true);
                if matches!(func.kind, FunctionKind::Method(_)) {
                    let this = operands.remove(0);
                    if self.gen.gen.opts.host_side() {
                        uwrite!(self.src, "({this}).");
                    } else {
                        uwrite!(self.src, "({this}).get().");
                    }
                } else {
                    if matches!(func.kind, FunctionKind::Constructor(_))
                        && self.gen.gen.opts.host_side()
                    {
                        let _ = namespace.pop();
                    }
                    let mut relative = SourceWithState::default();
                    relative.qualify(&namespace);
                    self.push_str(&relative.src);
                }
                self.src.push_str(&func_name_h);
                if matches!(func.kind, FunctionKind::Constructor(_))
                    && self.gen.gen.opts.host_side()
                {
                    self.push_str("::New");
                }
                self.push_str("(");
                if self.gen.gen.opts.host {
                    if !matches!(func.kind, FunctionKind::Method(_)) {
                        self.push_str("exec_env");
                        if !operands.is_empty() {
                            self.push_str(", ");
                        }
                    }
                }
                self.push_str(&operands.join(", "));
                self.push_str(");\n");
                if self.needs_dealloc {
                    uwriteln!(
                        self.src,
                        "for (auto i: _deallocate) {{ free(i); }}\n
                        _deallocate.clear();"
                    );
                }
            }
            abi::Instruction::Return { amt, func } => {
                match amt {
                    0 => {}
                    _ => {
                        assert!(*amt == operands.len());
                        match &func.kind {
                            FunctionKind::Constructor(_)
                                if self.gen.gen.opts.is_only_handle(self.variant) =>
                            {
                                // strange but works
                                if matches!(self.variant, AbiVariant::GuestExport) {
                                    self.src.push_str("this->index = ");
                                } else {
                                    self.src.push_str("this->handle = ");
                                }
                            }
                            _ => self.src.push_str("return "),
                        }
                        if let Some(CabiPostInformation {
                            module: _,
                            name: _cabi_post_name,
                            ret_type: cabi_post_type,
                        }) = self.cabi_post.as_ref()
                        {
                            self.src.push_str("wit::guest_owned<");
                            self.src.push_str(cabi_post_type);
                            self.src.push_str(">(");
                        }
                        if *amt == 1 {
                            if operands[0].starts_with("std::move(") && !operands[0].contains('.') {
                                // remove the std::move due to return value optimization (and complex rules about when std::move harms)
                                self.src.push_str(&operands[0][9..]);
                            } else {
                                self.src.push_str(&operands[0]);
                            }
                        } else {
                            todo!();
                        }
                        if let Some(CabiPostInformation {
                            module: func_module,
                            name: func_name,
                            ret_type: _cabi_post_type,
                        }) = self.cabi_post.as_ref()
                        {
                            if self.gen.gen.opts.host {
                                let cabi_post_name = make_external_symbol(
                                    &func_module,
                                    &func_name,
                                    AbiVariant::GuestExport,
                                );
                                self.src.push_str(&format!(", wasm_results[0].of.i32, wasm_runtime_lookup_function(wasm_runtime_get_module_inst(exec_env), \"cabi_post_{}\", \"(i)\"), exec_env)", cabi_post_name));
                            } else {
                                let cabi_post_name = self.gen.declare_import(
                                    &format!("cabi_post_{func_module}"),
                                    func_name,
                                    &[WasmType::Pointer],
                                    &[],
                                );
                                self.src.push_str(&format!(", ret, {})", cabi_post_name));
                            }
                        }
                        if matches!(func.kind, FunctionKind::Constructor(_))
                            && self.gen.gen.opts.is_only_handle(self.variant)
                        {
                            // we wrapped the handle in an object, so unpack it
                            if self.gen.gen.opts.host_side() {
                                self.src.push_str(
                                    ".get_handle();
                                    this->rep = *lookup_resource(ret)",
                                );
                            } else {
                                self.src.push_str(".into_handle()");
                            }
                        }
                        self.src.push_str(";\n");
                    }
                }
            }
            abi::Instruction::Malloc { .. } => todo!(),
            abi::Instruction::GuestDeallocate { .. } => {
                uwriteln!(self.src, "free(static_cast<void*>({}));", operands[0]);
            }
            abi::Instruction::GuestDeallocateString => {
                uwriteln!(self.src, "if (({}) > 0) {{", operands[1]);
                uwriteln!(
                    self.src,
                    "wit::string::drop_raw(static_cast<void*>({}));",
                    operands[0]
                );
                uwriteln!(self.src, "}}");
            }
            abi::Instruction::GuestDeallocateList { element } => {
                let (body, results) = self.blocks.pop().unwrap();
                assert!(results.is_empty());
                let tmp = self.tmp();
                let ptr = self.tempname("ptr", tmp);
                let len = self.tempname("len", tmp);
                uwriteln!(self.src, "uint8_t* {ptr} = {};", operands[0]);
                uwriteln!(self.src, "size_t {len} = {};", operands[1]);
                let i = self.tempname("i", tmp);
                uwriteln!(self.src, "for (size_t {i} = 0; {i} < {len}; {i}++) {{");
                let size = self.gen.sizes.size(element);
                uwriteln!(
                    self.src,
                    "uint8_t* base = {ptr} + {i} * {size};",
                    size = size.format(POINTER_SIZE_EXPRESSION)
                );
                uwriteln!(self.src, "(void) base;");
                uwrite!(self.src, "{body}");
                uwriteln!(self.src, "}}");
                uwriteln!(self.src, "if ({len} > 0) {{");
                uwriteln!(self.src, "free(static_cast<void*>({ptr}));");
                uwriteln!(self.src, "}}");
            }
            abi::Instruction::GuestDeallocateVariant { blocks } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .collect::<Vec<_>>();

                uwriteln!(
                    self.src,
                    "switch (static_cast<int32_t>({})) {{",
                    operands[0]
                );
                for (i, (block, results)) in blocks.into_iter().enumerate() {
                    assert!(results.is_empty());
                    uwriteln!(self.src, "case {}: {{", i);
                    self.src.push_str(&block);
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }
            abi::Instruction::PointerLoad { offset } => {
                let ptr_type = self.gen.gen.opts.ptr_type();
                self.load(ptr_type, *offset, operands, results)
            }
            abi::Instruction::LengthLoad { offset } => {
                self.load("size_t", *offset, operands, results)
            }
            abi::Instruction::PointerStore { offset } => {
                let ptr_type = self.gen.gen.opts.ptr_type();
                self.store(ptr_type, *offset, operands)
            }
            abi::Instruction::LengthStore { offset } => self.store("size_t", *offset, operands),
            abi::Instruction::FutureLower { payload, .. } => {
                let lower_lift = self.lower_lift(payload.as_ref());
                results.push(format!(
                    "lower_future<{}, {lower_lift}>(std::move({}))",
                    self.gen.optional_type_name(
                        payload.as_ref(),
                        &self.namespace,
                        Flavor::InStruct
                    ),
                    operands[0]
                ));
            }
            abi::Instruction::FutureLift { payload, .. } => {
                let lower_lift = self.lower_lift(payload.as_ref());
                results.push(format!(
                    "lift_future<{}, {lower_lift}>({})",
                    self.gen.optional_type_name(
                        payload.as_ref(),
                        &self.namespace,
                        Flavor::InStruct
                    ),
                    operands[0]
                ));
            }
            abi::Instruction::StreamLower { payload, .. } => {
                self.lower_lift_stream(payload.as_ref());
                results.push(format!(
                    "lower_stream<{}>(std::move({}))",
                    self.gen.optional_type_name(
                        payload.as_ref(),
                        &self.namespace,
                        Flavor::InStruct
                    ),
                    operands[0]
                ));
            }
            abi::Instruction::StreamLift { payload, .. } => {
                self.lower_lift_stream(payload.as_ref());
                results.push(format!(
                    "lift_stream<{}>({})",
                    self.gen.optional_type_name(
                        payload.as_ref(),
                        &self.namespace,
                        Flavor::InStruct
                    ),
                    operands[0]
                ));
            }
            abi::Instruction::ErrorContextLower { .. } => todo!(),
            abi::Instruction::ErrorContextLift { .. } => todo!(),
            abi::Instruction::Flush { amt } => {
                for i in operands.iter().take(*amt) {
                    let tmp = self.tmp();
                    let result = format!("result{}", tmp);
                    uwriteln!(self.src, "auto {result} = {};", move_if_necessary(i));
                    results.push(result);
                }
            }
            abi::Instruction::AsyncTaskReturn { .. } => todo!(),
            abi::Instruction::DropHandle { .. } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: ArchitectureSize, align: Alignment) -> Self::Operand {
        let tmp = self.tmp();
        let size_string = size.format(POINTER_SIZE_EXPRESSION);
        let tp = match align {
            Alignment::Bytes(bytes) => match bytes.get() {
                1 => "uint8_t",
                2 => "uint16_t",
                4 => "uint32_t",
                8 => "uint64_t",
                _ => todo!(),
            },
            Alignment::Pointer => "uintptr_t",
        };
        let static_var = if self.gen.in_guest_import {
            ""
        } else {
            "static "
        };
        uwriteln!(
            self.src,
            "{static_var}{tp} ret_area[({size_string}+sizeof({tp})-1)/sizeof({tp})];"
        );
        uwriteln!(
            self.src,
            "{} ptr{tmp} = static_cast<{0}>(&ret_area);",
            self.gen.gen.opts.ptr_type(),
        );

        format!("ptr{}", tmp)
    }

    fn push_block(&mut self) {
        let prev = core::mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<Self::Operand>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = core::mem::replace(&mut self.src, to_restore);
        self.blocks.push((src.into(), core::mem::take(operands)));
    }

    fn sizes(&self) -> &wit_bindgen_core::wit_parser::SizeAlign {
        &self.gen.sizes
    }

    fn is_list_canonical(
        &self,
        resolve: &Resolve,
        ty: &wit_bindgen_core::wit_parser::Type,
    ) -> bool {
        if !resolve.all_bits_valid(ty) {
            return false;
        }
        match ty {
            Type::Id(id) => !self.gen.gen.types.get(*id).has_resource,
            _ => true,
        }
    }
}

/// This describes the common ABI function referenced or implemented, the C++ side might correspond to a different type
enum SpecialMethod {
    None,
    ResourceDrop, // ([export]) [resource-drop]
    ResourceNew,  // [export][resource-new]
    ResourceRep,  // [export][resource-rep]
    Dtor,         // [dtor] (guest export only)
    Allocate,     // internal: allocate new object (called from generated code)
}

fn is_special_method(func: &Function) -> SpecialMethod {
    if matches!(func.kind, FunctionKind::Static(_)) {
        if func.name.starts_with("[resource-drop]") {
            SpecialMethod::ResourceDrop
        } else if func.name.starts_with("[resource-new]") {
            SpecialMethod::ResourceNew
        } else if func.name.starts_with("[resource-rep]") {
            SpecialMethod::ResourceRep
        } else if func.name.starts_with("[dtor]") {
            SpecialMethod::Dtor
        } else if func.name == "$alloc" {
            SpecialMethod::Allocate
        } else {
            SpecialMethod::None
        }
    } else {
        SpecialMethod::None
    }
}
