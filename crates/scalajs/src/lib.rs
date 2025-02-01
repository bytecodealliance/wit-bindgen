use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write};
use std::str::FromStr;
use wit_bindgen_core::wit_parser::{
    Docs, Function, FunctionKind, Handle, Interface, InterfaceId, PackageName, Resolve, Results,
    Type, TypeDef, TypeDefKind, TypeId, TypeOwner, WorldId, WorldKey,
};
use wit_bindgen_core::Direction::{Export, Import};
use wit_bindgen_core::{uwrite, uwriteln, Direction, Files, WorldGenerator};

// TODO: need to use https://github.com/golemcloud/jco/blob/main/crates/js-component-bindgen/src/names.rs#L81 for every name, and use JSName where it differs from the scala name

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

struct ScalaJsWorld {
    opts: Opts,
    generated_files: Vec<ScalaJsFile>,
    keywords: ScalaKeywords,
    overrides: HashMap<TypeId, String>,
    imports: HashSet<InterfaceId>,
    exports: HashSet<InterfaceId>,
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
        }
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
        let mut scalajs_iface = ScalaJsInterface::new(
            wit_name.clone(),
            resolve,
            iface,
            &self.opts,
            Import,
            &self.keywords,
            &mut self.overrides,
            &self.imports,
            &self.exports,
        );
        scalajs_iface.generate();
        self.generated_files.push(scalajs_iface.finalize());

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
        let mut scalajs_iface = ScalaJsInterface::new(
            wit_name.clone(),
            resolve,
            iface,
            &self.opts,
            Export,
            &self.keywords,
            &mut self.overrides,
            &self.imports,
            &self.exports,
        );
        scalajs_iface.generate();
        self.generated_files.push(scalajs_iface.finalize());

        Ok(())
    }

    fn import_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        // TODO
        println!("import_funcs: {:?}", funcs);
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
        _resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        // TODO
        println!("import_types: {:?}", types);
    }

    fn finish(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        files: &mut Files,
    ) -> anyhow::Result<()> {
        for file in &self.generated_files {
            files.push(&file.path(), file.source.as_bytes());
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
    opts: &'a Opts,
    resolve: &'a Resolve,
    interface: &'a Interface,
    interface_id: InterfaceId,
    direction: Direction,
    scala_keywords: &'a ScalaKeywords,
    overrides: &'a mut HashMap<TypeId, String>,
    imports: &'a HashSet<InterfaceId>,
    exports: &'a HashSet<InterfaceId>,
}

impl<'a> ScalaJsInterface<'a> {
    // TODO: should just get a reference to ScalaJsWorld
    pub fn new(
        wit_name: String,
        resolve: &'a Resolve,
        interface_id: InterfaceId,
        opts: &'a Opts,
        direction: Direction,
        scala_keywords: &'a ScalaKeywords,
        overrides: &'a mut HashMap<TypeId, String>,
        imports: &'a HashSet<InterfaceId>,
        exports: &'a HashSet<InterfaceId>,
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

        let package = package_name_to_segments(&opts, &package_name, &direction, &scala_keywords);

        Self {
            wit_name,
            name,
            source: "".to_string(),
            package,
            opts,
            resolve,
            interface,
            interface_id,
            direction,
            scala_keywords,
            overrides,
            imports,
            exports,
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
        uwriteln!(source, "  trait {} {{", self.name);
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
        uwriteln!(source, "import {}wit._", self.opts.base_package_prefix());
        uwriteln!(source, "");

        uwriteln!(
            source,
            "package object {} {{",
            self.scala_keywords.escape(self.name.to_snake_case())
        );
    }

    fn collect_type_definition_snippets(&mut self) -> Vec<String> {
        let mut types = Vec::new();

        for (type_name, type_id) in &self.interface.types {
            let type_def = &self.resolve.types[*type_id];

            let type_name = self.overrides.get(type_id).unwrap_or(type_name);
            let type_name = if type_name.eq_ignore_ascii_case(&self.name) {
                let overridden_type_name = format!("{}Type", type_name);
                self.overrides
                    .insert(*type_id, overridden_type_name.clone());
                overridden_type_name
            } else {
                type_name.clone()
            };

            if let Some(typ) = self.render_typedef(&type_name, type_def) {
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
            let scala_func_name = self.scala_keywords.escape(func_name.to_lower_camel_case());

            match func.kind {
                FunctionKind::Freestanding => {
                    let args = self.render_args(func.params.iter());

                    let ret = match &func.results {
                        Results::Named(params) if params.len() == 0 => "Unit".to_string(),
                        Results::Named(_) => panic!("Named results not supported yet"), // TODO
                        Results::Anon(typ) => self.render_type_reference(typ),
                    };

                    let mut function = String::new();
                    write_doc_comment(&mut function, "    ", &func.docs);

                    let postfix = match self.direction {
                        Import => " = js.native",
                        Export => "",
                    };

                    uwriteln!(
                        function,
                        "    def {scala_func_name}({args}): {ret}{postfix}"
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

    fn render_args<'b>(&self, params: impl Iterator<Item = &'b (String, Type)>) -> String {
        let mut args = Vec::new();
        for (param_name, param_typ) in params {
            let param_typ = self.render_type_reference(param_typ);
            let param_name = self.scala_keywords.escape(param_name.to_lower_camel_case());
            args.push(format!("{param_name}: {param_typ}"));
        }
        args.join(", ")
    }

    fn render_return_type(&self, results: &Results) -> String {
        match results {
            Results::Named(params) if params.len() == 0 => "Unit".to_string(),
            Results::Named(_) => panic!("Named results not supported yet"), // TODO
            Results::Anon(typ) => self.render_type_reference(typ),
        }
    }

    fn render_type_reference(&self, typ: &Type) -> String {
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
                let typ = &self.resolve.types[*id];
                self.render_typedef_reference(typ)
            }
        }
    }

    fn render_typedef_reference(&self, typ: &TypeDef) -> String {
        match &typ.kind {
            TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Flags(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Type(_)
            | TypeDefKind::Variant(_) => {
                let prefix =
                    match self.render_owner(&typ.owner, &typ.kind == &TypeDefKind::Resource) {
                        Some(owner) => format!("{owner}."),
                        None => "".to_string(),
                    };
                format!(
                    "{}{}",
                    prefix,
                    self.scala_keywords.escape(
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
                let typ = &self.resolve.types[*id];
                self.render_typedef_reference(typ)
            }
            TypeDefKind::Tuple(tuple) => {
                let arity = tuple.types.len();

                let mut parts = Vec::new();
                for part in &tuple.types {
                    parts.push(self.render_type_reference(part));
                }
                format!("WitTuple{arity}[{}]", parts.join(", "))
            }
            TypeDefKind::Option(option) => {
                if !maybe_null(&self.resolve, option) {
                    format!("Nullable[{}]", self.render_type_reference(option))
                } else {
                    format!("WitOption[{}]", self.render_type_reference(option))
                }
            }
            TypeDefKind::Result(result) => {
                let ok = result
                    .ok
                    .map(|ok| self.render_type_reference(&ok))
                    .unwrap_or("Unit".to_string());
                let err = result
                    .err
                    .map(|err| self.render_type_reference(&err))
                    .unwrap_or("Unit".to_string());
                format!("WitResult[{ok}, {err}]")
            }
            TypeDefKind::List(list) => {
                format!("WitList[{}]", self.render_type_reference(list))
            }
            TypeDefKind::Future(_) => panic!("Futures not supported yet"),
            TypeDefKind::Stream(_) => panic!("Streams not supported yet"),
            TypeDefKind::ErrorContext => panic!("ErrorContext not supported yet"),
            TypeDefKind::Unknown => panic!("Unknown type"),
        }
    }

    fn render_owner(&self, owner: &TypeOwner, is_resource: bool) -> Option<String> {
        match owner {
            TypeOwner::World(id) => {
                let world = &self.resolve.worlds[*id];
                // TODO: assuming an object or trait is also generated per world?
                Some(
                    self.scala_keywords
                        .escape(world.name.clone().to_pascal_case()),
                )
            }
            TypeOwner::Interface(id) if id == &self.interface_id => {
                if is_resource && self.direction == Import {
                    Some(self.name.clone())
                } else {
                    None
                }
            }
            TypeOwner::Interface(id) => {
                let iface = &self.resolve.interfaces[*id];
                let name = iface.name.clone().expect("Interface must have a name");
                let package_id = iface.package.expect("Interface must have a package");

                let package = &self.resolve.packages[package_id];
                let direction = self.interface_direction(id);

                let mut segments = package_name_to_segments(
                    &self.opts,
                    &package.name,
                    &direction,
                    self.scala_keywords,
                );
                segments.push(self.scala_keywords.escape(name.to_snake_case()));

                if is_resource && direction == Import {
                    segments.push(self.scala_keywords.escape(name.to_pascal_case()));
                }

                Some(segments.join("."))
            }
            TypeOwner::None => None,
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

    fn render_typedef(&self, name: &str, typ: &TypeDef) -> Option<String> {
        let scala_name = self.scala_keywords.escape(name.to_pascal_case());

        let mut source = String::new();
        match &typ.kind {
            TypeDefKind::Record(record) => {
                let mut fields = Vec::new();
                for field in &record.fields {
                    let typ = self.render_type_reference(&field.ty);
                    let field_name = self.scala_keywords.escape(field.name.to_lower_camel_case());
                    let field_name0 = self
                        .scala_keywords
                        .escape(format!("{}0", field.name.to_lower_camel_case()));
                    fields.push((field_name, field_name0, typ, &field.docs));
                }

                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object {{");
                for (field_name, _, typ, docs) in &fields {
                    write_doc_comment(&mut source, "    ", &docs);
                    uwriteln!(source, "    val {field_name}: {typ}");
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
                    uwriteln!(source, "        val {field_name}: {typ} = {field_name0}");
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
                    let field_name = self.scala_keywords.escape(flag.name.to_lower_camel_case());
                    let field_name0 = self
                        .scala_keywords
                        .escape(format!("{}0", flag.name.to_lower_camel_case()));
                    fields.push((field_name, field_name0, typ, &flag.docs));
                }

                write_doc_comment(&mut source, "  ", &typ.docs);
                uwriteln!(source, "  sealed trait {scala_name} extends js.Object {{");
                for (field_name, _, typ, docs) in &fields {
                    write_doc_comment(&mut source, "    ", docs);
                    uwriteln!(source, "    val {field_name}: {typ}");
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
                    uwriteln!(source, "        val {field_name}: {typ} = {field_name0}");
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
                    let part = self.render_type_reference(part);
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
                    let scala_case_name =
                        self.scala_keywords.escape(case_name.to_lower_camel_case());
                    match &case.ty {
                        Some(ty) => {
                            let typ = self.render_type_reference(ty);
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
                    let scala_case_name =
                        self.scala_keywords.escape(case_name.to_lower_camel_case());
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
                let typ = self.render_type_reference(option);
                if !maybe_null(&self.resolve, option) {
                    uwriteln!(source, "  type {scala_name} = Nullable[{typ}]");
                } else {
                    uwriteln!(source, "  type {scala_name} = WitOption[{typ}]");
                }
            }
            TypeDefKind::Result(result) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let ok = result
                    .ok
                    .map(|ok| self.render_type_reference(&ok))
                    .unwrap_or("Unit".to_string());
                let err = result
                    .err
                    .map(|err| self.render_type_reference(&err))
                    .unwrap_or("Unit".to_string());
                uwriteln!(source, "  type {scala_name} = WitResult[{ok}, {err}]");
            }
            TypeDefKind::List(list) => {
                write_doc_comment(&mut source, "  ", &typ.docs);
                let typ = self.render_type_reference(list);
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
                let typ = self.render_type_reference(reftyp);
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
        let scala_resource_name = owner.scala_keywords.escape(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        uwriteln!(class_header, "    @js.native");
        uwrite!(class_header, "    class {}(", scala_resource_name);

        let mut class_source = String::new();
        uwriteln!(class_source, ") extends js.Object {{");

        let mut object_source = String::new();
        uwriteln!(object_source, "    @js.native");
        uwriteln!(
            object_source,
            "    object {} extends js.Object {{",
            scala_resource_name
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
                let args = self.owner.render_args(func.params.iter().skip(1));
                let ret = self.owner.render_return_type(&func.results);
                let scala_func_name = self.get_func_name("[method]", func_name);

                let overrd = if self
                    .owner
                    .scala_keywords
                    .base_methods
                    .contains(&scala_func_name)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                uwriteln!(
                    self.class_source,
                    "      {overrd}def {scala_func_name}({args}): {ret} = js.native"
                );
            }
            FunctionKind::Static(_) => {
                let args = self.owner.render_args(func.params.iter());
                let ret = self.owner.render_return_type(&func.results);

                let scala_func_name = self.get_func_name("[static]", func_name);
                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                uwriteln!(
                    self.object_source,
                    "      def {scala_func_name}({args}): {ret} = js.native"
                );
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.render_args(func.params.iter());
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

    fn get_func_name(&self, prefix: &str, func_name: &str) -> String {
        self.owner.scala_keywords.escape(
            func_name
                .strip_prefix(prefix)
                .unwrap()
                .strip_prefix(&self.resource_name)
                .unwrap()
                .to_lower_camel_case(),
        )
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
        let scala_resource_name = owner.scala_keywords.escape(resource_name.to_pascal_case());

        let mut class_header = String::new();
        write_doc_comment(&mut class_header, "    ", &resource.docs);

        uwrite!(class_header, "  abstract class {}(", scala_resource_name);

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
                let args = self.owner.render_args(func.params.iter().skip(1));
                let ret = self.owner.render_return_type(&func.results);
                let scala_func_name = self.get_func_name("[method]", func_name);

                let overrd = if self
                    .owner
                    .scala_keywords
                    .base_methods
                    .contains(&scala_func_name)
                {
                    "override "
                } else {
                    ""
                };

                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                uwriteln!(
                    self.class_source,
                    "    {overrd}def {scala_func_name}({args}): {ret}"
                );
            }
            FunctionKind::Static(_) => {
                let args = self.owner.render_args(func.params.iter());
                let ret = self.owner.render_return_type(&func.results);

                let scala_func_name = self.get_func_name("[static]", func_name);
                write_doc_comment(&mut self.class_source, "      ", &func.docs);
                uwriteln!(
                    self.static_methods,
                    "  def {scala_func_name}({args}): {ret}"
                );
            }
            FunctionKind::Constructor(_) => {
                let args = self.owner.render_args(func.params.iter());
                self.constructor_args = args;
            }
        }
    }

    pub fn finalize(self) -> String {
        let mut class_source = self.class_header;
        uwrite!(class_source, "{}", self.constructor_args);
        uwriteln!(class_source, "{}", self.class_source);
        uwriteln!(class_source, "  }}");
        format!("{}\n{}\n", class_source, self.static_methods)
    }

    fn get_func_name(&self, prefix: &str, func_name: &str) -> String {
        self.owner.scala_keywords.escape(
            func_name
                .strip_prefix(prefix)
                .unwrap()
                .strip_prefix(&self.resource_name)
                .unwrap()
                .to_lower_camel_case(),
        )
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

/// Tests whether `ty` can be represented with `null`, and if it can then
/// the "other type" is returned. If `Some` is returned that means that `ty`
/// is `null | <return>`. If `None` is returned that means that `null` can't
/// be used to represent `ty`.
pub fn as_nullable<'a>(resolve: &'a Resolve, ty: &'a Type) -> Option<&'a Type> {
    let id = match ty {
        Type::Id(id) => *id,
        _ => return None,
    };
    match &resolve.types[id].kind {
        // If `ty` points to an `option<T>`, then `ty` can be represented
        // with `null` if `t` itself can't be represented with null. For
        // example `option<option<u32>>` can't be represented with `null`
        // since that's ambiguous if it's `none` or `some(none)`.
        //
        // Note, oddly enough, that `option<option<option<u32>>>` can be
        // represented as `null` since:
        //
        // * `null` => `none`
        // * `{ tag: "none" }` => `some(none)`
        // * `{ tag: "some", val: null }` => `some(some(none))`
        // * `{ tag: "some", val: 1 }` => `some(some(some(1)))`
        //
        // It's doubtful anyone would actually rely on that though due to
        // how confusing it is.
        TypeDefKind::Option(t) => {
            if !maybe_null(resolve, t) {
                Some(t)
            } else {
                None
            }
        }
        TypeDefKind::Type(t) => as_nullable(resolve, t),
        _ => None,
    }
}

pub fn maybe_null(resolve: &Resolve, ty: &Type) -> bool {
    as_nullable(resolve, ty).is_some()
}
