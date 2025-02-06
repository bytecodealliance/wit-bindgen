use crate::interface::ScalaJsInterface;
use crate::jco::{maybe_null, to_js_identifier};
use crate::{Opts, ScalaDialect, ScalaJs};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use wit_bindgen_core::wit_parser::{
    Docs, Handle, InterfaceId, PackageName, Resolve, Results, Tuple, Type, TypeDef, TypeDefKind,
    TypeId, TypeOwner,
};
use wit_bindgen_core::Direction::{Export, Import};
use wit_bindgen_core::{uwrite, uwriteln, Direction};

pub struct ScalaKeywords {
    keywords: HashSet<String>,
    pub base_methods: HashSet<String>,
}

impl ScalaKeywords {
    pub fn new(dialect: &ScalaDialect) -> Self {
        let mut keywords = HashSet::new();
        keywords.insert("abstract".to_string());
        keywords.insert("case".to_string());
        keywords.insert("do".to_string());
        keywords.insert("else".to_string());
        keywords.insert("finally".to_string());
        keywords.insert("for".to_string());
        keywords.insert("import".to_string());
        keywords.insert("lazy".to_string());
        keywords.insert("object".to_string());
        keywords.insert("override".to_string());
        keywords.insert("return".to_string());
        keywords.insert("sealed".to_string());
        keywords.insert("trait".to_string());
        keywords.insert("try".to_string());
        keywords.insert("var".to_string());
        keywords.insert("while".to_string());
        keywords.insert("catch".to_string());
        keywords.insert("class".to_string());
        keywords.insert("extends".to_string());
        keywords.insert("false".to_string());
        keywords.insert("forSome".to_string());
        keywords.insert("if".to_string());
        keywords.insert("macro".to_string());
        keywords.insert("match".to_string());
        keywords.insert("new".to_string());
        keywords.insert("package".to_string());
        keywords.insert("private".to_string());
        keywords.insert("super".to_string());
        keywords.insert("this".to_string());
        keywords.insert("true".to_string());
        keywords.insert("type".to_string());
        keywords.insert("with".to_string());
        keywords.insert("yield".to_string());
        keywords.insert("def".to_string());
        keywords.insert("final".to_string());
        keywords.insert("implicit".to_string());
        keywords.insert("null".to_string());
        keywords.insert("protected".to_string());
        keywords.insert("throw".to_string());
        keywords.insert("val".to_string());
        keywords.insert("_".to_string());
        keywords.insert(":".to_string());
        keywords.insert("=".to_string());
        keywords.insert("=>".to_string());
        keywords.insert("<-".to_string());
        keywords.insert("<:".to_string());
        keywords.insert("<%".to_string());
        keywords.insert("=>>".to_string());
        keywords.insert(">:".to_string());
        keywords.insert("#".to_string());
        keywords.insert("@".to_string());
        keywords.insert("\u{21D2}".to_string());
        keywords.insert("\u{2190}".to_string());

        let mut base_methods = HashSet::new();
        base_methods.insert("equals".to_string());
        base_methods.insert("hashCode".to_string());
        base_methods.insert("toString".to_string());
        base_methods.insert("clone".to_string());
        base_methods.insert("finalize".to_string());
        base_methods.insert("getClass".to_string());
        base_methods.insert("notify".to_string());
        base_methods.insert("notifyAll".to_string());
        base_methods.insert("wait".to_string());
        base_methods.insert("isInstanceOf".to_string());
        base_methods.insert("asInstanceOf".to_string());
        base_methods.insert("synchronized".to_string());
        base_methods.insert("ne".to_string());
        base_methods.insert("eq".to_string());
        base_methods.insert("hasOwnProperty".to_string());
        base_methods.insert("isPrototypeOf".to_string());
        base_methods.insert("propertyIsEnumerable".to_string());
        base_methods.insert("toLocaleString".to_string());
        base_methods.insert("valueOf".to_string());

        match dialect {
            ScalaDialect::Scala2 => {}
            ScalaDialect::Scala3 => {
                keywords.insert("enum".to_string());
                keywords.insert("export".to_string());
                keywords.insert("given".to_string());
                keywords.insert("?=>".to_string());
                keywords.insert("then".to_string());
            }
        }

        Self {
            keywords,
            base_methods,
        }
    }

    pub(crate) fn escape(&self, ident: impl AsRef<str>) -> String {
        if self.keywords.contains(ident.as_ref()) {
            format!("`{}`", ident.as_ref())
        } else {
            ident.as_ref().to_string()
        }
    }
}

// TODO: refactor to context, include resolve
pub trait OwnerContext {
    fn is_local_import(&self, id: &InterfaceId, is_resource: bool) -> Option<Option<String>>;
}

impl OwnerContext for ScalaJs {
    fn is_local_import(&self, _id: &InterfaceId, _is_resource: bool) -> Option<Option<String>> {
        None
    }
}

impl OwnerContext for ScalaJsContext {
    fn is_local_import(&self, _id: &InterfaceId, _is_resource: bool) -> Option<Option<String>> {
        None
    }
}

impl<'a> OwnerContext for ScalaJsInterface<'a> {
    fn is_local_import(&self, id: &InterfaceId, is_resource: bool) -> Option<Option<String>> {
        if id == &self.interface_id {
            if is_resource && self.direction == Import {
                Some(Some(self.name.clone()))
            } else {
                Some(None)
            }
        } else {
            None
        }
    }
}

pub struct ScalaJsContext {
    pub opts: Opts,
    pub keywords: ScalaKeywords,
    pub overrides: HashMap<TypeId, String>,
    pub imports: HashSet<InterfaceId>,
    pub exports: HashSet<InterfaceId>,
}

impl ScalaJsContext {
    pub fn encode_name(&self, name: impl AsRef<str>) -> EncodedName {
        let name = name.as_ref();
        let scala_name = self.keywords.escape(name);
        let js_name = to_js_identifier(name);

        let rename_attribute = if scala_name != js_name && scala_name != format!("`{js_name}`") {
            format!("@JSName(\"{js_name}\")")
        } else {
            "".to_string()
        };
        EncodedName {
            scala: scala_name,
            js: js_name,
            rename_attribute,
        }
    }

    pub fn render_args<'b>(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        params: impl Iterator<Item = &'b (String, Type)>,
    ) -> String {
        let mut args = Vec::new();
        for (param_name, param_typ) in params {
            let param_typ = self.render_type_reference(owner_context, resolve, param_typ);
            let param_name = self.encode_name(param_name.to_lower_camel_case());
            args.push(format!("{}: {param_typ}", param_name.scala));
        }
        args.join(", ")
    }

    pub fn render_return_type(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        results: &Results,
    ) -> String {
        match results {
            Results::Named(results) if results.len() == 0 => "Unit".to_string(),
            Results::Named(results) if results.len() == 1 => self.render_type_reference(
                owner_context,
                resolve,
                &results.iter().next().unwrap().1,
            ),
            Results::Named(results) => self.render_tuple(
                owner_context,
                resolve,
                &Tuple {
                    types: results.iter().map(|(_, typ)| typ.clone()).collect(),
                },
            ),
            Results::Anon(typ) => self.render_type_reference(owner_context, resolve, typ),
        }
    }

    fn render_type_reference(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        typ: &Type,
    ) -> String {
        match typ {
            Type::Bool => "Boolean".to_string(),
            Type::U8 => "Byte".to_string(),
            Type::U16 => "Short".to_string(),
            Type::U32 => "Int".to_string(),
            Type::U64 => "Long".to_string(),
            Type::S8 => "Byte".to_string(),
            Type::S16 => "Short".to_string(),
            Type::S32 => "Int".to_string(),
            Type::S64 => "Long".to_string(),
            Type::F32 => "Float".to_string(),
            Type::F64 => "Double".to_string(),
            Type::Char => "Char".to_string(),
            Type::String => "String".to_string(),
            Type::Id(id) => {
                let typ = &resolve.types[*id];
                self.render_typedef_reference(owner_context, resolve, typ)
            }
        }
    }

    fn render_typedef_reference(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        typ: &TypeDef,
    ) -> String {
        match &typ.kind {
            TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Flags(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Type(_)
            | TypeDefKind::Variant(_) => {
                let prefix = match self.render_owner(
                    owner_context,
                    resolve,
                    &typ.owner,
                    &typ.kind == &TypeDefKind::Resource,
                ) {
                    Some(owner) => format!("{owner}."),
                    None => "".to_string(),
                };
                format!(
                    "{}{}",
                    prefix,
                    self.keywords.escape(
                        typ.name
                            .clone()
                            .expect("Anonymous types are not supported")
                            .to_pascal_case()
                    )
                )
            }
            TypeDefKind::Handle(handle) => {
                let id = match handle {
                    Handle::Own(id) => id,
                    Handle::Borrow(id) => id,
                };
                let typ = &resolve.types[*id];
                self.render_typedef_reference(owner_context, resolve, typ)
            }
            TypeDefKind::Tuple(tuple) => self.render_tuple(owner_context, resolve, tuple),
            TypeDefKind::Option(option) => {
                if !maybe_null(resolve, option) {
                    format!(
                        "Nullable[{}]",
                        self.render_type_reference(owner_context, resolve, option)
                    )
                } else {
                    format!(
                        "WitOption[{}]",
                        self.render_type_reference(owner_context, resolve, option)
                    )
                }
            }
            TypeDefKind::Result(result) => {
                let ok = result
                    .ok
                    .map(|ok| self.render_type_reference(owner_context, resolve, &ok))
                    .unwrap_or("Unit".to_string());
                let err = result
                    .err
                    .map(|err| self.render_type_reference(owner_context, resolve, &err))
                    .unwrap_or("Unit".to_string());
                format!("WitResult[{ok}, {err}]")
            }
            TypeDefKind::List(list) => {
                format!(
                    "WitList[{}]",
                    self.render_type_reference(owner_context, resolve, list)
                )
            }
            TypeDefKind::Future(_) => panic!("Futures not supported yet"),
            TypeDefKind::Stream(_) => panic!("Streams not supported yet"),
            TypeDefKind::ErrorContext => panic!("ErrorContext not supported yet"),
            TypeDefKind::Unknown => panic!("Unknown type"),
        }
    }

    fn render_tuple(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        tuple: &Tuple,
    ) -> String {
        let arity = tuple.types.len();

        let mut parts = Vec::new();
        for part in &tuple.types {
            parts.push(self.render_type_reference(owner_context, resolve, part));
        }
        format!("WitTuple{arity}[{}]", parts.join(", "))
    }

    fn render_owner(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        owner: &TypeOwner,
        is_resource: bool,
    ) -> Option<String> {
        match owner {
            TypeOwner::World(id) => {
                let world = &resolve.worlds[*id];

                let name = world.name.clone().to_snake_case();

                let package_name = resolve.packages
                    [world.package.expect("missing package for world")]
                .name
                .clone();

                let mut package =
                    package_name_to_segments(&self.opts, &package_name, &Import, &self.keywords);

                package.push(self.keywords.escape(name));

                Some(package.join("."))
            }
            TypeOwner::Interface(id) => match owner_context.is_local_import(id, is_resource) {
                Some(Some(name)) => Some(name),
                Some(None) => None,
                None => {
                    let iface = &resolve.interfaces[*id];
                    let name = iface.name.clone().expect("Interface must have a name");
                    let package_id = iface.package.expect("Interface must have a package");

                    let package = &resolve.packages[package_id];
                    let direction = self.interface_direction(id);

                    let mut segments = package_name_to_segments(
                        &self.opts,
                        &package.name,
                        &direction,
                        &self.keywords,
                    );
                    segments.push(self.keywords.escape(name.to_snake_case()));

                    if is_resource && direction == Import {
                        segments.push(self.keywords.escape(name.to_pascal_case()));
                    }

                    Some(segments.join("."))
                }
            },
            TypeOwner::None => None,
        }
    }

    pub(crate) fn render_typedef(
        &self,
        owner_context: &impl OwnerContext,
        resolve: &Resolve,
        name: &str,
        typ: &TypeDef,
    ) -> Option<String> {
        let encoded_name = self.encode_name(name.to_pascal_case());
        let scala_name = encoded_name.scala;

        let mut source = String::new();
        match &typ.kind {
            TypeDefKind::Record(record) => {
                let mut fields = Vec::new();
                for field in &record.fields {
                    let typ = self.render_type_reference(owner_context, resolve, &field.ty);
                    let field_name = self.encode_name(field.name.to_lower_camel_case());
                    let field_name0 = self
                        .keywords
                        .escape(format!("{}0", field.name.to_lower_camel_case()));
                    fields.push((field_name, field_name0, typ, &field.docs));
                }

                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object {{");
                for (field_name, _, typ, docs) in &fields {
                    write_doc_comment(&mut source, "    ", &docs);
                    field_name.write_rename_attribute(&mut source, "    ");
                    uwriteln!(source, "    val {}: {typ}", field_name.scala);
                }
                uwriteln!(source, "  }}");
                uwriteln!(source, "");
                uwriteln!(source, "  case object {scala_name} {{");
                uwriteln!(source, "    def apply(");
                for (_, field_name0, typ, _) in &fields {
                    uwriteln!(source, "      {field_name0}: {typ},");
                }
                uwriteln!(source, "    ): {scala_name} = {{");
                uwriteln!(source, "      new {scala_name} {{");
                for (field_name, field_name0, typ, _) in &fields {
                    field_name.write_rename_attribute(&mut source, "        ");
                    uwriteln!(
                        source,
                        "        val {}: {typ} = {field_name0}",
                        field_name.scala
                    );
                }
                uwriteln!(source, "      }}");
                uwriteln!(source, "    }}");
                uwriteln!(source, "  }}");
            }
            TypeDefKind::Resource => {
                // resource wrappers are generated separately
            }
            TypeDefKind::Handle(_) => {
                panic!("Unexpected top-level handle type");
            }
            TypeDefKind::Flags(flags) => {
                let mut fields = Vec::new();
                for flag in &flags.flags {
                    let typ = "Boolean".to_string();
                    let field_name = self.encode_name(flag.name.to_lower_camel_case());
                    let field_name0 = self
                        .keywords
                        .escape(format!("{}0", flag.name.to_lower_camel_case()));
                    fields.push((field_name, field_name0, typ, &flag.docs));
                }

                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object {{");
                for (field_name, _, typ, docs) in &fields {
                    write_doc_comment(&mut source, "    ", docs);
                    field_name.write_rename_attribute(&mut source, "    ");
                    uwriteln!(source, "    val {}: {typ}", field_name.scala);
                }
                uwriteln!(source, "  }}");
                uwriteln!(source, "");
                uwriteln!(source, "  case object {scala_name} {{");
                uwriteln!(source, "    def apply(");
                for (_, field_name0, typ, _) in &fields {
                    uwriteln!(source, "      {field_name0}: {typ},");
                }
                uwriteln!(source, "    ): {scala_name} = {{");
                uwriteln!(source, "      new {scala_name} {{");
                for (field_name, field_name0, typ, _) in &fields {
                    field_name.write_rename_attribute(&mut source, "        ");
                    uwriteln!(
                        source,
                        "        val {}: {typ} = {field_name0}",
                        field_name.scala
                    );
                }
                uwriteln!(source, "      }}");
                uwriteln!(source, "    }}");
                uwriteln!(source, "  }}");
            }
            TypeDefKind::Tuple(tuple) => {
                let arity = tuple.types.len();
                write_doc_comment(&mut source, "  ", &typ.docs);
                uwrite!(source, "  type {scala_name} = WitTuple{arity}[");
                for (idx, part) in tuple.types.iter().enumerate() {
                    let part = self.render_type_reference(owner_context, resolve, part);
                    uwrite!(source, "{part}");
                    if idx < tuple.types.len() - 1 {
                        uwrite!(source, ", ");
                    }
                }
                uwriteln!(source, "]");
            }
            TypeDefKind::Variant(variant) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object {{");
                uwriteln!(source, "    type Type");
                uwriteln!(source, "    val tag: String");
                uwriteln!(source, "    val `val`: js.UndefOr[Type]");
                uwriteln!(source, "  }}");
                uwriteln!(source, "");
                uwriteln!(source, "  object {scala_name} {{");
                for case in &variant.cases {
                    let case_name = &case.name;
                    let scala_case_name = self.keywords.escape(case_name.to_lower_camel_case());
                    match &case.ty {
                        Some(ty) => {
                            let typ = self.render_type_reference(owner_context, resolve, ty);
                            write_doc_comment(&mut source, "    ", &case.docs);
                            uwriteln!(source, "    def {scala_case_name}(value: {typ}): {scala_name} = new {scala_name} {{");
                            uwriteln!(source, "      type Type = {typ}");
                            uwriteln!(source, "      val tag: String = \"{case_name}\"");
                            uwriteln!(source, "      val `val`: js.UndefOr[Type] = value");
                            uwriteln!(source, "    }}");
                        }
                        None => {
                            write_doc_comment(&mut source, "    ", &case.docs);
                            uwriteln!(
                                source,
                                "    def {scala_case_name}(): {scala_name} = new {scala_name} {{"
                            );
                            uwriteln!(source, "      type Type = Unit");
                            uwriteln!(source, "      val tag: String = \"{case_name}\"");
                            uwriteln!(source, "      val `val`: js.UndefOr[Type] = ()");
                            uwriteln!(source, "    }}");
                        }
                    }
                }
                uwriteln!(source, "  }}");
            }
            TypeDefKind::Enum(enm) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object");
                uwriteln!(source, "");
                uwriteln!(source, "  object {scala_name} {{");
                for case in &enm.cases {
                    let case_name = &case.name;
                    let scala_case_name = self.keywords.escape(case_name.to_lower_camel_case());
                    write_doc_comment(&mut source, "    ", &case.docs);
                    uwriteln!(
                        source,
                        "    def {scala_case_name}: {scala_name} = \"{case_name}\".asInstanceOf[{scala_name}]",
                    );
                }
                uwriteln!(source, "  }}");
            }
            TypeDefKind::Option(option) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let typ = self.render_type_reference(owner_context, resolve, option);
                if !maybe_null(resolve, option) {
                    uwriteln!(source, "  type {scala_name} = Nullable[{typ}]");
                } else {
                    uwriteln!(source, "  type {scala_name} = WitOption[{typ}]");
                }
            }
            TypeDefKind::Result(result) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let ok = result
                    .ok
                    .map(|ok| self.render_type_reference(owner_context, resolve, &ok))
                    .unwrap_or("Unit".to_string());
                let err = result
                    .err
                    .map(|err| self.render_type_reference(owner_context, resolve, &err))
                    .unwrap_or("Unit".to_string());
                uwriteln!(source, "  type {scala_name} = WitResult[{ok}, {err}]");
            }
            TypeDefKind::List(list) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let typ = self.render_type_reference(owner_context, resolve, list);
                uwriteln!(source, "  type {scala_name} = WitList[{typ}]");
            }
            TypeDefKind::Future(_) => {
                panic!("Futures are not supported yet");
            }
            TypeDefKind::Stream(_) => {
                panic!("Streams are not supported yet");
            }
            TypeDefKind::ErrorContext => {
                panic!("ErrorContext is not supported yet");
            }
            TypeDefKind::Type(reftyp) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let typ = self.render_type_reference(owner_context, resolve, reftyp);
                uwriteln!(source, "  type {scala_name} = {typ}");
            }
            TypeDefKind::Unknown => {
                panic!("Unknown type");
            }
        }

        if source.len() > 0 {
            Some(source)
        } else {
            None
        }
    }

    fn interface_direction(&self, id: &InterfaceId) -> Direction {
        if self.imports.contains(id) {
            Import
        } else if self.exports.contains(id) {
            Export
        } else {
            // Have not seen it yet, so it must be also an export
            Export
        }
    }
}

pub struct ScalaJsFile {
    pub package: Vec<String>,
    pub name: String,
    pub source: String,
}

impl ScalaJsFile {
    pub fn path(&self, optional_root: &Option<String>) -> String {
        // TODO: use PathBuf
        match optional_root {
            Some(root) => format!("{}/{}/{}.scala", root, self.package.join("/"), self.name),
            None => format!("{}/{}.scala", self.package.join("/"), self.name),
        }
    }
}

pub fn package_name_to_segments(
    opts: &Opts,
    package_name: &PackageName,
    direction: &Direction,
    keywords: &ScalaKeywords,
) -> Vec<String> {
    let mut segments = opts.base_package_segments();

    if direction == &Export {
        segments.push("export".to_string());
    }

    segments.push(package_name.namespace.to_snake_case());
    segments.push(package_name.name.to_snake_case());
    if let Some(version) = &package_name.version {
        segments.push(format!("v{}", version.to_string().to_snake_case()));
    }
    segments.into_iter().map(|s| keywords.escape(s)).collect()
}

pub fn write_doc_comment(source: &mut impl Write, indent: &str, docs: &Docs) {
    // TODO: rewrite types in `` blocks?
    if !docs.is_empty() {
        uwriteln!(source, "{}/**", indent);
        for line in docs.contents.as_ref().unwrap().lines() {
            uwriteln!(source, "{} * {}", indent, line);
        }
        uwriteln!(source, "{} */", indent);
    }
}

#[allow(dead_code)]
pub struct EncodedName {
    pub scala: String,
    pub js: String,
    pub rename_attribute: String,
}

impl EncodedName {
    pub fn write_rename_attribute(&self, target: &mut impl Write, ident: &str) {
        if self.rename_attribute.len() > 0 {
            uwriteln!(target, "{}{}", ident, self.rename_attribute);
        }
    }
}
