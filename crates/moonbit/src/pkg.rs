use std::collections::HashMap;

use heck::{ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    Ns, dealias,
    wit_parser::{
        Function, FunctionKind, Handle, InterfaceId, Resolve, Type, TypeDef, TypeDefKind,
        TypeOwner, WorldId, WorldKey,
    },
};

pub(crate) const FFI_DIR: &str = "ffi";

#[derive(Default)]
pub(crate) struct Imports {
    pub packages: HashMap<String, String>,
    ns: Ns,
}

pub(crate) struct MoonbitSignature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub result_type: Option<Type>,
}

#[derive(Default)]
pub(crate) struct PkgResolver {
    pub resolve: Resolve,
    // Packages imported by each package
    pub package_import: HashMap<String, Imports>,
    pub import_interface_names: HashMap<InterfaceId, String>,
    pub export_interface_names: HashMap<InterfaceId, String>,
}

impl PkgResolver {
    pub(crate) fn qualify_package(&mut self, this: &str, name: &str) -> String {
        if name != this {
            let imports = self.package_import.entry(this.to_string()).or_default();
            if let Some(alias) = imports.packages.get(name) {
                format!("@{alias}.")
            } else {
                let alias = imports
                    .ns
                    .tmp(&name.split(".").last().unwrap().to_lower_camel_case());
                imports
                    .packages
                    .entry(name.to_string())
                    .or_insert(alias.clone());
                format!("@{alias}.")
            }
        } else {
            "".into()
        }
    }

    pub(crate) fn qualifier(&mut self, this: &str, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.export_interface_names.get(id) {
                if name != this {
                    return self.qualify_package(this, &name.clone());
                }
            } else if let Some(name) = self.import_interface_names.get(id) {
                if name != this {
                    return self.qualify_package(this, &name.clone());
                }
            }
        } else if let TypeOwner::World(id) = &ty.owner {
            let name = PkgResolver::world_name(&self.resolve, *id);
            if name != this {
                return self.qualify_package(this, &name.clone());
            }
        }

        String::new()
    }

    pub(crate) fn func_call(
        &mut self,
        this: &str,
        func: &Function,
        func_interface: &str,
    ) -> String {
        match func.kind {
            FunctionKind::Freestanding => {
                format!(
                    "{}{}",
                    self.qualify_package(this, func_interface),
                    func.name.to_moonbit_ident()
                )
            }
            FunctionKind::AsyncFreestanding => {
                format!(
                    "{}{}",
                    self.qualify_package(this, func_interface),
                    func.name.to_moonbit_ident()
                )
            }
            FunctionKind::Constructor(ty) => {
                let name = self.type_constructor(this, &Type::Id(ty));
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
                let name = self.type_constructor(this, &Type::Id(ty));
                format!(
                    "{}::{}",
                    name,
                    func.name.split(".").last().unwrap().to_moonbit_ident()
                )
            }
        }
    }

    pub(crate) fn type_constructor(&mut self, this: &str, ty: &Type) -> String {
        match ty {
            Type::ErrorContext => unimplemented!("moonbit error context type name"),
            Type::Id(id) => {
                let ty = self.resolve.types[dealias(&self.resolve, *id)].clone();
                match ty.kind {
                    TypeDefKind::Type(ty) => self.type_constructor(this, &ty),
                    TypeDefKind::Handle(handle) => {
                        let ty = match handle {
                            Handle::Own(ty) => ty,
                            Handle::Borrow(ty) => ty,
                        };
                        let ty = self.resolve.types[dealias(&self.resolve, ty)].clone();
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(this, &ty),
                                name.to_moonbit_type_ident()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                    TypeDefKind::Enum(_)
                    | TypeDefKind::Resource
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Record(_) => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(this, &ty),
                                name.to_moonbit_type_ident()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                    TypeDefKind::Result(_) => "Result".into(),
                    TypeDefKind::Option(_) => "Option".into(),
                    _ => {
                        unreachable!(
                            "Should not call constructor or method for builtin type: {:?}",
                            ty
                        )
                    }
                }
            }
            _ => {
                unreachable!(
                    "Should not call constructor or method for primitive types: {:?}",
                    ty
                )
            }
        }
    }

    pub(crate) fn type_name(&mut self, this: &str, ty: &Type) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "Byte".into(),
            Type::S32 | Type::S8 | Type::S16 => "Int".into(),
            Type::U16 | Type::U32 => "UInt".into(),
            Type::Char => "Char".into(),
            Type::U64 => "UInt64".into(),
            Type::S64 => "Int64".into(),
            Type::F32 => "Float".into(),
            Type::F64 => "Double".into(),
            Type::String => "String".into(),
            Type::ErrorContext => todo!("moonbit error context type name"),
            Type::Id(id) => {
                let ty = self.resolve.types[dealias(&self.resolve, *id)].clone();
                match ty.kind {
                    TypeDefKind::Type(ty) => self.type_name(this, &ty),
                    TypeDefKind::List(ty) => match ty {
                        Type::U8
                        | Type::U32
                        | Type::U64
                        | Type::S32
                        | Type::S64
                        | Type::F32
                        | Type::F64 => {
                            format!("FixedArray[{}]", self.type_name(this, &ty))
                        }
                        _ => format!("Array[{}]", self.type_name(this, &ty)),
                    },
                    TypeDefKind::Tuple(tuple) => {
                        format!(
                            "({})",
                            tuple
                                .types
                                .iter()
                                .map(|ty| self.type_name(this, ty))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                    TypeDefKind::Option(ty) => {
                        format!("{}?", self.type_name(this, &ty))
                    }
                    TypeDefKind::Result(result) => {
                        let mut name = |ty: &Option<Type>| {
                            ty.as_ref()
                                .map(|ty| self.type_name(this, ty))
                                .unwrap_or_else(|| "Unit".into())
                        };
                        let ok = name(&result.ok);
                        let err = name(&result.err);

                        format!("Result[{ok}, {err}]")
                    }
                    TypeDefKind::Handle(handle) => {
                        let ty = match handle {
                            Handle::Own(ty) => ty,
                            Handle::Borrow(ty) => ty,
                        };
                        let ty = self.resolve.types[dealias(&self.resolve, ty)].clone();
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(this, &ty),
                                name.to_moonbit_type_ident()
                            )
                        } else {
                            unreachable!()
                        }
                    }

                    TypeDefKind::Future(ty) => {
                        let qualifier = self.qualify_package(this, FFI_DIR);
                        format!(
                            "{}FutureReader[{}]",
                            qualifier,
                            ty.as_ref()
                                .map(|t| self.type_name(this, t))
                                .unwrap_or_else(|| "Unit".into())
                        )
                    }

                    TypeDefKind::Stream(ty) => {
                        let qualifier = self.qualify_package(this, FFI_DIR);
                        format!(
                            "{}StreamReader[{}]",
                            qualifier,
                            ty.as_ref()
                                .map(|t| self.type_name(this, t))
                                .unwrap_or_else(|| "Unit".into())
                        )
                    }

                    _ => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(this, &ty),
                                name.to_moonbit_type_ident()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn non_empty_type<'a>(&self, ty: Option<&'a Type>) -> Option<&'a Type> {
        if let Some(ty) = ty {
            let id = match ty {
                Type::Id(id) => *id,
                _ => return Some(ty),
            };
            match &self.resolve.types[id].kind {
                TypeDefKind::Type(t) => self.non_empty_type(Some(t)).map(|_| ty),
                TypeDefKind::Record(r) => (!r.fields.is_empty()).then_some(ty),
                TypeDefKind::Tuple(t) => (!t.types.is_empty()).then_some(ty),
                _ => Some(ty),
            }
        } else {
            None
        }
    }

    pub(crate) fn mbt_sig(
        &mut self,
        this: &str,
        func: &Function,
        ignore_param: bool,
    ) -> MoonbitSignature {
        let name = match func.kind {
            FunctionKind::Freestanding => func.name.to_moonbit_ident(),
            FunctionKind::Constructor(_) => {
                func.name.replace("[constructor]", "").to_moonbit_ident()
            }
            _ => func.name.split(".").last().unwrap().to_moonbit_ident(),
        };
        let type_name = match func.kind.resource() {
            Some(ty) => {
                format!("{}::", self.type_constructor(this, &Type::Id(ty)))
            }
            None => "".into(),
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let name = if ignore_param {
                    format!("_{}", name.to_moonbit_ident())
                } else {
                    name.to_moonbit_ident()
                };
                (name, *ty)
            })
            .collect::<Vec<_>>();

        MoonbitSignature {
            name: format!("{type_name}{name}"),
            params,
            result_type: func.result,
        }
    }

    pub(crate) fn world_name(resolve: &Resolve, world: WorldId) -> String {
        format!("world.{}", resolve.worlds[world].name.to_lower_camel_case())
    }

    pub(crate) fn interface_name(resolve: &Resolve, name: &WorldKey) -> String {
        let pkg = match name {
            WorldKey::Name(_) => None,
            WorldKey::Interface(id) => {
                let pkg = resolve.interfaces[*id].package.unwrap();
                Some(resolve.packages[pkg].name.clone())
            }
        };

        let name = match name {
            WorldKey::Name(name) => name,
            WorldKey::Interface(id) => resolve.interfaces[*id].name.as_ref().unwrap(),
        }
        .to_lower_camel_case();

        format!(
            "interface.{}{name}",
            if let Some(name) = &pkg {
                format!(
                    "{}.{}.",
                    name.namespace.to_moonbit_ident(),
                    name.name.to_moonbit_ident()
                )
            } else {
                String::new()
            }
        )
    }
}

pub(crate) trait ToMoonBitIdent: ToOwned {
    fn to_moonbit_ident(&self) -> Self::Owned;
}

impl ToMoonBitIdent for str {
    fn to_moonbit_ident(&self) -> String {
        // Escape MoonBit keywords and reserved keywords
        match self {
            // Keywords
            "as" | "else" | "extern" | "fn" | "fnalias" | "if" | "let" | "const" | "match" | "using"
            | "mut" | "type" | "typealias" | "struct" | "enum" | "trait" | "traitalias" | "derive"
            | "while" | "break" | "continue" | "import" | "return" | "throw" | "raise" | "try" | "catch"
            | "pub" | "priv" | "readonly" | "true" | "false" | "_" | "test" | "loop" | "for" | "in" | "impl"
            | "with" | "guard" | "async" | "is" | "suberror" | "and" | "letrec" | "enumview" | "noraise" 
            | "defer" | "init" | "main"
            // Reserved keywords
            | "module" | "move" | "ref" | "static" | "super" | "unsafe" | "use" | "where" | "await"
            | "dyn" | "abstract" | "do" | "final" | "macro" | "override" | "typeof" | "virtual" | "yield"
            | "local" | "method" | "alias" | "assert" | "package" | "recur" | "isnot" | "define" | "downcast"
            | "inherit" | "member" | "namespace" | "upcast" | "void" | "lazy" | "include" | "mixin"
            | "protected" | "sealed" | "constructor" | "atomic" | "volatile" | "anyframe" | "anytype"
            | "asm" | "comptime" | "errdefer" | "export" | "opaque" | "orelse" | "resume" | "threadlocal"
            | "unreachable" | "dynclass" | "dynobj" | "dynrec" | "var" | "finally" | "noasync" => {
                format!("{self}_")
            }
            _ => self.strip_prefix("[async]").unwrap_or(self).to_snake_case(),
        }
    }
}

pub(crate) trait ToMoonBitTypeIdent: ToOwned {
    fn to_moonbit_type_ident(&self) -> Self::Owned;
}

impl ToMoonBitTypeIdent for str {
    fn to_moonbit_type_ident(&self) -> String {
        // Escape MoonBit builtin types
        match self.to_upper_camel_case().as_str() {
            type_name @ ("Bool" | "Byte" | "Int16" | "UInt16" | "Int" | "Int64" | "UInt"
            | "UInt64" | "Float" | "Double" | "Error" | "Bytes" | "ReadonlyArray"
            | "Array" | "FixedArray" | "Map" | "String" | "StringBuilder"
            | "Option" | "Result" | "Char" | "Json") => {
                format!("{type_name}_")
            }
            type_name => type_name.to_owned(),
        }
    }
}
