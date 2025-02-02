mod jco;

use crate::jco::{maybe_null, to_js_identifier};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write};
use std::str::FromStr;
use wit_bindgen_core::wit_parser::{Docs, Function, FunctionKind, Handle, Interface, InterfaceId, PackageName, Resolve, Results, Tuple, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, World, WorldId, WorldKey};
use wit_bindgen_core::Direction::{Export, Import};
use wit_bindgen_core::{uwrite, uwriteln, Direction, Files, WorldGenerator};

#[derive(Debug, Clone)]
pub enum ScalaDialect {
    Scala2,
    Scala3,
}

impl Default for ScalaDialect {
    fn default() -> Self {
        ScalaDialect::Scala2
    }
}

impl Display for ScalaDialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalaDialect::Scala2 => write!(f, "scala2"),
            ScalaDialect::Scala3 => write!(f, "scala3"),
        }
    }
}

impl FromStr for ScalaDialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scala2" => Ok(ScalaDialect::Scala2),
            "scala3" => Ok(ScalaDialect::Scala3),
            _ => Err("Invalid Scala dialect".to_string()),
        }
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    #[cfg_attr(
        feature = "clap",
        clap(long, help = "Base package for generated Scala.js code")
    )]
    pub base_package: Option<String>,

    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            help = "Scala dialect to generate code for",
            default_value = "scala2"
        )
    )]
    pub scala_dialect: ScalaDialect,

    // TODO: generate skeleton mode - single file with the exported things to be implemented - destructive, will be wired to an explicit sbt command
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(ScalaJsWorld::new(self.clone()))
    }

    pub fn base_package_segments(&self) -> Vec<String> {
        self.base_package
            .clone()
            .map(|pkg| pkg.split('.').map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn base_package_prefix(&self) -> String {
        match &self.base_package {
            Some(pkg) => format!("{pkg}."),
            None => "".to_string(),
        }
    }
}

struct ScalaKeywords {
    keywords: HashSet<String>,
    base_methods: HashSet<String>,
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

    fn escape(&self, ident: impl AsRef<str>) -> String {
        if self.keywords.contains(ident.as_ref()) {
            format!("`{}`", ident.as_ref())
        } else {
            ident.as_ref().to_string()
        }
    }
}

// TODO: refactor to context, include resolve
trait OwnerContext {
    fn is_local_import(&self, id: &InterfaceId, is_resource: bool) -> Option<Option<String>>;
}

impl OwnerContext for ScalaJsWorld {
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

struct ScalaJsWorld {
    opts: Opts,
    generated_files: Vec<ScalaJsFile>,
    keywords: ScalaKeywords,
    overrides: HashMap<TypeId, String>,
    imports: HashSet<InterfaceId>,
    exports: HashSet<InterfaceId>,
    world_defs: HashMap<WorldId, String>,
}

impl ScalaJsWorld {
    fn new(opts: Opts) -> Self {
        let keywords = ScalaKeywords::new(&opts.scala_dialect);
        Self {
            opts,
            generated_files: Vec::new(),
            keywords,
            overrides: HashMap::new(),
            imports: HashSet::new(),
            exports: HashSet::new(),
            world_defs: HashMap::new(),
        }
    }

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

    fn render_args<'b>(&self, owner_context: &impl OwnerContext,  resolve: &Resolve, params: impl Iterator<Item = &'b (String, Type)>) -> String {
        let mut args = Vec::new();
        for (param_name, param_typ) in params {
            let param_typ = self.render_type_reference(owner_context, resolve, param_typ);
            let param_name = self.encode_name(param_name.to_lower_camel_case());
            args.push(format!("{}: {param_typ}", param_name.scala));
        }
        args.join(", ")
    }

    fn render_return_type(&self, owner_context: &impl OwnerContext, resolve: &Resolve, results: &Results) -> String {
        match results {
            Results::Named(results) if results.len() == 0 => "Unit".to_string(),
            Results::Named(results) if results.len() == 1 => {
                self.render_type_reference(owner_context, resolve, &results.iter().next().unwrap().1)
            }
            Results::Named(results) => self.render_tuple(owner_context, resolve, &Tuple {
                types: results.iter().map(|(_, typ)| typ.clone()).collect(),
            }),
            Results::Anon(typ) => self.render_type_reference(owner_context, resolve, typ),
        }
    }

    fn render_type_reference(&self, owner_context: &impl OwnerContext, resolve: &Resolve, typ: &Type) -> String {
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

    fn render_typedef_reference(&self, owner_context: &impl OwnerContext, resolve: &Resolve, typ: &TypeDef) -> String {
        match &typ.kind {
            TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Flags(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Type(_)
            | TypeDefKind::Variant(_) => {
                let prefix =
                    match self.render_owner(owner_context, resolve, &typ.owner, &typ.kind == &TypeDefKind::Resource) {
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
                    format!("Nullable[{}]", self.render_type_reference(owner_context, resolve, option))
                } else {
                    format!("WitOption[{}]", self.render_type_reference(owner_context, resolve, option))
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
                format!("WitList[{}]", self.render_type_reference(owner_context, resolve, list))
            }
            TypeDefKind::Future(_) => panic!("Futures not supported yet"),
            TypeDefKind::Stream(_) => panic!("Streams not supported yet"),
            TypeDefKind::ErrorContext => panic!("ErrorContext not supported yet"),
            TypeDefKind::Unknown => panic!("Unknown type"),
        }
    }

    fn render_tuple(&self, owner_context: &impl OwnerContext, resolve: &Resolve, tuple: &Tuple) -> String {
        let arity = tuple.types.len();

        let mut parts = Vec::new();
        for part in &tuple.types {
            parts.push(self.render_type_reference(owner_context, resolve, part));
        }
        format!("WitTuple{arity}[{}]", parts.join(", "))
    }

    fn render_owner(&self, owner_context: &impl OwnerContext, resolve: &Resolve, owner: &TypeOwner, is_resource: bool) -> Option<String> {
        match owner {
            TypeOwner::World(id) => {
                let world = &resolve.worlds[*id];

                let name = world
                    .name
                    .clone()
                    .to_snake_case();

                let package_name = resolve.packages
                    [world.package.expect("missing package for world")]
                    .name
                    .clone();

                let mut package = package_name_to_segments(
                    &self.opts,
                    &package_name,
                    &Import,
                    &self.keywords,
                );

                package.push(self.keywords.escape(name));

                Some(package.join("."), )
            }
            TypeOwner::Interface(id) => {
                match owner_context.is_local_import(id, is_resource) {
                    Some(Some(name)) => {
                        Some(name)
                    }
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
                }
            }
            TypeOwner::None => None,
        }
    }

    fn render_typedef(&self, owner_context: &impl OwnerContext, resolve: &Resolve, name: &str, typ: &TypeDef) -> Option<String> {
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
                    let scala_case_name = self
                        .keywords
                        .escape(case_name.to_lower_camel_case());
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
                    let scala_case_name = self
                        .keywords
                        .escape(case_name.to_lower_camel_case());
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

    fn generate_world_package_header(&self, resolve: &Resolve, world: &World) -> String {
        let name = world
            .name
            .clone()
            .to_snake_case();

        let package_name = resolve.packages
            [world.package.expect("missing package for world")]
            .name
            .clone();

        let package = package_name_to_segments(
            &self.opts,
            &package_name,
            &Import,
            &self.keywords,
        );

        let mut source = String::new();
        uwriteln!(source, "package {}",  package.join("."));

        uwriteln!(source, "");
        uwriteln!(source, "import scala.scalajs.js");
        uwriteln!(source, "import scala.scalajs.js.annotation._");
        uwriteln!(source, "import {}wit._", self.opts.base_package_prefix());
        uwriteln!(source, "");
        uwriteln!(source, "package object {} {{", name);
        source
    }
}

impl WorldGenerator for ScalaJsWorld {
    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let key = name;
        let wit_name = resolve.name_world_key(key);

        self.imports.insert(iface);
        let mut scalajs_iface =
            ScalaJsInterface::new(wit_name.clone(), resolve, iface, Import, self);
        scalajs_iface.generate();

        let file = scalajs_iface.finalize();
        self.generated_files.push(file);

        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        iface: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let key = name;
        let wit_name = resolve.name_world_key(key);

        self.exports.insert(iface);
        let mut scalajs_iface =
            ScalaJsInterface::new(wit_name.clone(), resolve, iface, Export, self);
        scalajs_iface.generate();

        let file = scalajs_iface.finalize();
        self.generated_files.push(file);

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world_id];

        if !self.world_defs.contains_key(&world_id) {
            let source = self.generate_world_package_header(resolve, world);
            self.world_defs.insert(world_id, source);
        }

        let mut func_imports = String::new();
        for (func_name, func) in funcs {
            uwriteln!(func_imports, "  @js.native");
            uwriteln!(func_imports, "  @JSImport(\"{}\", JSImport.Default)", func_name);
            let encoded_name = self.encode_name(func_name.to_lower_camel_case());
            let args = self.render_args(self, resolve, func.params.iter());
            let ret = self.render_return_type(self, resolve, &func.results);

            write_doc_comment(&mut func_imports, "  ", &func.docs);
            uwriteln!(func_imports, "  def {}({args}): {ret} = js.native", encoded_name.scala);
        }

        let world_source = self.world_defs.get_mut(&world_id).unwrap();
        uwriteln!(world_source, "{}", func_imports);
    }

    fn export_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        // TODO
        println!("export_funcs: {:?}", funcs);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world_id: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let world = &resolve.worlds[world_id];

        if !self.world_defs.contains_key(&world_id) {
            let source = self.generate_world_package_header(resolve, world);
            self.world_defs.insert(world_id, source);
        }

        let mut type_snippets = String::new();
        for (type_name, type_id) in types {
            if let Some(type_snippet) = self.render_typedef(self, resolve, type_name, &resolve.types[*type_id]) {
                uwriteln!(type_snippets, "{}", type_snippet);
                uwriteln!(type_snippets, "");
            }
        }

        let world_source = self.world_defs.get_mut(&world_id).unwrap();
        uwriteln!(world_source, "{}", type_snippets);
    }

    fn finish(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        files: &mut Files,
    ) -> anyhow::Result<()> {
        for file in &self.generated_files {
            files.push(&file.path(), file.source.as_bytes());
        }

        for (world_id, source) in &mut self.world_defs {
            uwriteln!(source, "}}");

            let world = &resolve.worlds[*world_id];
            let package_name = resolve.packages
                [world.package.expect("missing package for world")]
                .name
                .clone();

            let package = package_name_to_segments(
                &self.opts,
                &package_name,
                &Import,
                &self.keywords,
            );

            let path = format!("{}/{}.scala", package.join("/"), world.name.to_snake_case());
            files.push(&path, source.as_bytes());
        }

        let rt = render_runtime_module(&self.opts);
        files.push(&rt.path(), rt.source.as_bytes());

        Ok(())
    }
}

struct ScalaJsFile {
    package: Vec<String>,
    name: String,
    source: String,
}

impl ScalaJsFile {
    fn path(&self) -> String {
        format!("{}/{}.scala", self.package.join("/"), self.name)
    }
}

struct ScalaJsInterface<'a> {
    wit_name: String,
    name: String,
    source: String,
    package: Vec<String>,
    resolve: &'a Resolve,
    interface: &'a Interface,
    interface_id: InterfaceId,
    direction: Direction,
    generator: &'a mut ScalaJsWorld,
}

impl<'a> ScalaJsInterface<'a> {
    // TODO: should just get a reference to ScalaJsWorld
    pub fn new(
        wit_name: String,
        resolve: &'a Resolve,
        interface_id: InterfaceId,
        direction: Direction,
        generator: &'a mut ScalaJsWorld,
    ) -> Self {
        let interface = &resolve.interfaces[interface_id];
        let name = interface
            .name
            .clone()
            .unwrap_or(wit_name.clone())
            .to_pascal_case();

        let package_name = resolve.packages
            [interface.package.expect("missing package for interface")]
        .name
        .clone();

        let package = package_name_to_segments(
            &generator.opts,
            &package_name,
            &direction,
            &generator.keywords,
        );

        Self {
            wit_name,
            name,
            source: "".to_string(),
            package,
            resolve,
            interface,
            interface_id,
            direction,
            generator,
        }
    }

    pub fn generate(&mut self) {
        match self.direction {
            Import => self.generate_import(),
            Export => self.generate_export(),
        }
    }

    pub fn generate_import(&mut self) {
        let mut source = String::new();
        self.generate_package_header(&mut source);

        let types = self.collect_type_definition_snippets();
        let imported_resources = self.collect_imported_resources();
        let functions = self.collect_function_snippets();

        for typ in types {
            uwriteln!(source, "{}", typ);
            uwriteln!(source, "");
        }

        write_doc_comment(&mut source, "  ", &self.interface.docs);
        uwriteln!(source, "  @js.native");
        uwriteln!(source, "  trait {} extends js.Object {{", self.name);

        for (_, resource) in imported_resources {
            uwriteln!(source, "{}", resource.finalize());
            uwriteln!(source, "");
        }

        for function in functions {
            uwriteln!(source, "{function}");
        }

        uwriteln!(source, "  }}");

        uwriteln!(source, "");
        uwriteln!(source, "  @js.native");
        uwriteln!(
            source,
            "  @JSImport(\"{}\", JSImport.Namespace)",
            self.wit_name
        );
        uwriteln!(source, "  object {} extends {}", self.name, self.name);

        uwriteln!(source, "}}");
        self.source = source;
    }

    pub fn generate_export(&mut self) {
        let mut source = String::new();
        self.generate_package_header(&mut source);

        let types = self.collect_type_definition_snippets();
        let exported_resources = self.collect_exported_resources();
        let functions = self.collect_function_snippets();

        for typ in types {
            uwriteln!(source, "{}", typ);
            uwriteln!(source, "");
        }

        for (_, resource) in exported_resources {
            uwriteln!(source, "{}", resource.finalize());
            uwriteln!(source, "");
        }

        write_doc_comment(&mut source, "  ", &self.interface.docs);
        uwriteln!(source, "  trait {} extends js.Object {{", self.name);
        for function in functions {
            uwriteln!(source, "{function}");
        }
        uwriteln!(source, "  }}");

        uwriteln!(source, "}}");
        self.source = source;
    }

    fn generate_package_header(&mut self, source: &mut String) {
        uwriteln!(source, "package {}", self.package.join("."));
        uwriteln!(source, "");
        uwriteln!(source, "import scala.scalajs.js");
        uwriteln!(source, "import scala.scalajs.js.annotation._");
        uwriteln!(
            source,
            "import {}wit._",
            self.generator.opts.base_package_prefix()
        );
        uwriteln!(source, "");

        uwriteln!(
            source,
            "package object {} {{",
            self.generator.keywords.escape(self.name.to_snake_case())
        );
    }

    fn collect_type_definition_snippets(&mut self) -> Vec<String> {
        let mut types = Vec::new();

        for (type_name, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];

            let type_name = self.generator.overrides.get(type_id).unwrap_or(type_name);
            let type_name = if type_name.eq_ignore_ascii_case(&self.name) {
                let overridden_type_name = format!("{}Type", type_name);
                self.generator
                    .overrides
                    .insert(*type_id, overridden_type_name.clone());
                overridden_type_name
            } else {
                type_name.clone()
            };

            if let Some(typ) = self.generator.render_typedef(self, &self.resolve, &type_name, type_def) {
                types.push(typ);
            }
        }

        types
    }

    fn collect_imported_resources(&self) -> HashMap<TypeId, ScalaJsImportedResource> {
        let mut imported_resources = HashMap::new();
        for (_, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];
            if let TypeDefKind::Resource = &type_def.kind {
                imported_resources
                    .entry(*type_id)
                    .or_insert_with(|| ScalaJsImportedResource::new(self, *type_id));
            }
        }

        for (func_name, func) in &self.interface.functions {
            match func.kind {
                FunctionKind::Method(resource_type)
                | FunctionKind::Static(resource_type)
                | FunctionKind::Constructor(resource_type) => {
                    let resource = imported_resources
                        .entry(resource_type)
                        .or_insert_with(|| ScalaJsImportedResource::new(self, resource_type));
                    resource.add_function(func_name, func);
                }
                FunctionKind::Freestanding => {}
            }
        }

        imported_resources
    }

    fn collect_exported_resources(&self) -> HashMap<TypeId, ScalaJsExportedResource> {
        let mut exported_resources = HashMap::new();

        for (_, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];
            if let TypeDefKind::Resource = &type_def.kind {
                exported_resources
                    .entry(*type_id)
                    .or_insert_with(|| ScalaJsExportedResource::new(self, *type_id));
            }
        }

        for (func_name, func) in &self.interface.functions {
            match func.kind {
                FunctionKind::Method(resource_type)
                | FunctionKind::Static(resource_type)
                | FunctionKind::Constructor(resource_type) => {
                    let resource = exported_resources
                        .entry(resource_type)
                        .or_insert_with(|| ScalaJsExportedResource::new(self, resource_type));
                    resource.add_function(func_name, func);
                }
                FunctionKind::Freestanding => {}
            }
        }

        exported_resources
    }

    fn collect_function_snippets(&self) -> Vec<String> {
        let mut functions = Vec::new();

        for (func_name, func) in &self.interface.functions {
            let func_name = self.generator.encode_name(func_name.to_lower_camel_case());

            match func.kind {
                FunctionKind::Freestanding => {
                    let args = self.generator.render_args(self, self.resolve, func.params.iter());
                    let ret = self.generator.render_return_type(self, self.resolve, &func.results);

                    let mut function = String::new();
                    write_doc_comment(&mut function, "    ", &func.docs);

                    let postfix = match self.direction {
                        Import => " = js.native",
                        Export => "",
                    };

                    func_name.write_rename_attribute(&mut function, "    ");
                    uwriteln!(
                        function,
                        "    def {}({args}): {ret}{postfix}",
                        func_name.scala
                    );
                    functions.push(function);
                }
                FunctionKind::Method(_)
                | FunctionKind::Static(_)
                | FunctionKind::Constructor(_) => {}
            }
        }

        functions
    }

    pub fn finalize(self) -> ScalaJsFile {
        ScalaJsFile {
            package: self.package,
            name: self.name,
            source: self.source,
        }
    }
}

struct ScalaJsImportedResource<'a> {
    owner: &'a ScalaJsInterface<'a>,
    _resource_id: TypeId,
    resource_name: String,
    class_header: String,
    class_source: String,
    object_source: String,
    constructor_args: String,
}

impl<'a> ScalaJsImportedResource<'a> {
    pub fn new(owner: &'a ScalaJsInterface<'a>, resource_id: TypeId) -> Self {
        let resource = &owner.resolve.types[resource_id];
        let resource_name = resource
            .name
            .clone()
            .expect("Anonymous resources not supported");
        let encoded_resource_name = owner.generator.encode_name(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        uwriteln!(class_header, "    @js.native");
        encoded_resource_name.write_rename_attribute(&mut class_header, "    ");
        uwrite!(class_header, "    class {}(", encoded_resource_name.scala);

        let mut class_source = String::new();
        uwriteln!(class_source, ") extends js.Object {{");

        let mut object_source = String::new();
        uwriteln!(object_source, "    @js.native");
        uwriteln!(
            object_source,
            "    object {} extends js.Object {{",
            encoded_resource_name.scala
        );

        Self {
            owner,
            _resource_id: resource_id,
            resource_name,
            class_header,
            class_source,
            object_source,
            constructor_args: String::new(),
        }
    }

    pub fn add_function(&mut self, func_name: &str, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding => unreachable!(),
            FunctionKind::Method(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter().skip(1));
                let ret = self.owner.generator.render_return_type(self.owner, self.owner.resolve, &func.results);
                let encoded_func_name = self.get_func_name("[method]", func_name);

                let overrd = if self
                    .owner
                    .generator
                    .keywords
                    .base_methods
                    .contains(&encoded_func_name.scala)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                encoded_func_name.write_rename_attribute(&mut self.class_source, "      ");
                uwriteln!(
                    self.class_source,
                    "      {overrd}def {}({args}): {ret} = js.native",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Static(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter());
                let ret = self.owner.generator.render_return_type(self.owner, self.owner.resolve, &func.results);

                let encoded_func_name = self.get_func_name("[static]", func_name);
                write_doc_comment(&mut self.object_source, "      ", &func.docs);
                encoded_func_name.write_rename_attribute(&mut self.object_source, "      ");
                uwriteln!(
                    self.object_source,
                    "      def {}({args}): {ret} = js.native",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter());
                self.constructor_args = args;
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = self.class_header;
        uwrite!(class_source, "{}", self.constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "    }}");
        let mut object_source = self.object_source;
        uwriteln!(object_source, "    }}");
        format!("{}\n{}\n", class_source, object_source)
    }

    fn get_func_name(&self, prefix: &str, func_name: &str) -> EncodedName {
        let name = func_name
            .strip_prefix(prefix)
            .unwrap()
            .strip_prefix(&self.resource_name)
            .unwrap()
            .to_lower_camel_case();
        self.owner.generator.encode_name(name)
    }
}

struct ScalaJsExportedResource<'a> {
    owner: &'a ScalaJsInterface<'a>,
    _resource_id: TypeId,
    resource_name: String,
    class_header: String,
    class_source: String,
    static_methods: String,
    constructor_args: String,
}

impl<'a> ScalaJsExportedResource<'a> {
    pub fn new(owner: &'a ScalaJsInterface<'a>, resource_id: TypeId) -> Self {
        let resource = &owner.resolve.types[resource_id];
        let resource_name = resource
            .name
            .clone()
            .expect("Anonymous resources not supported");
        let encoded_resource_name = owner.generator.encode_name(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        encoded_resource_name.write_rename_attribute(&mut class_header, "    // ");
        uwrite!(
            class_header,
            "  abstract class {}(",
            encoded_resource_name.scala
        );

        let mut class_source = String::new();
        uwriteln!(class_source, ") extends js.Object {{");

        Self {
            owner,
            _resource_id: resource_id,
            resource_name,
            class_header,
            class_source,
            static_methods: String::new(),
            constructor_args: String::new(),
        }
    }

    pub fn add_function(&mut self, func_name: &str, func: &Function) {
        match &func.kind {
            FunctionKind::Freestanding => unreachable!(),
            FunctionKind::Method(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter().skip(1));
                let ret = self.owner.generator.render_return_type(self.owner, self.owner.resolve, &func.results);
                let encoded_func_name = self.get_func_name("[method]", func_name);

                let overrd = if self
                    .owner
                    .generator
                    .keywords
                    .base_methods
                    .contains(&encoded_func_name.scala)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                encoded_func_name.write_rename_attribute(&mut self.class_source, "      ");
                uwriteln!(
                    self.class_source,
                    "    {overrd}def {}({args}): {ret}",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Static(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter());
                let ret = self.owner.generator.render_return_type(self.owner, self.owner.resolve, &func.results);

                let encoded_func_name = self.get_func_name("[static]", func_name);
                write_doc_comment(&mut self.static_methods, "      ", &func.docs);
                uwriteln!(self.static_methods, "    // @JSExportStatic");
                encoded_func_name.write_rename_attribute(&mut self.static_methods, "    ");
                uwriteln!(
                    self.static_methods,
                    "    def {}({args}): {ret}",
                    encoded_func_name.scala
                );
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.generator.render_args(self.owner, self.owner.resolve, func.params.iter());
                self.constructor_args = args;
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = self.class_header;
        uwrite!(class_source, "{}", self.constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "  }}");
        uwriteln!(class_source, "");

        let scala_resource_name = self
            .owner
            .generator
            .keywords
            .escape(self.resource_name.to_pascal_case());
        uwriteln!(class_source, "  trait {}Static {{", scala_resource_name);
        uwriteln!(class_source, "{}", self.static_methods);
        uwriteln!(class_source, "  }}");
        class_source
    }

    fn get_func_name(&self, prefix: &str, func_name: &str) -> EncodedName {
        let name = func_name
            .strip_prefix(prefix)
            .unwrap()
            .strip_prefix(&self.resource_name)
            .unwrap()
            .to_lower_camel_case();
        self.owner.generator.encode_name(name)
    }
}

fn render_runtime_module(opts: &Opts) -> ScalaJsFile {
    let wit_scala = include_str!("../scala/wit.scala");

    let mut package = opts.base_package_segments();
    package.push("wit".to_string());

    let mut source = String::new();
    uwriteln!(source, "package {}", opts.base_package_segments().join("."));
    uwriteln!(source, "");
    uwriteln!(source, "{wit_scala}");

    ScalaJsFile {
        package,
        name: "package".to_string(),
        source,
    }
}

fn package_name_to_segments(
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

fn write_doc_comment(source: &mut impl Write, indent: &str, docs: &Docs) {
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
struct EncodedName {
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
