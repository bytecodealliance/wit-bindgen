use anyhow::Result;
use heck::*;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use wit_bindgen_core::{Files, Source, WorldGenerator, wit_parser::*};

#[derive(Default)]
struct D {
    world_src: Source,
    opts: Opts,

    cur_world_fqn: String,
    interfaces: HashMap<InterfaceId, InterfaceSource>,

    cur_interface: Option<InterfaceId>,
}

#[derive(Default)]
struct InterfaceSource {
    fqn: String,
    src: Source,
    imported: bool,
    exported: bool,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Where to place output files
    #[cfg_attr(feature = "clap", arg(skip))]
    out_dir: Option<PathBuf>,
}

impl Opts {
    pub fn build(mut self, out_dir: Option<&PathBuf>) -> Box<dyn WorldGenerator> {
        let mut r = D::default();
        self.out_dir = out_dir.cloned();
        r.opts = self.clone();
        Box::new(r)
    }
}

fn escape_d_identifier(name: &str) -> &str {
    match name {
        // Escape D keywords.
        // Source: https://dlang.org/spec/lex.html#keywords
        "abstract" => "abstract_",
        "alias" => "alias_",
        "align" => "align_",
        "asm" => "asm_",
        "assert" => "assert_",
        "auto" => "auto_",

        "body" => "body_",
        "bool" => "bool_",
        "break" => "break_",
        "byte" => "byte_",

        "case" => "case_",
        "cast" => "cast_",
        "catch" => "catch_",
        "cdouble" => "cdouble_",
        "cent" => "cent_",
        "cfloat" => "cfloat_",
        "char" => "char_",
        "class" => "class_",
        "const" => "const_",
        "continue" => "continue_",
        "creal" => "creal_",

        "dchar" => "dchar_",
        "debug" => "debug_",
        "default" => "default_",
        "delegate" => "delegate_",
        "delete" => "delete_",
        "deprecated" => "deprecated_",
        "do" => "do_",
        "double" => "double_",

        "else" => "else_",
        "enum" => "enum_",
        "export" => "export_",
        "extern" => "extern_",

        "false" => "false_",
        "final" => "final_",
        "finally" => "finally_",
        "float" => "float_",
        "for" => "for_",
        "foreach" => "foreach_",
        "foreach_reverse" => "foreach_reverse_",
        "function" => "function_",

        "goto" => "goto_",

        "idouble" => "idouble_",
        "if" => "if_",
        "ifloat" => "ifloat_",
        "immutable" => "immutable_",
        "import" => "import_",
        "in" => "in_",
        "inout" => "inout_",
        "int" => "int_",
        "interface" => "interface_",
        "invariant" => "invariant_",
        "ireal" => "ireal_",
        "is" => "is_",

        "lazy" => "lazy_",
        "long" => "long_",

        "macro" => "macro_",
        "mixin" => "mixin_",
        "module" => "module_",

        "new" => "new_",
        "nothrow" => "nothrow_",
        "null" => "null_",

        "out" => "out_",
        "override" => "override_",

        "package" => "package_",
        "pragma" => "pragma_",
        "private" => "private_",
        "protected" => "protected_",
        "public" => "public_",
        "pure" => "pure_",

        "real" => "real_",
        "ref" => "ref_",
        "return" => "return_",

        "scope" => "scope_",
        "shared" => "shared_",
        "short" => "short_",
        "static" => "static_",
        "struct" => "struct_",
        "super" => "super_",
        "switch" => "switch_",
        "synchronized" => "synchronized_",

        "template" => "template_",
        "this" => "this_",
        "throw" => "throw_",
        "true" => "true_",
        "try" => "try_",
        "typeid" => "typeid_",
        "typeof" => "typeof_",

        "ubyte" => "ubyte_",
        "ucent" => "ucent_",
        "uint" => "uint_",
        "ulong" => "ulong_",
        "union" => "union_",
        "unittest" => "unittest_",
        "ushort" => "ushort_",

        "version" => "version_",
        "void" => "void_",

        "wchar" => "wchar_",
        "while" => "while_",
        "with" => "with_",

        s => s,
    }
}

fn get_package_fqn(id: PackageId, resolve: &Resolve) -> String {
    let mut ns = String::new();

    let pkg = &resolve.packages[id];
    ns.push_str("wit.");
    ns.push_str(escape_d_identifier(&pkg.name.namespace.to_snake_case()));
    ns.push_str(".");
    ns.push_str(escape_d_identifier(&pkg.name.name.to_snake_case()));
    ns.push_str(".");
    let pkg_has_multiple_versions = resolve.packages.iter().any(|(_, p)| {
        p.name.namespace == pkg.name.namespace
            && p.name.name == pkg.name.name
            && p.name.version != pkg.name.version
    });
    if pkg_has_multiple_versions {
        if let Some(version) = &pkg.name.version {
            let version = version
                .to_string()
                .replace('.', "_")
                .replace('-', "_")
                .replace('+', "_");
            ns.push_str(&version);
            ns.push_str(".");
        }
    }
    ns
}

fn get_interface_fqn(
    interface_id: &WorldKey,
    cur_world_fqn: &String,
    resolve: &Resolve,
    is_export: bool,
) -> String {
    let mut ns = String::new();
    match interface_id {
        WorldKey::Name(name) => {
            ns.push_str(cur_world_fqn);
            if is_export {
                ns.push_str(".exports")
            } else {
                ns.push_str(".imports")
            }
            ns.push_str(".");
            ns.push_str(escape_d_identifier(&name.to_snake_case()))
        }
        WorldKey::Interface(id) => {
            let iface = &resolve.interfaces[*id];
            ns.push_str(&get_package_fqn(iface.package.unwrap(), resolve));
            ns.push_str(escape_d_identifier(
                &iface.name.as_ref().unwrap().to_snake_case(),
            ))
        }
    }
    ns
}

fn get_world_fqn(id: WorldId, resolve: &Resolve) -> String {
    let mut ns = String::new();

    let world = &resolve.worlds[id];
    ns.push_str(&get_package_fqn(world.package.unwrap(), resolve));
    ns.push_str(escape_d_identifier(&world.name.to_snake_case()));
    ns
}

impl D {
    fn get_type_fqn(&self, name: &str, owner: &TypeOwner) -> String {
        match owner {
            TypeOwner::None => String::from(name),
            TypeOwner::Interface(id) => {
                format!(
                    "{}.{}",
                    self.interfaces[id].fqn,
                    escape_d_identifier(&name.to_upper_camel_case())
                )
            }
            TypeOwner::World(_) => format!(
                "{}.{}",
                self.cur_world_fqn,
                escape_d_identifier(&name.to_upper_camel_case())
            ),
        }
    }

    fn get_type_name(&self, name: &str, owner: &TypeOwner) -> String {
        match &owner {
            TypeOwner::Interface(id) => Some(id),
            _ => None,
        }
        .zip(self.cur_interface.as_ref())
        .filter(|(id, cur_id)| id == cur_id)
        .map_or_else(
            || self.get_type_fqn(name, owner),
            |_| escape_d_identifier(&name.to_upper_camel_case()).into(),
        )
    }

    fn prepare_interface_bindings(
        &self,
        id: InterfaceId,
        fqn: &String,
        cur_world_fqn: &String,
        resolve: &Resolve,
    ) -> Source {
        let mut src = Source::default();
        let interface = &resolve.interfaces[id];

        src.push_str(&format!(
            "/++\n{}\n+/\n",
            interface.docs.contents.as_deref().unwrap_or_default()
        ));

        src.push_str(&format!("module {};\n\n", fqn));

        src.push_str("import wit.common;\n\n");

        let mut deps = BTreeSet::new();

        for dep_id in resolve.interface_direct_deps(id) {
            deps.insert(dep_id);
        }

        for dep_id in deps {
            let wrapped_dep_id = WorldKey::Interface(dep_id);
            src.push_str(&format!(
                "static import {};\n",
                get_interface_fqn(&wrapped_dep_id, cur_world_fqn, resolve, false)
            ));
        }

        src.push_str("\n// Type defines\n");

        for (name, id) in &interface.types {
            let type_src = self.generate_type_declaration(name, *id, resolve);
            src.append_src(&type_src);
        }

        src
    }

    fn generate_type_use(&self, r#type: &Type, resolve: &Resolve) -> Cow<'static, str> {
        match r#type {
            Type::Bool => Cow::Borrowed("bool"),
            Type::U8 => Cow::Borrowed("ubyte"),
            Type::U16 => Cow::Borrowed("ushort"),
            Type::U32 => Cow::Borrowed("uint"),
            Type::U64 => Cow::Borrowed("ulong"),
            Type::S8 => Cow::Borrowed("byte"),
            Type::S16 => Cow::Borrowed("short"),
            Type::S32 => Cow::Borrowed("int"),
            Type::S64 => Cow::Borrowed("long"),
            Type::F32 => Cow::Borrowed("float"),
            Type::F64 => Cow::Borrowed("double"),
            Type::Char => Cow::Borrowed("dchar"),
            Type::String => Cow::Borrowed("String"),
            Type::ErrorContext => {
                todo!("use of `error_context`!");
            }
            Type::Id(id) => {
                let typedef = &resolve.types[*id];
                match &typedef.owner {
                    TypeOwner::None => match &typedef.kind {
                        TypeDefKind::Handle(handle) => todo!("use of `TypeDefKind::Handle`"),
                        TypeDefKind::Tuple(tuple) => Cow::Owned(format!(
                            "Tuple!({})",
                            tuple
                                .types
                                .iter()
                                .map(|ty| self.generate_type_use(ty, resolve).into_owned())
                                .collect::<Vec<String>>()
                                .join(", ")
                        )),
                        TypeDefKind::Option(opt_type) => Cow::Owned(format!(
                            "Option!({})",
                            self.generate_type_use(opt_type, resolve)
                        )),
                        TypeDefKind::Result(result) => Cow::Owned(format!(
                            "Result!({}, {})",
                            match result.ok {
                                Some(ok_type) => self.generate_type_use(&ok_type, resolve),
                                None => Cow::Borrowed("void"),
                            },
                            match result.err {
                                Some(err_type) => self.generate_type_use(&err_type, resolve),
                                None => Cow::Borrowed("void"),
                            }
                        )),
                        TypeDefKind::List(list_type) => Cow::Owned(format!(
                            "List!({})",
                            self.generate_type_use(list_type, resolve)
                        )),
                        TypeDefKind::Map(_, _) => todo!("use of `TypeDefKind::Map`"),
                        TypeDefKind::FixedLengthList(list_type, length) => Cow::Owned(format!(
                            "{}[{length}]",
                            self.generate_type_use(list_type, resolve)
                        )),
                        TypeDefKind::Future(future_type) => todo!("use of `TypeDefKind::Future`"),
                        TypeDefKind::Stream(stream_type) => todo!("use of `TypeDefKind::Stream`"),
                        TypeDefKind::Type(target_type) => {
                            self.generate_type_use(target_type, resolve)
                        }
                        TypeDefKind::Unknown => {
                            panic!("Trying to emit type use for `TypeDefKind::Unknown`?");
                        }
                        unhandled => {
                            panic!(
                                "Encountered unexpected use of ownerless typedef: {unhandled:?}."
                            );
                        }
                    },
                    _ => Cow::Owned(
                        self.get_type_name(typedef.name.as_ref().unwrap(), &typedef.owner),
                    ),
                }
            }
        }
    }

    fn generate_type_declaration(&self, name: &str, id: TypeId, resolve: &Resolve) -> Source {
        let mut src = Source::default();

        let typedef = &resolve.types[id];

        let upper_name = name.to_upper_camel_case();
        let escaped_name = escape_d_identifier(&upper_name);

        src.push_str(&format!(
            "\n/++\n{}\n+/\n",
            typedef.docs.contents.as_deref().unwrap_or_default()
        ));
        match &typedef.kind {
            TypeDefKind::Record(record) => {
                src.push_str(&format!("struct {escaped_name} {{\n"));

                let mut is_first = true;
                for field in &record.fields {
                    if is_first {
                        is_first = false;
                    } else {
                        src.push_str("\n");
                    }

                    src.push_str(&format!(
                        "/++\n{}\n+/\n",
                        field.docs.contents.as_deref().unwrap_or_default()
                    ));
                    src.push_str(&format!(
                        "{} {};\n",
                        self.generate_type_use(&field.ty, resolve),
                        field.name.to_lower_camel_case()
                    ));
                }

                src.push_str("}\n");
            }
            TypeDefKind::Resource => {
                //src.push_str(&format!("// TODO: def of resource - {name}"))
                todo!("def of `TypeDefKind::Resource`");
            }
            TypeDefKind::Handle(handle) => {
                todo!("def of `TypeDefKind::Handle`");
            }
            TypeDefKind::Flags(flags) => {
                let storage_type = match flags.repr() {
                    FlagsRepr::U8 => "ubyte",
                    FlagsRepr::U16 => "ushort",
                    FlagsRepr::U32(1) => "uint",
                    FlagsRepr::U32(2) => "ulong",
                    repr => todo!("flags {repr:?}"),
                };

                src.push_str(&format!("enum {escaped_name}_ : {storage_type} {{\n"));
                for (index, flag) in flags.flags.iter().enumerate() {
                    if index != 0 {
                        src.push_str("\n");
                    }
                    src.push_str(&format!(
                        "/++\n{}\n+/\n",
                        flag.docs.contents.as_deref().unwrap_or_default()
                    ));
                    src.push_str(&format!(
                        "{} = 1 << {index},\n",
                        escape_d_identifier(&flag.name.to_lower_camel_case())
                    ));
                }
                src.push_str(&format!(
                    "}}\n/// ditto\nalias {escaped_name} = Flags!{escaped_name}_;"
                ));
            }
            TypeDefKind::Tuple(tuple) => src.push_str(&format!(
                "alias {escaped_name} = Tuple!({});",
                tuple
                    .types
                    .iter()
                    .map(|ty| self.generate_type_use(ty, resolve).into_owned())
                    .collect::<Vec<String>>()
                    .join(", ")
            )),
            TypeDefKind::Variant(variant) => {
                let storage_type = match variant.tag() {
                    Int::U8 => "ubyte",
                    Int::U16 => "ushort",
                    Int::U32 => "uint",
                    Int::U64 => "ulong",
                };

                src.push_str(&format!("struct {escaped_name} {{\n"));
                src.deindent(1);
                src.push_str(&format!("@safe @nogc nothrow:\n"));
                src.indent(1);

                src.push_str(&format!("enum Tag : {storage_type} {{\n"));

                let mut is_first = true;
                for case in &variant.cases {
                    if is_first {
                        is_first = false;
                    } else {
                        src.push_str("\n");
                    }
                    src.push_str(&format!(
                        "/++\n{}\n+/\n",
                        case.docs.contents.as_deref().unwrap_or_default()
                    ));
                    src.push_str(&format!(
                        "{},\n",
                        escape_d_identifier(&case.name.to_lower_camel_case())
                    ));
                }

                src.push_str("}\n");

                src.deindent(1);
                src.push_str(&format!("\nprivate:\n"));
                src.indent(1);

                if variant.cases.iter().any(|case| case.ty.is_some()) {
                    src.push_str(&format!("union Storage {{\n"));
                    src.push_str("ubyte __zeroinit = 0;\n");
                    for case in &variant.cases {
                        if let Some(ty) = &case.ty {
                            src.push_str(&format!(
                                "{} {};\n",
                                self.generate_type_use(ty, resolve),
                                escape_d_identifier(&case.name.to_lower_camel_case())
                            ));
                        }
                    }

                    src.push_str("}\n\n");

                    src.push_str("Tag _tag;\n");
                    src.push_str("Storage _storage;\n\n");
                } else {
                    src.push_str("Tag _tag;\n\n");
                }

                src.push_str("@disable this();\n");
                src.push_str("this(Tag tag, Storage storage = Storage.init) {\n");
                src.push_str("_tag = tag;\n");
                src.push_str("_storage = storage;\n");
                src.push_str("}\n");

                src.deindent(1);
                src.push_str(&format!("\npublic:\n"));
                src.indent(1);

                src.push_str("Tag tag() => _tag;\n");

                for case in &variant.cases {
                    src.push_str(&format!(
                        "\n/++\n{}\n+/\n",
                        case.docs.contents.as_deref().unwrap_or_default()
                    ));
                    let upper_case_name = case.name.to_upper_camel_case();
                    let escaped_upper_case_name = escape_d_identifier(&upper_case_name);

                    let lower_case_name = case.name.to_lower_camel_case();
                    let escaped_lower_case_name = escape_d_identifier(&lower_case_name);

                    if let Some(ty) = &case.ty {
                        src.push_str(&format!(
                            "static {escaped_name} {escaped_lower_case_name}({} val) {{\n",
                            self.generate_type_use(ty, resolve)
                        ));
                        src.push_str("Storage storage;\n");
                        src.push_str(&format!("storage.{escaped_lower_case_name} = val;\n"));
                        src.push_str(&format!(
                            "return {escaped_name}(Tag.{escaped_lower_case_name}, storage);\n"
                        ));
                        src.push_str("}\n");

                        src.push_str(&format!(
                            "/// ditto\n ref inout({}) get{escaped_upper_case_name}() inout return\n",
                            self.generate_type_use(ty, resolve)
                        ));

                        src.push_str(&format!("in (is{escaped_upper_case_name}) "));
                        src.push_str(&format!(
                            "do {{ return _storage.{escaped_lower_case_name}; }}\n"
                        ));
                    } else {
                        src.push_str(&format!(
                            "static {escaped_name} {escaped_lower_case_name}() => {escaped_name}(Tag.{escaped_lower_case_name});\n",
                        ));
                    }
                    src.push_str(&format!(
                        "/// ditto\nbool is{escaped_upper_case_name}() const => _tag == Tag.{escaped_lower_case_name};\n",
                    ));
                }

                src.push_str("}\n");
            }
            TypeDefKind::Enum(r#enum) => {
                let storage_type = match r#enum.tag() {
                    Int::U8 => "ubyte",
                    Int::U16 => "ushort",
                    Int::U32 => "uint",
                    Int::U64 => "ulong",
                };

                src.push_str(&format!("enum {escaped_name} : {storage_type} {{\n"));

                let mut is_first = true;
                for case in &r#enum.cases {
                    if is_first {
                        is_first = false;
                    } else {
                        src.push_str("\n");
                    }
                    src.push_str(&format!(
                        "/++\n{}\n+/\n",
                        case.docs.contents.as_deref().unwrap_or_default()
                    ));
                    src.push_str(&format!(
                        "{},\n",
                        escape_d_identifier(&case.name.to_lower_camel_case())
                    ));
                }

                src.push_str(&format!("}}"));
            }
            TypeDefKind::Option(opt_type) => src.push_str(&format!(
                "alias {escaped_name} = Option!({});",
                self.generate_type_use(opt_type, resolve)
            )),
            TypeDefKind::Result(result) => src.push_str(&format!(
                "alias {escaped_name} = Result!({}, {});",
                match result.ok {
                    Some(ok_type) => self.generate_type_use(&ok_type, resolve),
                    None => Cow::Borrowed("void"),
                },
                match result.err {
                    Some(err_type) => self.generate_type_use(&err_type, resolve),
                    None => Cow::Borrowed("void"),
                }
            )),
            TypeDefKind::List(list_type) => src.push_str(&format!(
                "alias {escaped_name} = List!({});",
                self.generate_type_use(list_type, resolve)
            )),
            TypeDefKind::Map(_, _) => {
                todo!("def of `TypeDefKind::Map`");
            }
            TypeDefKind::FixedLengthList(list_type, length) => {
                src.push_str(&format!(
                    "alias {escaped_name} = {}[{length}];",
                    self.generate_type_use(&list_type, resolve),
                ));
            }
            TypeDefKind::Future(future_type) => {
                todo!("def of `TypeDefKind::Future`");
            }
            TypeDefKind::Stream(stream_type) => {
                todo!("def of `TypeDefKind::Stream`");
            }
            TypeDefKind::Type(target_type) => {
                src.push_str(&format!(
                    "alias {escaped_name} = {};",
                    self.generate_type_use(&target_type, resolve),
                ));
            }
            TypeDefKind::Unknown => {
                panic!("Trying to emit type declaration for `TypeDefKind::Unknown`?");
            }
        }
        src.push_str("\n");
        src
    }
}

impl WorldGenerator for D {
    fn uses_nominal_type_ids(&self) -> bool {
        false
    }

    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        self.cur_world_fqn = get_world_fqn(world, resolve);

        let world = &resolve.worlds[world];

        self.world_src.push_str(&format!(
            "/++\n{}\n+/\n",
            world.docs.contents.as_deref().unwrap_or_default()
        ));

        self.world_src
            .push_str(&format!("module {};\n\n", self.cur_world_fqn));

        self.world_src.push_str("import wit.common;\n\n");

        self.world_src.push_str("// Interface imports\n");
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        self.cur_interface = Some(id);
        let interface_src = match self.interfaces.get_mut(&id) {
            Some(src) => src,
            None => {
                eprintln!("Import {id:?}");
                let new_fqn = get_interface_fqn(name, &self.cur_world_fqn, resolve, false);

                let mut result_init = InterfaceSource::default();
                result_init.fqn = new_fqn;
                self.interfaces.insert(id, result_init);

                let new_src = self.prepare_interface_bindings(
                    id,
                    &self.interfaces.get(&id).unwrap().fqn,
                    &self.cur_world_fqn,
                    resolve,
                );

                let result = self.interfaces.get_mut(&id).unwrap();
                result.src = new_src;
                result
            }
        };

        if interface_src.imported {
            return Ok(());
        }
        interface_src.imported = true;

        self.world_src
            .push_str(&format!("public import {};\n", &self.interfaces[&id].fqn));

        self.cur_interface = None;
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        self.world_src.push_str(&format!("\n// Type imports\n"));
        for (name, id) in types {
            let type_src = self.generate_type_declaration(name, *id, resolve);
            self.world_src.append_src(&type_src);
        }
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        self.world_src.push_str(&format!("\n// Function imports\n"));
        for (name, func) in funcs {
            self.world_src
                .push_str(&format!("// Import function: {name}\n"));
        }
    }

    fn pre_export_interface(&mut self, resolve: &Resolve, files: &mut Files) -> Result<()> {
        self.world_src.push_str("\n// Interface exports\n");
        self.world_src
            .push_str("mixin template Exports(alias Impl) {\n");

        Ok(())
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        self.cur_interface = Some(id);
        let interface = &resolve.interfaces[id];
        let interface_src = match self.interfaces.get_mut(&id) {
            Some(src) => src,
            None => {
                eprintln!("Export {id:?}");
                let new_fqn = get_interface_fqn(name, &self.cur_world_fqn, resolve, true);

                let mut result_init = InterfaceSource::default();
                result_init.fqn = new_fqn;
                self.interfaces.insert(id, result_init);

                let new_src = self.prepare_interface_bindings(
                    id,
                    &self.interfaces.get(&id).unwrap().fqn,
                    &self.cur_world_fqn,
                    resolve,
                );

                let result = self.interfaces.get_mut(&id).unwrap();
                result.src = new_src;
                result
            }
        };

        if interface_src.exported {
            return Ok(());
        }
        interface_src.exported = true;

        self.world_src.push_str(&format!(
            "mixin imported!\"{}\".Exports!Impl;\n",
            interface_src.fqn
        ));

        interface_src
            .src
            .push_str("\nmixin template Exports(alias Impl) {\n");

        interface_src
            .src
            .push_str(&format!("// Function exports\n"));
        for (name, func) in &interface.functions {
            interface_src
                .src
                .push_str(&format!("// Export function: {name}\n"));
        }

        interface_src.src.push_str("}\n");

        self.cur_interface = None;
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        self.world_src.push_str(&format!("\n// Function exports\n"));
        for (name, func) in funcs {
            self.world_src
                .push_str(&format!("// Export function: {name}\n"));
        }
        Ok(())
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        // Close out interface exports
        self.world_src.push_str("}\n");

        let mut world_filepath = PathBuf::from_iter(get_world_fqn(id, resolve).split("."));
        world_filepath.push("package.d");

        files.push(
            world_filepath.to_str().unwrap(),
            self.world_src.as_str().as_bytes(),
        );

        files.push("wit/common.d", include_bytes!("wit_common.d"));

        for (_, interface_src) in &self.interfaces {
            let mut interface_filepath = PathBuf::from_iter(interface_src.fqn.split("."));
            interface_filepath.add_extension("d");

            files.push(
                interface_filepath.to_str().unwrap(),
                interface_src.src.as_bytes(),
            );
        }
        Ok(())
    }
}
