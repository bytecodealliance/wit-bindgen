use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::mem;
use wit_bindgen_gen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_gen_core::{wit_parser::*, Direction, Files, Generator, Ns};

pub mod dependencies;
pub mod source;

use dependencies::Dependencies;
use source::Source;

#[derive(Default)]
pub struct WasmtimePy {
    src: Source,
    in_import: bool,
    opts: Opts,
    guest_imports: HashMap<String, Imports>,
    guest_exports: HashMap<String, Exports>,
    sizes: SizeAlign,
    /// Tracks the intrinsics and Python imports needed
    deps: Dependencies,
    /// Whether the Python Union being emited will wrap its cases with dataclasses
    union_representation: HashMap<String, PyUnionRepresentation>,
}

#[derive(Debug, Clone, Copy)]
enum PyUnionRepresentation {
    /// A union whose inner types are used directly
    Raw,
    /// A union whose inner types have been wrapped in dataclasses
    Wrapped,
}

#[derive(Default)]
struct Imports {
    freestanding_funcs: Vec<Import>,
    resource_funcs: BTreeMap<ResourceId, Vec<Import>>,
}

struct Import {
    name: String,
    src: Source,
    wasm_ty: String,
    pysig: String,
}

#[derive(Default)]
struct Exports {
    freestanding_funcs: Vec<Source>,
    resource_funcs: BTreeMap<ResourceId, Vec<Source>>,
    fields: BTreeMap<String, &'static str>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    #[cfg_attr(feature = "structopt", structopt(long = "no-typescript"))]
    pub no_typescript: bool,
}

impl Opts {
    pub fn build(self) -> WasmtimePy {
        let mut r = WasmtimePy::new();
        r.opts = self;
        r
    }
}

impl WasmtimePy {
    pub fn new() -> WasmtimePy {
        WasmtimePy::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the Wasmtime-py host-side
        // bindings, we don't use any extra adapter layer between guest wasm
        // modules and the host. When the guest imports functions using the
        // `GuestImport` ABI, the host directly implements the `GuestImport`
        // ABI, even though the host is *exporting* functions. Similarly, when
        // the guest exports functions using the `GuestExport` ABI, the host
        // directly imports them with the `GuestExport` ABI, even though the
        // host is *importing* functions.
        match dir {
            Direction::Import => AbiVariant::GuestExport,
            Direction::Export => AbiVariant::GuestImport,
        }
    }

    /// Creates a `Source` with all of the required intrinsics
    fn intrinsics(&mut self, iface: &Interface) -> Source {
        if iface.resources.len() > 0 {
            self.deps.needs_resources = true;
            self.deps.pyimport("typing", "runtime_checkable");
        }
        self.deps.intrinsics()
    }
}

fn array_ty(iface: &Interface, ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Unit | Type::Bool => None,
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
        Type::Handle(_) => None,
        Type::String => None,
        Type::Id(id) => match &iface.types[*id].kind {
            TypeDefKind::Type(t) => array_ty(iface, t),
            _ => None,
        },
    }
}

impl Generator for WasmtimePy {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.sizes.fill(iface);
        self.in_import = variant == AbiVariant::GuestImport;
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.pyimport("dataclasses", "dataclass");
        builder.push_str("@dataclass\n");
        builder.push_str(&format!("class {}:\n", name.to_camel_case()));
        builder.indent();
        builder.docstring(docs);
        for field in record.fields.iter() {
            builder.comment(&field.docs);
            let field_name = field.name.to_snake_case();
            builder.push_str(&format!("{field_name}: "));
            builder.print_ty(&field.ty, true);
            builder.push_str("\n");
        }
        if record.fields.is_empty() {
            builder.push_str("pass\n");
        }
        builder.dedent();
        builder.push_str("\n");
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.comment(docs);
        builder.push_str(&format!("{} = ", name.to_camel_case()));
        builder.print_tuple(tuple);
        builder.push_str("\n");
    }

    fn type_flags(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.pyimport("enum", "Flag");
        builder.pyimport("enum", "auto");
        builder.push_str(&format!("class {}(Flag):\n", name.to_camel_case()));
        builder.indent();
        builder.docstring(docs);
        for flag in flags.flags.iter() {
            let flag_name = flag.name.to_shouty_snake_case();
            builder.comment(&flag.docs);
            builder.push_str(&format!("{flag_name} = auto()\n"));
        }
        if flags.flags.is_empty() {
            builder.push_str("pass\n");
        }
        builder.dedent();
        builder.push_str("\n");
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.pyimport("dataclasses", "dataclass");
        let mut cases = Vec::new();
        for case in variant.cases.iter() {
            builder.docstring(&case.docs);
            builder.push_str("@dataclass\n");
            let case_name = format!("{}{}", name.to_camel_case(), case.name.to_camel_case());
            builder.push_str(&format!("class {case_name}:\n"));
            builder.indent();
            builder.push_str("value: ");
            builder.print_ty(&case.ty, true);
            builder.push_str("\n");
            builder.dedent();
            builder.push_str("\n");
            cases.push(case_name);
        }

        builder.deps.pyimport("typing", "Union");
        builder.comment(docs);
        builder.push_str(&format!(
            "{} = Union[{}]\n",
            name.to_camel_case(),
            cases.join(", "),
        ));
        builder.push_str("\n");
    }

    /// Appends a Python definition for the provided Union to the current `Source`.
    /// e.g. `MyUnion = Union[float, str, int]`
    fn type_union(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        let mut py_type_classes = BTreeSet::new();
        for case in union.cases.iter() {
            py_type_classes.insert(py_type_class_of(&case.ty));
        }

        let mut builder = self.src.builder(&mut self.deps, iface);
        if py_type_classes.len() != union.cases.len() {
            // Some of the cases are not distinguishable
            self.union_representation
                .insert(name.to_string(), PyUnionRepresentation::Wrapped);
            builder.print_union_wrapped(name, union, docs);
        } else {
            // All of the cases are distinguishable
            self.union_representation
                .insert(name.to_string(), PyUnionRepresentation::Raw);
            builder.print_union_raw(name, union, docs);
        }
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.pyimport("typing", "Optional");
        builder.comment(docs);
        builder.push_str(&name.to_camel_case());
        builder.push_str(" = Optional[");
        builder.print_ty(payload, true);
        builder.push_str("]\n\n");
    }

    fn type_expected(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        expected: &Expected,
        docs: &Docs,
    ) {
        self.deps.needs_expected = true;

        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.comment(docs);
        builder.push_str(&format!("{} = Expected[", name.to_camel_case()));
        builder.print_ty(&expected.ok, true);
        builder.push_str(", ");
        builder.print_ty(&expected.err, true);
        builder.push_str("]\n\n");
    }

    fn type_enum(&mut self, iface: &Interface, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.pyimport("enum", "Enum");
        builder.push_str(&format!("class {}(Enum):\n", name.to_camel_case()));
        builder.indent();
        builder.docstring(docs);
        for (i, case) in enum_.cases.iter().enumerate() {
            builder.comment(&case.docs);

            // TODO this handling of digits should be more general and
            // shouldn't be here just to fix the one case in wasi where an
            // enum variant is "2big" and doesn't generate valid Python. We
            // should probably apply this to all generated Python
            // identifiers.
            let mut name = case.name.to_shouty_snake_case();
            if name.chars().next().unwrap().is_digit(10) {
                name = format!("_{}", name);
            }
            builder.push_str(&format!("{} = {}\n", name, i));
        }
        builder.dedent();
        builder.push_str("\n");
    }

    fn type_resource(&mut self, _iface: &Interface, _ty: ResourceId) {
        // if !self.in_import {
        //     self.exported_resources.insert(ty);
        // }
    }

    fn type_alias(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.comment(docs);
        builder.push_str(&format!("{} = ", name.to_camel_case()));
        builder.print_ty(ty, false);
        builder.push_str("\n");
    }

    fn type_list(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let mut builder = self.src.builder(&mut self.deps, iface);
        builder.comment(docs);
        builder.push_str(&format!("{} = ", name.to_camel_case()));
        builder.print_list(ty);
        builder.push_str("\n");
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.type_alias(iface, id, name, ty, docs);
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let mut pysig = Source::default();
        let mut builder = pysig.builder(&mut self.deps, iface);
        builder.print_sig(func, self.in_import);
        let pysig = pysig.to_string();

        let mut func_body = Source::default();
        let mut builder = func_body.builder(&mut self.deps, iface);

        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        builder.push_str(&format!(
            "def {}(caller: wasmtime.Caller",
            func.name.to_snake_case(),
        ));
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            builder.push_str(", ");
            let name = format!("arg{}", i);
            builder.push_str(&name);
            builder.push_str(": ");
            builder.push_str(wasm_ty_typing(*param));
            params.push(name);
        }
        builder.push_str(") -> ");
        match sig.results.len() {
            0 => builder.push_str("None"),
            1 => builder.push_str(wasm_ty_typing(sig.results[0])),
            _ => unimplemented!(),
        }
        builder.push_str(":\n");
        builder.indent();
        drop(builder);

        let mut f = FunctionBindgen::new(self, params);
        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            mut locals,
            ..
        } = f;

        let mut builder = func_body.builder(&mut self.deps, iface);
        if needs_memory {
            // TODO: hardcoding "memory"
            builder.push_str("m = caller[\"memory\"]\n");
            builder.push_str("assert(isinstance(m, wasmtime.Memory))\n");
            builder.deps.pyimport("typing", "cast");
            builder.push_str("memory = cast(wasmtime.Memory, m)\n");
            locals.insert("memory").unwrap();
        }

        if let Some(name) = needs_realloc {
            builder.push_str(&format!("realloc = caller[\"{}\"]\n", name));
            builder.push_str("assert(isinstance(realloc, wasmtime.Func))\n");
            locals.insert("realloc").unwrap();
        }

        if let Some(name) = needs_free {
            builder.push_str(&format!("free = caller[\"{}\"]\n", name));
            builder.push_str("assert(isinstance(free, wasmtime.Func))\n");
            locals.insert("free").unwrap();
        }
        builder.push_str(&src);
        builder.dedent();

        let mut wasm_ty = String::from("wasmtime.FuncType([");
        wasm_ty.push_str(
            &sig.params
                .iter()
                .map(|t| wasm_ty_ctor(*t))
                .collect::<Vec<_>>()
                .join(", "),
        );
        wasm_ty.push_str("], [");
        wasm_ty.push_str(
            &sig.results
                .iter()
                .map(|t| wasm_ty_ctor(*t))
                .collect::<Vec<_>>()
                .join(", "),
        );
        wasm_ty.push_str("])");
        let import = Import {
            name: func.name.clone(),
            src: func_body,
            wasm_ty,
            pysig,
        };
        let imports = self
            .guest_imports
            .entry(iface.name.to_string())
            .or_insert(Imports::default());
        let dst = match &func.kind {
            FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                &mut imports.freestanding_funcs
            }
            FunctionKind::Method { resource, .. } => imports
                .resource_funcs
                .entry(*resource)
                .or_insert(Vec::new()),
        };
        dst.push(import);
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "import" uses the "export" ABI variant on the inside of
    // this `Generator` implementation.
    fn import(&mut self, iface: &Interface, func: &Function) {
        assert!(!func.is_async, "async not supported yet");
        let mut func_body = Source::default();
        let mut builder = func_body.builder(&mut self.deps, iface);

        // Print the function signature
        let params = builder.print_sig(func, self.in_import);
        builder.push_str(":\n");
        builder.indent();
        drop(builder);

        // Use FunctionBindgen call
        let src_object = match &func.kind {
            FunctionKind::Freestanding => "self".to_string(),
            FunctionKind::Static { .. } => "obj".to_string(),
            FunctionKind::Method { .. } => "self._obj".to_string(),
        };
        let mut f = FunctionBindgen::new(self, params);
        f.src_object = src_object;
        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            src_object,
            ..
        } = f;
        let mut builder = func_body.builder(&mut self.deps, iface);
        if needs_memory {
            // TODO: hardcoding "memory"
            builder.push_str(&format!("memory = {}._memory;\n", src_object));
        }

        if let Some(name) = &needs_realloc {
            builder.push_str(&format!(
                "realloc = {}._{}\n",
                src_object,
                name.to_snake_case(),
            ));
        }

        if let Some(name) = &needs_free {
            builder.push_str(&format!(
                "free = {}._{}\n",
                src_object,
                name.to_snake_case(),
            ));
        }
        builder.push_str(&src);
        builder.dedent();

        let exports = self
            .guest_exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);
        if needs_memory {
            exports
                .fields
                .insert("memory".to_string(), "wasmtime.Memory");
        }
        if let Some(name) = &needs_realloc {
            exports.fields.insert(name.clone(), "wasmtime.Func");
        }
        if let Some(name) = &needs_free {
            exports.fields.insert(name.clone(), "wasmtime.Func");
        }
        exports.fields.insert(func.name.clone(), "wasmtime.Func");

        let dst = match &func.kind {
            FunctionKind::Freestanding => &mut exports.freestanding_funcs,
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => {
                exports
                    .resource_funcs
                    .entry(*resource)
                    .or_insert(Vec::new())
            }
        };
        dst.push(func_body);
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        self.deps.pyimport("typing", "Any");
        self.deps.pyimport("abc", "abstractmethod");

        let types = mem::take(&mut self.src);
        let intrinsics = self.intrinsics(iface);

        for (k, v) in self.deps.pyimports.iter() {
            match v {
                Some(list) => {
                    let list = list.iter().cloned().collect::<Vec<_>>().join(", ");
                    self.src.push_str(&format!("from {} import {}\n", k, list));
                }
                None => {
                    self.src.push_str(&format!("import {}\n", k));
                }
            }
        }
        self.src.push_str("import wasmtime\n");
        self.src.push_str(
            "
                try:
                    from typing import Protocol
                except ImportError:
                    class Protocol: # type: ignore
                        pass
            ",
        );
        self.src.push_str("\n");

        if self.deps.needs_t_typevar {
            self.src.push_str("T = TypeVar('T')\n");
        }

        self.src.push_str(&intrinsics);
        for (id, r) in iface.resources.iter() {
            let name = r.name.to_camel_case();
            if self.in_import {
                self.src.push_str("@runtime_checkable\n");
                self.src.push_str(&format!("class {}(Protocol):\n", name));
                self.src.indent();
                self.src.push_str("def drop(self) -> None:\n");
                self.src.indent();
                self.src.push_str("pass\n");
                self.src.dedent();

                for (_, funcs) in self.guest_imports.iter() {
                    if let Some(funcs) = funcs.resource_funcs.get(&id) {
                        for func in funcs {
                            self.src.push_str("@abstractmethod\n");
                            self.src.push_str(&func.pysig);
                            self.src.push_str(":\n");
                            self.src.indent();
                            self.src.push_str("raise NotImplementedError\n");
                            self.src.dedent();
                        }
                    }
                }
                self.src.dedent();
            } else {
                self.src.push_str(&format!("class {}:\n", name));
                self.src.indent();
                self.src.push_str(&format!(
                    "
                        _wasm_val: int
                        _refcnt: int
                        _obj: '{iface}'
                        _destroyed: bool

                        def __init__(self, val: int, obj: '{iface}') -> None:
                            self._wasm_val = val
                            self._refcnt = 1
                            self._obj = obj
                            self._destroyed = False

                        def clone(self) -> '{name}':
                            self._refcnt += 1
                            return self

                        def drop(self, store: wasmtime.Storelike) -> None:
                            self._refcnt -= 1;
                            if self._refcnt != 0:
                                return
                            assert(not self._destroyed)
                            self._destroyed = True
                            self._obj._canonical_abi_drop_{drop}(store, self._wasm_val)

                        def __del__(self) -> None:
                            if not self._destroyed:
                                raise RuntimeError('wasm object not dropped')
                    ",
                    name = name,
                    iface = iface.name.to_camel_case(),
                    drop = name.to_snake_case(),
                ));

                for (_, exports) in self.guest_exports.iter() {
                    if let Some(funcs) = exports.resource_funcs.get(&id) {
                        for func in funcs {
                            self.src.push_str(func);
                        }
                    }
                }

                self.src.dedent();
            }
        }
        self.src.push_str(&types);

        for (module, funcs) in mem::take(&mut self.guest_imports) {
            self.src
                .push_str(&format!("class {}(Protocol):\n", module.to_camel_case()));
            self.src.indent();
            for func in funcs.freestanding_funcs.iter() {
                self.src.push_str("@abstractmethod\n");
                self.src.push_str(&func.pysig);
                self.src.push_str(":\n");
                self.src.indent();
                self.src.push_str("raise NotImplementedError\n");
                self.src.dedent();
            }
            self.src.dedent();
            self.src.push_str("\n");

            self.src.push_str(&format!(
                "def add_{}_to_linker(linker: wasmtime.Linker, store: wasmtime.Store, host: {}) -> None:\n",
                module.to_snake_case(),
                module.to_camel_case(),
            ));
            self.src.indent();

            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "_resources{}: Slab[{}] = Slab()\n",
                    id.index(),
                    r.name.to_camel_case()
                ));
            }

            for func in funcs
                .freestanding_funcs
                .iter()
                .chain(funcs.resource_funcs.values().flat_map(|v| v))
            {
                self.src.push_str(&format!("ty = {}\n", func.wasm_ty));
                self.src.push_str(&func.src);
                self.src.push_str(&format!(
                    "linker.define('{}', '{}', wasmtime.Func(store, ty, {}, access_caller = True))\n",
                    iface.name,
                    func.name,
                    func.name.to_snake_case(),
                ));
            }

            for (id, resource) in iface.resources.iter() {
                let snake = resource.name.to_snake_case();

                self.src.push_str(&format!(
                    "def resource_drop_{}(i: int) -> None:\n  _resources{}.remove(i).drop()\n",
                    snake,
                    id.index(),
                ));
                self.src
                    .push_str("ty = wasmtime.FuncType([wasmtime.ValType.i32()], [])\n");
                self.src.push_str(&format!(
                    "linker.define(\
                        'canonical_abi', \
                        'resource_drop_{}', \
                        wasmtime.Func(store, ty, resource_drop_{})\
                    )\n",
                    resource.name, snake,
                ));
            }
            self.src.dedent();
        }

        // This is exculsively here to get mypy to not complain about empty
        // modules, this probably won't really get triggered much in practice
        if !self.in_import && self.guest_exports.is_empty() {
            self.src
                .push_str(&format!("class {}:\n", iface.name.to_camel_case()));
            self.src.indent();
            if iface.resources.len() == 0 {
                self.src.push_str("pass\n");
            } else {
                for (_, r) in iface.resources.iter() {
                    self.src.push_str(&format!(
                        "_canonical_abi_drop_{}: wasmtime.Func\n",
                        r.name.to_snake_case(),
                    ));
                }
            }
            self.src.dedent();
        }

        for (module, exports) in mem::take(&mut self.guest_exports) {
            let module = module.to_camel_case();
            self.src.push_str(&format!("class {}:\n", module));
            self.src.indent();

            self.src.push_str("instance: wasmtime.Instance\n");
            for (name, ty) in exports.fields.iter() {
                self.src
                    .push_str(&format!("_{}: {}\n", name.to_snake_case(), ty));
            }
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "_resource{}_slab: Slab[{}]\n",
                    id.index(),
                    r.name.to_camel_case(),
                ));
                self.src.push_str(&format!(
                    "_canonical_abi_drop_{}: wasmtime.Func\n",
                    r.name.to_snake_case(),
                ));
            }

            self.src.push_str("def __init__(self, store: wasmtime.Store, linker: wasmtime.Linker, module: wasmtime.Module):\n");
            self.src.indent();
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "
                       ty1 = wasmtime.FuncType([wasmtime.ValType.i32()], [])
                       ty2 = wasmtime.FuncType([wasmtime.ValType.i32()], [wasmtime.ValType.i32()])
                       def drop_{snake}(caller: wasmtime.Caller, idx: int) -> None:
                            self._resource{idx}_slab.remove(idx).drop(caller);
                       linker.define('canonical_abi', 'resource_drop_{name}', wasmtime.Func(store, ty1, drop_{snake}, access_caller = True))

                       def clone_{snake}(idx: int) -> int:
                            obj = self._resource{idx}_slab.get(idx)
                            return self._resource{idx}_slab.insert(obj.clone())
                       linker.define('canonical_abi', 'resource_clone_{name}', wasmtime.Func(store, ty2, clone_{snake}))

                       def get_{snake}(idx: int) -> int:
                            obj = self._resource{idx}_slab.get(idx)
                            return obj._wasm_val
                       linker.define('canonical_abi', 'resource_get_{name}', wasmtime.Func(store, ty2, get_{snake}))

                       def new_{snake}(val: int) -> int:
                            return self._resource{idx}_slab.insert({camel}(val, self))
                       linker.define('canonical_abi', 'resource_new_{name}', wasmtime.Func(store, ty2, new_{snake}))
                   ",
                    name = r.name,
                    camel = r.name.to_camel_case(),
                    snake = r.name.to_snake_case(),
                    idx = id.index(),
                ));
            }
            self.src
                .push_str("self.instance = linker.instantiate(store, module)\n");
            self.src
                .push_str("exports = self.instance.exports(store)\n");
            for (name, ty) in exports.fields.iter() {
                self.src.push_str(&format!(
                    "
                        {snake} = exports['{name}']
                        assert(isinstance({snake}, {ty}))
                        self._{snake} = {snake}
                    ",
                    name = name,
                    snake = name.to_snake_case(),
                    ty = ty,
                ));
            }
            for (id, r) in iface.resources.iter() {
                self.src.push_str(&format!(
                    "
                        self._resource{idx}_slab = Slab()
                        canon_drop_{snake} = exports['canonical_abi_drop_{name}']
                        assert(isinstance(canon_drop_{snake}, wasmtime.Func))
                        self._canonical_abi_drop_{snake} = canon_drop_{snake}
                    ",
                    idx = id.index(),
                    name = r.name,
                    snake = r.name.to_snake_case(),
                ));
            }
            self.src.dedent();

            for func in exports.freestanding_funcs.iter() {
                self.src.push_str(&func);
            }

            self.src.dedent();
        }

        files.push("bindings.py", self.src.as_bytes());
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut WasmtimePy,
    locals: Ns,
    src: Source,
    block_storage: Vec<Source>,
    blocks: Vec<(String, Vec<String>)>,
    needs_memory: bool,
    needs_realloc: Option<String>,
    needs_free: Option<String>,
    params: Vec<String>,
    payloads: Vec<String>,
    src_object: String,
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut WasmtimePy, params: Vec<String>) -> FunctionBindgen<'_> {
        let mut locals = Ns::default();
        locals.insert("len").unwrap(); // python built-in
        locals.insert("base").unwrap(); // may be used as loop var
        locals.insert("i").unwrap(); // may be used as loop var
        for param in params.iter() {
            locals.insert(param).unwrap();
        }
        FunctionBindgen {
            gen,
            locals,
            src: Source::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            needs_memory: false,
            needs_realloc: None,
            needs_free: None,
            params,
            payloads: Vec::new(),
            src_object: "self".to_string(),
        }
    }

    fn clamp<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        self.gen.deps.needs_clamp = true;
        results.push(format!("_clamp({}, {}, {})", operands[0], min, max));
    }

    fn load(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        self.needs_memory = true;
        self.gen.deps.needs_load = true;
        let tmp = self.locals.tmp("load");
        self.src.push_str(&format!(
            "{} = _load(ctypes.{}, memory, caller, {}, {})\n",
            tmp, ty, operands[0], offset,
        ));
        results.push(tmp);
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        self.needs_memory = true;
        self.gen.deps.needs_store = true;
        self.src.push_str(&format!(
            "_store(ctypes.{}, memory, caller, {}, {}, {})\n",
            ty, operands[1], offset, operands[0]
        ));
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, to_restore);
        self.blocks.push((src.into(), mem::take(operands)));
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
        let mut builder = self.src.builder(&mut self.gen.deps, iface);
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
                builder.deps.needs_validate_guest_char = true;
                results.push(format!("_validate_guest_char({})", operands[0]));
            }

            Instruction::I32FromChar => {
                results.push(format!("ord({})", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    match cast {
                        Bitcast::I32ToF32 => {
                            builder.deps.needs_i32_to_f32 = true;
                            results.push(format!("_i32_to_f32({})", op));
                        }
                        Bitcast::F32ToI32 => {
                            builder.deps.needs_f32_to_i32 = true;
                            results.push(format!("_f32_to_i32({})", op));
                        }
                        Bitcast::I64ToF64 => {
                            builder.deps.needs_i64_to_f64 = true;
                            results.push(format!("_i64_to_f64({})", op));
                        }
                        Bitcast::F64ToI64 => {
                            builder.deps.needs_f64_to_i64 = true;
                            results.push(format!("_f64_to_i64({})", op));
                        }
                        Bitcast::I64ToF32 => {
                            builder.deps.needs_i32_to_f32 = true;
                            results.push(format!("_i32_to_f32(({}) & 0xffffffff)", op));
                        }
                        Bitcast::F32ToI64 => {
                            builder.deps.needs_f32_to_i32 = true;
                            results.push(format!("_f32_to_i32({})", op));
                        }
                        Bitcast::I32ToI64 | Bitcast::I64ToI32 | Bitcast::None => {
                            results.push(op.clone())
                        }
                    }
                }
            }

            Instruction::UnitLower => {}
            Instruction::UnitLift => {
                results.push("None".to_string());
            }
            Instruction::BoolFromI32 => {
                let op = self.locals.tmp("operand");
                let ret = self.locals.tmp("boolean");
                builder.push_str(&format!(
                    "
                        {op} = {}
                        if {op} == 0:
                            {ret} = False
                        elif {op} == 1:
                            {ret} = True
                        else:
                            raise TypeError(\"invalid variant discriminant for bool\")
                    ",
                    operands[0]
                ));
                results.push(ret);
            }
            Instruction::I32FromBool => {
                results.push(format!("int({})", operands[0]));
            }

            // These instructions are used with handles when we're implementing
            // imports. This means we interact with the `resources` slabs to
            // translate the wasm-provided index into a Python value.
            Instruction::I32FromOwnedHandle { ty } => {
                results.push(format!("_resources{}.insert({})", ty.index(), operands[0]));
            }
            Instruction::HandleBorrowedFromI32 { ty } => {
                results.push(format!("_resources{}.get({})", ty.index(), operands[0]));
            }

            // These instructions are used for handles to objects owned in wasm.
            // This means that they're interacting with a wrapper class defined
            // in Python.
            Instruction::I32FromBorrowedHandle { ty } => {
                let obj = self.locals.tmp("obj");
                builder.push_str(&format!("{} = {}\n", obj, operands[0]));

                results.push(format!(
                    "{}._resource{}_slab.insert({}.clone())",
                    self.src_object,
                    ty.index(),
                    obj,
                ));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}._resource{}_slab.remove({})",
                    self.src_object,
                    ty.index(),
                    operands[0],
                ));
            }
            Instruction::RecordLower { record, .. } => {
                if record.fields.is_empty() {
                    return;
                }
                let tmp = self.locals.tmp("record");
                builder.push_str(&format!("{} = {}\n", tmp, operands[0]));
                for field in record.fields.iter() {
                    let name = self.locals.tmp("field");
                    builder.push_str(&format!(
                        "{} = {}.{}\n",
                        name,
                        tmp,
                        field.name.to_snake_case(),
                    ));
                    results.push(name);
                }
            }

            Instruction::RecordLift { name, .. } => {
                results.push(format!("{}({})", name.to_camel_case(), operands.join(", ")));
            }
            Instruction::TupleLower { tuple, .. } => {
                if tuple.types.is_empty() {
                    return;
                }
                builder.push_str("(");
                for _ in 0..tuple.types.len() {
                    let name = self.locals.tmp("tuplei");
                    builder.push_str(&name);
                    builder.push_str(",");
                    results.push(name);
                }
                builder.push_str(") = ");
                builder.push_str(&operands[0]);
                builder.push_str("\n");
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
                        let tmp = self.locals.tmp("flags");
                        builder.push_str(&format!("{tmp} = 0\n"));
                        for (i, op) in operands.iter().enumerate() {
                            let i = 32 * i;
                            builder.push_str(&format!("{tmp} |= {op} << {i}\n"));
                        }
                        tmp
                    }
                };
                results.push(format!("{}({})", name.to_camel_case(), operand));
            }
            Instruction::FlagsLower { flags, .. } => match flags.repr().count() {
                1 => results.push(format!("({}).value", operands[0])),
                n => {
                    let tmp = self.locals.tmp("flags");
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
                        builder.push_str("if ");
                    } else {
                        builder.push_str("elif ");
                    }

                    builder.push_str(&format!(
                        "isinstance({}, {}{}):\n",
                        operands[0],
                        name.to_camel_case(),
                        case.name.to_camel_case()
                    ));
                    builder.indent();
                    builder.push_str(&format!("{} = {}.value\n", payload, operands[0]));
                    builder.push_str(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        builder.push_str(&format!("{} = {}\n", results[i], result));
                    }
                    builder.dedent();
                }
                let variant_name = name.to_camel_case();
                builder.push_str("else:\n");
                builder.indent();
                builder.push_str(&format!(
                    "raise TypeError(\"invalid variant specified for {}\")\n",
                    variant_name
                ));
                builder.dedent();
            }

            Instruction::VariantLift {
                variant, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let result = self.locals.tmp("variant");
                builder.print_var_declaration(&result, &Type::Id(*ty));
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    if i == 0 {
                        builder.push_str("if ");
                    } else {
                        builder.push_str("elif ");
                    }
                    builder.push_str(&format!("{} == {}:\n", operands[0], i));
                    builder.indent();
                    builder.push_str(&block);

                    builder.push_str(&format!(
                        "{} = {}{}(",
                        result,
                        name.to_camel_case(),
                        case.name.to_camel_case()
                    ));
                    assert!(block_results.len() == 1);
                    builder.push_str(&block_results[0]);
                    builder.push_str(")\n");
                    builder.dedent();
                }
                builder.push_str("else:\n");
                builder.indent();
                let variant_name = name.to_camel_case();
                builder.push_str(&format!(
                    "raise TypeError(\"invalid variant discriminant for {}\")\n",
                    variant_name
                ));
                builder.dedent();
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

                // Assumes that type_union has been called for this union
                let union_representation = *self
                    .gen
                    .union_representation
                    .get(&name.to_string())
                    .unwrap();
                let name = name.to_camel_case();
                let op0 = &operands[0];
                for (i, ((case, (block, block_results)), payload)) in
                    union.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    builder.push_str(if i == 0 { "if " } else { "elif " });
                    builder.push_str(&format!("isinstance({op0}, "));
                    match union_representation {
                        // Prints the Python type for this union case
                        PyUnionRepresentation::Raw => {
                            builder.print_ty(&case.ty, false);
                        }
                        // Prints the name of this union cases dataclass
                        PyUnionRepresentation::Wrapped => {
                            builder.push_str(&format!("{name}{i}"));
                        }
                    }
                    builder.push_str(&format!("):\n"));
                    builder.indent();
                    match union_representation {
                        // Uses the value directly
                        PyUnionRepresentation::Raw => {
                            builder.push_str(&format!("{payload} = {op0}\n"))
                        }
                        // Uses this union case dataclass's inner value
                        PyUnionRepresentation::Wrapped => {
                            builder.push_str(&format!("{payload} = {op0}.value\n"))
                        }
                    }
                    builder.push_str(&block);
                    for (i, result) in block_results.iter().enumerate() {
                        builder.push_str(&format!("{} = {result}\n", results[i]));
                    }
                    builder.dedent();
                }
                builder.push_str("else:\n");
                builder.indent();
                builder.push_str(&format!(
                    "raise TypeError(\"invalid variant specified for {name}\")\n"
                ));
                builder.dedent();
            }

            Instruction::UnionLift {
                union, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let result = self.locals.tmp("variant");
                builder.print_var_declaration(&result, &Type::Id(*ty));
                // Assumes that type_union has been called for this union
                let union_representation = *self
                    .gen
                    .union_representation
                    .get(&name.to_string())
                    .unwrap();
                let name = name.to_camel_case();
                let op0 = &operands[0];
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    builder.push_str(if i == 0 { "if " } else { "elif " });
                    builder.push_str(&format!("{op0} == {i}:\n"));
                    builder.indent();
                    builder.push_str(&block);
                    assert!(block_results.len() == 1);
                    let block_result = &block_results[0];
                    builder.push_str(&format!("{result} = "));
                    match union_representation {
                        // Uses the passed value directly
                        PyUnionRepresentation::Raw => builder.push_str(block_result),
                        // Constructs an instance of the union cases dataclass
                        PyUnionRepresentation::Wrapped => {
                            builder.push_str(&format!("{name}{i}({block_result})"))
                        }
                    }
                    builder.newline();
                    builder.dedent();
                }
                builder.push_str("else:\n");
                builder.indent();
                builder.push_str(&format!(
                    "raise TypeError(\"invalid variant discriminant for {name}\")\n",
                ));
                builder.dedent();
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
                builder.push_str(&format!("if {op0} is None:\n"));

                builder.indent();
                builder.push_str(&none);
                for (dst, result) in results.iter().zip(&none_results) {
                    builder.push_str(&format!("{dst} = {result}\n"));
                }
                builder.dedent();
                builder.push_str("else:\n");
                builder.indent();
                builder.push_str(&format!("{some_payload} = {op0}\n"));
                builder.push_str(&some);
                for (dst, result) in results.iter().zip(&some_results) {
                    builder.push_str(&format!("{dst} = {result}\n"));
                }
                builder.dedent();
            }

            Instruction::OptionLift { ty, .. } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.len() == 1);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];
                assert_eq!(none_results[0], "None");

                let result = self.locals.tmp("option");
                builder.print_var_declaration(&result, &Type::Id(*ty));

                let op0 = &operands[0];
                builder.push_str(&format!("if {op0} == 0:\n"));
                builder.indent();
                builder.push_str(&none);
                builder.push_str(&format!("{result} = None\n"));
                builder.dedent();
                builder.push_str(&format!("elif {op0} == 1:\n"));
                builder.indent();
                builder.push_str(&some);
                builder.push_str(&format!("{result} = {some_result}\n"));
                builder.dedent();

                builder.push_str("else:\n");
                builder.indent();
                builder.push_str("raise TypeError(\"invalid variant discriminant for option\")\n");
                builder.dedent();

                results.push(result);
            }

            Instruction::ExpectedLower {
                results: result_types,
                ..
            } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let err_payload = self.payloads.pop().unwrap();
                let ok_payload = self.payloads.pop().unwrap();

                for _ in 0..result_types.len() {
                    results.push(self.locals.tmp("variant"));
                }

                let op0 = &operands[0];
                builder.push_str(&format!("if isinstance({op0}, Ok):\n"));

                builder.indent();
                builder.push_str(&format!("{ok_payload} = {op0}.value\n"));
                builder.push_str(&ok);
                for (dst, result) in results.iter().zip(&ok_results) {
                    builder.push_str(&format!("{dst} = {result}\n"));
                }
                builder.dedent();
                builder.push_str(&format!("elif isinstance({op0}, Err):\n"));
                builder.indent();
                builder.push_str(&format!("{err_payload} = {op0}.value\n"));
                builder.push_str(&err);
                for (dst, result) in results.iter().zip(&err_results) {
                    builder.push_str(&format!("{dst} = {result}\n"));
                }
                builder.dedent();
                builder.push_str("else:\n");
                builder.indent();
                builder.push_str(&format!(
                    "raise TypeError(\"invalid variant specified for expected\")\n",
                ));
                builder.dedent();
            }

            Instruction::ExpectedLift { ty, .. } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                assert!(err_results.len() == 1);
                let err_result = &err_results[0];
                assert!(ok_results.len() == 1);
                let ok_result = &ok_results[0];

                let result = self.locals.tmp("expected");
                builder.print_var_declaration(&result, &Type::Id(*ty));

                let op0 = &operands[0];
                builder.push_str(&format!("if {op0} == 0:\n"));
                builder.indent();
                builder.push_str(&ok);
                builder.push_str(&format!("{result} = Ok({ok_result})\n"));
                builder.dedent();
                builder.push_str(&format!("elif {op0} == 1:\n"));
                builder.indent();
                builder.push_str(&err);
                builder.push_str(&format!("{result} = Err({err_result})\n"));
                builder.dedent();

                builder.push_str("else:\n");
                builder.indent();
                builder
                    .push_str("raise TypeError(\"invalid variant discriminant for expected\")\n");
                builder.dedent();

                results.push(result);
            }

            Instruction::EnumLower { .. } => results.push(format!("({}).value", operands[0])),

            Instruction::EnumLift { name, .. } => {
                results.push(format!("{}({})", name.to_camel_case(), operands[0]));
            }

            Instruction::ListCanonLower { element, realloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.needs_memory = true;
                self.needs_realloc = Some(realloc.to_string());

                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                let array_ty = array_ty(iface, element).unwrap();
                builder.deps.needs_list_canon_lower = true;
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                builder.push_str(&format!(
                    "{}, {} = _list_canon_lower({}, ctypes.{}, {}, {}, realloc, memory, caller)\n",
                    ptr, len, operands[0], array_ty, size, align,
                ));
                results.push(ptr);
                results.push(len);
            }
            Instruction::ListCanonLift { element, free, .. } => {
                self.needs_memory = true;
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                builder.push_str(&format!("{} = {}\n", ptr, operands[0]));
                builder.push_str(&format!("{} = {}\n", len, operands[1]));
                let array_ty = array_ty(iface, element).unwrap();
                builder.deps.needs_list_canon_lift = true;
                let lift = format!(
                    "_list_canon_lift({}, {}, {}, ctypes.{}, memory, caller)",
                    ptr,
                    len,
                    self.gen.sizes.size(element),
                    array_ty,
                );
                builder.deps.pyimport("typing", "cast");
                let align = self.gen.sizes.align(element);
                match free {
                    Some(free) => {
                        self.needs_free = Some(free.to_string());
                        let list = self.locals.tmp("list");
                        builder.push_str(&list);
                        builder.push_str(" = cast(");
                        builder.print_list(element);
                        builder.push_str(", ");
                        builder.push_str(&lift);
                        builder.push_str(")\n");
                        builder.push_str(&format!("free(caller, {}, {}, {})\n", ptr, len, align));
                        results.push(list);
                    }
                    None => {
                        let mut result_src = Source::default();
                        drop(builder);
                        let mut builder = result_src.builder(&mut self.gen.deps, iface);
                        builder.push_str("cast(");
                        builder.print_list(element);
                        builder.push_str(", ");
                        builder.push_str(&lift);
                        builder.push_str(")");
                        results.push(result_src.to_string());
                    }
                }
            }
            Instruction::StringLower { realloc } => {
                // Lowering only happens when we're passing strings into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.needs_memory = true;
                self.needs_realloc = Some(realloc.to_string());

                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                builder.deps.needs_encode_utf8 = true;
                builder.push_str(&format!(
                    "{}, {} = _encode_utf8({}, realloc, memory, caller)\n",
                    ptr, len, operands[0],
                ));
                results.push(ptr);
                results.push(len);
            }
            Instruction::StringLift { free, .. } => {
                self.needs_memory = true;
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                builder.push_str(&format!("{} = {}\n", ptr, operands[0]));
                builder.push_str(&format!("{} = {}\n", len, operands[1]));
                builder.deps.needs_decode_utf8 = true;
                let result = format!("_decode_utf8(memory, caller, {}, {})", ptr, len);
                match free {
                    Some(free) => {
                        self.needs_free = Some(free.to_string());
                        let list = self.locals.tmp("list");
                        builder.push_str(&format!("{} = {}\n", list, result));
                        self.src
                            .push_str(&format!("free(caller, {}, {}, 1)\n", ptr, len));
                        results.push(list);
                    }
                    None => results.push(result),
                }
            }

            Instruction::ListLower { element, realloc } => {
                let base = self.payloads.pop().unwrap();
                let e = self.payloads.pop().unwrap();
                let realloc = realloc.unwrap();
                let (body, body_results) = self.blocks.pop().unwrap();
                assert!(body_results.is_empty());
                let vec = self.locals.tmp("vec");
                let result = self.locals.tmp("result");
                let len = self.locals.tmp("len");
                self.needs_realloc = Some(realloc.to_string());
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                builder.push_str(&format!("{} = {}\n", vec, operands[0]));
                builder.push_str(&format!("{} = len({})\n", len, vec));

                // ... then realloc space for the result in the guest module
                builder.push_str(&format!(
                    "{} = realloc(caller, 0, 0, {}, {} * {})\n",
                    result, align, len, size,
                ));
                builder.push_str(&format!("assert(isinstance({}, int))\n", result));

                // ... then consume the vector and use the block to lower the
                // result.
                let i = self.locals.tmp("i");
                builder.push_str(&format!("for {} in range(0, {}):\n", i, len));
                builder.indent();
                builder.push_str(&format!("{} = {}[{}]\n", e, vec, i));
                builder.push_str(&format!("{} = {} + {} * {}\n", base, result, i, size));
                builder.push_str(&body);
                builder.dedent();

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                let base = self.payloads.pop().unwrap();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                builder.push_str(&format!("{} = {}\n", ptr, operands[0]));
                builder.push_str(&format!("{} = {}\n", len, operands[1]));
                let result = self.locals.tmp("result");
                builder.push_str(&format!("{}: List[", result));
                builder.print_ty(element, true);
                builder.push_str("] = []\n");

                let i = self.locals.tmp("i");
                builder.push_str(&format!("for {} in range(0, {}):\n", i, len));
                builder.indent();
                builder.push_str(&format!("{} = {} + {} * {}\n", base, ptr, i, size));
                builder.push_str(&body);
                assert_eq!(body_results.len(), 1);
                builder.push_str(&format!("{}.append({})\n", result, body_results[0]));
                builder.dedent();

                if let Some(free) = free {
                    self.needs_free = Some(free.to_string());
                    builder.push_str(&format!(
                        "free(caller, {}, {} * {}, {})\n",
                        ptr, len, size, align,
                    ));
                }
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
            Instruction::CallWasm {
                iface: _,
                name,
                sig,
            } => {
                if sig.results.len() > 0 {
                    for i in 0..sig.results.len() {
                        if i > 0 {
                            builder.push_str(", ");
                        }
                        let ret = self.locals.tmp("ret");
                        builder.push_str(&ret);
                        results.push(ret);
                    }
                    builder.push_str(" = ");
                }
                builder.push_str(&self.src_object);
                builder.push_str("._");
                builder.push_str(&name.to_snake_case());
                builder.push_str("(caller");
                if operands.len() > 0 {
                    builder.push_str(", ");
                }
                builder.push_str(&operands.join(", "));
                builder.push_str(")\n");
                for (ty, name) in sig.results.iter().zip(results.iter()) {
                    let ty = match ty {
                        WasmType::I32 | WasmType::I64 => "int",
                        WasmType::F32 | WasmType::F64 => "float",
                    };
                    self.src
                        .push_str(&format!("assert(isinstance({}, {}))\n", name, ty));
                }
            }
            Instruction::CallInterface { module: _, func } => {
                match &func.result {
                    Type::Unit => {
                        results.push("".to_string());
                    }
                    _ => {
                        let result = self.locals.tmp("ret");
                        builder.push_str(&result);
                        results.push(result);
                        builder.push_str(" = ");
                    }
                }
                match &func.kind {
                    FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                        builder.push_str(&format!(
                            "host.{}({})",
                            func.name.to_snake_case(),
                            operands.join(", "),
                        ));
                    }
                    FunctionKind::Method { name, .. } => {
                        builder.push_str(&format!(
                            "{}.{}({})",
                            operands[0],
                            name.to_snake_case(),
                            operands[1..].join(", "),
                        ));
                    }
                }
                builder.push_str("\n");
            }

            Instruction::Return { amt, .. } => match amt {
                0 => {}
                1 => builder.push_str(&format!("return {}\n", operands[0])),
                _ => {
                    self.src
                        .push_str(&format!("return ({})\n", operands.join(", ")));
                }
            },

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

            Instruction::Malloc {
                realloc,
                size,
                align,
            } => {
                self.needs_realloc = Some(realloc.to_string());
                let ptr = self.locals.tmp("ptr");
                builder.push_str(&format!(
                    "
                        {ptr} = realloc(caller, 0, 0, {align}, {size})
                        assert(isinstance({ptr}, int))
                    ",
                ));
                results.push(ptr);
            }

            i => unimplemented!("{:?}", i),
        }
    }
}

fn py_type_class_of(ty: &Type) -> PyTypeClass {
    match ty {
        Type::Unit => PyTypeClass::None,
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
        Type::Handle(_) | Type::Id(_) => PyTypeClass::Custom,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PyTypeClass {
    None,
    Int,
    Str,
    Float,
    Custom,
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
