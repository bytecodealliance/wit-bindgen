use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::{collections::BTreeSet, mem};

use anyhow::Result;
use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};

use wit_bindgen_c::{
    flags_repr, get_nonempty_type, int_repr, is_arg_by_pointer, is_empty_type, owns_anything,
};
use wit_bindgen_core::wit_parser::{InterfaceId, Resolve, TypeOwner, WorldId};
use wit_bindgen_core::{
    uwriteln,
    wit_parser::{Field, Function, Handle, SizeAlign, Type, TypeDefKind, TypeId, WorldKey},
    Files, InterfaceGenerator as _, Source, WorldGenerator,
};

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

fn avoid_keyword(s: &str) -> String {
    if GOKEYWORDS.contains(&s) {
        format!("{s}_")
    } else {
        s.into()
    }
}

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
    world: String,
    needs_result_option: bool,
    needs_import_unsafe: bool,
    needs_fmt_import: bool,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, WorldKey>,
    types: HashMap<TypeId, (HashSet<String>, wit_bindgen_core::Source)>,

    // Interfaces who have had their types printed.
    //
    // This is used to guard against printing the types for an interface twice.
    // The same interface can be both imported and exported in which case only
    // one set of types is generated and all bindings for both imports and
    // exports use that set of types.
    interfaces_with_types_printed: HashSet<InterfaceId>,
}

impl TinyGo {
    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a Option<&'a WorldKey>,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: Source::default(),
            gen: self,
            resolve,
            interface: None,
            name,
            public_anonymous_types: BTreeSet::new(),
            in_import,
            export_funcs: Vec::new(),
        }
    }

    fn finish_types(&mut self, resolve: &Resolve) {
        for (id, _) in resolve.types.iter() {
            if let Some((_, ty)) = self.types.get(&id) {
                self.src.push_str(&ty);
            }
        }
    }
}

impl WorldGenerator for TinyGo {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.world = name.to_string();
        self.sizes.fill(resolve);
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

        let binding = Some(name);
        let mut gen = self.interface(resolve, &binding, true);
        gen.interface = Some(id);
        if gen.gen.interfaces_with_types_printed.insert(id) {
            gen.types(id);
        }

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.import(resolve, func);
        }
        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
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

        let mut gen = self.interface(resolve, &None, true);
        for (_name, func) in funcs.iter() {
            gen.import(resolve, func);
        }
        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
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

        let binding = Some(name);
        let mut gen = self.interface(resolve, &binding, false);
        gen.interface = Some(id);
        if gen.gen.interfaces_with_types_printed.insert(id) {
            gen.types(id);
        }

        for (_name, func) in resolve.interfaces[id].functions.iter() {
            gen.export(resolve, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
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

        let mut gen = self.interface(resolve, &None, false);
        for (_name, func) in funcs.iter() {
            gen.export(resolve, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(resolve, &None, false);
        for (name, id) in types {
            gen.define_type(name, *id);
        }
        gen.finish();
        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        // make sure all types are defined on top of the file
        let src = mem::take(&mut self.src);
        self.finish_types(resolve);
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
        self.src.push_str("import \"C\"\n\n");

        if self.needs_import_unsafe {
            self.src.push_str("import \"unsafe\"\n\n");
        }
        if self.needs_fmt_import {
            self.src.push_str("import \"fmt\"\n\n");
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
                    Ok ResultKind = iota
                    Err
                )

                type Result[T any, E any] struct {{
                    Kind ResultKind
                    Val  T
                    Err  E
                }}

                func (r Result[T, E]) IsOk() bool {{
                    return r.Kind == Ok
                }}

                func (r Result[T, E]) IsErr() bool {{
                    return r.Kind == Err
                }}

                func (r Result[T, E]) Unwrap() T {{
                    if r.Kind != Ok {{
                        panic(\"Result is Err\")
                    }}
                    return r.Val
                }}

                func (r Result[T, E]) UnwrapErr() E {{
                    if r.Kind != Err {{
                        panic(\"Result is Ok\")
                    }}
                    return r.Err
                }}

                func (r *Result[T, E]) Set(val T) T {{
                    r.Kind = Ok
                    r.Val = val
                    return val
                }}

                func (r *Result[T, E]) SetErr(err E) E {{
                    r.Kind = Err
                    r.Err = err
                    return err
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
    gen: &'a mut TinyGo,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
    name: &'a Option<&'a WorldKey>,
    public_anonymous_types: BTreeSet<TypeId>,
    in_import: bool,
    export_funcs: Vec<(String, String)>,
}

impl InterfaceGenerator<'_> {
    fn get_func_name(&self, name: &str) -> String {
        format!("{}{}", self.get_package_name(), name.to_upper_camel_case())
    }

    fn get_type_name(&self, ty_name: &str, convert: bool) -> String {
        let mut name = String::new();
        let package_name = match self.name {
            Some(key) => self.get_ty_name_with(key),
            None => self.gen.world.to_upper_camel_case(),
        };
        let ty_name = if convert {
            ty_name.to_upper_camel_case()
        } else {
            ty_name.into()
        };
        name.push_str(&package_name);
        name.push_str(&ty_name);
        name
    }

    fn get_c_func_name(&self, func_name: &str) -> String {
        let mut name = String::new();
        match self.name {
            Some(WorldKey::Name(k)) => name.push_str(&k.to_snake_case()),
            Some(WorldKey::Interface(id)) => {
                if !self.in_import {
                    name.push_str("exports_");
                }
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_snake_case());
                name.push('_');
                name.push_str(&pkg.name.name.to_snake_case());
                name.push('_');
                name.push_str(&iface.name.as_ref().unwrap().to_snake_case());
            }
            None => name.push_str(&self.gen.world.to_snake_case()),
        }
        name.push('_');
        name.push_str(&func_name.to_snake_case());
        name
    }

    fn get_package_name(&self) -> String {
        match self.name {
            Some(key) => self.get_package_name_with(key),
            None => self.gen.world.to_upper_camel_case(),
        }
    }

    fn get_package_name_with(&self, key: &WorldKey) -> String {
        let mut name = String::new();
        match key {
            WorldKey::Name(k) => name.push_str(&k.to_upper_camel_case()),
            WorldKey::Interface(id) => {
                if !self.in_import {
                    name.push_str("Exports");
                }
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_upper_camel_case());
                name.push_str(&pkg.name.name.to_upper_camel_case());
                name.push_str(&iface.name.as_ref().unwrap().to_upper_camel_case());
            }
        }
        name
    }

    fn get_ty_name_with(&self, key: &WorldKey) -> String {
        let mut name = String::new();
        match key {
            WorldKey::Name(k) => name.push_str(&k.to_upper_camel_case()),
            WorldKey::Interface(id) => {
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_upper_camel_case());
                name.push_str(&pkg.name.name.to_upper_camel_case());
                name.push_str(&iface.name.as_ref().unwrap().to_upper_camel_case());
            }
        }
        name
    }

    fn get_interface_var_name(&self) -> String {
        let mut name = String::new();
        match self.name {
            Some(WorldKey::Name(k)) => name.push_str(&k.to_snake_case()),
            Some(WorldKey::Interface(id)) => {
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_snake_case());
                name.push('_');
                name.push_str(&pkg.name.name.to_snake_case());
                name.push('_');
                name.push_str(&iface.name.as_ref().unwrap().to_snake_case());
            }
            None => name.push_str(&self.gen.world.to_snake_case()),
        }
        name
    }

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
                    wit_bindgen_core::wit_parser::TypeDefKind::Type(ty) => self.get_ty(ty),
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
                            self.get_optional_ty(r.ok.as_ref()),
                            self.get_optional_ty(r.err.as_ref())
                        )
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            if let TypeOwner::Interface(owner) = ty.owner {
                                let key = &self.gen.interface_names[&owner];
                                let iface = self.get_ty_name_with(key);
                                format!("{iface}{name}", name = name.to_upper_camel_case())
                            } else {
                                self.get_type_name(name, true)
                            }
                        } else {
                            self.public_anonymous_types.insert(*id);
                            self.get_type_name(&self.get_ty_name(&Type::Id(*id)), false)
                        }
                    }
                }
            }
        }
    }

    fn get_c_ty_without_package(&self, ty: &Type) -> String {
        match ty {
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
                    namespace = self.gen.world.to_snake_case()
                )
            }
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.name {
                    Some(name) => match ty.owner {
                        TypeOwner::Interface(owner) => {
                            let key = &self.gen.interface_names[&owner];
                            let mut ns = match &key {
                                WorldKey::Name(name) => name.to_snake_case(),
                                WorldKey::Interface(id) => {
                                    let mut ns = String::new();
                                    let iface = &self.resolve.interfaces[*id];
                                    let pkg = &self.resolve.packages[iface.package.unwrap()];
                                    ns.push_str(&pkg.name.namespace.to_snake_case());
                                    ns.push('_');
                                    ns.push_str(&pkg.name.name.to_snake_case());
                                    ns.push('_');
                                    ns.push_str(&iface.name.as_ref().unwrap().to_snake_case());
                                    ns
                                }
                            };
                            ns.push('_');
                            ns.push_str(name.to_snake_case().as_str());
                            ns.push_str("_t");
                            ns
                        }
                        TypeOwner::World(owner) => {
                            format!(
                                "{namespace}_{name}_t",
                                namespace = self.resolve.worlds[owner].name.to_snake_case(),
                                name = name.to_snake_case(),
                            )
                        }
                        TypeOwner::None => {
                            format!("{name}_t", name = self.gen.world.to_snake_case(),)
                        }
                    },
                    None => match &ty.kind {
                        TypeDefKind::Type(t) => self.get_c_ty_without_package(t),
                        _ => format!(
                            "{namespace}_{name}_t",
                            namespace = self.gen.world.to_snake_case(),
                            name = self.get_c_ty_name(&Type::Id(*id)),
                        ),
                    },
                }
            }
        }
    }

    fn get_c_ty(&self, ty: &Type) -> String {
        let res = self.get_c_ty_without_package(ty);
        if res == "bool" {
            return res;
        }
        format!("C.{res}")
    }

    fn get_ty_name(&self, ty: &Type) -> String {
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
                if let Some(name) = &ty.name {
                    let prefix = match ty.owner {
                        TypeOwner::World(owner) => {
                            self.resolve.worlds[owner].name.to_upper_camel_case()
                        }
                        TypeOwner::Interface(owner) => {
                            let key = &self.gen.interface_names[&owner];
                            self.get_ty_name_with(key)
                        }
                        TypeOwner::None => "".into(),
                    };
                    return format!(
                        "{prefix}{name}",
                        prefix = prefix,
                        name = name.to_upper_camel_case()
                    );
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.get_ty_name(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Resource
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(t) => {
                        let mut src = String::new();
                        src.push_str("Tuple");
                        src.push_str(&t.types.len().to_string());
                        for ty in t.types.iter() {
                            src.push_str(&self.get_ty_name(ty));
                        }
                        src.push('T');
                        src
                    }
                    TypeDefKind::Option(t) => {
                        let mut src = String::new();
                        src.push_str("Option");
                        src.push_str(&self.get_ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Result(r) => {
                        let mut src = String::new();
                        src.push_str("Result");
                        src.push_str(&self.get_optional_ty_name(r.ok.as_ref()));
                        src.push_str(&self.get_optional_ty_name(r.ok.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::List(t) => {
                        let mut src = String::new();
                        src.push_str("List");
                        src.push_str(&self.get_ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Future(t) => {
                        let mut src = String::new();
                        src.push_str("Future");
                        src.push_str(&self.get_optional_ty_name(t.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Stream(t) => {
                        let mut src = String::new();
                        src.push_str("Stream");
                        src.push_str(&self.get_optional_ty_name(t.element.as_ref()));
                        src.push_str(&self.get_optional_ty_name(t.end.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Handle(Handle::Own(ty)) => {
                        let mut src = String::new();
                        src.push_str("Own");
                        src.push_str(&self.get_ty_name(&Type::Id(*ty)));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Handle(Handle::Borrow(ty)) => {
                        let mut src = String::new();
                        src.push_str("Borrow");
                        src.push_str(&self.get_ty_name(&Type::Id(*ty)));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
        }
    }

    fn get_optional_ty_name(&self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty_name(ty),
            None => "Empty".into(),
        }
    }

    fn get_c_ty_name(&self, ty: &Type) -> String {
        let mut name = String::new();
        wit_bindgen_c::push_ty_name(
            self.resolve,
            ty,
            &self.gen.interface_names,
            &self.gen.world,
            &mut name,
        );
        name
    }

    fn get_func_params(&mut self, _resolve: &Resolve, func: &Function) -> String {
        let mut params = String::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }

            params.push_str(&avoid_keyword(&name.to_snake_case()));

            params.push(' ');
            params.push_str(&self.get_ty(param));
        }
        params
    }

    fn get_func_results(&mut self, _resolve: &Resolve, func: &Function) -> String {
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

    fn print_c_result(&mut self, src: &mut Source, name: &str, param: &Type, in_import: bool) {
        self.print_c_param(src, name, param, in_import);
    }

    fn print_c_param(&mut self, src: &mut Source, name: &str, param: &Type, in_import: bool) {
        let pointer_prefix = if in_import { "&" } else { "*" };
        let mut prefix = String::new();
        let mut param_name = String::new();
        let mut postfix = String::new();

        if in_import {
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
            postfix.push_str(&self.get_c_ty(param));
        }
        src.push_str(&format!("{prefix}{param_name}{postfix}"));
    }

    fn print_c_func_params(
        &mut self,
        params: &mut Source,
        _resolve: &Resolve,
        func: &Function,
        in_import: bool,
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
            self.print_c_param(
                params,
                &avoid_keyword(&name.to_snake_case()),
                param,
                in_import,
            );
        }
    }

    fn print_c_func_results(
        &mut self,
        src: &mut Source,
        _resolve: &Resolve,
        func: &Function,
        in_import: bool,
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
                    self.print_c_result(src, "ret", return_ty, in_import);
                    src.push_str(")");
                } else {
                    src.push_str(")");
                    if !in_import {
                        src.push_str(&format!(" {ty}", ty = self.get_c_ty(return_ty)));
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
                    if in_import {
                        src.push_str(&format!("&ret{i}"));
                    } else {
                        src.push_str(&format!("ret{i} *{ty}", i = i, ty = self.get_c_ty(ty)));
                    }
                }
                src.push_str(")");
            }
        }
    }

    fn get_c_func_signature(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        in_import: bool,
    ) -> String {
        let mut src = Source::default();
        let func_name = if in_import {
            self.get_c_func_name(&func.name)
        } else {
            self.get_func_name(&func.name)
        };

        if !in_import {
            src.push_str("func ");
        } else {
            src.push_str("C.");
        }
        src.push_str(&func_name);
        src.push_str("(");

        // prepare args
        self.print_c_func_params(&mut src, resolve, func, in_import);

        // prepare returns
        self.print_c_func_results(&mut src, resolve, func, in_import);
        src.to_string()
    }

    fn get_free_c_arg(&mut self, ty: &Type, arg: &str) -> String {
        let mut ty_name = self.get_c_ty(ty);
        let it: Vec<&str> = ty_name.split('_').collect();
        ty_name = it[..it.len() - 1].join("_");
        format!("defer {ty_name}_free({arg})\n")
    }

    fn get_func_signature_no_interface(&mut self, resolve: &Resolve, func: &Function) -> String {
        format!(
            "{}({}){}",
            func.name.to_upper_camel_case(),
            self.get_func_params(resolve, func),
            self.get_func_results(resolve, func)
        )
    }

    fn print_func_signature(&mut self, resolve: &Resolve, func: &Function) {
        self.src.push_str("func ");
        let func_name = self.get_package_name();
        self.src.push_str(&func_name);
        let func_sig = self.get_func_signature_no_interface(resolve, func);
        self.src.push_str(&func_sig);
        self.src.push_str("{\n");
    }

    fn get_field_name(&mut self, field: &Field) -> String {
        field.name.to_upper_camel_case()
    }

    fn extract_result_ty(&self, ty: &Type) -> (Option<Type>, Option<Type>) {
        //TODO: don't copy from the C code
        // optimization on the C size.
        // See https://github.com/bytecodealliance/wit-bindgen/pull/450
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Result(r) => (r.ok, r.err),
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => (None, None),
            },
            _ => (None, None),
        }
    }

    fn extract_list_ty(&self, ty: &Type) -> Option<&Type> {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::List(l) => Some(l),
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
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

    fn get_optional_ty(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty(ty),
            None => "struct{}".into(),
        }
    }

    fn print_anonymous_type(&mut self, ty: TypeId) {
        let kind = &self.resolve.types[ty].kind;
        match kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {
                unreachable!()
            }
            TypeDefKind::Tuple(t) => {
                let prev = mem::replace(&mut self.src, Source::default());

                let ty_name = self.get_ty_name(&Type::Id(ty));
                let name = self.get_type_name(&ty_name, false);
                if let Some((prev_names, _)) = self.gen.types.get(&ty) {
                    for prev_name in prev_names {
                        if prev_name != &name {
                            self.src.push_str(&format!("type {prev_name} = {name}\n"));
                        }
                    }
                }

                self.src.push_str(&format!("type {name} struct {{\n",));
                for (i, ty) in t.types.iter().enumerate() {
                    let ty = self.get_ty(ty);
                    self.src.push_str(&format!("   F{i} {ty}\n",));
                }
                self.src.push_str("}\n\n");

                self.finish_ty(ty, name, prev)
            }
            TypeDefKind::Option(_t) => {}
            TypeDefKind::Result(_r) => {}
            TypeDefKind::List(_l) => {}
            TypeDefKind::Future(_) => todo!("print_anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("print_anonymous_type for stream"),
            TypeDefKind::Handle(_) => todo!("print_anonymous_type for handle"),
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

    fn finish_ty(&mut self, id: TypeId, name: String, source: wit_bindgen_core::Source) {
        // insert or replace the type
        let (names, s) = self.gen.types.entry(id).or_default();
        // Keep track of all the names the type is called in case we have to alias it
        names.insert(name);
        *s = mem::replace(&mut self.src, source);
    }

    fn import(&mut self, resolve: &Resolve, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        // lower params to c
        func.params.iter().for_each(|(name, ty)| {
            func_bindgen.lower(&avoid_keyword(&name.to_snake_case()), ty, false);
        });
        // lift results from c
        match func.results.len() {
            0 => {}
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lift("ret", ty);
            }
            _ => {
                for (i, ty) in func.results.iter_types().enumerate() {
                    func_bindgen.lift(&format!("ret{i}"), ty);
                }
            }
        };
        let c_args = func_bindgen.c_args;
        let ret = func_bindgen.args;
        let lower_src = func_bindgen.lower_src.to_string();
        let lift_src = func_bindgen.lift_src.to_string();

        // // print function signature
        self.print_func_signature(resolve, func);

        // body
        // prepare args
        self.src.push_str(lower_src.as_str());

        self.import_invoke(resolve, func, c_args, &lift_src, ret);

        // return

        self.src.push_str("}\n\n");
    }

    fn import_invoke(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        _c_args: Vec<String>,
        lift_src: &str,
        ret: Vec<String>,
    ) {
        let invoke = self.get_c_func_signature(resolve, func, true);
        match func.results.len() {
            0 => {
                self.src.push_str(&invoke);
                self.src.push_str("\n");
            }
            1 => {
                let return_ty = func.results.iter_types().next().unwrap();
                if is_arg_by_pointer(self.resolve, return_ty) {
                    let c_ret_type = self.get_c_ty(return_ty);
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
                    let ty_name = self.get_c_ty(ty);
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
        match func.results.len() {
            0 => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(&avoid_keyword(&name.to_snake_case()), ty);
                });
            }
            1 => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(&avoid_keyword(&name.to_snake_case()), ty);
                });
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lower("result", ty, true);
            }
            _ => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(&avoid_keyword(&name.to_snake_case()), ty);
                });
                for (i, ty) in func.results.iter_types().enumerate() {
                    func_bindgen.lower(&format!("result{i}"), ty, true);
                }
            }
        };

        let args = func_bindgen.args;
        let ret = func_bindgen.c_args;
        let lift_src = func_bindgen.lift_src.to_string();
        let lower_src = func_bindgen.lower_src.to_string();

        let interface_method_decl = self.get_func_signature_no_interface(resolve, func);
        let export_func = {
            let mut src = String::new();
            // header
            src.push_str("//export ");
            let name = self.get_c_func_name(&func.name);
            src.push_str(&name);
            src.push('\n');

            // signature
            src.push_str(&self.get_c_func_signature(resolve, func, false));
            src.push_str(" {\n");

            // free all the parameters
            for (name, ty) in func.params.iter() {
                if owns_anything(resolve, ty, &|_, _| todo!("support resources")) {
                    let free = self.get_free_c_arg(ty, &avoid_keyword(&name.to_snake_case()));
                    src.push_str(&free);
                }
            }

            // prepare args

            src.push_str(&lift_src);

            // invoke
            let invoke = format!(
                "{}.{}({})",
                &self.get_interface_var_name(),
                &func.name.to_upper_camel_case(),
                args.iter()
                    .enumerate()
                    .map(|(i, name)| format!(
                        "{}{}",
                        name,
                        if i < func.params.len() - 1 { ", " } else { "" }
                    ))
                    .collect::<String>()
            );

            // prepare ret
            match func.results.len() {
                0 => {
                    src.push_str(&format!("{invoke}\n"));
                }
                1 => {
                    let return_ty = func.results.iter_types().next().unwrap();
                    if is_empty_type(self.resolve, return_ty) {
                        src.push_str(&format!("{invoke}\n"));
                    } else {
                        src.push_str(&format!("result := {invoke}\n"));
                    }
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
        self.export_funcs.push((interface_method_decl, export_func));
    }

    fn finish(&mut self) {
        let src = mem::take(&mut self.src);
        while let Some(ty) = self.public_anonymous_types.pop_last() {
            self.print_anonymous_type(ty);
        }
        self.src.push_str(&src);

        if !self.in_import && !self.export_funcs.is_empty() {
            let interface_var_name = &self.get_interface_var_name();
            let interface_name = &self.get_package_name();

            self.src
                .push_str(format!("var {interface_var_name} {interface_name} = nil\n").as_str());
            self.src.push_str(
                format!(
                    "func Set{interface_name}(i {interface_name}) {{\n    {interface_var_name} = i\n}}\n"
                )
                .as_str(),
            );
            self.src
                .push_str(format!("type {interface_name} interface {{\n").as_str());
            for (interface_func_declaration, _) in &self.export_funcs {
                self.src
                    .push_str(format!("{interface_func_declaration}\n").as_str());
            }
            self.src.push_str("}\n");

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(export_func);
            }
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        record: &wit_bindgen_core::wit_parser::Record,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for field in record.fields.iter() {
            let ty = self.get_ty(&field.ty);
            let name = self.get_field_name(field);
            self.src.push_str(&format!("   {name} {ty}\n",));
        }
        self.src.push_str("}\n\n");
        self.finish_ty(id, name, prev)
    }

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &wit_bindgen_core::wit_parser::Docs) {
        _ = (id, name, docs);
        todo!()
    }

    fn type_flags(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        flags: &wit_bindgen_core::wit_parser::Flags,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
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
        self.finish_ty(id, name, prev)
    }

    fn type_tuple(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        tuple: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for (i, case) in tuple.types.iter().enumerate() {
            let ty = self.get_ty(case);
            self.src.push_str(&format!("F{i} {ty}\n",));
        }
        self.src.push_str("}\n\n");
        self.finish_ty(id, name, prev)
    }

    fn type_variant(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        variant: &wit_bindgen_core::wit_parser::Variant,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
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
            if let Some(ty) = get_nonempty_type(self.resolve, case.ty.as_ref()) {
                self.gen.needs_fmt_import = true;
                self.print_accessor_methods(&name, &case_name, ty);
            } else {
                self.print_constructor_method_without_value(&name, &case_name);
            }
        }
        self.finish_ty(id, name, prev)
    }

    fn type_option(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        self.get_ty(&Type::Id(id));
        self.finish_ty(id, name.to_owned(), prev)
    }

    fn type_result(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        self.get_ty(&Type::Id(id));
        self.finish_ty(id, name.to_owned(), prev)
    }

    fn type_union(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        union: &wit_bindgen_core::wit_parser::Union,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str("const (\n");

        for (i, _case) in union.cases.iter().enumerate() {
            let case_name = format!("F{i}");
            self.print_variant_field(&name, &case_name, i);
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str("val any\n");
        self.src.push_str("}\n\n");

        self.print_kind_method(&name);

        for (i, case) in union.cases.iter().enumerate() {
            self.gen.needs_fmt_import = true;

            let case_name = format!("F{i}");
            self.print_accessor_methods(&name, &case_name, &case.ty);
        }
        self.finish_ty(id, name.to_owned(), prev)
    }

    fn type_enum(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        enum_: &wit_bindgen_core::wit_parser::Enum,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
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
        self.finish_ty(id, name, prev)
    }

    fn type_alias(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
        self.finish_ty(id, name, prev)
    }

    fn type_list(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let prev = mem::take(&mut self.src);
        let name = self.get_type_name(name, true);
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
        self.finish_ty(id, name, prev)
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
    _func: &'a Function,
    c_args: Vec<String>,
    args: Vec<String>,
    lower_src: Source,
    lift_src: Source,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(interface: &'a mut InterfaceGenerator<'b>, func: &'a Function) -> Self {
        Self {
            interface,
            _func: func,
            c_args: Vec::new(),
            args: Vec::new(),
            lower_src: Source::default(),
            lift_src: Source::default(),
        }
    }

    fn lower(&mut self, name: &str, ty: &Type, in_export: bool) {
        let lower_name = format!("lower_{name}");
        self.lower_value(name, ty, lower_name.as_ref());

        // Check whether or not the C variable needs to be freed.
        // If this variable is in export function, which will be returned to host to use.
        //    There is no need to free return variables.
        // If this variable does not own anything, it does not need to be freed.
        // If this variable is in inner node of the recursive call, no need to be freed.
        //    This is because the root node's call to free will recursively free the whole tree.
        // Otherwise, free this variable.
        if !in_export
            && owns_anything(self.interface.resolve, ty, &|_, _| {
                todo!("support resources")
            })
        {
            self.lower_src
                .push_str(&self.interface.get_free_c_arg(ty, &format!("&{lower_name}")));
        }
        self.c_args.push(lower_name);
    }

    fn lower_list_value(&mut self, param: &str, l: &Type, lower_name: &str) {
        let list_ty = self.interface.get_c_ty(l);
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
            if !is_empty_type(self.interface.resolve, &ok_inner) {
                self.interface.gen.needs_import_unsafe = true;
                let c_target_name = self.interface.get_c_ty(&ok_inner);
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
        }
        self.lower_src.push_str("} else {\n");
        if let Some(err_inner) = err {
            if !is_empty_type(self.interface.resolve, &err_inner) {
                self.interface.gen.needs_import_unsafe = true;
                let c_target_name = self.interface.get_c_ty(&err_inner);
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
                uwriteln!(
                    self.lower_src,
                    "var {lower_name} {value}",
                    value = self.interface.get_c_ty(ty),
                );
                uwriteln!(
                    self.lower_src,
                    "
                    {lower_name}.ptr = C.CString({param})
                    {lower_name}.len = C.size_t(len({param}))"
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.resolve.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for field in r.fields.iter() {
                            let c_field_name = &self.get_c_field_name(field);
                            let field_name = &self.interface.get_field_name(field);

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
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
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
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        uwriteln!(self.lower_src, "if {param}.IsSome() {{");
                        if !is_empty_type(self.interface.resolve, o) {
                            self.lower_value(
                                &format!("{param}.Unwrap()"),
                                o,
                                &format!("{lower_name}_val"),
                            );
                            uwriteln!(self.lower_src, "{lower_name}.val = {lower_name}_val");
                            uwriteln!(self.lower_src, "{lower_name}.is_some = true");
                        }
                        self.lower_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));

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
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));

                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        self.lower_list_value(param, l, lower_name);
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lower_src,
                            "var {lower_name} {value}",
                            value = self.interface.get_c_ty(t),
                        );
                        self.lower_value(param, t, &format!("{lower_name}_val"));
                        uwriteln!(self.lower_src, "{lower_name} = {lower_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;

                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            if let Some(ty) =
                                get_nonempty_type(self.interface.resolve, case.ty.as_ref())
                            {
                                let name = self.interface.get_c_ty(ty);
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
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
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
                    TypeDefKind::Union(u) => {
                        self.interface.gen.needs_import_unsafe = true;

                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in u.cases.iter().enumerate() {
                            let case_name = format!("F{i}");
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            let name = self.interface.get_c_ty(&case.ty);
                            uwriteln!(
                                self.lower_src,
                                "
                                {lower_name}.tag = {i}
                                {lower_name}_ptr := (*{name})(unsafe.Pointer(&{lower_name}.val))"
                            );

                            self.lower_value(
                                &format!("{param}.Get{case_name}()"),
                                &case.ty,
                                &format!("{lower_name}_val"),
                            );
                            uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");

                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(_) => todo!("impl handle"),
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
            a => {
                uwriteln!(
                    self.lower_src,
                    "{lower_name} := {c_type_name}({param_name})",
                    c_type_name = self.interface.get_c_ty(a),
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
                uwriteln!(
                    self.lift_src,
                    "var {name} {value}
                    {lift_name} = C.GoStringN({param}.ptr, C.int({param}.len))",
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
                            "var {name} {value}",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for field in r.fields.iter() {
                            let field_name = &self.interface.get_field_name(field);
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
                            "var {name} {value}
                            {lift_name} = {field}({param})",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                    }
                    TypeDefKind::Tuple(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
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
                        let lift_type = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {lift_type}");
                        uwriteln!(self.lift_src, "if {param}.is_some {{");
                        self.lift_value(&format!("{param}.val"), o, &format!("{lift_name}_val"));

                        uwriteln!(self.lift_src, "{lift_name}.Set({lift_name}_val)");
                        self.lift_src.push_str("} else {\n");
                        uwriteln!(self.lift_src, "{lift_name}.Unset()");
                        self.lift_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        self.interface.gen.needs_result_option = true;
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty}");
                        let (ok, err) = self.interface.extract_result_ty(&Type::Id(*id));

                        // normal result route
                        uwriteln!(self.lift_src, "if {param}.is_err {{");
                        if let Some(err_inner) = err {
                            let err_inner_name = self.interface.get_c_ty(&err_inner);
                            if !is_empty_type(self.interface.resolve, &err_inner) {
                                self.interface.gen.needs_import_unsafe = true;
                                uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{err_inner_name})(unsafe.Pointer(&{param}.val))");
                            }
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
                            let ok_inner_name = self.interface.get_c_ty(&ok_inner);
                            if !is_empty_type(self.interface.resolve, &ok_inner) {
                                self.interface.gen.needs_import_unsafe = true;
                                uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{ok_inner_name})(unsafe.Pointer(&{param}.val))");
                            }
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
                        let c_ty_name = self.interface.get_c_ty(l);
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
                            "var {lift_name} {value}",
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        self.lift_value(param, t, &format!("{lift_name}_val"));
                        uwriteln!(self.lift_src, "{lift_name} = {lift_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            if let Some(ty) =
                                get_nonempty_type(self.interface.resolve, case.ty.as_ref())
                            {
                                let ty_name = self.interface.get_c_ty(ty);
                                uwriteln!(
                                    self.lift_src,
                                    "{lift_name}_ptr := *(*{ty_name})(unsafe.Pointer(&{param}.val))"
                                );
                                self.lift_value(
                                    &format!("{lift_name}_ptr"),
                                    ty,
                                    &format!("{lift_name}_val"),
                                );
                                uwriteln!(
                                    self.lift_src,
                                    "{lift_name} = {name}{case_name}({lift_name}_val)"
                                )
                            } else {
                                uwriteln!(self.lift_src, "{lift_name} = {name}{case_name}()");
                            }
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(self.lift_src, "if {param} == {i} {{");
                            uwriteln!(self.lift_src, "{lift_name} = {name}{case_name}()");
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Union(u) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in u.cases.iter().enumerate() {
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            let ty = self.interface.get_c_ty(&case.ty);
                            let case_name = format!("F{i}");
                            uwriteln!(
                                self.lift_src,
                                "
                                {lift_name}_ptr := *(*{ty})(unsafe.Pointer(&{param}.val))"
                            );
                            self.lift_value(
                                &format!("{lift_name}_ptr"),
                                &case.ty,
                                &format!("{lift_name}_val"),
                            );
                            uwriteln!(
                                self.lift_src,
                                "{lift_name} = {name}{case_name}({lift_name}_val)"
                            );
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(_) => todo!("impl handle"),
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
