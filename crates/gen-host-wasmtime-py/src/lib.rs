//! Code generator for the `wasmtime` PyPI package.
//!
//! This crate will generate bindings for a single component, like JS, for
//! Python source code. Component-model types are translated to Python and the
//! component is executed using the `wasmtime` PyPI package which is bindings to
//! the `wasmtime` C API which is built on the `wasmtime` Rust API.
//!
//! The generated structure of the bindings looks like follows:
//!
//! ```ignore
//! out_dir/
//!     __init__.py
//!     types.py                # types shared by all imports/exports
//!     imports/                # only present if interfaces are imported
//!         __init__.py         # reexports `Foo` protocols for each interface
//!         foo.py              # types and definitions specific to interface `foo`
//!         ..
//!     exports/                # only present with exported interfaces
//!         __init__.py         # empty file
//!         bar.py              # contains `Bar` as the exported interface
//!         ..
//! ```
//!
//! The top-level `__init__.py` contains a `class Foo` where `Foo` is the name
//! fo the component. It contains top-level functions for all top-level exports
//! and exported instances are modeled as a method which returns a struct from
//! `exports/*.py`.

use heck::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write;
use std::mem;
use wasmtime_environ::component::{
    CanonicalOptions, Component, CoreDef, CoreExport, Export, ExportItem, GlobalInitializer,
    InstantiateModule, LowerImport, RuntimeInstanceIndex, StaticModuleIndex, StringEncoding,
};
use wasmtime_environ::{EntityIndex, ModuleTranslation, PrimaryMap};
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Ns, WorldGenerator,
};
use wit_component::ComponentInterfaces;

mod imports;
mod source;

use source::Source;

#[derive(Default)]
struct WasmtimePy {
    opts: Opts,

    // `$out_dir/__init__.py`
    init: Source,
    // `$out_dir/types.py`
    types: Source,
    // `$out_dir/intrinsics.py`
    intrinsics: Source,
    // `$out_dir/imports/__init__.py`
    imports_init: Source,
    // `$out_dir/exports/$name.py`
    exports: BTreeMap<String, Source>,

    /// Known imported interfaces to have as an argument to construction of the
    /// main component.
    imports: Vec<String>,

    /// All intrinsics emitted to `self.intrinsics` so far.
    all_intrinsics: BTreeSet<&'static str>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(self) -> Box<dyn ComponentGenerator> {
        let mut r = WasmtimePy::default();
        r.opts = self;
        Box::new(r)
    }
}

impl WasmtimePy {
    fn interface<'a>(&'a mut self, iface: &'a Interface, at_root: bool) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            gen: self,
            iface,
            src: Source::default(),
            at_root,
            self_module_path: "",
        }
    }

    fn print_result(&mut self) {
        if !self.all_intrinsics.insert("result_type") {
            return;
        }

        self.types.pyimport("dataclasses", "dataclass");
        self.types.pyimport("typing", "TypeVar");
        self.types.pyimport("typing", "Generic");
        self.types.pyimport("typing", "Union");
        self.types.push_str(
            "
                T = TypeVar('T')
                @dataclass
                class Ok(Generic[T]):
                    value: T
                E = TypeVar('E')
                @dataclass
                class Err(Generic[E]):
                    value: E

                Result = Union[Ok[T], Err[E]]
            ",
        );
    }
}

fn array_ty(iface: &Interface, ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Bool => None,
        Type::U8 => Some("c_uint8"),
        Type::S8 => Some("c_int8"),
        Type::U16 => Some("c_uint16"),
        Type::S16 => Some("c_int16"),
        Type::U32 => Some("c_uint32"),
        Type::S32 => Some("c_int32"),
        Type::U64 => Some("c_uint64"),
        Type::S64 => Some("c_int64"),
        Type::Float32 => Some("c_float"),
        Type::Float64 => Some("c_double"),
        Type::Char => None,
        Type::String => None,
        Type::Id(id) => match &iface.types[*id].kind {
            TypeDefKind::Type(t) => array_ty(iface, t),
            _ => None,
        },
    }
}

impl ComponentGenerator for WasmtimePy {
    fn instantiate(
        &mut self,
        name: &str,
        component: &Component,
        modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
        interfaces: &ComponentInterfaces,
    ) {
        self.init.pyimport("wasmtime", None);

        let camel = name.to_upper_camel_case();
        let imports = if !component.import_types.is_empty() {
            self.init
                .pyimport(".imports", format!("{camel}Imports").as_str());
            format!(", import_object: {camel}Imports")
        } else {
            String::new()
        };

        uwriteln!(self.init, "class {camel}:");
        self.init.indent();

        self.init.push_str("\n");

        uwriteln!(
            self.init,
            "def __init__(self, store: wasmtime.Store{imports}):"
        );
        self.init.indent();
        let mut i = Instantiator {
            name,
            gen: self,
            modules,
            component,
            interfaces,
            instances: PrimaryMap::default(),
            lifts: 0,
        };
        for init in component.initializers.iter() {
            i.global_initializer(init);
        }
        let (lifts, nested) = i.exports(&component.exports, interfaces.default.as_ref());
        i.gen.init.dedent();

        i.generate_lifts(&camel, None, &lifts);
        for (name, lifts) in nested {
            i.generate_lifts(&camel, Some(name), &lifts);
        }
        i.gen.init.dedent();
    }

    fn finish_component(&mut self, _name: &str, files: &mut Files) {
        if !self.imports_init.is_empty() {
            files.push("imports/__init__.py", self.imports_init.finish().as_bytes());
        }
        if !self.types.is_empty() {
            files.push("types.py", self.types.finish().as_bytes());
        }
        if !self.intrinsics.is_empty() {
            files.push("intrinsics.py", self.intrinsics.finish().as_bytes());
        }

        for (name, src) in self.exports.iter() {
            let snake = name.to_snake_case();
            files.push(&format!("exports/{snake}.py"), src.finish().as_bytes());
        }
        if !self.exports.is_empty() {
            files.push("exports/__init__.py", b"");
        }

        files.push("__init__.py", self.init.finish().as_bytes());
    }
}

struct Instantiator<'a> {
    name: &'a str,
    gen: &'a mut WasmtimePy,
    modules: &'a PrimaryMap<StaticModuleIndex, ModuleTranslation<'a>>,
    instances: PrimaryMap<RuntimeInstanceIndex, StaticModuleIndex>,
    interfaces: &'a ComponentInterfaces,
    component: &'a Component,
    lifts: usize,
}

struct Lift<'a> {
    callee: String,
    opts: &'a CanonicalOptions,
    iface: &'a Interface,
    func: &'a Function,
}

impl<'a> Instantiator<'a> {
    fn global_initializer(&mut self, init: &GlobalInitializer) {
        match init {
            GlobalInitializer::InstantiateModule(m) => match m {
                InstantiateModule::Static(idx, args) => self.instantiate_static_module(*idx, args),

                // This is only needed when instantiating an imported core wasm
                // module which while easy to implement here is not possible to
                // test at this time so it's left unimplemented.
                InstantiateModule::Import(..) => unimplemented!(),
            },

            GlobalInitializer::LowerImport(i) => self.lower_import(i),

            GlobalInitializer::ExtractMemory(m) => {
                let def = self.core_export(&m.export);
                let i = m.index.as_u32();
                uwriteln!(self.gen.init, "core_memory{i} = {def}");
                uwriteln!(
                    self.gen.init,
                    "assert(isinstance(core_memory{i}, wasmtime.Memory))"
                );
                uwriteln!(self.gen.init, "self._core_memory{i} = core_memory{i}",);
            }
            GlobalInitializer::ExtractRealloc(r) => {
                let def = self.core_def(&r.def);
                let i = r.index.as_u32();
                uwriteln!(self.gen.init, "realloc{i} = {def}");
                uwriteln!(
                    self.gen.init,
                    "assert(isinstance(realloc{i}, wasmtime.Func))"
                );
                uwriteln!(self.gen.init, "self._realloc{i} = realloc{i}",);
            }
            GlobalInitializer::ExtractPostReturn(p) => {
                let def = self.core_def(&p.def);
                let i = p.index.as_u32();
                uwriteln!(self.gen.init, "post_return{i} = {def}");
                uwriteln!(
                    self.gen.init,
                    "assert(isinstance(post_return{i}, wasmtime.Func))"
                );
                uwriteln!(self.gen.init, "self._post_return{i} = post_return{i}",);
            }

            // This is only used for a "degenerate component" which internally
            // has a function that always traps. While this should be trivial to
            // implement (generate a JS function that always throws) there's no
            // way to test this at this time so leave this unimplemented.
            GlobalInitializer::AlwaysTrap(_) => unimplemented!(),

            // This is only used when the component exports core wasm modules,
            // but that's not possible to test right now so leave these as
            // unimplemented.
            GlobalInitializer::SaveStaticModule(_) => unimplemented!(),
            GlobalInitializer::SaveModuleImport(_) => unimplemented!(),

            // This is required when strings pass between components within a
            // component and may change encodings. This is left unimplemented
            // for now since it can't be tested and additionally JS doesn't
            // support multi-memory which transcoders rely on anyway.
            GlobalInitializer::Transcoder(_) => unimplemented!(),
        }
    }

    fn instantiate_static_module(&mut self, idx: StaticModuleIndex, args: &[CoreDef]) {
        let i = self.instances.push(idx);
        let core_file_name = self.gen.core_file_name(&self.name, idx.as_u32());
        self.gen.init.pyimport("os", None);

        uwriteln!(
            self.gen.init,
            "path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '{}')",
            core_file_name,
        );
        uwriteln!(
            self.gen.init,
            "module = wasmtime.Module.from_file(store.engine, path)"
        );
        uwrite!(
            self.gen.init,
            "instance{} = wasmtime.Instance(store, module, [",
            i.as_u32()
        );
        if !args.is_empty() {
            self.gen.init.push_str("\n");
            self.gen.init.indent();
            for arg in args {
                let def = self.core_def(arg);
                uwriteln!(self.gen.init, "{def},");
            }
            self.gen.init.dedent();
        }
        uwriteln!(self.gen.init, "]).exports(store)");
    }

    fn lower_import(&mut self, import: &LowerImport) {
        // Determine the `Interface` that this import corresponds to. At this
        // time `wit-component` only supports root-level imports of instances
        // where instances export functions.
        let (import_index, path) = &self.component.imports[import.import];
        let (import_name, _import_ty) = &self.component.import_types[*import_index];
        assert_eq!(path.len(), 1);
        let iface = &self.interfaces.imports[import_name.as_str()];
        let func = iface.functions.iter().find(|f| f.name == path[0]).unwrap();

        let index = import.index.as_u32();
        let callee = format!(
            "import_object.{}.{}",
            import_name.to_snake_case(),
            func.name.to_snake_case()
        );

        // Generate an inline function "closure" which will capture the
        // `imports` argument provided to the constructor of this class and have
        // the core wasm signature for this function. Using prior local
        // variables the function here will perform all liftings/lowerings.
        uwrite!(
            self.gen.init,
            "def lowering{index}_callee(caller: wasmtime.Caller"
        );
        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        let mut params = Vec::new();
        for (i, param_ty) in sig.params.iter().enumerate() {
            self.gen.init.push_str(", ");
            let param = format!("arg{i}");
            uwrite!(self.gen.init, "{param}: {}", wasm_ty_typing(*param_ty));
            params.push(param);
        }
        self.gen.init.push_str(") -> ");
        match sig.results.len() {
            0 => self.gen.init.push_str("None"),
            1 => self.gen.init.push_str(wasm_ty_typing(sig.results[0])),
            _ => unimplemented!(),
        }
        self.gen.init.push_str(":\n");
        self.gen.init.indent();

        let iface_snake = iface.name.to_snake_case();
        self.gen.init.pyimport(".imports", iface_snake.as_str());
        let self_module_path = format!("{iface_snake}.");

        self.bindgen(
            params,
            callee,
            &import.options,
            iface,
            func,
            AbiVariant::GuestImport,
            "self",
            self_module_path,
            true,
        );
        self.gen.init.dedent();

        // Use the `wasmtime` package's embedder methods of creating a wasm
        // function to finish the construction here.
        uwrite!(self.gen.init, "lowering{index}_ty = wasmtime.FuncType([");
        for param in sig.params.iter() {
            self.gen.init.push_str(wasm_ty_ctor(*param));
            self.gen.init.push_str(", ");
        }
        self.gen.init.push_str("], [");
        for param in sig.results.iter() {
            self.gen.init.push_str(wasm_ty_ctor(*param));
            self.gen.init.push_str(", ");
        }
        self.gen.init.push_str("])\n");
        uwriteln!(
            self.gen.init,
            "lowering{index} = wasmtime.Func(store, lowering{index}_ty, lowering{index}_callee, access_caller = True)"
        );
    }

    fn bindgen(
        &mut self,
        params: Vec<String>,
        callee: String,
        opts: &CanonicalOptions,
        iface: &Interface,
        func: &Function,
        abi: AbiVariant,
        this: &str,
        self_module_path: String,
        at_root: bool,
    ) {
        // Technically it wouldn't be the hardest thing in the world to support
        // other string encodings, but for now the code generator was originally
        // written to support utf-8 so let's just leave it at that for now. In
        // the future when it's easier to produce components with non-utf-8 this
        // can be plumbed through to string lifting/lowering below.
        assert_eq!(opts.string_encoding, StringEncoding::Utf8);

        let memory = match opts.memory {
            Some(idx) => Some(format!("{this}._core_memory{}", idx.as_u32())),
            None => None,
        };
        let realloc = match opts.realloc {
            Some(idx) => Some(format!("{this}._realloc{}", idx.as_u32())),
            None => None,
        };
        let post_return = match opts.post_return {
            Some(idx) => Some(format!("{this}._post_return{}", idx.as_u32())),
            None => None,
        };

        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        let mut locals = Ns::default();
        locals.insert("len").unwrap(); // python built-in
        locals.insert("base").unwrap(); // may be used as loop var
        locals.insert("i").unwrap(); // may be used as loop var
        let mut f = FunctionBindgen {
            locals,
            payloads: Vec::new(),
            sizes,
            // Generate source directly onto `init`
            src: mem::take(&mut self.gen.init),
            gen: self.gen,
            block_storage: Vec::new(),
            blocks: Vec::new(),
            callee,
            memory,
            realloc,
            params,
            post_return,
            iface,
            self_module_path,
            at_root,
        };
        iface.call(
            abi,
            match abi {
                AbiVariant::GuestImport => LiftLower::LiftArgsLowerResults,
                AbiVariant::GuestExport => LiftLower::LowerArgsLiftResults,
            },
            func,
            &mut f,
        );

        // Swap the printed source back into the destination of our `init`, and
        // at this time `f.src` should be empty.
        mem::swap(&mut f.src, &mut f.gen.init);
        assert!(f.src.is_empty());
    }

    fn core_def(&self, def: &CoreDef) -> String {
        match def {
            CoreDef::Export(e) => self.core_export(e),
            CoreDef::Lowered(i) => format!("lowering{}", i.as_u32()),
            CoreDef::AlwaysTrap(_) => unimplemented!(),
            CoreDef::InstanceFlags(_) => unimplemented!(),
            CoreDef::Transcoder(_) => unimplemented!(),
        }
    }

    fn core_export<T>(&self, export: &CoreExport<T>) -> String
    where
        T: Into<EntityIndex> + Copy,
    {
        let name = match &export.item {
            ExportItem::Index(idx) => {
                let module = &self.modules[self.instances[export.instance]].module;
                let idx = (*idx).into();
                module
                    .exports
                    .iter()
                    .filter_map(|(name, i)| if *i == idx { Some(name) } else { None })
                    .next()
                    .unwrap()
            }
            ExportItem::Name(s) => s,
        };
        let i = export.instance.as_u32() as usize;
        format!("instance{i}[\"{name}\"]")
    }

    /// Extract the `LiftedFunction` exports to a format that's easier to
    /// process for this generator. For now all lifted functions are either
    /// "root" lifted functions or one-level-nested for an exported interface.
    ///
    /// As worlds shape up and more of a component's structure is expressible in
    /// `*.wit` this method will likely need to change.
    fn exports(
        &mut self,
        exports: &'a IndexMap<String, Export>,
        iface: Option<&'a Interface>,
    ) -> (Vec<Lift<'a>>, BTreeMap<&'a str, Vec<Lift<'a>>>) {
        let mut toplevel = Vec::new();
        let mut nested = BTreeMap::new();
        for (name, export) in exports {
            let name = name.as_str();
            match export {
                Export::LiftedFunction {
                    ty: _,
                    func,
                    options,
                } => {
                    // For each lifted function the callee `wasmtime.Func` is
                    // saved into a per-instance field which is then referenced
                    // as the callee when the relevant function is invoked.
                    let def = self.core_def(func);
                    let callee = format!("lift_callee{}", self.lifts);
                    self.lifts += 1;
                    uwriteln!(self.gen.init, "{callee} = {def}");
                    uwriteln!(self.gen.init, "assert(isinstance({callee}, wasmtime.Func))");
                    uwriteln!(self.gen.init, "self.{callee} = {callee}");
                    let iface = iface.unwrap();
                    let func = iface.functions.iter().find(|f| f.name == *name).unwrap();
                    toplevel.push(Lift {
                        callee,
                        opts: options,
                        iface,
                        func,
                    });
                }

                Export::Instance(exports) => {
                    let iface = &self.interfaces.exports[name];
                    let (my_toplevel, my_nested) = self.exports(exports, Some(iface));
                    // More than one level of nesting not supported at this
                    // time.
                    assert!(my_nested.is_empty());

                    let prev = nested.insert(name, my_toplevel);
                    assert!(prev.is_none());
                }

                // ignore type exports for now
                Export::Type(_) => {}

                // This can't be tested at this time so leave it unimplemented
                Export::Module(_) => unimplemented!(),
            }
        }
        (toplevel, nested)
    }

    fn generate_lifts(&mut self, camel_component: &str, ns: Option<&str>, lifts: &[Lift<'_>]) {
        let mut this = "self".to_string();

        // If these exports are going into a non-default interface then a new
        // `class` is generated in the corresponding file which will be
        // constructed with the "root" class. Generate the class here, its one
        // field of the root class, and then an associated constructor for the
        // root class to have. Finally the root class grows a method here as
        // well to return the nested instance.
        if let Some(ns) = ns {
            let src = self.gen.exports.get_mut(ns).unwrap();
            let camel = ns.to_upper_camel_case();
            let snake = ns.to_snake_case();
            uwriteln!(src, "class {camel}:");
            src.indent();
            src.pyimport("..", camel_component);
            uwriteln!(src, "component: {camel_component}\n");
            uwriteln!(
                src,
                "def __init__(self, component: {camel_component}) -> None:"
            );
            src.indent();
            uwriteln!(src, "self.component = component");
            src.dedent();

            this.push_str(".component");

            self.gen.init.pyimport(".exports", snake.as_str());
            uwriteln!(self.gen.init, "def {snake}(self) -> {snake}.{camel}:");
            self.gen.init.indent();
            uwriteln!(self.gen.init, "return {snake}.{camel}(self)");
            self.gen.init.dedent();

            // Swap the two sources so the generation into `init` will go into
            // the right place
            mem::swap(&mut self.gen.init, src);
        }

        for lift in lifts {
            // Go through some small gymnastics to print the function signature
            // here.
            let mut src = mem::take(&mut self.gen.init);
            let params = with_igen(&mut src, self.gen, lift.iface, ns.is_none(), "", |gen| {
                gen.print_sig(lift.func, false)
            });
            self.gen.init = src;
            self.gen.init.push_str(":\n");

            // Defer to `self.bindgen` for the body of the function.
            self.gen.init.indent();
            self.gen.init.docstring(&lift.func.docs);
            self.bindgen(
                params,
                format!("{this}.{}", lift.callee),
                lift.opts,
                lift.iface,
                lift.func,
                AbiVariant::GuestExport,
                &this,
                String::new(),
                ns.is_none(),
            );
            self.gen.init.dedent();
        }

        // Undo the swap done above.
        if let Some(ns) = ns {
            self.gen.init.dedent();
            mem::swap(&mut self.gen.init, self.gen.exports.get_mut(ns).unwrap());
        }
    }
}

impl WorldGenerator for WasmtimePy {
    fn import(&mut self, name: &str, iface: &Interface, files: &mut Files) {
        let mut gen = self.interface(iface, false);
        gen.types();

        // Generate a "protocol" class which I'm led to believe is the rough
        // equivalent of a Rust trait in Python for this imported interface.
        // This will be referenced in the constructor for the main component.
        let camel = name.to_upper_camel_case();
        let snake = name.to_snake_case();
        gen.src.pyimport("typing", "Protocol");
        uwriteln!(gen.src, "class {camel}(Protocol):");
        gen.src.indent();
        for func in iface.functions.iter() {
            gen.src.pyimport("abc", "abstractmethod");
            gen.src.push_str("@abstractmethod\n");
            gen.print_sig(func, true);
            gen.src.push_str(":\n");
            gen.src.indent();
            gen.src.push_str("raise NotImplementedError\n");
            gen.src.dedent();
        }
        gen.src.dedent();
        gen.src.push_str("\n");

        let src = gen.src.finish();
        files.push(&format!("imports/{snake}.py"), src.as_bytes());
        self.imports.push(name.to_string());
    }

    fn export(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = self.interface(iface, false);
        gen.types();

        // Only generate types for exports and this will get finished later on
        // as lifted functions need to be inserted into these files as they're
        // discovered.
        let src = gen.src;
        self.exports.insert(name.to_string(), src);
    }

    fn export_default(&mut self, _name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = self.interface(iface, true);

        // Generate types and imports directly into `__init__.py` for the
        // default export, and exported functions (lifted functions) will get
        // generate later.
        mem::swap(&mut gen.src, &mut gen.gen.init);
        gen.types();
        mem::swap(&mut gen.src, &mut gen.gen.init);
    }

    fn finish(&mut self, name: &str, _interfaces: &ComponentInterfaces, _files: &mut Files) {
        if !self.imports.is_empty() {
            let camel = name.to_upper_camel_case();
            self.imports_init.pyimport("dataclasses", "dataclass");
            uwriteln!(self.imports_init, "@dataclass");
            uwriteln!(self.imports_init, "class {camel}Imports:");
            self.imports_init.indent();
            for import in self.imports.iter() {
                let snake = import.to_snake_case();
                let camel = import.to_upper_camel_case();
                self.imports_init
                    .pyimport(&format!(".{snake}"), camel.as_str());
                uwriteln!(self.imports_init, "{snake}: {camel}");
            }
            self.imports_init.dedent();
        }
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut WasmtimePy,
    iface: &'a Interface,
    at_root: bool,
    self_module_path: &'a str,
}

#[derive(Debug, Clone, Copy)]
enum PyUnionRepresentation {
    /// A union whose inner types are used directly
    Raw,
    /// A union whose inner types have been wrapped in dataclasses
    Wrapped,
}

impl InterfaceGenerator<'_> {
    fn import_result_type(&mut self) {
        self.gen.print_result();
        let path = if self.at_root { ".types" } else { "..types" };
        self.src.pyimport(path, "Result");
    }

    fn print_ty(&mut self, ty: &Type, forward_ref: bool) {
        match ty {
            Type::Bool => self.src.push_str("bool"),
            Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::U64
            | Type::S64 => self.src.push_str("int"),
            Type::Float32 | Type::Float64 => self.src.push_str("float"),
            Type::Char => self.src.push_str("str"),
            Type::String => self.src.push_str("str"),
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    self.src.push_str(self.self_module_path);
                    self.src.push_str(&name.to_upper_camel_case());
                    return;
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(t, forward_ref),
                    TypeDefKind::Tuple(t) => self.print_tuple(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unreachable!()
                    }
                    TypeDefKind::Option(t) => {
                        self.src.pyimport("typing", "Optional");
                        self.src.push_str("Optional[");
                        self.print_ty(t, true);
                        self.src.push_str("]");
                    }
                    TypeDefKind::Result(r) => {
                        self.import_result_type();
                        self.src.push_str("Result[");
                        self.print_optional_ty(r.ok.as_ref(), true);
                        self.src.push_str(", ");
                        self.print_optional_ty(r.err.as_ref(), true);
                        self.src.push_str("]");
                    }
                    TypeDefKind::List(t) => self.print_list(t),
                    TypeDefKind::Future(t) => {
                        self.src.push_str("Future[");
                        self.print_optional_ty(t.as_ref(), true);
                        self.src.push_str("]");
                    }
                    TypeDefKind::Stream(s) => {
                        self.src.push_str("Stream[");
                        self.print_optional_ty(s.element.as_ref(), true);
                        self.src.push_str(", ");
                        self.print_optional_ty(s.end.as_ref(), true);
                        self.src.push_str("]");
                    }
                }
            }
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>, forward_ref: bool) {
        match ty {
            Some(ty) => self.print_ty(ty, forward_ref),
            None => self.src.push_str("None"),
        }
    }

    fn print_tuple(&mut self, tuple: &Tuple) {
        if tuple.types.is_empty() {
            return self.src.push_str("None");
        }
        self.src.pyimport("typing", "Tuple");
        self.src.push_str("Tuple[");
        for (i, t) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.src.push_str(", ");
            }
            self.print_ty(t, true);
        }
        self.src.push_str("]");
    }

    fn print_list(&mut self, element: &Type) {
        match element {
            Type::U8 => self.src.push_str("bytes"),
            t => {
                self.src.pyimport("typing", "List");
                self.src.push_str("List[");
                self.print_ty(t, true);
                self.src.push_str("]");
            }
        }
    }

    fn print_sig(&mut self, func: &Function, in_import: bool) -> Vec<String> {
        self.src.push_str("def ");
        self.src.push_str(&func.name.to_snake_case());
        if in_import {
            self.src.push_str("(self");
        } else {
            self.src.pyimport("wasmtime", None);
            self.src.push_str("(self, caller: wasmtime.Store");
        }
        let mut params = Vec::new();
        for (param, ty) in func.params.iter() {
            self.src.push_str(", ");
            self.src.push_str(&param.to_snake_case());
            params.push(param.to_snake_case());
            self.src.push_str(": ");
            self.print_ty(ty, true);
        }
        self.src.push_str(") -> ");
        match func.results.len() {
            0 => self.src.push_str("None"),
            1 => self.print_ty(func.results.iter_types().next().unwrap(), true),
            _ => {
                self.src.pyimport("typing", "Tuple");
                self.src.push_str("Tuple[");
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.print_ty(ty, true);
                }
                self.src.push_str("]");
            }
        }
        params
    }

    /// Print a wrapped union definition.
    /// e.g.
    /// ```py
    /// @dataclass
    /// class Foo0:
    ///     value: int
    ///
    /// @dataclass
    /// class Foo1:
    ///     value: int
    ///
    /// Foo = Union[Foo0, Foo1]
    /// ```
    pub fn print_union_wrapped(&mut self, name: &str, union: &Union, docs: &Docs) {
        self.src.pyimport("dataclasses", "dataclass");
        let mut cases = Vec::new();
        let name = name.to_upper_camel_case();
        for (i, case) in union.cases.iter().enumerate() {
            self.src.push_str("@dataclass\n");
            let name = format!("{name}{i}");
            self.src.push_str(&format!("class {name}:\n"));
            self.src.indent();
            self.src.docstring(&case.docs);
            self.src.push_str("value: ");
            self.print_ty(&case.ty, true);
            self.src.newline();
            self.src.dedent();
            self.src.newline();
            cases.push(name);
        }

        self.src.pyimport("typing", "Union");
        self.src.comment(docs);
        self.src
            .push_str(&format!("{name} = Union[{}]\n", cases.join(", ")));
        self.src.newline();
    }

    pub fn print_union_raw(&mut self, name: &str, union: &Union, docs: &Docs) {
        self.src.pyimport("typing", "Union");
        self.src.comment(docs);
        for case in union.cases.iter() {
            self.src.comment(&case.docs);
        }
        self.src.push_str(&name.to_upper_camel_case());
        self.src.push_str(" = Union[");
        let mut first = true;
        for case in union.cases.iter() {
            if !first {
                self.src.push_str(",");
            }
            self.print_ty(&case.ty, true);
            first = false;
        }
        self.src.push_str("]\n\n");
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.src.pyimport("dataclasses", "dataclass");
        self.src.push_str("@dataclass\n");
        self.src
            .push_str(&format!("class {}:\n", name.to_upper_camel_case()));
        self.src.indent();
        self.src.docstring(docs);
        for field in record.fields.iter() {
            self.src.comment(&field.docs);
            let field_name = field.name.to_snake_case();
            self.src.push_str(&format!("{field_name}: "));
            self.print_ty(&field.ty, true);
            self.src.push_str("\n");
        }
        if record.fields.is_empty() {
            self.src.push_str("pass\n");
        }
        self.src.dedent();
        self.src.push_str("\n");
    }

    fn type_tuple(&mut self, _id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.src.comment(docs);
        self.src
            .push_str(&format!("{} = ", name.to_upper_camel_case()));
        self.print_tuple(tuple);
        self.src.push_str("\n");
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.src.pyimport("enum", "Flag");
        self.src.pyimport("enum", "auto");
        self.src
            .push_str(&format!("class {}(Flag):\n", name.to_upper_camel_case()));
        self.src.indent();
        self.src.docstring(docs);
        for flag in flags.flags.iter() {
            let flag_name = flag.name.to_shouty_snake_case();
            self.src.comment(&flag.docs);
            self.src.push_str(&format!("{flag_name} = auto()\n"));
        }
        if flags.flags.is_empty() {
            self.src.push_str("pass\n");
        }
        self.src.dedent();
        self.src.push_str("\n");
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.src.pyimport("dataclasses", "dataclass");
        let mut cases = Vec::new();
        for case in variant.cases.iter() {
            self.src.docstring(&case.docs);
            self.src.push_str("@dataclass\n");
            let case_name = format!(
                "{}{}",
                name.to_upper_camel_case(),
                case.name.to_upper_camel_case()
            );
            self.src.push_str(&format!("class {case_name}:\n"));
            self.src.indent();
            match &case.ty {
                Some(ty) => {
                    self.src.push_str("value: ");
                    self.print_ty(ty, true);
                }
                None => self.src.push_str("pass"),
            }
            self.src.push_str("\n");
            self.src.dedent();
            self.src.push_str("\n");
            cases.push(case_name);
        }

        self.src.pyimport("typing", "Union");
        self.src.comment(docs);
        self.src.push_str(&format!(
            "{} = Union[{}]\n",
            name.to_upper_camel_case(),
            cases.join(", "),
        ));
        self.src.push_str("\n");
    }

    /// Appends a Python definition for the provided Union to the current `Source`.
    /// e.g. `MyUnion = Union[float, str, int]`
    fn type_union(&mut self, _id: TypeId, name: &str, union: &Union, docs: &Docs) {
        match classify_union(union.cases.iter().map(|t| t.ty)) {
            PyUnionRepresentation::Wrapped => {
                self.print_union_wrapped(name, union, docs);
            }
            PyUnionRepresentation::Raw => {
                self.print_union_raw(name, union, docs);
            }
        }
    }

    fn type_option(&mut self, _id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        self.src.pyimport("typing", "Optional");
        self.src.comment(docs);
        self.src.push_str(&name.to_upper_camel_case());
        self.src.push_str(" = Optional[");
        self.print_ty(payload, true);
        self.src.push_str("]\n\n");
    }

    fn type_result(&mut self, _id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.import_result_type();

        self.src.comment(docs);
        self.src
            .push_str(&format!("{} = Result[", name.to_upper_camel_case()));
        self.print_optional_ty(result.ok.as_ref(), true);
        self.src.push_str(", ");
        self.print_optional_ty(result.err.as_ref(), true);
        self.src.push_str("]\n\n");
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.src.pyimport("enum", "Enum");
        self.src
            .push_str(&format!("class {}(Enum):\n", name.to_upper_camel_case()));
        self.src.indent();
        self.src.docstring(docs);
        for (i, case) in enum_.cases.iter().enumerate() {
            self.src.comment(&case.docs);

            // TODO this handling of digits should be more general and
            // shouldn't be here just to fix the one case in wasi where an
            // enum variant is "2big" and doesn't generate valid Python. We
            // should probably apply this to all generated Python
            // identifiers.
            let mut name = case.name.to_shouty_snake_case();
            if name.chars().next().unwrap().is_digit(10) {
                name = format!("_{}", name);
            }
            self.src.push_str(&format!("{} = {}\n", name, i));
        }
        self.src.dedent();
        self.src.push_str("\n");
    }

    fn type_alias(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.src.comment(docs);
        self.src
            .push_str(&format!("{} = ", name.to_upper_camel_case()));
        self.print_ty(ty, false);
        self.src.push_str("\n");
    }

    fn type_list(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.src.comment(docs);
        self.src
            .push_str(&format!("{} = ", name.to_upper_camel_case()));
        self.print_list(ty);
        self.src.push_str("\n");
    }

    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(id, name, ty, docs);
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut WasmtimePy,
    iface: &'a Interface,
    locals: Ns,
    src: Source,
    block_storage: Vec<source::Body>,
    blocks: Vec<(String, Vec<String>)>,
    params: Vec<String>,
    payloads: Vec<String>,
    sizes: SizeAlign,

    memory: Option<String>,
    realloc: Option<String>,
    post_return: Option<String>,
    callee: String,
    self_module_path: String,
    at_root: bool,
}

impl FunctionBindgen<'_> {
    fn clamp<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        let clamp = self.print_clamp();
        results.push(format!("{clamp}({}, {min}, {max})", operands[0]));
    }

    fn load(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        let load = self.print_load();
        let memory = self.memory.as_ref().unwrap();
        let tmp = self.locals.tmp("load");
        self.src.pyimport("ctypes", None);
        uwriteln!(
            self.src,
            "{tmp} = {load}(ctypes.{ty}, {memory}, caller, {}, {offset})",
            operands[0],
        );
        results.push(tmp);
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        let store = self.print_store();
        let memory = self.memory.as_ref().unwrap();
        self.src.pyimport("ctypes", None);
        uwriteln!(
            self.src,
            "{store}(ctypes.{ty}, {memory}, caller, {}, {offset}, {})",
            operands[1],
            operands[0]
        );
    }

    fn print_ty(&mut self, ty: &Type) {
        with_igen(
            &mut self.src,
            self.gen,
            self.iface,
            self.at_root,
            &self.self_module_path,
            |gen| gen.print_ty(ty, false),
        )
    }

    fn print_list(&mut self, element: &Type) {
        with_igen(
            &mut self.src,
            self.gen,
            self.iface,
            self.at_root,
            &self.self_module_path,
            |gen| gen.print_list(element),
        )
    }

    fn print_intrinsic(
        &mut self,
        name: &'static str,
        gen: impl FnOnce(&str, &mut Source),
    ) -> &'static str {
        let path = if self.at_root {
            ".intrinsics"
        } else {
            "..intrinsics"
        };
        self.src.pyimport(path, name);
        if !self.gen.all_intrinsics.insert(name) {
            return name;
        }
        gen(name, &mut self.gen.intrinsics);
        return name;
    }

    fn print_validate_guest_char(&mut self) -> &'static str {
        self.print_intrinsic("_validate_guest_char", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: int) -> str:
                        if i > 0x10ffff or (i >= 0xd800 and i <= 0xdfff):
                            raise TypeError('not a valid char')
                        return chr(i)
                ",
            );
        })
    }

    fn print_i32_to_f32(&mut self) -> &'static str {
        self.print_i32_to_f32_cvts();
        self.print_intrinsic("_i32_to_f32", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: int) -> float:
                        _i32_to_f32_i32[0] = i     # type: ignore
                        return _i32_to_f32_f32[0]  # type: ignore
                ",
            );
        })
    }

    fn print_f32_to_i32(&mut self) -> &'static str {
        self.print_i32_to_f32_cvts();
        self.print_intrinsic("_f32_to_i32", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: float) -> int:
                        _i32_to_f32_f32[0] = i    # type: ignore
                        return _i32_to_f32_i32[0] # type: ignore
                ",
            );
        })
    }

    fn print_i32_to_f32_cvts(&mut self) {
        if !self.gen.all_intrinsics.insert("i32_to_f32_cvts") {
            return;
        }
        self.gen.intrinsics.pyimport("ctypes", None);
        self.gen
            .intrinsics
            .push_str("_i32_to_f32_i32 = ctypes.pointer(ctypes.c_int32(0))\n");
        self.gen.intrinsics.push_str(
            "_i32_to_f32_f32 = ctypes.cast(_i32_to_f32_i32, ctypes.POINTER(ctypes.c_float))\n",
        );
    }

    fn print_i64_to_f64(&mut self) -> &'static str {
        self.print_i64_to_f64_cvts();
        self.print_intrinsic("_i64_to_f64", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: int) -> float:
                        _i64_to_f64_i64[0] = i     # type: ignore
                        return _i64_to_f64_f64[0]  # type: ignore
                ",
            );
        })
    }

    fn print_f64_to_i64(&mut self) -> &'static str {
        self.print_i64_to_f64_cvts();
        self.print_intrinsic("_f64_to_i64", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: float) -> int:
                        _i64_to_f64_f64[0] = i    # type: ignore
                        return _i64_to_f64_i64[0] # type: ignore
                ",
            );
        })
    }

    fn print_i64_to_f64_cvts(&mut self) {
        if !self.gen.all_intrinsics.insert("i64_to_f64_cvts") {
            return;
        }
        self.gen.intrinsics.pyimport("ctypes", None);
        self.gen
            .intrinsics
            .push_str("_i64_to_f64_i64 = ctypes.pointer(ctypes.c_int64(0))\n");
        self.gen.intrinsics.push_str(
            "_i64_to_f64_f64 = ctypes.cast(_i64_to_f64_i64, ctypes.POINTER(ctypes.c_double))\n",
        );
    }

    fn print_clamp(&mut self) -> &'static str {
        self.print_intrinsic("_clamp", |name, src| {
            uwriteln!(
                src,
                "
                    def {name}(i: int, min: int, max: int) -> int:
                        if i < min or i > max:
                            raise OverflowError(f'must be between {{min}} and {{max}}')
                        return i
                ",
            );
        })
    }

    fn print_load(&mut self) -> &'static str {
        self.print_intrinsic("_load", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "Any");
            uwriteln!(
                src,
                "
                    def {name}(ty: Any, mem: wasmtime.Memory, store: wasmtime.Storelike, base: int, offset: int) -> Any:
                        ptr = (base & 0xffffffff) + offset
                        if ptr + ctypes.sizeof(ty) > mem.data_len(store):
                            raise IndexError('out-of-bounds store')
                        raw_base = mem.data_ptr(store)
                        c_ptr = ctypes.POINTER(ty)(
                            ty.from_address(ctypes.addressof(raw_base.contents) + ptr)
                        )
                        return c_ptr[0]
                ",
            );
        })
    }

    fn print_store(&mut self) -> &'static str {
        self.print_intrinsic("_store", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "Any");
            uwriteln!(
                src,
                "
                    def {name}(ty: Any, mem: wasmtime.Memory, store: wasmtime.Storelike, base: int, offset: int, val: Any) -> None:
                        ptr = (base & 0xffffffff) + offset
                        if ptr + ctypes.sizeof(ty) > mem.data_len(store):
                            raise IndexError('out-of-bounds store')
                        raw_base = mem.data_ptr(store)
                        c_ptr = ctypes.POINTER(ty)(
                            ty.from_address(ctypes.addressof(raw_base.contents) + ptr)
                        )
                        c_ptr[0] = val
                ",
            );
        })
    }

    fn print_decode_utf8(&mut self) -> &'static str {
        self.print_intrinsic("_decode_utf8", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "Tuple");
            uwriteln!(
                src,
                "
                    def {name}(mem: wasmtime.Memory, store: wasmtime.Storelike, ptr: int, len: int) -> str:
                        ptr = ptr & 0xffffffff
                        len = len & 0xffffffff
                        if ptr + len > mem.data_len(store):
                            raise IndexError('string out of bounds')
                        base = mem.data_ptr(store)
                        base = ctypes.POINTER(ctypes.c_ubyte)(
                            ctypes.c_ubyte.from_address(ctypes.addressof(base.contents) + ptr)
                        )
                        return ctypes.string_at(base, len).decode('utf-8')
                ",
            );
        })
    }

    fn print_encode_utf8(&mut self) -> &'static str {
        self.print_intrinsic("_encode_utf8", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "Tuple");
            uwriteln!(
                src,
                "
                    def {name}(val: str, realloc: wasmtime.Func, mem: wasmtime.Memory, store: wasmtime.Storelike) -> Tuple[int, int]:
                        bytes = val.encode('utf8')
                        ptr = realloc(store, 0, 0, 1, len(bytes))
                        assert(isinstance(ptr, int))
                        ptr = ptr & 0xffffffff
                        if ptr + len(bytes) > mem.data_len(store):
                            raise IndexError('string out of bounds')
                        base = mem.data_ptr(store)
                        base = ctypes.POINTER(ctypes.c_ubyte)(
                            ctypes.c_ubyte.from_address(ctypes.addressof(base.contents) + ptr)
                        )
                        ctypes.memmove(base, bytes, len(bytes))
                        return (ptr, len(bytes))
                ",
            );
        })
    }

    fn print_canon_lift(&mut self) -> &'static str {
        self.print_intrinsic("_list_canon_lift", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "List");
            src.pyimport("typing", "Any");
            // TODO: this is doing a native-endian read, not a little-endian
            // read
            uwriteln!(
                src,
                "
                    def {name}(ptr: int, len: int, size: int, ty: Any, mem: wasmtime.Memory ,store: wasmtime.Storelike) -> Any:
                        ptr = ptr & 0xffffffff
                        len = len & 0xffffffff
                        if ptr + len * size > mem.data_len(store):
                            raise IndexError('list out of bounds')
                        raw_base = mem.data_ptr(store)
                        base = ctypes.POINTER(ty)(
                            ty.from_address(ctypes.addressof(raw_base.contents) + ptr)
                        )
                        if ty == ctypes.c_uint8:
                            return ctypes.string_at(base, len)
                        return base[:len]
                ",
            );
        })
    }

    fn print_canon_lower(&mut self) -> &'static str {
        self.print_intrinsic("_list_canon_lower", |name, src| {
            src.pyimport("wasmtime", None);
            src.pyimport("ctypes", None);
            src.pyimport("typing", "Tuple");
            src.pyimport("typing", "List");
            src.pyimport("typing", "Any");
            // TODO: is there a faster way to memcpy other than iterating over
            // the input list?
            // TODO: this is doing a native-endian write, not a little-endian
            // write
            uwriteln!(
                src,
                "
                    def {name}(list: Any, ty: Any, size: int, align: int, realloc: wasmtime.Func, mem: wasmtime.Memory, store: wasmtime.Storelike) -> Tuple[int, int]:
                        total_size = size * len(list)
                        ptr = realloc(store, 0, 0, align, total_size)
                        assert(isinstance(ptr, int))
                        ptr = ptr & 0xffffffff
                        if ptr + total_size > mem.data_len(store):
                            raise IndexError('list realloc return of bounds')
                        raw_base = mem.data_ptr(store)
                        base = ctypes.POINTER(ty)(
                            ty.from_address(ctypes.addressof(raw_base.contents) + ptr)
                        )
                        for i, val in enumerate(list):
                            base[i] = val
                        return (ptr, len(list))
                ",
            );
        })
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.sizes
    }

    fn push_block(&mut self) {
        self.block_storage.push(self.src.take_body());
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = self.src.replace_body(to_restore);
        self.blocks.push((src, mem::take(operands)));
    }

    fn return_pointer(&mut self, _iface: &Interface, _size: usize, _align: usize) -> String {
        unimplemented!()
    }

    fn is_list_canonical(&self, iface: &Interface, ty: &Type) -> bool {
        array_ty(iface, ty).is_some()
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::ConstZero { tys } => {
                for t in tys.iter() {
                    match t {
                        WasmType::I32 | WasmType::I64 => results.push("0".to_string()),
                        WasmType::F32 | WasmType::F64 => results.push("0.0".to_string()),
                    }
                }
            }

            // The representation of i32 in Python is a number, so 8/16-bit
            // values get further clamped to ensure that the upper bits aren't
            // set when we pass the value, ensuring that only the right number
            // of bits are transferred.
            Instruction::U8FromI32 => self.clamp(results, operands, u8::MIN, u8::MAX),
            Instruction::S8FromI32 => self.clamp(results, operands, i8::MIN, i8::MAX),
            Instruction::U16FromI32 => self.clamp(results, operands, u16::MIN, u16::MAX),
            Instruction::S16FromI32 => self.clamp(results, operands, i16::MIN, i16::MAX),
            // Ensure the bits of the number are treated as unsigned.
            Instruction::U32FromI32 => {
                results.push(format!("{} & 0xffffffff", operands[0]));
            }
            // All bigints coming from wasm are treated as signed, so convert
            // it to ensure it's treated as unsigned.
            Instruction::U64FromI64 => {
                results.push(format!("{} & 0xffffffffffffffff", operands[0]));
            }
            // Nothing to do signed->signed where the representations are the
            // same.
            Instruction::S32FromI32 | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap())
            }

            // All values coming from the host and going to wasm need to have
            // their ranges validated, since the host could give us any value.
            Instruction::I32FromU8 => self.clamp(results, operands, u8::MIN, u8::MAX),
            Instruction::I32FromS8 => self.clamp(results, operands, i8::MIN, i8::MAX),
            Instruction::I32FromU16 => self.clamp(results, operands, u16::MIN, u16::MAX),
            Instruction::I32FromS16 => self.clamp(results, operands, i16::MIN, i16::MAX),
            // TODO: need to do something to get this to be represented as signed?
            Instruction::I32FromU32 => {
                self.clamp(results, operands, u32::MIN, u32::MAX);
            }
            Instruction::I32FromS32 => self.clamp(results, operands, i32::MIN, i32::MAX),
            // TODO: need to do something to get this to be represented as signed?
            Instruction::I64FromU64 => self.clamp(results, operands, u64::MIN, u64::MAX),
            Instruction::I64FromS64 => self.clamp(results, operands, i64::MIN, i64::MAX),

            // Python uses `float` for f32/f64, so everything is equivalent
            // here.
            Instruction::Float32FromF32
            | Instruction::Float64FromF64
            | Instruction::F32FromFloat32
            | Instruction::F64FromFloat64 => results.push(operands.pop().unwrap()),

            // Validate that i32 values coming from wasm are indeed valid code
            // points.
            Instruction::CharFromI32 => {
                let validate = self.print_validate_guest_char();
                results.push(format!("{validate}({})", operands[0]));
            }

            Instruction::I32FromChar => {
                results.push(format!("ord({})", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    match cast {
                        Bitcast::I32ToF32 => {
                            let cvt = self.print_i32_to_f32();
                            results.push(format!("{cvt}({})", op));
                        }
                        Bitcast::F32ToI32 => {
                            let cvt = self.print_f32_to_i32();
                            results.push(format!("{cvt}({})", op));
                        }
                        Bitcast::I64ToF64 => {
                            let cvt = self.print_i64_to_f64();
                            results.push(format!("{cvt}({})", op));
                        }
                        Bitcast::F64ToI64 => {
                            let cvt = self.print_f64_to_i64();
                            results.push(format!("{cvt}({})", op));
                        }
                        Bitcast::I64ToF32 => {
                            let cvt = self.print_i32_to_f32();
                            results.push(format!("{cvt}(({}) & 0xffffffff)", op));
                        }
                        Bitcast::F32ToI64 => {
                            let cvt = self.print_f32_to_i32();
                            results.push(format!("{cvt}({})", op));
                        }
                        Bitcast::I32ToI64 | Bitcast::I64ToI32 | Bitcast::None => {
                            results.push(op.clone())
                        }
                    }
                }
            }

            Instruction::BoolFromI32 => {
                let op = self.locals.tmp("operand");
                let ret = self.locals.tmp("boolean");

                uwriteln!(self.src, "{op} = {}", operands[0]);
                uwriteln!(self.src, "if {op} == 0:");
                self.src.indent();
                uwriteln!(self.src, "{ret} = False");
                self.src.dedent();
                uwriteln!(self.src, "elif {op} == 1:");
                self.src.indent();
                uwriteln!(self.src, "{ret} = True");
                self.src.dedent();
                uwriteln!(self.src, "else:");
                self.src.indent();
                uwriteln!(
                    self.src,
                    "raise TypeError(\"invalid variant discriminant for bool\")"
                );
                self.src.dedent();
                results.push(ret);
            }
            Instruction::I32FromBool => {
                results.push(format!("int({})", operands[0]));
            }

            Instruction::RecordLower { record, .. } => {
                if record.fields.is_empty() {
                    return;
                }
                let tmp = self.locals.tmp("record");
                uwriteln!(self.src, "{tmp} = {}", operands[0]);
                for field in record.fields.iter() {
                    let name = self.locals.tmp("field");
                    uwriteln!(self.src, "{name} = {tmp}.{}", field.name.to_snake_case(),);
                    results.push(name);
                }
            }

            Instruction::RecordLift { name, .. } => {
                results.push(format!(
                    "{}{}({})",
                    self.self_module_path,
                    name.to_upper_camel_case(),
                    operands.join(", ")
                ));
            }
            Instruction::TupleLower { tuple, .. } => {
                if tuple.types.is_empty() {
                    return;
                }
                self.src.push_str("(");
                for _ in 0..tuple.types.len() {
                    let name = self.locals.tmp("tuplei");
                    uwrite!(self.src, "{name},");
                    results.push(name);
                }
                uwriteln!(self.src, ") = {}", operands[0]);
            }
            Instruction::TupleLift { .. } => {
                if operands.is_empty() {
                    results.push("None".to_string());
                } else {
                    results.push(format!("({},)", operands.join(", ")));
                }
            }
            Instruction::FlagsLift { name, .. } => {
                let operand = match operands.len() {
                    1 => operands[0].clone(),
                    _ => {
                        let tmp = self.locals.tmp("bits");
                        uwriteln!(self.src, "{tmp} = 0");
                        for (i, op) in operands.iter().enumerate() {
                            let i = 32 * i;
                            uwriteln!(self.src, "{tmp} |= {op} << {i}\n");
                        }
                        tmp
                    }
                };
                results.push(format!(
                    "{}{}({operand})",
                    self.self_module_path,
                    name.to_upper_camel_case()
                ));
            }
            Instruction::FlagsLower { flags, .. } => match flags.repr().count() {
                1 => results.push(format!("({}).value", operands[0])),
                n => {
                    let tmp = self.locals.tmp("bits");
                    self.src
                        .push_str(&format!("{tmp} = ({}).value\n", operands[0]));
                    for i in 0..n {
                        let i = 32 * i;
                        results.push(format!("({tmp} >> {i}) & 0xffffffff"));
                    }
                }
            },

            Instruction::VariantPayloadName => {
                let name = self.locals.tmp("payload");
                results.push(name.clone());
                self.payloads.push(name);
            }

            Instruction::VariantLower {
                variant,
                results: result_types,
                name,
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

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                for (i, ((case, (block, block_results)), payload)) in
                    variant.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    if i == 0 {
                        self.src.push_str("if ");
                    } else {
                        self.src.push_str("elif ");
                    }

                    uwriteln!(
                        self.src,
                        "isinstance({}, {}{}{}):",
                        operands[0],
                        self.self_module_path,
                        name.to_upper_camel_case(),
                        case.name.to_upper_camel_case()
                    );
                    self.src.indent();
                    if case.ty.is_some() {
                        uwriteln!(self.src, "{payload} = {}.value", operands[0]);
                    }
                    self.src.push_str(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        uwriteln!(self.src, "{} = {result}", results[i]);
                    }
                    self.src.dedent();
                }
                let variant_name = name.to_upper_camel_case();
                self.src.push_str("else:\n");
                self.src.indent();
                uwriteln!(
                    self.src,
                    "raise TypeError(\"invalid variant specified for {variant_name}\")",
                );
                self.src.dedent();
            }

            Instruction::VariantLift {
                variant, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let result = self.locals.tmp("variant");
                uwrite!(self.src, "{result}: ");
                self.print_ty(&Type::Id(*ty));
                self.src.push_str("\n");
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    if i == 0 {
                        self.src.push_str("if ");
                    } else {
                        self.src.push_str("elif ");
                    }
                    uwriteln!(self.src, "{} == {i}:", operands[0]);
                    self.src.indent();
                    self.src.push_str(&block);

                    uwrite!(
                        self.src,
                        "{result} = {}{}{}(",
                        self.self_module_path,
                        name.to_upper_camel_case(),
                        case.name.to_upper_camel_case()
                    );
                    if block_results.len() > 0 {
                        assert!(block_results.len() == 1);
                        self.src.push_str(&block_results[0]);
                    }
                    self.src.push_str(")\n");
                    self.src.dedent();
                }
                self.src.push_str("else:\n");
                self.src.indent();
                let variant_name = name.to_upper_camel_case();
                uwriteln!(
                    self.src,
                    "raise TypeError(\"invalid variant discriminant for {variant_name}\")",
                );
                self.src.dedent();
                results.push(result);
            }

            Instruction::UnionLower {
                union,
                results: result_types,
                name,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                let payloads = self
                    .payloads
                    .drain(self.payloads.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                let union_representation = classify_union(union.cases.iter().map(|c| c.ty));
                let name = name.to_upper_camel_case();
                let op0 = &operands[0];
                for (i, ((case, (block, block_results)), payload)) in
                    union.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    self.src.push_str(if i == 0 { "if " } else { "elif " });
                    uwrite!(self.src, "isinstance({op0}, ");
                    match union_representation {
                        // Prints the Python type for this union case
                        PyUnionRepresentation::Raw => self.print_ty(&case.ty),
                        // Prints the name of this union cases dataclass
                        PyUnionRepresentation::Wrapped => {
                            uwrite!(self.src, "{}{name}{i}", self.self_module_path);
                        }
                    }
                    uwriteln!(self.src, "):");
                    self.src.indent();
                    match union_representation {
                        // Uses the value directly
                        PyUnionRepresentation::Raw => {
                            uwriteln!(self.src, "{payload} = {op0}")
                        }
                        // Uses this union case dataclass's inner value
                        PyUnionRepresentation::Wrapped => {
                            uwriteln!(self.src, "{payload} = {op0}.value")
                        }
                    }
                    self.src.push_str(&block);
                    for (i, result) in block_results.iter().enumerate() {
                        uwriteln!(self.src, "{} = {result}", results[i]);
                    }
                    self.src.dedent();
                }
                self.src.push_str("else:\n");
                self.src.indent();
                uwriteln!(
                    self.src,
                    "raise TypeError(\"invalid variant specified for {name}\")"
                );
                self.src.dedent();
            }

            Instruction::UnionLift {
                union, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let result = self.locals.tmp("variant");
                uwrite!(self.src, "{result}: ");
                self.print_ty(&Type::Id(*ty));
                self.src.push_str("\n");
                let union_representation = classify_union(union.cases.iter().map(|c| c.ty));
                let name = name.to_upper_camel_case();
                let op0 = &operands[0];
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    self.src.push_str(if i == 0 { "if " } else { "elif " });
                    uwriteln!(self.src, "{op0} == {i}:");
                    self.src.indent();
                    self.src.push_str(&block);
                    assert!(block_results.len() == 1);
                    let block_result = &block_results[0];
                    uwrite!(self.src, "{result} = ");
                    match union_representation {
                        // Uses the passed value directly
                        PyUnionRepresentation::Raw => self.src.push_str(block_result),
                        // Constructs an instance of the union cases dataclass
                        PyUnionRepresentation::Wrapped => {
                            uwrite!(
                                self.src,
                                "{}{name}{i}({block_result})",
                                self.self_module_path
                            )
                        }
                    }
                    self.src.newline();
                    self.src.dedent();
                }
                self.src.push_str("else:\n");
                self.src.indent();
                uwriteln!(
                    self.src,
                    "raise TypeError(\"invalid variant discriminant for {name}\")\n",
                );
                self.src.dedent();
                results.push(result);
            }

            Instruction::OptionLower {
                results: result_types,
                ..
            } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let _none_payload = self.payloads.pop().unwrap();

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                let op0 = &operands[0];
                uwriteln!(self.src, "if {op0} is None:");

                self.src.indent();
                self.src.push_str(&none);
                for (dst, result) in results.iter().zip(&none_results) {
                    uwriteln!(self.src, "{dst} = {result}");
                }
                self.src.dedent();
                self.src.push_str("else:\n");
                self.src.indent();
                uwriteln!(self.src, "{some_payload} = {op0}");
                self.src.push_str(&some);
                for (dst, result) in results.iter().zip(&some_results) {
                    uwriteln!(self.src, "{dst} = {result}");
                }
                self.src.dedent();
            }

            Instruction::OptionLift { ty, .. } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.len() == 0);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];

                let result = self.locals.tmp("option");
                uwrite!(self.src, "{result}: ");
                self.print_ty(&Type::Id(*ty));
                self.src.push_str("\n");

                let op0 = &operands[0];
                uwriteln!(self.src, "if {op0} == 0:");
                self.src.indent();
                self.src.push_str(&none);
                uwriteln!(self.src, "{result} = None");
                self.src.dedent();
                uwriteln!(self.src, "elif {op0} == 1:");
                self.src.indent();
                self.src.push_str(&some);
                uwriteln!(self.src, "{result} = {some_result}");
                self.src.dedent();

                self.src.push_str("else:\n");
                self.src.indent();
                self.src
                    .push_str("raise TypeError(\"invalid variant discriminant for option\")\n");
                self.src.dedent();

                results.push(result);
            }

            Instruction::ResultLower {
                results: result_types,
                ..
            } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let err_payload = self.payloads.pop().unwrap();
                let ok_payload = self.payloads.pop().unwrap();
                let path = if self.at_root { ".types" } else { "..types" };
                self.src.pyimport(path, "Ok");
                self.src.pyimport(path, "Err");

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                let op0 = &operands[0];
                uwriteln!(self.src, "if isinstance({op0}, Ok):");

                self.src.indent();
                uwriteln!(self.src, "{ok_payload} = {op0}.value");
                self.src.push_str(&ok);
                for (dst, result) in results.iter().zip(&ok_results) {
                    uwriteln!(self.src, "{dst} = {result}");
                }
                self.src.dedent();
                uwriteln!(self.src, "elif isinstance({op0}, Err):");
                self.src.indent();
                uwriteln!(self.src, "{err_payload} = {op0}.value");
                self.src.push_str(&err);
                for (dst, result) in results.iter().zip(&err_results) {
                    uwriteln!(self.src, "{dst} = {result}");
                }
                self.src.dedent();
                self.src.push_str("else:\n");
                self.src.indent();
                self.src.push_str(&format!(
                    "raise TypeError(\"invalid variant specified for expected\")\n",
                ));
                self.src.dedent();
            }

            Instruction::ResultLift { ty, .. } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let none = String::from("None");
                let err_result = err_results.get(0).unwrap_or(&none);
                let ok_result = ok_results.get(0).unwrap_or(&none);

                let path = if self.at_root { ".types" } else { "..types" };
                self.src.pyimport(path, "Ok");
                self.src.pyimport(path, "Err");

                let result = self.locals.tmp("expected");
                uwrite!(self.src, "{result}: ");
                self.print_ty(&Type::Id(*ty));
                self.src.push_str("\n");

                let op0 = &operands[0];
                uwriteln!(self.src, "if {op0} == 0:");
                self.src.indent();
                self.src.push_str(&ok);
                uwriteln!(self.src, "{result} = Ok({ok_result})");
                self.src.dedent();
                uwriteln!(self.src, "elif {op0} == 1:");
                self.src.indent();
                self.src.push_str(&err);
                uwriteln!(self.src, "{result} = Err({err_result})");
                self.src.dedent();

                self.src.push_str("else:\n");
                self.src.indent();
                self.src
                    .push_str("raise TypeError(\"invalid variant discriminant for expected\")\n");
                self.src.dedent();

                results.push(result);
            }

            Instruction::EnumLower { .. } => results.push(format!("({}).value", operands[0])),

            Instruction::EnumLift { name, .. } => {
                results.push(format!(
                    "{}{}({})",
                    self.self_module_path,
                    name.to_upper_camel_case(),
                    operands[0]
                ));
            }

            Instruction::ListCanonLower { element, .. } => {
                let lower = self.print_canon_lower();
                let realloc = self.realloc.as_ref().unwrap();
                let memory = self.memory.as_ref().unwrap();

                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                let array_ty = array_ty(iface, element).unwrap();
                let size = self.sizes.size(element);
                let align = self.sizes.align(element);
                uwriteln!(
                    self.src,
                    "{ptr}, {len} = {lower}({}, ctypes.{array_ty}, {size}, {align}, {realloc}, {memory}, caller)",
                    operands[0],
                );
                results.push(ptr);
                results.push(len);
            }
            Instruction::ListCanonLift { element, .. } => {
                let lift = self.print_canon_lift();
                let memory = self.memory.as_ref().unwrap();
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                uwriteln!(self.src, "{ptr} = {}", operands[0]);
                uwriteln!(self.src, "{len} = {}", operands[1]);
                let array_ty = array_ty(iface, element).unwrap();
                self.src.pyimport("ctypes", None);
                let lift = format!(
                    "{lift}({ptr}, {len}, {}, ctypes.{array_ty}, {memory}, caller)",
                    self.sizes.size(element),
                );
                self.src.pyimport("typing", "cast");
                let list = self.locals.tmp("list");
                uwrite!(self.src, "{list} = cast(");
                self.print_list(element);
                uwriteln!(self.src, ", {lift})");
                results.push(list);
            }
            Instruction::StringLower { .. } => {
                let encode = self.print_encode_utf8();
                let realloc = self.realloc.as_ref().unwrap();
                let memory = self.memory.as_ref().unwrap();

                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                uwriteln!(
                    self.src,
                    "{ptr}, {len} = {encode}({}, {realloc}, {memory}, caller)",
                    operands[0],
                );
                results.push(ptr);
                results.push(len);
            }
            Instruction::StringLift => {
                let decode = self.print_decode_utf8();
                let memory = self.memory.as_ref().unwrap();
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                uwriteln!(self.src, "{ptr} = {}", operands[0]);
                uwriteln!(self.src, "{len} = {}", operands[1]);
                let list = self.locals.tmp("list");
                uwriteln!(
                    self.src,
                    "{list} = {decode}({memory}, caller, {ptr}, {len})"
                );
                results.push(list);
            }

            Instruction::ListLower { element, .. } => {
                let base = self.payloads.pop().unwrap();
                let e = self.payloads.pop().unwrap();
                let realloc = self.realloc.as_ref().unwrap();
                let (body, body_results) = self.blocks.pop().unwrap();
                assert!(body_results.is_empty());
                let vec = self.locals.tmp("vec");
                let result = self.locals.tmp("result");
                let len = self.locals.tmp("len");
                let size = self.sizes.size(element);
                let align = self.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                uwriteln!(self.src, "{vec} = {}", operands[0]);
                uwriteln!(self.src, "{len} = len({vec})");

                // ... then realloc space for the result in the guest module
                uwriteln!(
                    self.src,
                    "{result} = {realloc}(caller, 0, 0, {align}, {len} * {size})",
                );
                uwriteln!(self.src, "assert(isinstance({result}, int))");

                // ... then consume the vector and use the block to lower the
                // result.
                let i = self.locals.tmp("i");
                uwriteln!(self.src, "for {i} in range(0, {len}):");
                self.src.indent();
                uwriteln!(self.src, "{e} = {vec}[{i}]");
                uwriteln!(self.src, "{base} = {result} + {i} * {size}");
                self.src.push_str(&body);
                self.src.dedent();

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                let base = self.payloads.pop().unwrap();
                let size = self.sizes.size(element);
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                uwriteln!(self.src, "{ptr} = {}", operands[0]);
                uwriteln!(self.src, "{len} = {}", operands[1]);
                let result = self.locals.tmp("result");
                uwrite!(self.src, "{result}: ");
                self.print_list(element);
                uwriteln!(self.src, " = []");

                let i = self.locals.tmp("i");
                assert_eq!(body_results.len(), 1);
                let body_result0 = &body_results[0];

                uwriteln!(self.src, "for {i} in range(0, {len}):");
                self.src.indent();
                uwriteln!(self.src, "{base} = {ptr} + {i} * {size}");
                self.src.push_str(&body);
                uwriteln!(self.src, "{result}.append({body_result0})");
                self.src.dedent();
                results.push(result);
            }

            Instruction::IterElem { .. } => {
                let name = self.locals.tmp("e");
                results.push(name.clone());
                self.payloads.push(name);
            }
            Instruction::IterBasePointer => {
                let name = self.locals.tmp("base");
                results.push(name.clone());
                self.payloads.push(name);
            }
            Instruction::CallWasm { sig, .. } => {
                if sig.results.len() > 0 {
                    for i in 0..sig.results.len() {
                        if i > 0 {
                            self.src.push_str(", ");
                        }
                        let ret = self.locals.tmp("ret");
                        self.src.push_str(&ret);
                        results.push(ret);
                    }
                    self.src.push_str(" = ");
                }
                self.src.push_str(&self.callee);
                self.src.push_str("(caller");
                if operands.len() > 0 {
                    self.src.push_str(", ");
                }
                self.src.push_str(&operands.join(", "));
                self.src.push_str(")\n");
                for (ty, name) in sig.results.iter().zip(results.iter()) {
                    let ty = match ty {
                        WasmType::I32 | WasmType::I64 => "int",
                        WasmType::F32 | WasmType::F64 => "float",
                    };
                    self.src
                        .push_str(&format!("assert(isinstance({}, {}))\n", name, ty));
                }
            }
            Instruction::CallInterface { func } => {
                for i in 0..func.results.len() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    let result = self.locals.tmp("ret");
                    self.src.push_str(&result);
                    results.push(result);
                }
                if func.results.len() > 0 {
                    self.src.push_str(" = ");
                }
                match &func.kind {
                    FunctionKind::Freestanding => {
                        self.src
                            .push_str(&format!("{}({})", self.callee, operands.join(", "),));
                    }
                }
                self.src.push_str("\n");
            }

            Instruction::Return { amt, .. } => {
                if let Some(s) = &self.post_return {
                    self.src.push_str(&format!("{s}(caller, ret)\n"));
                }
                match amt {
                    0 => {}
                    1 => self.src.push_str(&format!("return {}\n", operands[0])),
                    _ => {
                        self.src
                            .push_str(&format!("return ({})\n", operands.join(", ")));
                    }
                }
            }

            Instruction::I32Load { offset } => self.load("c_int32", *offset, operands, results),
            Instruction::I64Load { offset } => self.load("c_int64", *offset, operands, results),
            Instruction::F32Load { offset } => self.load("c_float", *offset, operands, results),
            Instruction::F64Load { offset } => self.load("c_double", *offset, operands, results),
            Instruction::I32Load8U { offset } => self.load("c_uint8", *offset, operands, results),
            Instruction::I32Load8S { offset } => self.load("c_int8", *offset, operands, results),
            Instruction::I32Load16U { offset } => self.load("c_uint16", *offset, operands, results),
            Instruction::I32Load16S { offset } => self.load("c_int16", *offset, operands, results),
            Instruction::I32Store { offset } => self.store("c_uint32", *offset, operands),
            Instruction::I64Store { offset } => self.store("c_uint64", *offset, operands),
            Instruction::F32Store { offset } => self.store("c_float", *offset, operands),
            Instruction::F64Store { offset } => self.store("c_double", *offset, operands),
            Instruction::I32Store8 { offset } => self.store("c_uint8", *offset, operands),
            Instruction::I32Store16 { offset } => self.store("c_uint16", *offset, operands),

            Instruction::Malloc { size, align, .. } => {
                let realloc = self.realloc.as_ref().unwrap();
                let ptr = self.locals.tmp("ptr");
                uwriteln!(self.src, "{ptr} = {realloc}(caller, 0, 0, {align}, {size})");
                uwriteln!(self.src, "assert(isinstance({ptr}, int))");
                results.push(ptr);
            }

            i => unimplemented!("{:?}", i),
        }
    }
}

fn classify_union(types: impl Iterator<Item = Type>) -> PyUnionRepresentation {
    #[derive(Debug, Hash, PartialEq, Eq)]
    enum PyTypeClass {
        Int,
        Str,
        Float,
        Custom,
    }

    let mut py_type_classes = HashSet::new();
    for ty in types {
        let class = match ty {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64 => PyTypeClass::Int,
            Type::Float32 | Type::Float64 => PyTypeClass::Float,
            Type::Char | Type::String => PyTypeClass::Str,
            Type::Id(_) => PyTypeClass::Custom,
        };
        if !py_type_classes.insert(class) {
            // Some of the cases are not distinguishable
            return PyUnionRepresentation::Wrapped;
        }
    }
    PyUnionRepresentation::Raw
}

fn wasm_ty_ctor(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "wasmtime.ValType.i32()",
        WasmType::I64 => "wasmtime.ValType.i64()",
        WasmType::F32 => "wasmtime.ValType.f32()",
        WasmType::F64 => "wasmtime.ValType.f64()",
    }
}

fn wasm_ty_typing(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int",
        WasmType::I64 => "int",
        WasmType::F32 => "float",
        WasmType::F64 => "float",
    }
}

/// Creates a temporary `InterfaceGenerator` with the given parameters to get
/// access to the various `print_*` methods on it.
fn with_igen<R>(
    src: &mut Source,
    gen: &mut WasmtimePy,
    iface: &Interface,
    at_root: bool,
    self_module_path: &str,
    f: impl FnOnce(&mut InterfaceGenerator<'_>) -> R,
) -> R {
    // The `print_ty` method is on `InterfaceGenerator` so jerry-rig one of
    // those "quickly" to defer to it.
    let mut gen = InterfaceGenerator {
        src: mem::take(src),
        gen,
        iface,
        at_root,
        self_module_path,
    };
    let ret = f(&mut gen);
    *src = gen.src;
    ret
}
