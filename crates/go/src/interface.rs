use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use heck::{ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_c::{
    c_func_name, gen_type_name, is_arg_by_pointer, owner_namespace as c_owner_namespace,
    CTypeNameInfo,
};
use wit_bindgen_core::wit_parser::{
    Docs, Enum, Field, Flags, Function, FunctionKind, Handle, InterfaceId, LiveTypes, Record,
    Resolve, Result_, Tuple, Type, TypeDefKind, TypeId, TypeOwner, Variant, WorldKey,
};
use wit_bindgen_core::{uwriteln, Direction, InterfaceGenerator as _, Source};

use super::{avoid_keyword, bindgen, TinyGo};

pub(crate) struct InterfaceGenerator<'a> {
    pub(crate) src: Source,
    pub(crate) preamble: Source,
    pub(crate) gen: &'a mut TinyGo,
    pub(crate) resolve: &'a Resolve,
    pub(crate) interface: Option<(InterfaceId, &'a WorldKey)>,
    pub(crate) direction: Direction,
    pub(crate) export_funcs: Vec<(String, String)>,
    pub(crate) methods: HashMap<TypeId, Vec<(String, String)>>,
    // tracking all the exported resources used in generating the
    // resource interface and the resource destructors
    // this interface-level tracking is needed to prevent duplicated
    // resource declaration which has been declared in other interfaces
    pub(crate) exported_resources: HashSet<TypeId>,
    pub(crate) wasm_import_module: Option<&'a str>,
}

impl InterfaceGenerator<'_> {
    pub(crate) fn define_interface_types(&mut self, id: InterfaceId) {
        let mut live = LiveTypes::default();
        live.add_interface(self.resolve, id);
        self.define_live_types(&live);
    }

    pub(crate) fn define_function_types(&mut self, funcs: &[(&str, &Function)]) {
        let mut live = LiveTypes::default();
        for (_, func) in funcs {
            live.add_func(self.resolve, func);
        }
        self.define_live_types(&live);
    }

    pub(crate) fn define_live_types(&mut self, live: &LiveTypes) {
        for ty in live.iter() {
            if self.gen.c_type_names.contains_key(&ty) {
                continue;
            }

            // add C type
            let (info, encoded) = gen_type_name(&self.resolve, ty);
            let mut name = match info {
                CTypeNameInfo::Anonymous { is_prim: true } => self.gen.world.to_snake_case(),
                _ => self.c_owner_namespace(ty),
            };

            let prev = self.gen.c_type_namespaces.insert(ty, name.clone());
            assert!(prev.is_none());

            name.push('_');
            name.push_str(&encoded);
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
    pub(crate) fn owner_namespace(&self, id: TypeId) -> String {
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

    pub(crate) fn c_owner_namespace(&self, id: TypeId) -> String {
        c_owner_namespace(
            self.interface,
            matches!(self.direction, Direction::Import),
            self.gen.world.clone(),
            self.resolve,
            id,
            &Default::default(),
        )
    }

    /// Returns the namespace of the current interface.
    ///
    /// If self is not an interface, returns the namespace of the world.
    pub(crate) fn namespace(&self) -> String {
        match self.interface {
            Some((_, key)) => self.interface_identifier(key),
            None => self.gen.world.to_upper_camel_case(),
        }
    }

    pub(crate) fn c_namespace_of_resource(&self, id: TypeId) -> String {
        self.gen.c_type_namespaces[&id].clone()
    }

    /// Returns the identifier of the given interface.
    pub(crate) fn interface_identifier(&self, key: &WorldKey) -> String {
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
                    let version = version.to_string().replace(['.', '-', '+'], "_");
                    name.push_str(&version);
                    name.push('_');
                }
                name.push_str(&iface.name.as_ref().unwrap().to_upper_camel_case());
                name
            }
        }
    }

    /// Returns the function name of the given function.
    pub(crate) fn func_name(&self, func: &Function) -> String {
        match func.kind {
            FunctionKind::Freestanding => func.name.to_upper_camel_case(),
            FunctionKind::Static(_) => func.name.replace('.', " ").to_upper_camel_case(),
            FunctionKind::Method(_) => match self.direction {
                Direction::Import => func.name.split('.').last().unwrap().to_upper_camel_case(),
                Direction::Export => func.name.replace('.', " ").to_upper_camel_case(),
            },
            FunctionKind::Constructor(id) => match self.direction {
                Direction::Import => {
                    let resource_name = self.resolve.types[id].name.as_deref().unwrap();
                    format!("New{}", resource_name.to_upper_camel_case())
                }
                Direction::Export => func.name.replace('.', " ").to_upper_camel_case(),
            },
        }
    }

    /// Returns the type name of the given type.
    ///
    /// Type name is prefixed with the namespace of the interface.
    /// If convert is true, the type name is converted to upper camel case.
    /// Otherwise, the type name is not converted.
    pub(crate) fn type_name(&self, ty_name: &str, convert: bool) -> String {
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
    pub(crate) fn get_interface_var_name(&self) -> String {
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
    pub(crate) fn get_ty(&mut self, ty: &Type) -> String {
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
            Type::F32 => "float32".into(),
            Type::F64 => "float64".into(),
            Type::Char => "rune".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &self.resolve().types[*id];
                match &ty.kind {
                    TypeDefKind::List(ty) => {
                        format!("[]{}", self.get_ty(ty))
                    }
                    TypeDefKind::Option(o) => {
                        self.gen.with_result_option(true);
                        format!("Option[{}]", self.get_ty(o))
                    }
                    TypeDefKind::Result(r) => {
                        self.gen.with_result_option(true);
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
    pub(crate) fn ty_name(&self, ty: &Type) -> String {
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
            Type::F32 => "F32".into(),
            Type::F64 => "F64".into(),
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
    pub(crate) fn optional_ty_name(&self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.ty_name(ty),
            None => "Empty".into(),
        }
    }

    pub(crate) fn func_params(&mut self, func: &Function) -> String {
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

    pub(crate) fn get_func_params_common(
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

    pub(crate) fn func_results(&mut self, func: &Function) -> String {
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

    pub(crate) fn c_param(
        &mut self,
        src: &mut Source,
        name: &str,
        param: &Type,
        direction: Direction,
    ) {
        // If direction is `Import`, this function is invoked as calling an imported function.
        // The parameter uses `&` to dereference argument of pointer type.
        // The & is added as a prefix to the argument name. And there is no
        // type declaration needed to be added to the argument.
        //
        // If direction is `Export`, this function is invoked in printing export function signature.
        // It uses the form of `<param-name> *C.<param-type>` to print each parameter in the function, where
        // * is only used if the parameter is of pointer type.

        let is_pointer = is_arg_by_pointer(self.resolve, param);
        let mut prefix = String::new();
        let mut param_name = String::new();
        let mut postfix = String::new();

        match direction {
            Direction::Import => {
                if is_pointer {
                    prefix.push_str("&");
                }
                if name != "ret" {
                    param_name = format!("lower_{name}");
                } else {
                    param_name.push_str(name);
                }
            }
            Direction::Export => {
                if is_pointer {
                    postfix.push_str("*");
                }
                param_name.push_str(name);
                postfix.push_str(&self.gen.get_c_ty(param));
            }
        }
        src.push_str(&format!("{prefix}{param_name} {postfix}"));
    }

    // Append C params to source.
    pub(crate) fn c_func_params(
        &mut self,
        params: &mut Source,
        func: &Function,
        direction: Direction,
    ) {
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            self.c_param(
                params,
                &avoid_keyword(&name.to_snake_case()),
                param,
                direction,
            );
        }
    }

    pub(crate) fn c_func_returns(
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
                    self.c_param(src, "ret", return_ty, direction);
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

    pub(crate) fn c_func_sig(
        &mut self,
        resolve: &Resolve,
        func: &Function,
        direction: Direction,
    ) -> String {
        let mut src = Source::default();
        let func_name = if matches!(direction, Direction::Import) {
            c_func_name(
                matches!(direction, Direction::Import),
                self.resolve,
                &self.gen.world,
                self.interface.map(|(_, key)| key),
                func,
                &Default::default(),
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
        self.c_func_params(&mut src, func, direction);

        // prepare returns
        self.c_func_returns(&mut src, resolve, func, direction);
        src.to_string()
    }

    pub(crate) fn free_c_arg(&mut self, ty: &Type, arg: &str) -> String {
        let mut ty_name = self.gen.get_c_ty(ty);
        let it: Vec<&str> = ty_name.split('_').collect();
        ty_name = it[..it.len() - 1].join("_");
        format!("defer {ty_name}_free({arg})\n")
    }

    // This is useful in defining functions in the exported interface that the guest needs to implement
    pub(crate) fn func_sig_with_no_namespace(&mut self, func: &Function) -> String {
        format!(
            "{}({}){}",
            self.func_name(func),
            self.func_params(func),
            self.func_results(func)
        )
    }

    pub(crate) fn func_sig(&mut self, func: &Function) {
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
        let func_sig = self.func_sig_with_no_namespace(func);
        self.src.push_str(&func_sig);
        self.src.push_str("{\n");
    }

    pub(crate) fn field_name(&mut self, field: &Field) -> String {
        field.name.to_upper_camel_case()
    }

    pub(crate) fn extract_result_ty(&self, ty: &Type) -> (Option<Type>, Option<Type>) {
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

    pub(crate) fn extract_list_ty(&self, ty: &Type) -> Option<&Type> {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::List(l) => Some(l),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn is_empty_tuple_ty(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Tuple(t) => t.types.is_empty(),
                _ => false,
            },
            _ => false,
        }
    }

    pub(crate) fn optional_ty(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty(ty),
            None => "struct{}".into(),
        }
    }

    pub(crate) fn anonymous_type(&mut self, ty: TypeId) {
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

    pub(crate) fn print_constructor_method_without_value(&mut self, name: &str, case_name: &str) {
        uwriteln!(
            self.src,
            "func {name}{case_name}() {name} {{
                return {name}{{kind: {name}Kind{case_name}}}
            }}
            ",
        );
    }

    pub(crate) fn print_accessor_methods(&mut self, name: &str, case_name: &str, ty: &Type) {
        self.gen.with_fmt_import(true);
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

    pub(crate) fn print_kind_method(&mut self, name: &str) {
        uwriteln!(
            self.src,
            "func (n {name}) Kind() {name}Kind {{
                return n.kind
            }}
            "
        );
    }

    pub(crate) fn print_variant_field(&mut self, name: &str, case_name: &str, i: usize) {
        if i == 0 {
            self.src
                .push_str(&format!("   {name}Kind{case_name} {name}Kind = iota\n",));
        } else {
            self.src.push_str(&format!("   {name}Kind{case_name}\n",));
        }
    }

    pub(crate) fn import(&mut self, resolve: &Resolve, func: &Function) {
        let mut func_bindgen = bindgen::FunctionBindgen::new(self, func);
        func_bindgen.process_args();
        func_bindgen.process_returns();
        let ret = func_bindgen.args;
        let lower_src = func_bindgen.lower_src;
        let lift_src = func_bindgen.lift_src;

        // // print function signature
        self.func_sig(func);

        // body
        // prepare args
        self.src.push_str(&lower_src);

        self.import_invoke(resolve, func, &lift_src, ret);

        // return

        self.src.push_str("}\n\n");
    }

    pub(crate) fn import_invoke(
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

    pub(crate) fn export(&mut self, resolve: &Resolve, func: &Function) {
        let mut func_bindgen = bindgen::FunctionBindgen::new(self, func);
        func_bindgen.process_args();
        func_bindgen.process_returns();

        let args = func_bindgen.args;
        let ret = func_bindgen.c_args;
        let lift_src = func_bindgen.lift_src;
        let lower_src = func_bindgen.lower_src;

        // This variable holds the declaration functions in the exported interface that user
        // needs to implement.
        let interface_method_decl = self.func_sig_with_no_namespace(func);
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
                &Default::default(),
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

    pub(crate) fn finish(&mut self) {
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

            for id in &self.exported_resources {
                // generate an interface that contains all the methods
                // that the guest code needs to implement.
                let ty_name = self.gen.type_names.get(id).unwrap();

                self.src.push_str(&format!("type {ty_name} interface {{\n"));
                if self.methods.get(id).is_none() {
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

    pub(crate) fn print_export_interface(&mut self) {
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

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, _docs: &Docs) {
        let name = self.type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for field in record.fields.iter() {
            let ty = self.get_ty(&field.ty);
            let name = self.field_name(field);
            self.src.push_str(&format!("   {name} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_resource(&mut self, id: TypeId, name: &str, _docs: &Docs) {
        let type_name = self.type_name(name, true);
        let private_type_name = type_name.to_snake_case();
        // for imports, generate a `int32` type for resource handle representation.
        // for exports, generate a map to store unique IDs of resources to their
        // resource interfaces, which are implemented by guest code.
        match self.direction {
            Direction::Import => {
                self.src.push_str(&format!(
                    "// {type_name} is a handle to imported resource {name}\n"
                ));
                self.src.push_str(&format!("type {type_name} int32\n\n"));
                let import_module = self.wasm_import_module.unwrap().to_string();

                // generate [resource-drop] function
                uwriteln!(
                    self.src,
                    "//go:wasmimport {import_module} [resource-drop]{name}
                    func _{type_name}_drop(self {type_name})

                    func (self {type_name}) Drop() {{
                        _{type_name}_drop(self)
                    }}
                    "
                );
            }
            Direction::Export => {
                // generate a typedef struct for export resource
                let c_typedef_target = self.gen.c_type_names[&id].clone();
                let ns = self.c_namespace_of_resource(id);
                let snake = self.resolve.types[id]
                    .name
                    .as_ref()
                    .unwrap()
                    .to_snake_case();
                let mut own = ns.clone();
                own.push_str("_own_");
                own.push_str(&snake);
                own.push_str("_t");

                // generate a typedef struct for export resource
                // the typedef struct is a dummy struct that contains a
                // Go binding specific handle field. This handle field is used
                // to retrieve the exported Go struct from the exported C struct.
                self.preamble
                    .push_str(&format!("// typedef struct {c_typedef_target} "));
                self.preamble.push_str("{");
                self.preamble.push_str("\n");
                self.preamble.push_str("//  int32_t __handle; \n");
                self.preamble.push_str("// ");
                self.preamble.push_str("} ");
                self.preamble.push_str(&c_typedef_target);
                self.preamble.push_str(";\n");

                // import "sync" for Mutex
                self.gen.with_sync_import(true);
                self.src
                    .push_str(&format!("// resource {type_name} internal bookkeeping"));
                uwriteln!(
                    self.src,
                    "
                    var (
                        // a map of indexed {type_name} instances
                        // this is used to retrieve the instance from exported C resources
                        // This establishes a link between the exported C struct and the exported Go struct.
                        // This map will be recycled when the dtor is called.
                        {private_type_name}_pointers = make(map[int32]{type_name})
                        {private_type_name}_next_id int32 = 0
                        {private_type_name}_mu sync.Mutex

                        // a map of {type_name} instances to their owning handlers. This is used to
                        // retrieve the owning handler that is necessary to implement the Drop() method.
                        // Note that the owning handler only exists after the constructor has been called.
                        // This map will be recycled when the dtor is called.
                        {private_type_name}_to_own_handlers sync.Map
                    )
                    "
                );

                uwriteln!(
                    self.src,
                    "
                    // link the instance to its owning handler
                    func set{type_name}OwningHandler(self {type_name}, owningHandler int32) {{
                        {private_type_name}_to_own_handlers.Store(self, owningHandler)
                    }}

                    // get the owning handler for the instance
                    func get{type_name}OwningHandler(self {type_name}) int32 {{
                        owningHandler, ok := {private_type_name}_to_own_handlers.Load(self)
                        if !ok {{
                            panic(\"Internal error: owning handler not found\")
                        }}
                        return owningHandler.(int32)
                    }}
                    "
                );

                // generate [dtor] function for exported resources
                let namespace = self.c_owner_namespace(id);
                let snake = name.to_snake_case();
                let func_name = format!("{}_{}", namespace, snake).to_lower_camel_case();
                self.src
                    .push_str(&format!("//export {namespace}_{snake}_destructor\n"));
                uwriteln!(
                    self.src,
                    "func {func_name}Destructor(self *C.{c_typedef_target}) {{
                        {private_type_name} := {private_type_name}_pointers[int32(self.__handle)]
                        {private_type_name}_to_own_handlers.Delete({private_type_name})
                        delete({private_type_name}_pointers, int32(self.__handle))
                        C.free(unsafe.Pointer(self))
                    }}
                    ",
                );

                self.gen.with_import_unsafe(true);

                // generate [resource-drop] function
                uwriteln!(
                    self.src,
                    "func Drop{type_name}(self {type_name}) {{
                        owningHandler := get{type_name}OwningHandler(self)
                        var cOwningHandler C.{own}
                        cOwningHandler.__handle = C.int32_t(owningHandler)
                        C.{ns}_{snake}_drop_own(cOwningHandler)
                    }}
                    ",
                );

                // book keep the exported resource type
                self.exported_resources.insert(id);
                self.gen.exported_resources.insert(id);
            }
        };
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, _docs: &Docs) {
        let name = self.type_name(name, true);

        // TODO: use flags repr to determine how many flags are needed
        self.src.push_str(&format!("type {name} uint64\n"));
        self.src.push_str("const (\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            let case_flag = flag.name.to_upper_camel_case();

            if i == 0 {
                self.src.push_str(&format!(
                    "   {name}_{flag} {name} = 1 << iota\n",
                    name = name,
                    flag = case_flag,
                ));
            } else {
                self.src.push_str(&format!(
                    "   {name}_{flag}\n",
                    name = name,
                    flag = case_flag,
                ));
            }
        }
        self.src.push_str(")\n\n");
    }

    fn type_tuple(&mut self, _id: TypeId, name: &str, tuple: &Tuple, _docs: &Docs) {
        let name = self.type_name(name, true);
        self.src.push_str(&format!("type {name} struct {{\n",));
        for (i, case) in tuple.types.iter().enumerate() {
            let ty = self.get_ty(case);
            self.src.push_str(&format!("F{i} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, _docs: &Docs) {
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
                self.gen.with_fmt_import(true);
                self.print_accessor_methods(&name, &case_name, ty);
            } else {
                self.print_constructor_method_without_value(&name, &case_name);
            }
        }
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, _docs: &Docs) {
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

    fn type_alias(&mut self, _id: TypeId, name: &str, ty: &Type, _docs: &Docs) {
        let name = self.type_name(name, true);
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_list(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        // no impl since these types are generated as anonymous types
    }

    fn type_option(&mut self, _id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        // no impl since these types are generated as anonymous types
    }

    fn type_result(&mut self, _id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        // no impl since these types are generated as anonymous types
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        todo!("type_builtin")
    }
}
