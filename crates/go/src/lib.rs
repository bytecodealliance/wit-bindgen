use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::mem;

use anyhow::Result;
use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};

use wit_bindgen_c::{
    c_func_name, find_live_export_types, flags_repr, int_repr, is_arg_by_pointer,
    owner_namespace as c_owner_namespace, push_ty_name,
};
use wit_bindgen_core::wit_parser::{FunctionKind, LiveTypes};
use wit_bindgen_core::wit_parser::{
    Handle::Borrow, Handle::Own, InterfaceId, Resolve, TypeOwner, WorldId,
};
use wit_bindgen_core::Direction;
use wit_bindgen_core::{
    dealias, uwriteln,
    wit_parser::{Field, Function, Handle, SizeAlign, Type, TypeDefKind, TypeId, WorldKey},
    Files, InterfaceGenerator as _, Source, WorldGenerator,
};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(TinyGo {
            _opts: self.clone(),
            ..TinyGo::default()
        })
    }
}

#[derive(Default)]
pub struct TinyGo {
    _opts: Opts,
    src: Source,

    // the parts immediately precede the import of "C"
    preamble: Source,

    world: String,

    // whether the generated code needs to import result and option
    needs_result_option: bool,

    // whether the generated code needs to import "unsafe"
    needs_import_unsafe: bool,

    // whether the generated code needs to import "fmt"
    needs_fmt_import: bool,

    // whether the generated code needs to import "sync"
    needs_sync_import: bool,

    sizes: SizeAlign,

    // mapping from interface ID to the name of the interface
    interface_names: HashMap<InterfaceId, WorldKey>,

    // C type names
    c_type_names: HashMap<TypeId, String>,

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
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: Source::default(),
            preamble: Source::default(),
            gen: self,
            resolve,
            interface: None,
            direction,
            export_funcs: Vec::new(),
            methods: HashMap::new(),
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
        let live_import_types = find_live_export_types(resolve, world);
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

        if self.needs_import_unsafe {
            self.src.push_str("import \"unsafe\"\n");
        }
        if self.needs_fmt_import {
            self.src.push_str("import \"fmt\"\n");
        }
        if self.needs_sync_import {
            self.src.push_str("import \"sync\"\n\n");
        }
        self.src.push_str(&src);

        let world = &resolve.worlds[id];
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            self.src.as_bytes(),
        );
        if self.needs_result_option {
            let mut result_option_src = Source::default();
            uwriteln!(
                result_option_src,
                "package {snake}

                // inspired from https://github.com/moznion/go-optional

                type optionKind int

                const (
                    none optionKind = iota
                    some
                )

                type Option[T any] struct {{
                    kind optionKind
                    val  T
                }}

                // IsNone returns true if the option is None.
                func (o Option[T]) IsNone() bool {{
                    return o.kind == none
                }}

                // IsSome returns true if the option is Some.
                func (o Option[T]) IsSome() bool {{
                    return o.kind == some
                }}

                // Unwrap returns the value if the option is Some.
                func (o Option[T]) Unwrap() T {{
                    if o.kind != some {{
                        panic(\"Option is None\")
                    }}
                    return o.val
                }}

                // Set sets the value and returns it.
                func (o *Option[T]) Set(val T) T {{
                    o.kind = some
                    o.val = val
                    return val
                }}

                // Unset sets the value to None.
                func (o *Option[T]) Unset() {{
                    o.kind = none
                }}

                // Some is a constructor for Option[T] which represents Some.
                func Some[T any](v T) Option[T] {{
                    return Option[T]{{
                        kind: some,
                        val:  v,
                    }}
                }}

                // None is a constructor for Option[T] which represents None.
                func None[T any]() Option[T] {{
                    return Option[T]{{
                        kind: none,
                    }}
                }}

                type ResultKind int

                const (
                    resultOk ResultKind = iota
                    resultErr
                )

                type Result[T any, E any] struct {{
                    kind ResultKind
                    resultOk   T
                    resultErr  E
                }}

                // IsOk returns true if the result is Ok.
                func (r Result[T, E]) IsOk() bool {{
                    return r.kind == resultOk
                }}

                // IsErr returns true if the result is Err.
                func (r Result[T, E]) IsErr() bool {{
                    return r.kind == resultErr
                }}

                // Unwrap returns the value if the result is Ok.
                func (r Result[T, E]) Unwrap() T {{
                    if r.kind != resultOk {{
                        panic(\"Result is Err\")
                    }}
                    return r.resultOk
                }}

                // UnwrapErr returns the value if the result is Err.
                func (r Result[T, E]) UnwrapErr() E {{
                    if r.kind != resultErr {{
                        panic(\"Result is Ok\")
                    }}
                    return r.resultErr
                }}

                // Set sets the value and returns it.
                func (r *Result[T, E]) Set(val T) T {{
                    r.kind = resultOk
                    r.resultOk = val
                    return val
                }}

                // SetErr sets the value and returns it.
                func (r *Result[T, E]) SetErr(val E) E {{
                    r.kind = resultErr
                    r.resultErr = val
                    return val
                }}

                // Ok is a constructor for Result[T, E] which represents Ok.
                func Ok[T any, E any](v T) Result[T, E] {{
                    return Result[T, E]{{
                        kind: resultOk,
                        resultOk:   v,
                    }}
                }}

                // Err is a constructor for Result[T, E] which represents Err.
                func Err[T any, E any](v E) Result[T, E] {{
                    return Result[T, E]{{
                        kind: resultErr,
                        resultErr:  v,
                    }}
                }}
                "
            );
            files.push(
                &format!("{}_types.go", world.name.to_kebab_case()),
                result_option_src.as_bytes(),
            );
        }

        let mut opts = wit_bindgen_c::Opts::default();
        opts.no_sig_flattening = true;
        opts.build()
            .generate(resolve, id, files)
            .expect("C generator should be infallible")
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    preamble: Source,
    gen: &'a mut TinyGo,
    resolve: &'a Resolve,
    interface: Option<(InterfaceId, &'a WorldKey)>,
    direction: Direction,
    export_funcs: Vec<(String, String)>,
    methods: HashMap<TypeId, Vec<(String, String)>>,
}

impl InterfaceGenerator<'_> {
    fn define_interface_types(&mut self, id: InterfaceId) {
        let mut live = LiveTypes::default();
        live.add_interface(self.resolve, id);
        self.define_live_types(&live);
    }

    fn define_function_types(&mut self, funcs: &[(&str, &Function)]) {
        let mut live = LiveTypes::default();
        for (_, func) in funcs {
            live.add_func(self.resolve, func);
        }
        self.define_live_types(&live);
    }

    fn define_live_types(&mut self, live: &LiveTypes) {
        for ty in live.iter() {
            if self.gen.c_type_names.contains_key(&ty) {
                continue;
            }

            // add C type
            let mut name = self.c_owner_namespace(ty);
            name.push_str("_");
            push_ty_name(self.resolve, &Type::Id(ty), &mut name);
            name.push_str("_t");
            let prev = self.gen.c_type_names.insert(ty, name.clone());
            assert!(prev.is_none());

            // add Go types to the list
            let mut name = self.owner_namespace(ty);
            name.push_str(&self.ty_name(&Type::Id(ty)));

            let prev = self.gen.type_names.insert(ty, name.clone());
            assert!(prev.is_none());

            // define Go types
            match &self.resolve.types[ty].name {
                Some(name) => self.define_type(name, ty),
                None => self.anonymous_type(ty),
            }
        }
    }

    /// Given a type ID, returns the namespace of the type.
    fn owner_namespace(&self, id: TypeId) -> String {
        let ty = &self.resolve.types[id];
        match (ty.owner, self.interface) {
            // If this type is owned by an interface, then we must be generating
            // bindings for that interface to proceed.
            (TypeOwner::Interface(a), Some((b, key))) if a == b => self.interface_identifier(key),

            // If this type has no owner then it's an anonymous type. Here it's
            // assigned to whatever we happen to be generating bindings for.
            (TypeOwner::None, Some((_, key))) => self.interface_identifier(key),
            (TypeOwner::None, None) => self.gen.world.to_upper_camel_case(),

            // If this type is owned by a world then we must not be generating
            // bindings for an interface.
            (TypeOwner::World(_), None) => self.gen.world.to_upper_camel_case(),
            (TypeOwner::World(_), Some(_)) => unreachable!(),
            (TypeOwner::Interface(_), None) => unreachable!(),
            (TypeOwner::Interface(_), Some(_)) => unreachable!(),
        }
    }

    fn c_owner_namespace(&self, id: TypeId) -> String {
        c_owner_namespace(
            self.interface,
            matches!(self.direction, Direction::Import),
            self.gen.world.clone(),
            self.resolve,
            id,
        )
    }

    /// Returns the namespace of the current interface.
    ///
    /// If self is not an interface, returns the namespace of the world.
    fn namespace(&self) -> String {
        match self.interface {
            Some((_, key)) => self.interface_identifier(key),
            None => self.gen.world.to_upper_camel_case(),
        }
    }

    /// Returns the identifier of the given interface.
    fn interface_identifier(&self, key: &WorldKey) -> String {
        match key {
            WorldKey::Name(k) => k.to_upper_camel_case(),
            WorldKey::Interface(id) => {
                let mut name = String::new();
                if matches!(self.direction, Direction::Export) {
                    name.push_str("Exports");
                }
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_upper_camel_case());
                name.push_str(&pkg.name.name.to_upper_camel_case());
                if let Some(version) = &pkg.name.version {
                    let version = version
                        .to_string()
                        .replace('.', "_")
                        .replace('-', "_")
                        .replace('+', "_");
                    name.push_str(&version);
                    name.push_str("_");
                }
                name.push_str(&iface.name.as_ref().unwrap().to_upper_camel_case());
                name
            }
        }
    }

    /// Returns the function name of the given function.
    fn func_name(&self, func: &Function) -> String {
        match func.kind {
            FunctionKind::Freestanding => func.name.to_upper_camel_case(),
            FunctionKind::Static(s) => func.name.replace(".", " ").to_upper_camel_case(),
            FunctionKind::Method(_) => match self.direction {
                Direction::Import => func.name.split(".").last().unwrap().to_upper_camel_case(),
                Direction::Export => func.name.replace(".", " ").to_upper_camel_case(),
            },
            FunctionKind::Constructor(id) => match self.direction {
                Direction::Import => {
                    let resource_name = self.resolve.types[id].name.as_deref().unwrap();
                    format!("New{}", resource_name.to_upper_camel_case())
                }
                Direction::Export => func.name.replace(".", " ").to_upper_camel_case(),
            },
        }
    }

    /// Returns the type name of the given type.
    ///
    /// Type name is prefixed with the namespace of the interface.
    /// If convert is true, the type name is converted to upper camel case.
    /// Otherwise, the type name is not converted.
    fn type_name(&self, ty_name: &str, convert: bool) -> String {
        let mut name = String::new();
        let namespace = self.namespace();
        let ty_name = if convert {
            ty_name.to_upper_camel_case()
        } else {
            ty_name.into()
        };
        name.push_str(&namespace);
        name.push_str(&ty_name);
        name
    }

    /// A special variable generated for exported interfaces.
    ///
    /// This variable is used to store the exported interface.
    fn get_interface_var_name(&self) -> String {
        self.namespace().to_snake_case()
    }

    /// Returns the type representation of the given type.
    ///
    /// There are some special cases:
    ///    1. If the type is list, the type representation is `[]<element-type>`.
    ///    2. If the type is option, the type representation is `Option[<element-type>]`.
    ///    3. If the type is result, the type representation is `Result[<ok-type>, <err-type>]`.
    ///
    /// For any other ID type, the type representation is the type name of the ID.
    fn get_ty(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "uint8".into(),
            Type::U16 => "uint16".into(),
            Type::U32 => "uint32".into(),
            Type::U64 => "uint64".into(),
            Type::S8 => "int8".into(),
            Type::S16 => "int16".into(),
            Type::S32 => "int32".into(),
            Type::S64 => "int64".into(),
            Type::Float32 => "float32".into(),
            Type::Float64 => "float64".into(),
            Type::Char => "rune".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &self.resolve().types[*id];
                match &ty.kind {
                    wit_bindgen_core::wit_parser::TypeDefKind::List(ty) => {
                        format!("[]{}", self.get_ty(ty))
                    }
                    wit_bindgen_core::wit_parser::TypeDefKind::Option(o) => {
                        self.gen.needs_result_option = true;
                        format!("Option[{}]", self.get_ty(o))
                    }
                    wit_bindgen_core::wit_parser::TypeDefKind::Result(r) => {
                        self.gen.needs_result_option = true;
                        format!(
                            "Result[{}, {}]",
                            self.optional_ty(r.ok.as_ref()),
                            self.optional_ty(r.err.as_ref())
                        )
                    }
                    _ => self.gen.type_names.get(id).unwrap().to_owned(),
                }
            }
        }
    }

    /// Returns the type name of the given type.
    ///
    /// This function does not prefixed the type name with the namespace of the type owner.
    fn ty_name(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "U8".into(),
            Type::U16 => "U16".into(),
            Type::U32 => "U32".into(),
            Type::U64 => "U64".into(),
            Type::S8 => "S8".into(),
            Type::S16 => "S16".into(),
            Type::S32 => "S32".into(),
            Type::S64 => "S64".into(),
            Type::Float32 => "F32".into(),
            Type::Float64 => "F64".into(),
            Type::Char => "Byte".into(),
            Type::String => "String".into(),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                // if a type has name, return the name
                if let Some(name) = &ty.name {
                    return name.to_upper_camel_case();
                }
                // otherwise, return the anonymous type name
                match &ty.kind {
                    TypeDefKind::Type(t) => self.ty_name(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Resource
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_) => {
                        // these types are not anonymous, and thus have a name
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(t) => {
                        let mut src = String::new();
                        src.push_str("Tuple");
                        src.push_str(&t.types.len().to_string());
                        for ty in t.types.iter() {
                            src.push_str(&self.ty_name(ty));
                        }
                        src.push('T');
                        src
                    }
                    TypeDefKind::Option(t) => {
                        let mut src = String::new();
                        src.push_str("Option");
                        src.push_str(&self.ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Result(r) => {
                        let mut src = String::new();
                        src.push_str("Result");
                        src.push_str(&self.optional_ty_name(r.ok.as_ref()));
                        src.push_str(&self.optional_ty_name(r.ok.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::List(t) => {
                        let mut src = String::new();
                        src.push_str("List");
                        src.push_str(&self.ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Future(t) => {
                        let mut src = String::new();
                        src.push_str("Future");
                        src.push_str(&self.optional_ty_name(t.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Stream(t) => {
                        let mut src = String::new();
                        src.push_str("Stream");
                        src.push_str(&self.optional_ty_name(t.element.as_ref()));
                        src.push_str(&self.optional_ty_name(t.end.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Handle(Handle::Own(ty)) => {
                        // Currently there is no different between Own and Borrow
                        // in the Go code. They are just represented as
                        // the name of the resource type.
                        let mut src = String::new();
                        let ty = &self.resolve.types[*ty];
                        if let Some(name) = &ty.name {
                            src.push_str(&name.to_upper_camel_case());
                        }
                        src
                    }
                    TypeDefKind::Handle(Handle::Borrow(ty)) => {
                        let mut src = String::new();
                        let ty = &self.resolve.types[*ty];
                        if let Some(name) = &ty.name {
                            src.push_str(&name.to_upper_camel_case());
                        }
                        src
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
        }
    }

    /// Used in get_ty_name to get the type name of the given type.
    fn optional_ty_name(&self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.ty_name(ty),
            None => "Empty".into(),
        }
    }

    fn func_params(&mut self, _resolve: &Resolve, func: &Function) -> String {
        let mut params = String::new();
        match func.kind {
            FunctionKind::Method(_) => {
                for (i, (name, param)) in func.params.iter().skip(1).enumerate() {
                    self.get_func_params_common(i, &mut params, name, param);
                }
            }
            _ => {
                for (i, (name, param)) in func.params.iter().enumerate() {
                    self.get_func_params_common(i, &mut params, name, param);
                }
            }
        }

        params
    }

    fn get_func_params_common(
        &mut self,
        i: usize,
        params: &mut String,
        name: &String,
        param: &Type,
    ) {
        if i > 0 {
            params.push_str(", ");
        }

        params.push_str(&avoid_keyword(&name.to_snake_case()));

        params.push(' ');
        params.push_str(&self.get_ty(param));
    }

    fn func_results(&mut self, _resolve: &Resolve, func: &Function) -> String {
        let mut results = String::new();
        results.push(' ');
        match func.results.len() {
            0 => {}
            1 => {
                results.push_str(&self.get_ty(func.results.iter_types().next().unwrap()));
                results.push(' ');
            }
            _ => {
                results.push('(');
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        results.push_str(", ");
                    }
                    results.push_str(&self.get_ty(ty));
                }
                results.push_str(") ");
            }
        }
        results
    }

    fn c_return(&mut self, src: &mut Source, name: &str, param: &Type, direction: Direction) {
        self.c_param(src, name, param, direction);
    }

    fn c_param(&mut self, src: &mut Source, name: &str, param: &Type, direction: Direction) {
        let pointer_prefix = if matches!(direction, Direction::Import) {
            "&"
        } else {
            "*"
        };
        let mut prefix = String::new();
        let mut param_name = String::new();
        let mut postfix = String::new();

        if matches!(direction, Direction::Import) {
            if is_arg_by_pointer(self.resolve, param) {
                prefix.push_str(pointer_prefix);
            }
            if name != "err" && name != "ret" {
                param_name = format!("lower_{name}");
            } else {
                param_name.push_str(name);
            }
        } else {
            postfix.push(' ');
            param_name.push_str(name);

            if is_arg_by_pointer(self.resolve, param) {
                postfix.push_str(pointer_prefix);
            }
            postfix.push_str(&self.gen.get_c_ty(param));
        }
        src.push_str(&format!("{prefix}{param_name}{postfix}"));
    }

    fn c_func_params(
        &mut self,
        params: &mut Source,
        _resolve: &Resolve,
        func: &Function,
        in_import: Direction,
    ) {
        // Append C params to source.
        //
        // If in_import is true, this function is invoked in `import_invoke` which uses `&` to dereference
        // argument of pointer type. The & is added as a prefix to the argument name. And there is no
        // type declaration needed to be added to the argument.
        //
        // If in_import is false, this function is invokved in printing export function signature.
        // It uses the form of `<param-name> *C.<param-type>` to print each parameter in the function, where
        // * is only used if the parameter is of pointer type.
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            self.c_param(
                params,
                &avoid_keyword(&name.to_snake_case()),
                param,
                in_import,
            );
        }
    }

    fn c_func_returns(
        &mut self,
        src: &mut Source,
        _resolve: &Resolve,
        func: &Function,
        direction: Direction,
    ) {
        let add_param_seperator = |src: &mut Source| {
            if !func.params.is_empty() {
                src.push_str(", ");
            }
        };
        match func.results.len() {
            0 => {
                // no return
                src.push_str(")");
            }
            1 => {
                // one return
                let return_ty = func.results.iter_types().next().unwrap();
                if is_arg_by_pointer(self.resolve, return_ty) {
                    add_param_seperator(src);
                    self.c_return(src, "ret", return_ty, direction);
                    src.push_str(")");
                } else {
                    src.push_str(")");
                    if matches!(direction, Direction::Export) {
                        src.push_str(&format!(" {ty}", ty = self.gen.get_c_ty(return_ty)));
                    }
                }
            }
            _n => {
                // multi-return
                add_param_seperator(src);
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        src.push_str(", ");
                    }
                    if matches!(direction, Direction::Import) {
                        src.push_str(&format!("&ret{i}"));
                    } else {
                        src.push_str(&format!("ret{i} *{ty}", i = i, ty = self.gen.get_c_ty(ty)));
                    }
                }
                src.push_str(")");
            }
        }
    }

    fn c_func_sig(&mut self, resolve: &Resolve, func: &Function, direction: Direction) -> String {
        let mut src = Source::default();
        let func_name = if matches!(direction, Direction::Import) {
            c_func_name(
                matches!(direction, Direction::Import),
                self.resolve,
                &self.gen.world,
                self.interface.map(|(_, key)| key),
                func,
            )
        } else {
            // do not want to generate public functions
            format!("{}{}", self.namespace(), self.func_name(func)).to_lower_camel_case()
        };

        if matches!(direction, Direction::Export) {
            src.push_str("func ");
        } else {
            src.push_str("C.");
        }
        src.push_str(&func_name);
        src.push_str("(");

        // prepare args
        self.c_func_params(&mut src, resolve, func, direction);

        // prepare returns
        self.c_func_returns(&mut src, resolve, func, direction);
        src.to_string()
    }

    fn free_c_arg(&mut self, ty: &Type, arg: &str) -> String {
        let mut ty_name = self.gen.get_c_ty(ty);
        let it: Vec<&str> = ty_name.split('_').collect();
        ty_name = it[..it.len() - 1].join("_");
        format!("defer {ty_name}_free({arg})\n")
    }

    // This is useful in defining functions in the exported interface that the guest needs to implement
    fn func_sig_with_no_namespace(&mut self, resolve: &Resolve, func: &Function) -> String {
        format!(
            "{}({}){}",
            self.func_name(func),
            self.func_params(resolve, func),
            self.func_results(resolve, func)
        )
    }

    fn func_sig(&mut self, resolve: &Resolve, func: &Function) {
        self.src.push_str("func ");

        match func.kind {
            FunctionKind::Freestanding => {
                let namespace = self.namespace();
                self.src.push_str(&namespace);
            }
            FunctionKind::Method(ty) => {
                let ty = self.get_ty(&Type::Id(ty));
                self.src.push_str(&format!("(self {ty}) ", ty = ty));
            }
            _ => {}
        }
        let func_sig = self.func_sig_with_no_namespace(resolve, func);
        self.src.push_str(&func_sig);
        self.src.push_str("{\n");
    }

    fn field_name(&mut self, field: &Field) -> String {
        field.name.to_upper_camel_case()
    }

    fn extract_result_ty(&self, ty: &Type) -> (Option<Type>, Option<Type>) {
        //TODO: don't copy from the C code
        // optimization on the C size.
        // See https://github.com/bytecodealliance/wit-bindgen/pull/450
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Result(r) => (r.ok, r.err),
                _ => (None, None),
            },
            _ => (None, None),
        }
    }

    fn extract_list_ty(&self, ty: &Type) -> Option<&Type> {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::List(l) => Some(l),
                _ => None,
            },
            _ => None,
        }
    }

    fn is_empty_tuple_ty(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Tuple(t) => t.types.is_empty(),
                _ => false,
            },
            _ => false,
        }
    }

    fn optional_ty(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty(ty),
            None => "struct{}".into(),
        }
    }

    fn anonymous_type(&mut self, ty: TypeId) {
        let kind = &self.resolve.types[ty].kind;
        match kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_) => {
                // no anonymous type for these types
                unreachable!()
            }
            TypeDefKind::Tuple(t) => {
                let ty_name = self.ty_name(&Type::Id(ty));
                let name = self.type_name(&ty_name, false);

                self.src.push_str(&format!("type {name} struct {{\n",));
                for (i, ty) in t.types.iter().enumerate() {
                    let ty = self.get_ty(ty);
                    self.src.push_str(&format!("   F{i} {ty}\n",));
                }
                self.src.push_str("}\n\n");
            }
            TypeDefKind::Option(_) | TypeDefKind::Result(_) | TypeDefKind::List(_) => {
                // no anonymous type needs to be generated here because we are using
                // Option[T], Result[T, E], and []T in Go
            }
            TypeDefKind::Handle(_) => {
                // although handles are anonymous types, they are generated in the
                // `type_resource` function as part of the resource type generation.
            }
            TypeDefKind::Future(_) => todo!("anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("anonymous_type for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn print_constructor_method_without_value(&mut self, name: &str, case_name: &str) {
        uwriteln!(
            self.src,
            "func {name}{case_name}() {name} {{
                return {name}{{kind: {name}Kind{case_name}}}
            }}
            ",
        );
    }

    fn print_accessor_methods(&mut self, name: &str, case_name: &str, ty: &Type) {
        self.gen.needs_fmt_import = true;
        let ty = self.get_ty(ty);
        uwriteln!(
            self.src,
            "func {name}{case_name}(v {ty}) {name} {{
                return {name}{{kind: {name}Kind{case_name}, val: v}}
            }}
            ",
        );
        uwriteln!(
            self.src,
            "func (n {name}) Get{case_name}() {ty} {{
                if g, w := n.Kind(), {name}Kind{case_name}; g != w {{
                    panic(fmt.Sprintf(\"Attr kind is %v, not %v\", g, w))
                }}
                return n.val.({ty})
            }}
            ",
        );
        uwriteln!(
            self.src,
            "func (n *{name}) Set{case_name}(v {ty}) {{
                n.val = v
                n.kind = {name}Kind{case_name}
            }}
            ",
        );
    }

    fn print_kind_method(&mut self, name: &str) {
        uwriteln!(
            self.src,
            "func (n {name}) Kind() {name}Kind {{
                return n.kind
            }}
            "
        );
    }

    fn print_variant_field(&mut self, name: &str, case_name: &str, i: usize) {
        if i == 0 {
            self.src
                .push_str(&format!("   {name}Kind{case_name} {name}Kind = iota\n",));
        } else {
            self.src.push_str(&format!("   {name}Kind{case_name}\n",));
        }
    }

    fn import(&mut self, resolve: &Resolve, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        func_bindgen.process_args();
        func_bindgen.process_returns();
        let ret = func_bindgen.args;
        let lower_src = func_bindgen.lower_src;
        let lift_src = func_bindgen.lift_src;

        // // print function signature
        self.func_sig(resolve, func);

        // body
        // prepare args
        self.src.push_str(&lower_src);

        self.import_invoke(resolve, func, &lift_src, ret);

        // return

        self.src.push_str("}\n\n");
    }

    fn import_invoke(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        lift_src: &Source,
        ret: Vec<String>,
    ) {
        let invoke = self.c_func_sig(resolve, func, Direction::Import);
        match func.results.len() {
            0 => {
                self.src.push_str(&invoke);
                self.src.push_str("\n");
            }
            1 => {
                let return_ty = func.results.iter_types().next().unwrap();
                if is_arg_by_pointer(self.resolve, return_ty) {
                    let c_ret_type = self.gen.get_c_ty(return_ty);
                    self.src.push_str(&format!("var ret {c_ret_type}\n"));
                    self.src.push_str(&invoke);
                    self.src.push_str("\n");
                } else {
                    self.src.push_str(&format!("ret := {invoke}\n"));
                }
                self.src.push_str(lift_src);
                self.src.push_str(&format!("return {ret}\n", ret = ret[0]));
            }
            _n => {
                for (i, ty) in func.results.iter_types().enumerate() {
                    let ty_name = self.gen.get_c_ty(ty);
                    let var_name = format!("ret{i}");
                    self.src.push_str(&format!("var {var_name} {ty_name}\n"));
                }
                self.src.push_str(&invoke);
                self.src.push_str("\n");
                self.src.push_str(lift_src);
                self.src.push_str("return ");
                for (i, _) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.src.push_str(&format!("lift_ret{i}"));
                }
                self.src.push_str("\n");
            }
        }
    }

    fn export(&mut self, resolve: &Resolve, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        func_bindgen.process_args();
        func_bindgen.process_returns();

        let args = func_bindgen.args;
        let ret = func_bindgen.c_args;
        let lift_src = func_bindgen.lift_src;
        let lower_src = func_bindgen.lower_src;

        // This variable holds the declaration functions in the exported interface that user
        // needs to implement.
        let interface_method_decl = self.func_sig_with_no_namespace(resolve, func);
        let export_func = {
            let mut src = String::new();
            // header
            src.push_str("//export ");
            let name = c_func_name(
                matches!(self.direction, Direction::Import),
                self.resolve,
                &self.gen.world,
                self.interface.map(|(_, key)| key),
                func,
            );
            src.push_str(&name);
            src.push('\n');

            // signature
            src.push_str(&self.c_func_sig(resolve, func, Direction::Export));
            src.push_str(" {\n");

            // free all the parameters
            for (name, ty) in func.params.iter() {
                // TODO: should test if owns anything
                if false {
                    let free = self.free_c_arg(ty, &avoid_keyword(&name.to_snake_case()));
                    src.push_str(&free);
                }
            }

            // prepare args

            src.push_str(&lift_src);

            // invoke
            let invoke = match func.kind {
                FunctionKind::Method(_) => {
                    format!(
                        "lift_self.{}({})",
                        self.func_name(func),
                        args.iter()
                            .enumerate()
                            .skip(1)
                            .map(|(i, name)| format!(
                                "{}{}",
                                name,
                                if i < func.params.len() - 1 { ", " } else { "" }
                            ))
                            .collect::<String>()
                    )
                }
                _ => format!(
                    "{}.{}({})",
                    &self.get_interface_var_name(),
                    self.func_name(func),
                    args.iter()
                        .enumerate()
                        .map(|(i, name)| format!(
                            "{}{}",
                            name,
                            if i < func.params.len() - 1 { ", " } else { "" }
                        ))
                        .collect::<String>()
                ),
            };

            // prepare ret
            match func.results.len() {
                0 => {
                    src.push_str(&format!("{invoke}\n"));
                }
                1 => {
                    let return_ty = func.results.iter_types().next().unwrap();
                    src.push_str(&format!("result := {invoke}\n"));
                    src.push_str(&lower_src);

                    let lower_result = &ret[0];
                    if is_arg_by_pointer(self.resolve, return_ty) {
                        src.push_str(&format!("*ret = {lower_result}\n"));
                    } else {
                        src.push_str(&format!("return {ret}\n", ret = &ret[0]));
                    }
                }
                _ => {
                    for i in 0..func.results.len() {
                        if i > 0 {
                            src.push_str(", ")
                        }
                        src.push_str(&format!("result{i}"));
                    }
                    src.push_str(&format!(" := {invoke}\n"));
                    src.push_str(&lower_src);
                    for (i, lower_result) in ret.iter().enumerate() {
                        src.push_str(&format!("*ret{i} = {lower_result}\n"));
                    }
                }
            };

            src.push_str("\n}\n");
            src
        };

        match func.kind {
            FunctionKind::Method(id) => {
                self.methods
                    .entry(id)
                    .or_default()
                    .push((interface_method_decl, export_func));
            }
            _ => {
                self.export_funcs.push((interface_method_decl, export_func));
            }
        }
    }

    fn finish(&mut self) {
        if !self.export_funcs.is_empty() {
            let interface_var_name = &self.get_interface_var_name();
            let interface_name = &self.namespace();

            self.src
                .push_str(format!("var {interface_var_name} {interface_name} = nil\n").as_str());
            uwriteln!(self.src,
                "// `Set{interface_name}` sets the `{interface_name}` interface implementation.
                // This function will need to be called by the init() function from the guest application.
                // It is expected to pass a guest implementation of the `{interface_name}` interface."
            );
            self.src.push_str(
                format!(
                    "func Set{interface_name}(i {interface_name}) {{\n    {interface_var_name} = i\n}}\n"
                )
                .as_str(),
            );

            self.print_export_interface();

            // print resources and methods

            for id in &self.gen.exported_resources {
                // generate an interface that contains all the methods
                // that the guest code needs to implement.
                let ty_name = self.gen.type_names.get(id).unwrap();

                self.src.push_str(&format!("type {ty_name} interface {{\n"));
                if self.methods.get(id) == None {
                    // if this resource has no methods, generate an empty interface
                    // note that constructor and static methods are included in the
                    // top level interface definition.
                    self.src.push_str("}\n\n");
                } else {
                    // otherwise, generate an interface that contains all the methods
                    for (interface_func_declaration, _) in &self.methods[id] {
                        self.src
                            .push_str(format!("{interface_func_declaration}\n").as_str());
                    }
                    self.src.push_str("}\n\n");

                    // generate each method as a private export function
                    for (_, export_func) in &self.methods[id] {
                        self.src.push_str(export_func);
                    }
                }
            }

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(export_func);
            }
        }
    }

    fn print_export_interface(&mut self) {
        let interface_name = &self.namespace();
        self.src
            .push_str(format!("type {interface_name} interface {{\n").as_str());
        for (interface_func_declaration, _) in &self.export_funcs {
            self.src
                .push_str(format!("{interface_func_declaration}\n").as_str());
        }
        self.src.push_str("}\n");
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        record: &wit_bindgen_core::wit_parser::Record,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for field in record.fields.iter() {
            let ty = self.get_ty(&field.ty);
            let name = self.field_name(field);
            self.src.push_str(&format!("   {name} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_resource(
        &mut self,
        id: TypeId,
        name: &str,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let type_name = self.type_name(name, true);
        // for imports, generate a `int32` type for resource handle representation.
        // for exports, generate a map to store unique IDs of resources to their
        // resource interfaces, which are implemented by guest code.
        if matches!(self.direction, Direction::Import) {
            self.src.push_str(&format!(
                "// {type_name} is a handle to imported resource {name}\n"
            ));
            self.src.push_str(&format!("type {type_name} int32\n\n"));
        } else {
            // generate a typedef struct for export resource
            let c_typedef_target = self.gen.c_type_names[&id].clone();

            self.preamble
                .push_str(&format!("// typedef struct {c_typedef_target} "));
            self.preamble.push_str("{");
            self.preamble.deindent(1);
            self.preamble.push_str("\n");
            self.preamble.push_str("//  int32_t __handle; \n");
            self.preamble.push_str("// ");
            self.preamble.push_str("} ");
            self.preamble.push_str(&c_typedef_target);
            self.preamble.push_str(";\n");

            // import "sync" for Mutex
            self.gen.needs_sync_import = true;
            self.src
                .push_str(&format!("// resource {type_name} internal bookkeeping"));
            let private_type_name = type_name.to_snake_case();
            uwriteln!(
                self.src,
                "
                var (
                    {private_type_name}_pointers = make(map[int32]{type_name})
                    {private_type_name}_next_id int32 = 0
                    {private_type_name}_mu sync.Mutex
                )
                "
            );

            // generate dtors for exported resources
            let namespace = self.c_owner_namespace(id);
            let snake = name.to_snake_case();
            let func_name = format!("{}_{}", namespace, snake).to_lower_camel_case();
            let private_type_name = type_name.to_snake_case();
            self.src
                .push_str(&format!("//export {namespace}_{snake}_destructor\n"));
            uwriteln!(
                self.src,
                "func {func_name}Destructor(self *C.{c_typedef_target}) {{
                    C.free(unsafe.Pointer(self))
                    delete({private_type_name}_pointers, int32(self.__handle))
                }}
                ",
            );

            self.gen.needs_import_unsafe = true;

            // book keep the exported resource type
            self.gen.exported_resources.insert(id);
        }
    }

    fn type_flags(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        flags: &wit_bindgen_core::wit_parser::Flags,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);

        // TODO: use flags repr to determine how many flags are needed
        self.src.push_str(&format!("type {name} uint64\n"));
        self.src.push_str("const (\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            if i == 0 {
                self.src.push_str(&format!(
                    "   {name}_{flag} {name} = 1 << iota\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            } else {
                self.src.push_str(&format!(
                    "   {name}_{flag}\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            }
        }
        self.src.push_str(")\n\n");
    }

    fn type_tuple(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        tuple: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for (i, case) in tuple.types.iter().enumerate() {
            let ty = self.get_ty(case);
            self.src.push_str(&format!("F{i} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_variant(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        variant: &wit_bindgen_core::wit_parser::Variant,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str("const (\n");

        for (i, case) in variant.cases.iter().enumerate() {
            let case_name = case.name.to_upper_camel_case();
            self.print_variant_field(&name, &case_name, i);
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str("val any\n");
        self.src.push_str("}\n\n");

        self.print_kind_method(&name);

        for case in variant.cases.iter() {
            let case_name = case.name.to_upper_camel_case();
            if let Some(ty) = case.ty.as_ref() {
                self.gen.needs_fmt_import = true;
                self.print_accessor_methods(&name, &case_name, ty);
            } else {
                self.print_constructor_method_without_value(&name, &case_name);
            }
        }
    }

    fn type_option(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        self.get_ty(&Type::Id(id));
    }

    fn type_result(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        self.get_ty(&Type::Id(id));
    }

    fn type_enum(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        enum_: &wit_bindgen_core::wit_parser::Enum,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str("const (\n");

        for (i, case) in enum_.cases.iter().enumerate() {
            let case_name = case.name.to_upper_camel_case();
            self.print_variant_field(&name, &case_name, i);
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str("}\n\n");

        self.print_kind_method(&name);

        for case in enum_.cases.iter() {
            let case_name = case.name.to_upper_camel_case();
            self.print_constructor_method_without_value(&name, &case_name);
        }
    }

    fn type_alias(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_list(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = self.type_name(name, true);
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_builtin(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!("type_builtin")
    }
}

struct FunctionBindgen<'a, 'b> {
    interface: &'a mut InterfaceGenerator<'b>,
    func: &'a Function,
    c_args: Vec<String>,
    args: Vec<String>,
    lower_src: Source,
    lift_src: Source,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(interface: &'a mut InterfaceGenerator<'b>, func: &'a Function) -> Self {
        Self {
            interface,
            func: func,
            c_args: Vec::new(),
            args: Vec::new(),
            lower_src: Source::default(),
            lift_src: Source::default(),
        }
    }

    fn process_args(&mut self) {
        self.func
            .params
            .iter()
            .for_each(|(name, ty)| match self.interface.direction {
                Direction::Import => self.lower(&avoid_keyword(&name.to_snake_case()), ty),
                Direction::Export => self.lift(&avoid_keyword(&name.to_snake_case()), ty),
            });
    }

    fn process_returns(&mut self) {
        match self.func.results.len() {
            0 => {}
            1 => {
                let ty = self.func.results.iter_types().next().unwrap();
                match self.interface.direction {
                    Direction::Import => self.lift("ret", ty),
                    Direction::Export => self.lower("result", ty),
                }
            }
            _ => {
                for (i, ty) in self.func.results.iter_types().enumerate() {
                    match self.interface.direction {
                        Direction::Import => self.lift(&format!("ret{i}"), ty),
                        Direction::Export => self.lower(&format!("result{i}"), ty),
                    }
                }
            }
        };
    }

    fn lower(&mut self, name: &str, ty: &Type) {
        let lower_name = format!("lower_{name}");
        self.lower_value(name, ty, lower_name.as_ref());

        // Check whether or not the C variable needs to be freed.
        // If this variable is in export function, which will be returned to host to use.
        //    There is no need to free return variables.
        // If this variable does not own anything, it does not need to be freed.
        // If this variable is in inner node of the recursive call, no need to be freed.
        //    This is because the root node's call to free will recursively free the whole tree.
        // Otherwise, free this variable.
        //
        // TODO: should test if free is necessary
        if matches!(self.interface.direction, Direction::Import) && false {
            self.lower_src
                .push_str(&self.interface.free_c_arg(ty, &format!("&{lower_name}")));
        }
        self.c_args.push(lower_name);
    }

    fn lower_list_value(&mut self, param: &str, l: &Type, lower_name: &str) {
        let list_ty = self.interface.gen.get_c_ty(l);
        uwriteln!(
            self.lower_src,
            "if len({param}) == 0 {{
                {lower_name}.ptr = nil
                {lower_name}.len = 0
            }} else {{
                var empty_{lower_name} {list_ty}
                {lower_name}.ptr = (*{list_ty})(C.malloc(C.size_t(len({param})) * C.size_t(unsafe.Sizeof(empty_{lower_name}))))
                {lower_name}.len = C.size_t(len({param}))"
        );

        uwriteln!(self.lower_src, "for {lower_name}_i := range {param} {{");
        uwriteln!(self.lower_src,
            "{lower_name}_ptr := (*{list_ty})(unsafe.Pointer(uintptr(unsafe.Pointer({lower_name}.ptr)) +
            uintptr({lower_name}_i)*unsafe.Sizeof(empty_{lower_name})))"
        );

        let param = &format!("{param}[{lower_name}_i]");
        let lower_name = &format!("{lower_name}_ptr");

        if let Some(inner) = self.interface.extract_list_ty(l) {
            self.lower_list_value(param, &inner.clone(), lower_name);
        } else {
            self.lower_value(param, l, &format!("{lower_name}_value"));
            uwriteln!(self.lower_src, "*{lower_name} = {lower_name}_value");
        }

        uwriteln!(self.lower_src, "}}");
        uwriteln!(self.lower_src, "}}");
    }

    fn lower_result_value(
        &mut self,
        param: &str,
        ty: &Type,
        lower_name: &str,
        lower_inner_name1: &str,
        lower_inner_name2: &str,
    ) {
        // lower_inner_name could be {lower_name}.val if it's used in import
        // else, it could be either ret or err

        let (ok, err) = self.interface.extract_result_ty(ty);
        uwriteln!(self.lower_src, "if {param}.IsOk() {{");
        if let Some(ok_inner) = ok {
            self.interface.gen.needs_import_unsafe = true;
            let c_target_name = self.interface.gen.get_c_ty(&ok_inner);
            uwriteln!(
                self.lower_src,
                "{lower_name}_ptr := (*{c_target_name})(unsafe.Pointer({lower_inner_name1}))"
            );
            self.lower_value(
                &format!("{param}.Unwrap()"),
                &ok_inner,
                &format!("{lower_name}_val"),
            );
            uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
        }
        self.lower_src.push_str("} else {\n");
        if let Some(err_inner) = err {
            self.interface.gen.needs_import_unsafe = true;
            let c_target_name = self.interface.gen.get_c_ty(&err_inner);
            uwriteln!(
                self.lower_src,
                "{lower_name}_ptr := (*{c_target_name})(unsafe.Pointer({lower_inner_name2}))"
            );
            self.lower_value(
                &format!("{param}.UnwrapErr()"),
                &err_inner,
                &format!("{lower_name}_val"),
            );
            uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
        }
        self.lower_src.push_str("}\n");
    }

    /// Lower a value to a string representation.
    ///
    /// # Parameters
    ///
    /// * `param` - The string representation of the parameter of a function
    /// * `ty` - A reference to a `Type` that specifies the type of the value.
    /// * `lower_name` - A reference to a string that represents the name to be used for the lower value.
    fn lower_value(&mut self, param: &str, ty: &Type, lower_name: &str) {
        match ty {
            Type::Bool => {
                uwriteln!(self.lower_src, "{lower_name} := {param}",);
            }
            Type::String => {
                self.interface.gen.needs_import_unsafe = true;
                uwriteln!(
                    self.lower_src,
                    "var {lower_name} {value}",
                    value = self.interface.gen.get_c_ty(ty),
                );
                uwriteln!(
                    self.lower_src,
                    "
                    // use unsafe.Pointer to avoid copy
                    {lower_name}.ptr = (*uint8)(unsafe.Pointer(C.CString({param})))
                    {lower_name}.len = C.size_t(len({param}))"
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.resolve.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for field in r.fields.iter() {
                            let c_field_name = &self.get_c_field_name(field);
                            let field_name = &self.interface.field_name(field);

                            self.lower_value(
                                &format!("{param}.{field_name}"),
                                &field.ty,
                                &format!("{lower_name}_{c_field_name}"),
                            );
                            uwriteln!(
                                self.lower_src,
                                "{lower_name}.{c_field_name} = {lower_name}_{c_field_name}"
                            )
                        }
                    }

                    TypeDefKind::Flags(f) => {
                        let int_repr = int_repr(flags_repr(f));
                        uwriteln!(self.lower_src, "{lower_name} := C.{int_repr}({param})");
                    }
                    TypeDefKind::Tuple(t) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, ty) in t.types.iter().enumerate() {
                            self.lower_value(
                                &format!("{param}.F{i}"),
                                ty,
                                &format!("{lower_name}_f{i}"),
                            );
                            uwriteln!(self.lower_src, "{lower_name}.f{i} = {lower_name}_f{i}");
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        uwriteln!(self.lower_src, "if {param}.IsSome() {{");
                        self.lower_value(
                            &format!("{param}.Unwrap()"),
                            o,
                            &format!("{lower_name}_val"),
                        );
                        uwriteln!(self.lower_src, "{lower_name}.val = {lower_name}_val");
                        uwriteln!(self.lower_src, "{lower_name}.is_some = true");
                        self.lower_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));

                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        uwriteln!(self.lower_src, "{lower_name}.is_err = {param}.IsErr()");
                        let inner_name = format!("&{lower_name}.val");
                        self.lower_result_value(
                            param,
                            &Type::Id(*id),
                            lower_name,
                            &inner_name,
                            &inner_name,
                        );
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));

                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        self.lower_list_value(param, l, lower_name);
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lower_src,
                            "var {lower_name} {value}",
                            value = self.interface.gen.get_c_ty(t),
                        );
                        self.lower_value(param, t, &format!("{lower_name}_val"));
                        uwriteln!(self.lower_src, "{lower_name} = {lower_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;

                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            if let Some(ty) = case.ty.as_ref() {
                                let name = self.interface.gen.get_c_ty(ty);
                                uwriteln!(
                                    self.lower_src,
                                    "
                                    {lower_name}.tag = {i}
                                    {lower_name}_ptr := (*{name})(unsafe.Pointer(&{lower_name}.val))"
                                );
                                self.lower_value(
                                    &format!("{param}.Get{case_name}()"),
                                    ty,
                                    &format!("{lower_name}_val"),
                                );
                                uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
                            } else {
                                uwriteln!(self.lower_src, "{lower_name}.tag = {i}");
                            }
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            uwriteln!(self.lower_src, "{lower_name} = {i}");
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(h) => {
                        match self.interface.direction {
                            Direction::Import => {
                                let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                                uwriteln!(
                                    self.lower_src,
                                    "var {lower_name} {c_typedef_target}
                                        {lower_name}.__handle = C.int32_t({param})"
                                )
                            }
                            Direction::Export => {
                                let resource = dealias(
                                    self.interface.resolve,
                                    *match h {
                                        Handle::Borrow(resource) => resource,
                                        Handle::Own(resource) => resource,
                                    },
                                );
                                // If the resource is exported, then `type_resource` have created a
                                // internal bookkeeping map for this resource. We will need to
                                // use the map to get the resource interface. Otherwise, this resource
                                // is imported, and we will use the generated i32 handle directly.
                                if self.interface.gen.exported_resources.contains(&resource) {
                                    let resource = dealias(self.interface.resolve, resource);
                                    let c_typedef_target =
                                        self.interface.gen.get_c_ty(&Type::Id(resource));
                                    let ty_name = self.interface.gen.type_names[&resource].clone();
                                    let private_type_name = ty_name.to_snake_case();
                                    self.interface.gen.needs_import_unsafe = true;
                                    uwriteln!(self.lower_src,
                                        "{private_type_name}_mu.Lock()
                                        {private_type_name}_next_id += 1
                                        {private_type_name}_pointers[{private_type_name}_next_id] = {param}
                                        {private_type_name}_mu.Unlock()
                                        {param}_c := (*{c_typedef_target})(unsafe.Pointer(C.malloc(C.size_t(unsafe.Sizeof({c_typedef_target}{{}})))))
                                        {param}_c.__handle = C.int32_t({private_type_name}_next_id)
                                        lower_{param} := C.{private_type_name}_new({param}_c) // pass the pointer directly"
                                    );
                                } else {
                                    // need to construct either an own or borrowed C handle type.
                                    let ns = self.interface.c_owner_namespace(*id);
                                    let snake = self.interface.resolve.types[resource]
                                        .name
                                        .as_ref()
                                        .unwrap();
                                    let c_typedef_target = match h {
                                        Own(_) => {
                                            let mut own = ns.clone();
                                            own.push_str("_own_");
                                            own.push_str(&snake);
                                            own.push_str("_t");
                                            own
                                        }
                                        Borrow(_) => {
                                            let mut borrow = ns.clone();
                                            borrow.push_str("_borrow_");
                                            borrow.push_str(&snake);
                                            borrow.push_str("_t");
                                            borrow
                                        }
                                    };
                                    uwriteln!(
                                        self.lower_src,
                                        "var {lower_name} C.{c_typedef_target}
                                        {lower_name}.__handle = C.int32_t({param})"
                                    );
                                }
                            }
                        }
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
            a => {
                uwriteln!(
                    self.lower_src,
                    "{lower_name} := {c_type_name}({param_name})",
                    c_type_name = self.interface.gen.get_c_ty(a),
                    param_name = param,
                );
            }
        }
    }

    fn lift(&mut self, name: &str, ty: &Type) {
        let lift_name = format!("lift_{name}");
        self.lift_value(name, ty, lift_name.as_str());
        self.args.push(lift_name);
    }

    fn lift_value(&mut self, param: &str, ty: &Type, lift_name: &str) {
        match ty {
            Type::Bool => {
                uwriteln!(self.lift_src, "{lift_name} := {param}");
            }
            Type::String => {
                self.interface.gen.needs_import_unsafe = true;
                uwriteln!(
                    self.lift_src,
                    "var {name} {value}
                    {lift_name} = C.GoStringN((*C.char)(unsafe.Pointer({param}.ptr)), C.int({param}.len))",
                    name = lift_name,
                    value = self.interface.get_ty(ty),
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.resolve.types[*id]; // receive type
                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for field in r.fields.iter() {
                            let field_name = &self.interface.field_name(field);
                            let c_field_name = &self.get_c_field_name(field);
                            self.lift_value(
                                &format!("{param}.{c_field_name}"),
                                &field.ty,
                                &format!("{lift_name}_{field_name}"),
                            );
                            uwriteln!(
                                self.lift_src,
                                "{lift_name}.{field_name} = {lift_name}_{field_name}"
                            );
                        }
                    }
                    TypeDefKind::Flags(_f) => {
                        let field = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}
                            {lift_name} = {field}({param})",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                    }
                    TypeDefKind::Tuple(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for (i, t) in t.types.iter().enumerate() {
                            self.lift_value(
                                &format!("{param}.f{i}"),
                                t,
                                &format!("{lift_name}_F{i}"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.F{i} = {lift_name}_F{i}");
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        uwriteln!(self.lift_src, "if {param}.is_some {{");
                        self.lift_value(&format!("{param}.val"), o, &format!("{lift_name}_val"));

                        uwriteln!(self.lift_src, "{lift_name}.Set({lift_name}_val)");
                        self.lift_src.push_str("} else {\n");
                        uwriteln!(self.lift_src, "{lift_name}.Unset()");
                        self.lift_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        self.interface.gen.needs_result_option = true;
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        let (ok, err) = self.interface.extract_result_ty(&Type::Id(*id));

                        // normal result route
                        uwriteln!(self.lift_src, "if {param}.is_err {{");
                        if let Some(err_inner) = err {
                            let err_inner_name = self.interface.gen.get_c_ty(&err_inner);
                            self.interface.gen.needs_import_unsafe = true;
                            uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{err_inner_name})(unsafe.Pointer(&{param}.val))");
                            self.lift_value(
                                &format!("{lift_name}_ptr"),
                                &err_inner,
                                &format!("{lift_name}_val"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.SetErr({lift_name}_val)")
                        } else {
                            uwriteln!(self.lift_src, "{lift_name}.SetErr(struct{{}}{{}})")
                        }
                        uwriteln!(self.lift_src, "}} else {{");
                        if let Some(ok_inner) = ok {
                            let ok_inner_name = self.interface.gen.get_c_ty(&ok_inner);
                            self.interface.gen.needs_import_unsafe = true;
                            uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{ok_inner_name})(unsafe.Pointer(&{param}.val))");
                            self.lift_value(
                                &format!("{lift_name}_ptr"),
                                &ok_inner,
                                &format!("{lift_name}_val"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.Set({lift_name}_val)")
                        }
                        uwriteln!(self.lift_src, "}}");
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let list_ty = self.interface.get_ty(&Type::Id(*id));
                        let c_ty_name = self.interface.gen.get_c_ty(l);
                        uwriteln!(self.lift_src, "var {lift_name} {list_ty}",);
                        uwriteln!(self.lift_src, "{lift_name} = make({list_ty}, {param}.len)");
                        uwriteln!(self.lift_src, "if {param}.len > 0 {{");
                        uwriteln!(self.lift_src, "for {lift_name}_i := 0; {lift_name}_i < int({param}.len); {lift_name}_i++ {{");
                        uwriteln!(self.lift_src, "var empty_{lift_name} {c_ty_name}");
                        uwriteln!(
                            self.lift_src,
                            "{lift_name}_ptr := *(*{c_ty_name})(unsafe.Pointer(uintptr(unsafe.Pointer({param}.ptr)) +
                            uintptr({lift_name}_i)*unsafe.Sizeof(empty_{lift_name})))"
                        );

                        // If l is an empty tuple, set _ = {lift_name}_ptr
                        // this is a special case needs to be handled
                        if self.interface.is_empty_tuple_ty(l) {
                            uwriteln!(self.lift_src, "_ = {lift_name}_ptr");
                        }

                        self.lift_value(
                            &format!("{lift_name}_ptr"),
                            l,
                            &format!("list_{lift_name}"),
                        );

                        uwriteln!(
                            self.lift_src,
                            "{lift_name}[{lift_name}_i] = list_{lift_name}"
                        );
                        self.lift_src.push_str("}\n");
                        self.lift_src.push_str("}\n");
                        // TODO: don't forget to free `ret`
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {lift_name} {ty_name}",
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        self.lift_value(param, t, &format!("{lift_name}_val"));
                        uwriteln!(self.lift_src, "{lift_name} = {lift_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let ty_name: String = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            if let Some(ty) = case.ty.as_ref() {
                                let c_ty_name = self.interface.gen.get_c_ty(ty);
                                uwriteln!(
                                    self.lift_src,
                                    "{lift_name}_ptr := *(*{c_ty_name})(unsafe.Pointer(&{param}.val))"
                                );
                                self.lift_value(
                                    &format!("{lift_name}_ptr"),
                                    ty,
                                    &format!("{lift_name}_val"),
                                );
                                uwriteln!(
                                    self.lift_src,
                                    "{lift_name} = {ty_name}{case_name}({lift_name}_val)"
                                )
                            } else {
                                uwriteln!(self.lift_src, "{lift_name} = {ty_name}{case_name}()");
                            }
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(self.lift_src, "if {param} == {i} {{");
                            uwriteln!(self.lift_src, "{lift_name} = {ty_name}{case_name}()");
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(h) => {
                        match self.interface.direction {
                            Direction::Import => {
                                let ty_name = self.interface.get_ty(&Type::Id(*id));
                                uwriteln!(
                                    self.lift_src,
                                    "var {lift_name} {ty_name}
                                    {lift_name} = {ty_name}({param}.__handle)
                                    "
                                );
                            }
                            Direction::Export => {
                                // If the resource is exported, then `type_resource` have created a
                                // internal bookkeeping map for this resource. We will need to
                                // use the map to get the resource interface. Otherwise, this resource
                                // is imported, and we will use the generated i32 handle directly.
                                let resource = dealias(
                                    self.interface.resolve,
                                    *match h {
                                        Handle::Borrow(resource) => resource,
                                        Handle::Own(resource) => resource,
                                    },
                                );
                                if self.interface.gen.exported_resources.contains(&resource) {
                                    let resource_name: String =
                                        self.interface.get_ty(&Type::Id(resource)).to_snake_case();
                                    match h {
                                        Own(_) => {
                                            uwriteln!(
                                                self.lift_src,
                                                "{lift_name}_rep := C.{resource_name}_rep({param})
                                                {lift_name}_handle := int32({lift_name}_rep.__handle)"
                                            );
                                        }
                                        Borrow(_) => {
                                            uwriteln!(
                                                self.lift_src,
                                                "{lift_name}_handle := int32({param}.__handle)"
                                            );
                                        }
                                    }
                                    uwriteln!(
                                        self.lift_src,
                                        "{lift_name}, ok := {resource_name}_pointers[{lift_name}_handle]
                                        if !ok {{
                                            panic(\"internal error: invalid handle\")
                                        }}"
                                    );
                                } else {
                                    let resource_name = self.interface.get_ty(&Type::Id(resource));
                                    uwriteln!(
                                        self.lift_src,
                                        "{lift_name} := {resource_name}({param}.__handle)",
                                    );
                                }
                            }
                        }
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
            a => {
                let target_name = self.interface.get_ty(a);

                uwriteln!(self.lift_src, "var {lift_name} {target_name}",);
                uwriteln!(self.lift_src, "{lift_name} = {target_name}({param})",);
            }
        }
    }

    fn get_c_field_name(&mut self, field: &Field) -> String {
        field.name.to_snake_case()
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
