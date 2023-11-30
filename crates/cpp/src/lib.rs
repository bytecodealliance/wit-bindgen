use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase, *};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write as FmtWrite,
    io::{Read, Write},
    process::{Command, Stdio},
};
use wit_bindgen_c::{to_c_ident, wasm_type};
use wit_bindgen_core::{
    abi::{self, AbiVariant, LiftLower, WasmSignature},
    abi::{Bindgen, WasmType},
    uwrite, uwriteln,
    wit_parser::{
        Docs, Function, FunctionKind, Handle, InterfaceId, Resolve, Results, SizeAlign, Type,
        TypeDefKind, TypeId, TypeOwner, WorldId, WorldKey,
    },
    Files, InterfaceGenerator, Source, WorldGenerator,
};

mod wamr;

pub const RESOURCE_BASE_CLASS_NAME: &str = "ResourceBase";
pub const OWNED_CLASS_NAME: &str = "Owned";

type CppType = String;

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
    needs_resources: bool,
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

#[derive(Default)]
struct Cpp {
    opts: Opts,
    c_src: SourceWithState,
    h_src: SourceWithState,
    dependencies: Includes,
    includes: Vec<String>,
    host_functions: HashMap<String, Vec<HostFunction>>,
    world: String,
    world_id: Option<WorldId>,
    imported_interfaces: HashSet<InterfaceId>,
    user_class_files: HashMap<String, String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Generate host bindings
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub host: bool,
    /// Generate code for directly linking to guest code
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub short_cut: bool,
    /// Call clang-format on the generated code
    #[cfg_attr(feature = "clap", arg(long, default_value_t = bool::default()))]
    pub format: bool,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = Cpp::new();
        r.opts = self;
        Box::new(r)
    }
}

impl Cpp {
    fn new() -> Cpp {
        Cpp::default()
    }

    fn include(&mut self, s: &str) {
        self.includes.push(s.to_string());
    }

    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a Option<&'a WorldKey>,
        in_import: bool,
        wasm_import_module: Option<String>,
    ) -> CppInterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(resolve);

        CppInterfaceGenerator {
            _src: Source::default(),
            gen: self,
            resolve,
            interface: None,
            _name: name,
            sizes,
            // public_anonymous_types: BTreeSet::new(),
            in_import,
            // export_funcs: Vec::new(),
            return_pointer_area_size: 0,
            return_pointer_area_align: 0,
            wasm_import_module,
        }
    }

    fn clang_format(code: &mut Source) {
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
        code.as_mut_string().truncate(0);
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(code.as_mut_string())
            .unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());
    }
}

impl WorldGenerator for Cpp {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.world = name.to_string();
        self.world_id = Some(world);
        //        self.sizes.fill(resolve);
        if !self.opts.host {
            uwriteln!(
                self.c_src.src,
                r#"#include "{}_cpp.h"
            #include <utility>
            #include <cstdlib> // realloc

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
    ) {
        self.imported_interfaces.insert(id);
        let wasm_import_module = resolve.name_world_key(name);
        let binding = Some(name);
        let mut gen = self.interface(resolve, &binding, true, Some(wasm_import_module));
        gen.interface = Some(id);
        // if self.gen.interfaces_with_types_printed.insert(id) {
        gen.types(id);
        // }

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            if matches!(func.kind, FunctionKind::Freestanding) {
                gen.generate_guest_import(func, id);
            }
        }
        // gen.finish();
    }

    fn export_interface(
        &mut self,
        _resolve: &Resolve,
        name: &WorldKey,
        _iface: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        self.h_src
            .src
            .push_str(&format!("// export_interface {name:?}\n"));
        Ok(())
    }

    fn import_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        todo!()
    }

    fn export_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn import_types(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        todo!()
    }

    fn finish(&mut self, resolve: &Resolve, world_id: WorldId, files: &mut Files) {
        let world = &resolve.worlds[world_id];
        let snake = world.name.to_snake_case();

        let mut h_str = SourceWithState::default();
        let mut c_str = SourceWithState::default();

        let version = env!("CARGO_PKG_VERSION");
        uwriteln!(
            h_str.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );

        if !self.opts.host {
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
                #define __CPP_HOST_BINDINGS_{0}_H\n",
                world.name.to_shouty_snake_case(),
            );
        }
        self.include("<cstdint>");
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
        if self.dependencies.needs_optional {
            self.include("<optional>");
        }
        if self.dependencies.needs_cstring {
            self.include("<cstring>");
        }
        if !self.opts.host && self.dependencies.needs_resources {
            self.include("<cassert>");
        }
        if self.opts.host && self.dependencies.needs_resources {
            self.include("<map>");
        }

        for include in self.includes.iter() {
            uwriteln!(h_str.src, "#include {include}");
        }

        uwriteln!(
            c_str.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );
        if !self.opts.host {
            // uwriteln!(c_str.src, "#include \"{snake}_cpp.h\"");
        } else {
            uwriteln!(c_str.src, "#include \"{snake}_cpp_host.h\"");
            if !self.opts.short_cut {
                uwriteln!(
                    c_str.src,
                    "#include <wasm_export.h> // wasm-micro-runtime header"
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
        }

        if self.dependencies.needs_resources {
            let namespace = namespace(resolve, &TypeOwner::World(world_id));
            h_str.change_namespace(&namespace);
            // this is export, not host
            if self.opts.host {
                uwriteln!(
                    h_str.src,
                    "template <class R>
                     class {RESOURCE_BASE_CLASS_NAME} {{
                            static std::map<int32_t, R> resources;
                        public:
                            static R* lookup_resource(int32_t id) {{
                                auto result = resources.find(id);
                                return result == resources.end() ? nullptr : &result->second;
                            }}
                            static int32_t store_resource(R && value) {{
                                auto last = resources.rbegin();
                                int32_t id = last == resources.rend() ? 0 : last->first+1;
                                resources.insert(std::pair<int32_t, R>(id, std::move(value)));
                                return id;
                            }}
                            static void remove_resource(int32_t id) {{
                                resources.erase(id);
                            }}
                        }}; 
                        template <typename T> struct {OWNED_CLASS_NAME} {{
                            T *ptr;
                        }};"
                );
            } else {
                // somehow spaces get removed, newlines remain (problem occurs before const&)
                // TODO: should into_handle become && ???
                uwriteln!(
                    h_str.src,
                    "class {RESOURCE_BASE_CLASS_NAME} {{
                            static const int32_t invalid = -1;
                            protected:
                            int32_t handle;
                            public:
                            {RESOURCE_BASE_CLASS_NAME}(int32_t h=invalid) : handle(h) {{}}
                            {RESOURCE_BASE_CLASS_NAME}({RESOURCE_BASE_CLASS_NAME}&&r) 
                                : handle(r.handle) {{ 
                                    r.handle=invalid; 
                            }}
                            {RESOURCE_BASE_CLASS_NAME}({RESOURCE_BASE_CLASS_NAME} 
                                const&) = delete;
                            void set_handle(int32_t h) {{ handle=h; }}
                            int32_t get_handle() const {{ return handle; }}
                            int32_t into_handle() {{
                                int32_t h= handle;
                                handle= invalid;
                                return h;
                            }}
                            {RESOURCE_BASE_CLASS_NAME}& operator=({RESOURCE_BASE_CLASS_NAME}&&r) {{
                                assert(handle<0);
                                handle= r.handle;
                                r.handle= invalid;
                                return *this;
                            }}
                            {RESOURCE_BASE_CLASS_NAME}& operator=({RESOURCE_BASE_CLASS_NAME} 
                                const&r) = delete;
                            }};"
                );
            }
        }
        h_str.change_namespace(&Vec::default());

        self.c_src.change_namespace(&Vec::default());
        c_str.src.push_str(&self.c_src.src);
        self.h_src.change_namespace(&Vec::default());
        h_str.src.push_str(&self.h_src.src);
        // c_str.push_str(&self.src.c_fns);

        // if self.src.h_defs.len() > 0 {
        //     h_str.push_str(&self.src.h_defs);
        // }

        // h_str.push_str(&self.src.h_fns);

        uwriteln!(c_str.src, "\n// Component Adapters");

        // c_str.push_str(&self.src.c_adapters);

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
            Self::clang_format(&mut c_str.src);
            Self::clang_format(&mut h_str.src);
        }

        if !self.opts.host {
            files.push(&format!("{snake}.cpp"), c_str.src.as_bytes());
            files.push(&format!("{snake}_cpp.h"), h_str.src.as_bytes());
        } else {
            files.push(&format!("{snake}_host.cpp"), c_str.src.as_bytes());
            files.push(&format!("{snake}_cpp_host.h"), h_str.src.as_bytes());
        }
        for (name, content) in self.user_class_files.iter() {
            files.push(&name, content.as_bytes());
        }
    }
}

// determine namespace
fn namespace(resolve: &Resolve, owner: &TypeOwner) -> Vec<String> {
    let mut result = Vec::default();
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
    fn change_namespace(&mut self, target: &Vec<String>) {
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
            uwrite!(self.src, "}}");
        }
        if same != self.namespace.len() {
            // finish closing brackets by a newline
            uwriteln!(self.src, "");
        }
        self.namespace.truncate(same);
        for i in target.iter().skip(same) {
            uwrite!(self.src, "namespace {} {{", i);
            self.namespace.push(i.clone());
        }
    }

    fn qualify(&mut self, target: &Vec<String>) {
        let mut same = 0;
        // itertools::fold_while?
        for (a, b) in self.namespace.iter().zip(target.iter()) {
            if a == b {
                same += 1;
            } else {
                break;
            }
        }
        // if same == 0 {
        //     self.src.push_str("::");
        // }
        for i in target.iter().skip(same) {
            uwrite!(self.src, "{i}::");
        }
    }
}

struct CppInterfaceGenerator<'a> {
    _src: Source,
    gen: &'a mut Cpp,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
    _name: &'a Option<&'a WorldKey>,
    sizes: SizeAlign,
    in_import: bool,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
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
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn func_namespace_name(&self, func: &Function) -> (Vec<String>, String) {
        let (object, owner) = match &func.kind {
            FunctionKind::Freestanding => None,
            FunctionKind::Method(i) => Some(i),
            FunctionKind::Static(i) => Some(i),
            FunctionKind::Constructor(i) => Some(i),
        }
        .map(|i| {
            let ty = &self.resolve.types[*i];
            (ty.name.as_ref().unwrap().to_pascal_case(), ty.owner)
        })
        .unwrap_or((
            Default::default(),
            TypeOwner::World(self.gen.world_id.unwrap()),
        ));
        let mut namespace = namespace(self.resolve, &owner);
        let is_drop = is_drop_method(func);
        let func_name_h = if !matches!(&func.kind, FunctionKind::Freestanding) {
            namespace.push(object.clone());
            if let FunctionKind::Constructor(_i) = &func.kind {
                object.clone()
            } else if is_drop {
                "~".to_string() + &object
            } else {
                func.item_name().to_pascal_case()
            }
        } else {
            func.name.to_pascal_case()
        };
        (namespace, func_name_h)
    }

    // print the signature of the lowered (wasm) function calling into highlevel
    fn print_export_signature(&mut self, func: &Function) -> Vec<String> {
        let is_drop = is_drop_method(func);
        let signature = if is_drop {
            WasmSignature {
                params: vec![WasmType::I32],
                results: Vec::new(),
                indirect_params: false,
                retptr: false,
            }
        } else {
            // TODO perhaps remember better names for the arguments
            self.resolve.wasm_signature(AbiVariant::GuestImport, func)
        };
        self.gen.c_src.src.push_str("static ");
        self.gen
            .c_src
            .src
            .push_str(if signature.results.is_empty() {
                "void"
            } else {
                wasm_type(signature.results[0])
            });
        self.gen.c_src.src.push_str(" ");
        let module_name = self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
        let export_name = CppInterfaceGenerator::export_name2(&module_name, &func.name);
        self.gen.c_src.src.push_str(&export_name);
        self.gen.c_src.src.push_str("(");
        if self.gen.opts.host {
            self.gen.c_src.src.push_str("wasm_exec_env_t exec_env, ");
        }
        let mut params = Vec::new();
        for (n, ty) in signature.params.iter().enumerate() {
            let name = format!("arg{n}");
            self.gen.c_src.src.push_str(wasm_type(*ty));
            self.gen.c_src.src.push_str(" ");
            self.gen.c_src.src.push_str(&name);
            params.push(name);
            if n + 1 != signature.params.len() {
                self.gen.c_src.src.push_str(", ");
            }
        }
        self.gen.c_src.src.push_str(")\n");
        if self.gen.opts.host {
            let signature = wamr::wamr_signature(self.resolve, func);
            let remember = HostFunction {
                wasm_name: func.name.clone(),
                wamr_signature: signature.to_string(),
                host_name: export_name.clone(),
            };
            self.gen
                .host_functions
                .entry(module_name)
                .and_modify(|v| v.push(remember.clone()))
                .or_insert(vec![remember]);
        }
        params
    }

    fn high_level_signature(
        &mut self,
        func: &Function,
        import: bool,
        from_namespace: &Vec<String>,
    ) -> HighlevelSignature {
        let mut res = HighlevelSignature::default();

        let (namespace, func_name_h) = self.func_namespace_name(func);
        res.name = func_name_h;
        res.namespace = namespace;
        let is_drop = is_drop_method(func);
        // we might want to separate c_sig and h_sig
        // let mut sig = String::new();
        if !matches!(&func.kind, FunctionKind::Constructor(_)) && !is_drop {
            match &func.results {
                wit_bindgen_core::wit_parser::Results::Named(n) => {
                    if n.is_empty() {
                        res.result = "void".into();
                    } else {
                        todo!();
                    }
                }
                wit_bindgen_core::wit_parser::Results::Anon(ty) => {
                    res.result = self.type_name(ty, from_namespace);
                }
            }
        }
        if matches!(func.kind, FunctionKind::Static(_)) && !is_drop {
            res.static_member = true;
        }
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i == 0 && name == "self" {
                res.implicit_self = true;
                continue;
            }
            res.arguments
                .push((name.to_snake_case(), self.type_name(param, &res.namespace)));
        }
        // default to non-const when exporting a method
        if matches!(func.kind, FunctionKind::Method(_)) && import {
            res.const_member = true;
        }
        res
    }

    fn print_signature(&mut self, func: &Function, import: bool) -> Vec<String> {
        let from_namespace = self.gen.h_src.namespace.clone();
        let cpp_sig = self.high_level_signature(func, import, &from_namespace);
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
        self.gen.h_src.src.push_str(";\n");
        drop(cpp_sig);

        // we want to separate the lowered signature (wasm) and the high level signature
        if !import {
            self.print_export_signature(func)
        } else {
            // recalulate with c file namespace
            let c_namespace = self.gen.c_src.namespace.clone();
            let cpp_sig = self.high_level_signature(func, import, &c_namespace);
            let mut params = Vec::new();
            self.gen.c_src.src.push_str(&cpp_sig.result);
            if !cpp_sig.result.is_empty() {
                self.gen.c_src.src.push_str(" ");
            }
            self.gen.c_src.qualify(&cpp_sig.namespace);
            self.gen.c_src.src.push_str(&cpp_sig.name);
            self.gen.c_src.src.push_str("(");
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

    fn generate_guest_import(&mut self, func: &Function, interface: InterfaceId) {
        let params = self.print_signature(func, !self.gen.opts.host);
        self.gen.c_src.src.push_str("{\n");
        let lift_lower = if self.gen.opts.host {
            LiftLower::LiftArgsLowerResults
        } else {
            LiftLower::LowerArgsLiftResults
        };
        if is_drop_method(func) {
            match lift_lower {
                LiftLower::LiftArgsLowerResults => {
                    let owner = &self.resolve.types[match &func.kind {
                        FunctionKind::Static(id) => *id,
                        _ => panic!("drop should be static"),
                    }];
                    self.gen.c_src.src.push_str("  ");
                    let mut namespace = namespace(self.resolve, &owner.owner);
                    namespace.push(owner.name.as_ref().unwrap().to_upper_camel_case());
                    self.gen.c_src.qualify(&namespace);
                    uwriteln!(self.gen.c_src.src, "remove_resource({});", params[0]);
                }
                LiftLower::LowerArgsLiftResults => {
                    let module_name = self.wasm_import_module.as_ref().map(|e| e.clone()).unwrap();
                    let name = self.declare_import(&module_name, &func.name, &[WasmType::I32], &[]);
                    uwriteln!(
                        self.gen.c_src.src,
                        "   if (handle>=0) {{
                                {name}(handle);
                            }}"
                    );
                }
            }
        } else {
            let namespace = if matches!(func.kind, FunctionKind::Freestanding) {
                namespace(self.resolve, &TypeOwner::Interface(interface))
            } else {
                let owner = &self.resolve.types[match &func.kind {
                    FunctionKind::Static(id) => *id,
                    FunctionKind::Constructor(id) => *id,
                    FunctionKind::Method(id) => *id,
                    FunctionKind::Freestanding => unreachable!(),
                }]
                .clone();
                let mut namespace = namespace(self.resolve, &owner.owner);
                namespace.push(owner.name.as_ref().unwrap().to_upper_camel_case());
                namespace
            };
            let mut f = FunctionBindgen::new(self, params);
            f.namespace = namespace;
            abi::call(
                f.gen.resolve,
                AbiVariant::GuestImport,
                lift_lower,
                func,
                &mut f,
            );
            let code = String::from(f.src);
            self.gen.c_src.src.push_str(&code);
        }
        self.gen.c_src.src.push_str("}\n");
    }

    pub fn type_path(&self, id: TypeId, owned: bool) -> String {
        self.type_path_with_name(
            id,
            if owned {
                self.result_name(id)
            } else {
                self.param_name(id)
            },
        )
    }

    fn type_path_with_name(&self, id: TypeId, name: String) -> String {
        if let TypeOwner::Interface(id) = self.resolve.types[id].owner {
            if let Some(path) = self.path_to_interface(id) {
                return format!("{path}::{name}");
            }
        }
        name
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let iface = &self.resolve.interfaces[interface];
        let name = iface.name.as_ref().unwrap();
        let mut full_path = String::new();
        full_path.push_str(name);
        Some(full_path)
    }

    fn param_name(&self, ty: TypeId) -> String {
        self.resolve.types[ty]
            .name
            .as_ref()
            .unwrap()
            .to_upper_camel_case()
    }

    fn result_name(&self, ty: TypeId) -> String {
        self.resolve.types[ty]
            .name
            .as_ref()
            .unwrap()
            .to_upper_camel_case()
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>, out: &mut String) {
        match ty {
            Some(ty) => self.push_ty_name(ty, out),
            None => out.push_str("void"),
        }
    }

    fn type_name(&mut self, ty: &Type, from_namespace: &Vec<String>) -> String {
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
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::String => {
                self.gen.dependencies.needs_string = true;
                "std::string".into()
            }
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Record(_r) => {
                    let ty = &self.resolve.types[*id];
                    let namespc = namespace(self.resolve, &ty.owner);
                    let mut relative = SourceWithState::default();
                    relative.namespace = from_namespace.clone();
                    relative.qualify(&namespc);
                    format!(
                        "{}{}",
                        relative.src.to_string(),
                        ty.name.as_ref().unwrap().to_pascal_case()
                    )
                }
                TypeDefKind::Resource => self.resolve.types[*id].name.as_ref().cloned().unwrap(),
                TypeDefKind::Handle(Handle::Own(id)) => {
                    self.type_name(&Type::Id(*id), from_namespace)
                }
                TypeDefKind::Handle(Handle::Borrow(id)) => {
                    "std::reference_wrapper<".to_string()
                        + &self.type_name(&Type::Id(*id), from_namespace)
                        + ">"
                }
                TypeDefKind::Flags(_) => "Flags".to_string(),
                TypeDefKind::Tuple(_) => "Tuple".to_string(),
                TypeDefKind::Variant(v) => {
                    let mut result = "std::variant<".to_string();
                    for (n, case) in v.cases.iter().enumerate() {
                        result += &case
                            .ty
                            .as_ref()
                            .map_or("void".to_string(), |ty| self.type_name(ty, from_namespace));
                        if n + 1 != v.cases.len() {
                            result += ", ";
                        }
                    }
                    result += ">";
                    result
                }
                TypeDefKind::Enum(_e) => {
                    let ty = &self.resolve.types[*id];
                    let namespc = namespace(self.resolve, &ty.owner);
                    let mut relative = SourceWithState::default();
                    relative.namespace = from_namespace.clone();
                    relative.qualify(&namespc);
                    format!(
                        "{}{}",
                        relative.src.to_string(),
                        ty.name.as_ref().unwrap().to_pascal_case()
                    )
                }
                TypeDefKind::Option(o) => {
                    "std::optional<".to_string() + &self.type_name(o, from_namespace) + ">"
                }
                TypeDefKind::Result(r) => {
                    "std::expected<".to_string()
                        + &r.ok
                            .as_ref()
                            .map_or("void".into(), |t| self.type_name(t, from_namespace))
                        + ", "
                        + &r.err
                            .as_ref()
                            .map_or("void".into(), |t| self.type_name(t, from_namespace))
                        + ">"
                }
                TypeDefKind::List(ty) => {
                    self.gen.dependencies.needs_vector = true;
                    "std::vector<".to_string() + &self.type_name(ty, from_namespace) + ">"
                }
                TypeDefKind::Future(_) => todo!(),
                TypeDefKind::Stream(_) => todo!(),
                TypeDefKind::Type(ty) => self.type_name(ty, from_namespace),
                TypeDefKind::Unknown => todo!(),
            },
        }
    }

    fn push_ty_name(&mut self, ty: &Type, out: &mut String) {
        wit_bindgen_c::push_ty_name(self.resolve, ty, out);
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

    fn declare_import(
        &mut self,
        module_name: &str,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        let mut args = String::default();
        for (n, param) in params.iter().enumerate() {
            args.push_str(wasm_type(*param));
            if n + 1 != params.len() {
                args.push_str(", ");
            }
        }
        let result = if results.is_empty() {
            "void"
        } else {
            wasm_type(results[0])
        };
        let (name, code) = Self::declare_import2(module_name, name, &args, result);
        self.gen.c_src.src.push_str(&code);
        name
    }

    fn docs(src: &mut Source, docs: &Docs) {
        if let Some(docs) = docs.contents.as_ref() {
            for line in docs.trim().lines() {
                src.push_str("// ");
                src.push_str(line);
                src.push_str("\n");
            }
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
        let namespc = namespace(self.resolve, &ty.owner);
        self.gen.h_src.change_namespace(&namespc);
        Self::docs(&mut self.gen.h_src.src, docs);
        let pascal = name.to_pascal_case();
        uwriteln!(self.gen.h_src.src, "struct {pascal} {{");
        for field in record.fields.iter() {
            Self::docs(&mut self.gen.h_src.src, &field.docs);
            let typename = self.type_name(&field.ty, &namespc);
            let fname = field.name.to_lower_camel_case();
            uwriteln!(self.gen.h_src.src, "{typename} {fname};");
        }
        uwriteln!(self.gen.h_src.src, "}};");
    }

    fn type_resource(
        &mut self,
        id: TypeId,
        name: &str,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let type_ = &self.resolve.types[id];
        if let TypeOwner::Interface(intf) = type_.owner {
            let import = self.gen.imported_interfaces.contains(&intf) ^ self.gen.opts.host;
            let mut world_name = self.gen.world.to_snake_case();
            world_name.push_str("::");
            let mut headerfile = SourceWithState::default();
            let namespc = namespace(self.resolve, &type_.owner);
            let pascal = name.to_upper_camel_case();
            let user_filename = namespc.join("-") + "-" + &pascal + ".h";
            if !import {
                // includes should be outside of namespaces
                self.gen.h_src.change_namespace(&Vec::default());
                // temporarily redirect header file declarations to an user controlled include file
                std::mem::swap(&mut headerfile, &mut self.gen.h_src);
                uwriteln!(
                    self.gen.h_src.src,
                    r#"/* User class definition file, autogenerated once, then user modified 
                    * Updated versions of this file are generated into {user_filename}.template.
                    */"#
                );
            }
            self.gen.h_src.change_namespace(&namespc);

            self.gen.dependencies.needs_resources = true;

            if !import {
                uwriteln!(self.gen.c_src.src, "template <class R> std::map<int32_t, R> {world_name}{RESOURCE_BASE_CLASS_NAME}<R>::resources;");
            }

            let base_type = if !import {
                format!("<{pascal}>")
            } else {
                String::default()
            };
            let derive = format!(" : public {world_name}{RESOURCE_BASE_CLASS_NAME}{base_type}");
            uwriteln!(self.gen.h_src.src, "class {pascal}{derive} {{\n");
            uwriteln!(self.gen.h_src.src, "public:\n");
            // destructor
            {
                let name = "[resource-drop]".to_string() + &name;
                let func = Function {
                    name: name,
                    kind: FunctionKind::Static(id),
                    params: vec![("self".into(), Type::Id(id))],
                    results: Results::Named(vec![]),
                    docs: Docs::default(),
                };
                self.generate_guest_import(&func, intf);
            }
            let funcs = self.resolve.interfaces[intf].functions.values();
            for func in funcs {
                self.generate_guest_import(func, intf);
            }

            if import {
                // consuming constructor from handle (bindings)
                uwriteln!(
                    self.gen.h_src.src,
                    "{pascal}({world_name}{RESOURCE_BASE_CLASS_NAME}&&);\n"
                );
                uwriteln!(self.gen.h_src.src, "{pascal}({pascal}&&) = default;\n");
            }
            uwriteln!(self.gen.h_src.src, "}};\n");
            if !import {
                // Finish the user controlled class template
                self.gen.h_src.change_namespace(&Vec::default());
                std::mem::swap(&mut headerfile, &mut self.gen.h_src);
                uwriteln!(self.gen.h_src.src, "#include \"{user_filename}\"");
                if self.gen.opts.format {
                    Cpp::clang_format(&mut headerfile.src);
                }
                self.gen
                    .user_class_files
                    .insert(user_filename + ".template", headerfile.src.to_string());
            }
        }
    }

    fn type_flags(
        &mut self,
        _id: TypeId,
        name: &str,
        _flags: &wit_bindgen_core::wit_parser::Flags,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        uwriteln!(self.gen.h_src.src, "// type_flags({name})");
    }

    fn type_tuple(
        &mut self,
        _id: TypeId,
        _name: &str,
        _flags: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_variant(
        &mut self,
        _id: TypeId,
        name: &str,
        _variant: &wit_bindgen_core::wit_parser::Variant,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        uwriteln!(self.gen.h_src.src, "// type_variant({name})");
    }

    fn type_option(
        &mut self,
        _id: TypeId,
        _name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_result(
        &mut self,
        _id: TypeId,
        _name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_enum(
        &mut self,
        id: TypeId,
        name: &str,
        enum_: &wit_bindgen_core::wit_parser::Enum,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let namespc = namespace(self.resolve, &ty.owner);
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

    fn type_alias(
        &mut self,
        id: TypeId,
        name: &str,
        alias_type: &wit_bindgen_core::wit_parser::Type,
        docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = &self.resolve.types[id];
        let namespc = namespace(self.resolve, &ty.owner);
        self.gen.h_src.change_namespace(&namespc);
        let pascal = name.to_pascal_case();
        Self::docs(&mut self.gen.h_src.src, docs);
        let typename = self.type_name(alias_type, &namespc);
        uwriteln!(self.gen.h_src.src, "using {pascal} = {typename};");
    }

    fn type_list(
        &mut self,
        _id: TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut CppInterfaceGenerator<'a>,
    params: Vec<String>,
    tmp: usize,
    // import_return_pointer_area_size: usize,
    // import_return_pointer_area_align: usize,
    namespace: Vec<String>,
    src: Source,
    block_storage: Vec<wit_bindgen_core::Source>,
    blocks: Vec<(String, Vec<String>)>,
    payloads: Vec<String>,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(gen: &'b mut CppInterfaceGenerator<'a>, params: Vec<String>) -> Self {
        Self {
            gen,
            params,
            tmp: 0,
            // import_return_pointer_area_size: 0,
            // import_return_pointer_area_align: 0,
            namespace: Default::default(),
            src: Default::default(),
            block_storage: Default::default(),
            blocks: Default::default(),
            payloads: Default::default(),
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

    fn typename_lift(&self, id: TypeId) -> String {
        self.gen.type_path(id, true)
    }

    fn let_results(&mut self, amt: usize, results: &mut Vec<String>) {
        match amt {
            0 => {}
            1 => {
                let tmp = self.tmp();
                let res = format!("result{}", tmp);
                self.push_str("auto ");
                self.push_str(&res);
                results.push(res);
                self.push_str(" = ");
            }
            _n => todo!(),
        }
    }

    fn load(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        results.push(format!("*(({}*) ({} + {}))", ty, operands[0], offset));
    }

    fn load_ext(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        self.load(ty, offset, operands, results);
        let result = results.pop().unwrap();
        results.push(format!("(int32_t) ({})", result));
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        uwriteln!(
            self.src,
            "*(({}*)({} + {})) = {};",
            ty,
            operands[1],
            offset,
            operands[0]
        );
    }

    fn has_resources(&self, id: &TypeId) -> bool {
        match &self.gen.resolve.types[*id].kind {
            TypeDefKind::Record(_) => todo!(),
            TypeDefKind::Resource => true,
            TypeDefKind::Handle(_) => true,
            TypeDefKind::Flags(_) => false,
            TypeDefKind::Tuple(_) => todo!(),
            TypeDefKind::Variant(_) => todo!(),
            TypeDefKind::Enum(_) => false,
            TypeDefKind::Option(_) => todo!(),
            TypeDefKind::Result(_) => todo!(),
            TypeDefKind::List(_) => todo!(),
            TypeDefKind::Future(_) => todo!(),
            TypeDefKind::Stream(_) => todo!(),
            TypeDefKind::Type(ty) => match ty {
                Type::Id(id) => self.has_resources(id),
                _ => false,
            },
            TypeDefKind::Unknown => todo!(),
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
            results.push(format!("({cvt}({}))", operands.pop().unwrap()));
        };

        match inst {
            abi::Instruction::GetArg { nth } => {
                if *nth == 0 && self.params[0].as_str() == "self" {
                    if self.gen.in_import ^ self.gen.gen.opts.host {
                        results.push("(*this)".to_string());
                    } else {
                        results.push("(*lookup_resource(self))".to_string());
                    }
                } else {
                    results.push(self.params[*nth].clone());
                }
            }
            abi::Instruction::I32Const { val } => results.push(format!("(int32_t({}))", val)),
            abi::Instruction::Bitcasts { casts: _ } => todo!(),
            abi::Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("int32_t(0)".to_string()),
                        WasmType::I64 => results.push("int64_t(0)".to_string()),
                        WasmType::F32 => results.push("0.0f".to_string()),
                        WasmType::F64 => results.push("0.0".to_string()),
                    }
                }
            }
            abi::Instruction::I32Load { offset } => {
                let tmp = self.tmp();
                uwriteln!(
                    self.src,
                    "int32_t l{tmp} = *((int32_t const*)({} + {offset}));",
                    operands[0]
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
            abi::Instruction::I32Store8 { offset } => self.store("int32_t", *offset, operands),
            abi::Instruction::I32Store16 { offset } => self.store("int32_t", *offset, operands),
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
            abi::Instruction::F32FromFloat32 => top_as("float"),
            abi::Instruction::F64FromFloat64 => top_as("double"),
            abi::Instruction::S8FromI32 => top_as("int8_t"),
            abi::Instruction::U8FromI32 => top_as("uint8_t"),
            abi::Instruction::S16FromI32 => top_as("int16_t"),
            abi::Instruction::U16FromI32 => top_as("uint16_t"),
            abi::Instruction::S32FromI32 => top_as("int32_t"),
            abi::Instruction::U32FromI32 => top_as("uint32_t"),
            abi::Instruction::S64FromI64 => top_as("int64_t"),
            abi::Instruction::U64FromI64 => top_as("uint64_t"),
            abi::Instruction::CharFromI32 => top_as("uint32_t"),
            abi::Instruction::Float32FromF32 => top_as("float"),
            abi::Instruction::Float64FromF64 => top_as("double"),
            abi::Instruction::BoolFromI32 => top_as("bool"),
            abi::Instruction::ListCanonLower {
                element: _,
                realloc: _,
            } => {
                results.push("ListCanonLower.addr".into());
                results.push("ListCanonLower.len".into());
            }
            abi::Instruction::StringLower { realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                let result = format!("result{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("auto const&{} = {};\n", val, operands[0]));
                    self.push_str(&format!("auto {} = (int32_t)({}.data());\n", ptr, val));
                    self.push_str(&format!("auto {} = (int32_t)({}.size());\n", len, val));
                    results.push(ptr);
                } else {
                    self.gen.gen.dependencies.needs_guest_alloc = true;
                    uwriteln!(self.src, "int32_t {result} = guest_alloc(exec_env, {len});");
                    uwriteln!(self.src, "memcpy(wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), {result}), {ptr}, {len});");
                    results.push(result);
                }
                results.push(len);
            }
            abi::Instruction::ListLower {
                element: _,
                realloc: _,
            } => {
                results.push("ListLower1".into());
                results.push("ListLower2".into());
            }
            abi::Instruction::ListCanonLift { element: _, ty: _ } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {};\n", len, operands[1]));
                let result = format!("std::vector<...>({0}, {0}+{1})", operands[0], len);
                results.push(result);
            }
            abi::Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                uwriteln!(self.src, "auto {} = {};\n", len, operands[1]);
                let result = format!("std::string((char const*)({}), {len})", operands[0]);
                results.push(result);
            }
            abi::Instruction::ListLift { element, ty: _ } => {
                // let body = self.blocks.pop().unwrap();
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
                self.push_str(&format!(
                    r#"auto mut {result} = std::vector<>();
                    {result}.reserve({len});
                    "#,
                ));

                uwriteln!(self.src, "for (unsigned i=0; i<{len}; ++i) {{");
                uwriteln!(self.src, "auto base = {base} + i * {size};");
                uwriteln!(self.src, "auto e{tmp} = todo();");
                uwriteln!(self.src, "{result}.push_back(e{tmp});");
                uwriteln!(self.src, "}}");
                results.push(result);
                // self.push_str(&format!(
                //     "{rt}::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                //     rt = self.gen.gen.runtime_path(),
                // ));
            }
            abi::Instruction::IterElem { element: _ } => results.push("IterElem".to_string()),
            abi::Instruction::IterBasePointer => results.push("base".to_string()),
            abi::Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for f in record.fields.iter() {
                    results.push(format!("({}).{}", op, to_c_ident(&f.name)));
                }
            }
            abi::Instruction::RecordLift {
                record,
                name: _,
                ty,
            } => {
                let mut result = self.typename_lift(*ty);
                result.push_str("{");
                for (_field, val) in record.fields.iter().zip(operands) {
                    result.push_str(&val);
                    result.push_str(", ");
                }
                result.push_str("}");
                results.push(result);
            }
            abi::Instruction::HandleLower {
                handle: Handle::Own(_ty),
                ..
            } => {
                let op = &operands[0];
                // let namespace = namespace(self.gen.resolve, &self.gen.resolve.types[*ty].owner);
                // let mut code = String::default();
                // for n in namespace {
                //     code.push_str(&n);
                //     code.push_str("::");
                // }
                results.push(format!("{op}.store_resource(std::move({op}))"));
            }
            abi::Instruction::HandleLower {
                handle: Handle::Borrow(_),
                ..
            } => {
                let op = &operands[0];
                results.push(format!("{op}.get_handle()"));
            }
            abi::Instruction::HandleLift {
                handle: _,
                name: _,
                ty: _,
            } => {
                let op = &operands[0];
                results.push(op.clone());
            }
            abi::Instruction::TupleLower { tuple: _, ty: _ } => {
                results.push("TupleLower1".into());
                results.push("TupleLower2".into());
            }
            abi::Instruction::TupleLift { tuple, ty: _ } => {
                let name = format!("tuple{}", self.tmp());
                uwrite!(self.src, "auto {name} std::tuple<");
                self.src.push_str(
                    &(tuple
                        .types
                        .iter()
                        .map(|t| self.gen.type_name(t, &self.namespace)))
                    .collect::<Vec<_>>()
                    .join(", "),
                );
                self.src.push_str(">(");
                self.src.push_str(&operands.join(", "));
                self.src.push_str(");\n");
                results.push(name);
            }
            abi::Instruction::FlagsLower {
                flags,
                name: _,
                ty: _,
            } => {
                let tmp = self.tmp();
                self.push_str(&format!("auto flags{} = {};\n", tmp, operands[0]));
                for i in 0..flags.repr().count() {
                    results.push(format!("((flags{} >> {})&1)!=0", tmp, i * 32));
                }
            }
            abi::Instruction::FlagsLift {
                flags: _,
                name: _,
                ty: _,
            } => results.push("FlagsLift".to_string()),
            abi::Instruction::VariantPayloadName => {
                let name = format!("payload{}", self.tmp());
                results.push(format!("*{}", name));
                self.payloads.push(name);
            }
            abi::Instruction::VariantLower {
                variant,
                name: _,
                ty: _,
                results: result_types,
            } => {
                //let name = self.gen.type_name(*ty);
                // let op0 = &operands[0];
                // self.push_str(&format!("({name}){op0}"));
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
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    variant_results.push(name);
                }

                let expr_to_match = format!("({}).tag", operands[0]);

                uwriteln!(self.src, "switch ((int32_t) {}) {{", expr_to_match);
                for (i, ((case, (block, block_results)), payload)) in
                    variant.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    uwriteln!(self.src, "case {}: {{", i);
                    if let Some(ty) = case.ty.as_ref() {
                        let ty = self.gen.type_name(ty, &self.namespace);
                        uwrite!(
                            self.src,
                            "const {} *{} = &({}).val",
                            ty,
                            payload,
                            operands[0],
                        );
                        self.src.push_str(".");
                        self.src.push_str(&to_c_ident(&case.name));
                        self.src.push_str(";\n");
                    }
                    self.src.push_str(&block);

                    for (name, result) in variant_results.iter().zip(&block_results) {
                        uwriteln!(self.src, "{} = {};", name, result);
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }
            abi::Instruction::VariantLift {
                variant,
                name: _,
                ty,
            } => {
                let mut result = String::new();
                result.push_str("{");

                let named_enum = variant.cases.iter().all(|c| c.ty.is_none());
                // let blocks = self
                //     .blocks
                //     .drain(self.blocks.len() - variant.cases.len()..)
                //     .collect::<Vec<_>>();
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
                let blocks: Vec<String> = Vec::new();
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
            abi::Instruction::EnumLower {
                enum_: _,
                name: _,
                ty: _,
            } => results.push(format!("int32_t({})", operands[0])),
            abi::Instruction::EnumLift {
                enum_: _,
                name: _,
                ty,
            } => {
                let typename = self.gen.type_name(&Type::Id(*ty), &self.namespace);
                results.push(format!("({typename}){}", &operands[0]));
            }
            abi::Instruction::OptionLower {
                payload: _,
                ty: _,
                results: _,
            } => self.push_str("OptionLower"),
            abi::Instruction::OptionLift { payload, ty: _ } => {
                let mut result: String = "std::optional<".into();
                result.push_str(&self.gen.type_name(*payload, &self.namespace));
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push(')');
                results.push(result);
            }
            abi::Instruction::ResultLower {
                result: _,
                ty: _,
                results: _,
            } => self.push_str("ResultLower"),
            abi::Instruction::ResultLift { result, ty: _ } => {
                let mut err = String::default(); //self.blocks.pop().unwrap();
                let mut ok = String::default(); //self.blocks.pop().unwrap();
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
                let mut ok_type = String::default();
                self.gen.print_optional_ty(result.ok.as_ref(), &mut ok_type);
                let mut err_type = String::default();
                self.gen
                    .print_optional_ty(result.err.as_ref(), &mut err_type);
                let type_name = format!("std::expected<{ok_type}, {err_type}>",);
                let err_type = "std::unexpected";
                let operand = &operands[0];
                results.push(format!(
                    "{operand}==0 \n? {type_name}({ok}) \n: {type_name}({err_type}({err}))"
                ));
            }
            abi::Instruction::CallWasm { name, sig } => {
                let module_name = self
                    .gen
                    .wasm_import_module
                    .as_ref()
                    .map(|e| e.clone())
                    .unwrap();
                let func = self
                    .gen
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
            abi::Instruction::CallInterface { func } => {
                // dbg!(func);
                self.let_results(func.results.len(), results);
                let (mut namespace, func_name_h) = self.gen.func_namespace_name(func);
                if matches!(func.kind, FunctionKind::Method(_)) {
                    let this = operands.remove(0);
                    self.gen.gen.c_src.qualify(&namespace);
                    uwrite!(self.src, "lookup_resource({this})->");
                } else {
                    if matches!(func.kind, FunctionKind::Constructor(_)) {
                        let _ = namespace.pop();
                    }
                    self.gen.gen.c_src.qualify(&namespace);
                }
                self.src.push_str(&func_name_h);
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
            }
            abi::Instruction::Return { amt, func } => {
                let import = !self.gen.gen.opts.host;
                match amt {
                    0 => {}
                    1 => {
                        match &func.kind {
                            FunctionKind::Constructor(_) if import => {
                                // strange but works
                                self.src.push_str("this->handle = ");
                            }
                            _ => self.src.push_str("return "),
                        }
                        self.src.push_str(&operands[0]);
                        self.src.push_str(";\n");
                    }
                    _ => todo!(),
                }
            }
            abi::Instruction::Malloc {
                realloc: _,
                size: _,
                align: _,
            } => todo!(),
            abi::Instruction::GuestDeallocate { size: _, align: _ } => todo!(),
            abi::Instruction::GuestDeallocateString => todo!(),
            abi::Instruction::GuestDeallocateList { element: _ } => todo!(),
            abi::Instruction::GuestDeallocateVariant { blocks: _ } => todo!(),
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> Self::Operand {
        let tmp = self.tmp();
        let elems = (size + (align - 1)) / align;
        let tp = match align {
            1 => "uint8_t",
            2 => "uint16_t",
            4 => "uint32_t",
            8 => "uint64_t",
            _ => todo!(),
        };
        uwriteln!(self.src, " {tp} ret_area[{elems}];");

        uwrite!(self.src, "int32_t ptr{tmp} = int32_t(&ret_area);");

        // Imports get a per-function return area to facilitate using the
        // stack whereas exports use a per-module return area to cut down on
        // stack usage. Note that for imports this also facilitates "adapter
        // modules" for components to not have data segments.
        // if self.gen.in_import {
        //     self.import_return_pointer_area_size = self.import_return_pointer_area_size.max(size);
        //     self.import_return_pointer_area_align =
        //         self.import_return_pointer_area_align.max(align);
        //     uwrite!(
        //         self.src,
        //         "int32_t ptr{tmp} = int32_t(&ret_area);"
        //     );
        // } else {
        //     self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
        //     self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
        //     uwriteln!(
        //         self.src,
        //         "int32_t ptr{tmp} = int32_t(&RET_AREA);"
        //     );
        // }
        format!("ptr{}", tmp)
    }

    fn push_block(&mut self) {
        let prev = core::mem::take(&mut self.src);
        self.block_storage.push(prev);
        //        uwriteln!(self.src, "// push_block()");
    }

    fn finish_block(&mut self, operands: &mut Vec<Self::Operand>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = core::mem::replace(&mut self.src, to_restore);
        self.blocks.push((src.into(), core::mem::take(operands)));
        //       uwriteln!(self.src, "// finish_block()");
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
            Type::Id(id) => !self.has_resources(id),
            _ => true,
        }
    }
}

// fn wasm_type(ty: WasmType) -> &'static str {
//     match ty {
//         WasmType::I32 => "int32_t",
//         WasmType::I64 => "int64_t",
//         WasmType::F32 => "float",
//         WasmType::F64 => "double",
//     }
// }

fn is_drop_method(func: &Function) -> bool {
    matches!(func.kind, FunctionKind::Static(_)) && func.name.starts_with("[resource-drop]")
}

fn is_arg_by_pointer(resolve: &Resolve, ty: &Type) -> bool {
    match ty {
        Type::Id(id) => match resolve.types[*id].kind {
            TypeDefKind::Type(t) => is_arg_by_pointer(resolve, &t),
            // this is different from C
            TypeDefKind::Resource => false,
            _ => wit_bindgen_c::is_arg_by_pointer(resolve, ty),
        },
        _ => wit_bindgen_c::is_arg_by_pointer(resolve, ty),
    }
}
