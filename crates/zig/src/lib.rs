use anyhow::{Result, bail};
use indexmap::{IndexMap, IndexSet};
use std::fmt::Write as _;
use wit_bindgen_core::{Files, Source, WorldGenerator, wit_parser::*};

const ABI_ZIG: &str = include_str!("abi.zig");

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct Opts {
    /// Base name for the generated root `.zig` file and subdirectory.
    #[cfg_attr(feature = "clap", arg(long, default_value = "wit"))]
    pub name: String,

    /// Import path for `abi.zig` relative to `<name>/<interface>.zig`.
    ///
    /// When omitted, `abi.zig` is emitted into `<name>/abi.zig` and imported as `"abi.zig"`.
    #[cfg_attr(feature = "clap", arg(long, value_name = "PATH"))]
    pub abi_path: Option<String>,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        Box::new(Zig { opts: self })
    }

    fn effective_name(&self) -> &str {
        if self.name.is_empty() {
            "wit"
        } else {
            &self.name
        }
    }
}

pub struct Zig {
    opts: Opts,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Style {
    Snake,
    Camel,
    Title,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Core {
    I32,
    I64,
    F32,
    F64,
}

impl Core {
    fn as_str(self) -> &'static str {
        match self {
            Core::I32 => "i32",
            Core::I64 => "i64",
            Core::F32 => "f32",
            Core::F64 => "f64",
        }
    }
}

struct Signature {
    params: Vec<Core>,
    result: Option<Core>,
    indirect: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Side {
    Import,
    Export,
}

fn words(name: &str, style: Style) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = style == Style::Title;
    for c in name.chars() {
        if c == '-' || c == '.' || c == '_' {
            if style == Style::Snake {
                out.push('_');
            } else {
                upper = true;
            }
            continue;
        }
        out.push(if upper { c.to_ascii_uppercase() } else { c });
        upper = false;
    }
    out
}

fn is_zig_keyword(name: &str) -> bool {
    matches!(
        name,
        "addrspace"
            | "align"
            | "allowzero"
            | "and"
            | "anyframe"
            | "anytype"
            | "asm"
            | "async"
            | "await"
            | "break"
            | "callconv"
            | "catch"
            | "comptime"
            | "const"
            | "continue"
            | "defer"
            | "else"
            | "enum"
            | "errdefer"
            | "error"
            | "export"
            | "extern"
            | "fn"
            | "for"
            | "if"
            | "inline"
            | "linksection"
            | "noalias"
            | "noinline"
            | "nosuspend"
            | "opaque"
            | "or"
            | "orelse"
            | "packed"
            | "pub"
            | "resume"
            | "return"
            | "struct"
            | "suspend"
            | "switch"
            | "test"
            | "threadlocal"
            | "try"
            | "tuple"
            | "union"
            | "unreachable"
            | "usingnamespace"
            | "var"
            | "volatile"
            | "while"
    )
}

fn is_zig_primitive(name: &str) -> bool {
    if matches!(
        name,
        "anyerror"
            | "anyframe"
            | "anyopaque"
            | "bool"
            | "c_char"
            | "c_int"
            | "c_long"
            | "c_longdouble"
            | "c_longlong"
            | "c_short"
            | "c_uint"
            | "c_ulong"
            | "c_ulonglong"
            | "c_ushort"
            | "c_void"
            | "comptime_float"
            | "comptime_int"
            | "f128"
            | "f16"
            | "f32"
            | "f64"
            | "f80"
            | "false"
            | "isize"
            | "noreturn"
            | "null"
            | "true"
            | "type"
            | "undefined"
            | "usize"
            | "void"
    ) {
        return true;
    }
    let bytes = name.as_bytes();
    if bytes.len() > 1 && (bytes[0] == b'i' || bytes[0] == b'u') {
        if bytes[1..].iter().all(|b| b.is_ascii_digit()) {
            return true;
        }
    }
    false
}

fn is_valid_zig_id(name: &str, allow_primitive: bool) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || name == "_" {
        return false;
    }
    for (i, &c) in bytes.iter().enumerate() {
        match c {
            b'_' | b'a'..=b'z' | b'A'..=b'Z' => {}
            b'0'..=b'9' if i > 0 => {}
            _ => return false,
        }
    }
    !is_zig_keyword(name) && (allow_primitive || !is_zig_primitive(name))
}

fn fmt_id_inner(name: &str, allow_primitive: bool) -> String {
    if is_valid_zig_id(name, allow_primitive) {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len() + 3);
    out.push_str("@\"");
    for c in name.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn fmt_id(name: &str) -> String {
    fmt_id_inner(name, false)
}

fn fmt_field_id(name: &str) -> String {
    fmt_id_inner(name, true)
}

fn join_core(a: Core, b: Core) -> Core {
    if a == b {
        return a;
    }
    if (a == Core::I32 && b == Core::F32) || (a == Core::F32 && b == Core::I32) {
        return Core::I32;
    }
    Core::I64
}

struct Gen<'a> {
    resolve: &'a Resolve,
    world_id: WorldId,
    abi_path: String,
    imports: IndexSet<InterfaceId>,
    interfaces: Vec<InterfaceId>,
    interface_names: IndexMap<InterfaceId, String>,
    module_names: IndexMap<InterfaceId, String>,

    current: Option<InterfaceId>,
    used: IndexSet<InterfaceId>,
    imported: IndexSet<InterfaceId>,
    has_root_module: bool,
    used_root: bool,
}

impl<'a> Gen<'a> {
    fn new(resolve: &'a Resolve, world_id: WorldId, abi_path: String) -> Result<Self> {
        let world = &resolve.worlds[world_id];
        let mut imports = IndexSet::new();
        let mut inline_names = IndexMap::new();

        for (key, item) in &world.imports {
            if let WorldItem::Interface { id, .. } = item {
                imports.insert(*id);
                if let WorldKey::Name(name) = key {
                    inline_names.insert(*id, name.clone());
                }
            }
        }
        for (key, item) in &world.exports {
            if let WorldItem::Interface { id, .. } = item {
                if let WorldKey::Name(name) = key {
                    inline_names.insert(*id, name.clone());
                }
            }
        }

        let mut interfaces = Vec::new();
        let mut interface_names = IndexMap::new();
        for (id, iface) in resolve.interfaces.iter() {
            if let Some(name) = &iface.name {
                interfaces.push(id);
                interface_names.insert(id, name.clone());
            } else if let Some(name) = inline_names.get(&id) {
                interfaces.push(id);
                interface_names.insert(id, name.clone());
            }
        }

        // Compute snake_case module names, disambiguating only if there is a collision.
        let mut counts: IndexMap<String, usize> = IndexMap::new();
        for &id in &interfaces {
            let base = words(&interface_names[&id], Style::Snake);
            *counts.entry(base).or_insert(0) += 1;
        }
        let mut module_names = IndexMap::new();
        let mut used_mod_names = IndexSet::new();
        for &id in &interfaces {
            let base = words(&interface_names[&id], Style::Snake);
            let candidate = if counts[&base] > 1 || base == "exports" || base == "abi" {
                if let Some(pkg_id) = resolve.interfaces[id].package {
                    let pkg = &resolve.packages[pkg_id].name;
                    format!(
                        "{}_{}_{}",
                        words(&pkg.namespace, Style::Snake),
                        words(&pkg.name, Style::Snake),
                        base
                    )
                } else {
                    format!("{base}_")
                }
            } else {
                base
            };
            let mut unique = candidate;
            while !used_mod_names.insert(unique.clone()) {
                unique.push('_');
            }
            module_names.insert(id, unique);
        }

        let has_root_module = world.imports.values().any(|item| match item {
            WorldItem::Function(_) => true,
            WorldItem::Type { id, .. } => !Self::is_interface_alias(resolve, *id),
            WorldItem::Interface { .. } => false,
        });

        let g = Self {
            resolve,
            world_id,
            abi_path,
            imports,
            interfaces,
            interface_names,
            module_names,
            current: None,
            used: IndexSet::new(),
            imported: IndexSet::new(),
            has_root_module,
            used_root: false,
        };
        g.validate()?;
        Ok(g)
    }

    fn is_interface_alias(resolve: &Resolve, id: TypeId) -> bool {
        if let TypeDefKind::Type(Type::Id(target)) = resolve.types[id].kind {
            matches!(resolve.types[target].owner, TypeOwner::Interface(_))
        } else {
            false
        }
    }

    fn is_borrow(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => matches!(
                self.resolve.types[*id].kind,
                TypeDefKind::Handle(Handle::Borrow(_))
            ),
            _ => false,
        }
    }

    fn is_own(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => matches!(
                self.resolve.types[*id].kind,
                TypeDefKind::Handle(Handle::Own(_))
            ),
            _ => false,
        }
    }

    fn children(&self, kind: &TypeDefKind) -> Vec<Type> {
        let mut out = Vec::new();
        match kind {
            TypeDefKind::Record(r) => {
                for f in &r.fields {
                    out.push(f.ty);
                }
            }
            TypeDefKind::Variant(v) => {
                for c in &v.cases {
                    if let Some(t) = c.ty {
                        out.push(t);
                    }
                }
            }
            TypeDefKind::Tuple(t) => {
                out.extend_from_slice(&t.types);
            }
            TypeDefKind::List(t)
            | TypeDefKind::FixedLengthList(t, _)
            | TypeDefKind::Option(t)
            | TypeDefKind::Type(t) => {
                out.push(*t);
            }
            TypeDefKind::Result(r) => {
                if let Some(t) = r.ok {
                    out.push(t);
                }
                if let Some(t) = r.err {
                    out.push(t);
                }
            }
            TypeDefKind::Map(k, v) => {
                out.push(*k);
                out.push(*v);
            }
            TypeDefKind::Enum(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Handle(_)
            | TypeDefKind::Resource
            | TypeDefKind::Future(_)
            | TypeDefKind::Stream(_)
            | TypeDefKind::Unknown => {}
        }
        out
    }

    fn is_exported(&self, iface: InterfaceId) -> bool {
        self.resolve.worlds[self.world_id]
            .exports
            .values()
            .any(|item| matches!(item, WorldItem::Interface { id, .. } if *id == iface))
    }

    fn validate(&self) -> Result<()> {
        for (_id, ty_def) in self.resolve.types.iter() {
            let label = ty_def.name.as_deref().unwrap_or("(anonymous)");
            match &ty_def.kind {
                TypeDefKind::Flags(f) if f.flags.len() > 32 => {
                    bail!("flags '{label}' has more than 32 members");
                }
                TypeDefKind::Resource => match ty_def.owner {
                    TypeOwner::Interface(owner) => {
                        if self.is_exported(owner) {
                            bail!(
                                "resource '{label}' is exported by the guest, which is not supported"
                            );
                        }
                    }
                    _ => bail!("resource '{label}' has no interface"),
                },
                TypeDefKind::Future(_) | TypeDefKind::Stream(_) | TypeDefKind::Map(_, _) => {
                    bail!("unsupported type kind in '{label}'");
                }
                _ => {}
            }
            if !matches!(ty_def.kind, TypeDefKind::List(_)) {
                for child in self.children(&ty_def.kind) {
                    if self.is_borrow(&child) {
                        bail!(
                            "'{label}' contains a borrowed handle, only parameters and lists may"
                        );
                    }
                }
            }
        }

        for &iface in &self.interfaces {
            for (name, func) in &self.resolve.interfaces[iface].functions {
                self.validate_func(name, func)?;
            }
        }

        let world = &self.resolve.worlds[self.world_id];
        for (key, item) in &world.imports {
            if let WorldItem::Function(func) = item {
                let name = self.resolve.name_world_key(key);
                self.validate_func(&name, func)?;
            }
        }
        for (key, item) in &world.exports {
            if let WorldItem::Function(func) = item {
                let name = self.resolve.name_world_key(key);
                self.validate_func(&name, func)?;
                for p in &func.params {
                    if self.is_borrow(&p.ty) {
                        bail!("export '{name}' borrows a handle, which is not supported");
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_func(&self, name: &str, func: &Function) -> Result<()> {
        match func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Method(_)
            | FunctionKind::Static(_)
            | FunctionKind::Constructor(_) => {}
            _ => bail!("function '{name}' is an async or unsupported function kind"),
        }
        if let Some(result) = &func.result {
            if self.is_borrow(result) {
                bail!("function '{name}' returns a borrowed handle");
            }
        }
        Ok(())
    }

    fn module_name(&self, iface: InterfaceId) -> &str {
        &self.module_names[&iface]
    }

    fn import_path(&self, iface: InterfaceId) -> String {
        if let Some(id) = self.resolve.id_of(iface) {
            id
        } else {
            self.interface_names[&iface].clone()
        }
    }

    fn func_name(&self, func: &Function) -> String {
        if func.name.starts_with("[constructor]") {
            return "init".to_string();
        }
        let base = match func.name.find('.') {
            Some(dot) => &func.name[dot + 1..],
            None => &func.name,
        };
        words(base, Style::Camel)
    }

    fn primitive_str(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Bool => Some("bool"),
            Type::U8 => Some("u8"),
            Type::U16 => Some("u16"),
            Type::U32 => Some("u32"),
            Type::U64 => Some("u64"),
            Type::S8 => Some("i8"),
            Type::S16 => Some("i16"),
            Type::S32 => Some("i32"),
            Type::S64 => Some("i64"),
            Type::F32 => Some("f32"),
            Type::F64 => Some("f64"),
            Type::Char => Some("u21"),
            Type::String => Some("[]const u8"),
            Type::ErrorContext => None,
            Type::Id(_) => None,
        }
    }

    fn write_ref(&mut self, w: &mut String, ty: Option<&Type>) {
        let Some(ty) = ty else {
            w.push_str("void");
            return;
        };
        if let Some(prim) = Self::primitive_str(ty) {
            w.push_str(prim);
            return;
        }
        if let Type::Id(id) = ty {
            self.write_type_id(w, *id);
        }
    }

    fn write_type_id(&mut self, w: &mut String, id: TypeId) {
        let ty_def = &self.resolve.types[id];
        if let Some(name) = &ty_def.name {
            match ty_def.owner {
                TypeOwner::Interface(owner) => {
                    if self.current != Some(owner) {
                        self.used.insert(owner);
                        let _ = write!(w, "{}.", fmt_id(self.module_name(owner)));
                    }
                    w.push_str(&words(name, Style::Title));
                    return;
                }
                TypeOwner::World(_) => {
                    if let TypeDefKind::Type(target) = &ty_def.kind {
                        if Self::is_interface_alias(self.resolve, id) {
                            self.write_ref(w, Some(target));
                            return;
                        }
                    }
                    if self.current.is_some() || !self.has_root_module {
                        self.used_root = true;
                        let _ = write!(w, "root.");
                    }
                    w.push_str(&words(name, Style::Title));
                    return;
                }
                TypeOwner::None => {}
            }
        }
        if let TypeDefKind::Handle(handle) = &ty_def.kind {
            match handle {
                Handle::Own(resource) => {
                    self.write_type_id(w, *resource);
                    return;
                }
                Handle::Borrow(resource) => {
                    w.push_str("abi.Borrowed(");
                    self.write_type_id(w, *resource);
                    w.push(')');
                    return;
                }
            }
        }
        self.write_shape(w, id);
    }

    fn write_shape(&mut self, w: &mut String, id: TypeId) {
        let kind = &self.resolve.types[id].kind;
        match kind {
            TypeDefKind::Type(t) => self.write_ref(w, Some(t)),
            TypeDefKind::List(t) => {
                w.push_str("[]const ");
                self.write_ref(w, Some(t));
            }
            TypeDefKind::FixedLengthList(t, size) => {
                let _ = write!(w, "[{size}]");
                self.write_ref(w, Some(t));
            }
            TypeDefKind::Option(t) => {
                w.push('?');
                self.write_ref(w, Some(t));
            }
            TypeDefKind::Result(r) => {
                w.push_str("abi.Result(");
                self.write_ref(w, r.ok.as_ref());
                w.push_str(", ");
                self.write_ref(w, r.err.as_ref());
                w.push(')');
            }
            TypeDefKind::Tuple(t) => {
                w.push_str("struct { ");
                for (i, item) in t.types.iter().enumerate() {
                    if i != 0 {
                        w.push_str(", ");
                    }
                    self.write_ref(w, Some(item));
                }
                w.push_str(" }");
            }
            _ => unreachable!("unexpected anonymous type: {:?}", kind),
        }
    }

    fn flatten(&self, ty: Option<&Type>, out: &mut Vec<Core>) {
        let Some(ty) = ty else {
            return;
        };
        match ty {
            Type::String => out.extend_from_slice(&[Core::I32, Core::I32]),
            Type::U64 | Type::S64 => out.push(Core::I64),
            Type::F32 => out.push(Core::F32),
            Type::F64 => out.push(Core::F64),
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::Char
            | Type::ErrorContext => out.push(Core::I32),
            Type::Id(id) => {
                let kind = &self.resolve.types[*id].kind;
                match kind {
                    TypeDefKind::Type(t) => self.flatten(Some(t), out),
                    TypeDefKind::Record(_) | TypeDefKind::Tuple(_) => {
                        for child in self.children(kind) {
                            self.flatten(Some(&child), out);
                        }
                    }
                    TypeDefKind::FixedLengthList(t, size) => {
                        for _ in 0..*size {
                            self.flatten(Some(t), out);
                        }
                    }
                    TypeDefKind::List(_) => out.extend_from_slice(&[Core::I32, Core::I32]),
                    TypeDefKind::Variant(v) => {
                        let payloads: Vec<Option<Type>> = v.cases.iter().map(|c| c.ty).collect();
                        self.flatten_cases(&payloads, out);
                    }
                    TypeDefKind::Option(t) => {
                        self.flatten_cases(&[None, Some(*t)], out);
                    }
                    TypeDefKind::Result(r) => {
                        self.flatten_cases(&[r.ok, r.err], out);
                    }
                    TypeDefKind::Enum(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Handle(_)
                    | TypeDefKind::Resource => out.push(Core::I32),
                    TypeDefKind::Future(_)
                    | TypeDefKind::Stream(_)
                    | TypeDefKind::Map(_, _)
                    | TypeDefKind::Unknown => unreachable!(),
                }
            }
        }
    }

    fn flatten_cases(&self, payloads: &[Option<Type>], out: &mut Vec<Core>) {
        let mut joined = Vec::new();
        for payload in payloads {
            let mut flat = Vec::new();
            self.flatten(payload.as_ref(), &mut flat);
            for (i, c) in flat.into_iter().enumerate() {
                if i < joined.len() {
                    joined[i] = join_core(joined[i], c);
                } else {
                    joined.push(c);
                }
            }
        }
        out.push(Core::I32);
        out.extend_from_slice(&joined);
    }

    fn signature(&self, func: &Function, side: Side) -> Signature {
        let mut params = Vec::new();
        for p in &func.params {
            self.flatten(Some(&p.ty), &mut params);
        }
        if params.len() > 16 {
            params.clear();
            params.push(Core::I32);
        }

        let mut results = Vec::new();
        self.flatten(func.result.as_ref(), &mut results);
        if results.len() <= 1 {
            let result = results.first().copied();
            return Signature {
                params,
                result,
                indirect: false,
            };
        }
        if side == Side::Import {
            params.push(Core::I32);
            return Signature {
                params,
                result: None,
                indirect: true,
            };
        }
        Signature {
            params,
            result: Some(Core::I32),
            indirect: true,
        }
    }

    fn write_docs(w: &mut String, docs: &Docs, indent: &str) {
        let Some(contents) = &docs.contents else {
            return;
        };
        for line in contents.split('\n') {
            let trimmed = line.trim_end_matches([' ', '\t', '\r']);
            if trimmed.is_empty() {
                let _ = writeln!(w, "{indent}///");
                continue;
            }
            let _ = write!(w, "{indent}/// ");
            for c in trimmed.chars() {
                if c == '\t' {
                    w.push_str("    ");
                } else {
                    w.push(c);
                }
            }
            w.push('\n');
        }
    }

    fn field_name(name: &str) -> String {
        fmt_field_id(&words(name, Style::Snake))
    }

    fn is_plain_alias(&self, id: TypeId) -> bool {
        let ty_def = &self.resolve.types[id];
        matches!(ty_def.kind, TypeDefKind::Type(Type::Id(_))) && ty_def.docs.contents.is_none()
    }

    fn method_owner(func: &Function) -> Option<TypeId> {
        match func.kind {
            FunctionKind::Method(id) | FunctionKind::Static(id) | FunctionKind::Constructor(id) => {
                Some(id)
            }
            _ => None,
        }
    }

    fn write_type(&mut self, w: &mut String, id: TypeId, funcs: &IndexMap<String, Function>) {
        let ty_def = &self.resolve.types[id];
        let wit_name = ty_def.name.as_deref().unwrap();
        let name = words(wit_name, Style::Title);

        Self::write_docs(w, &ty_def.docs, "");
        let _ = write!(w, "pub const {name} = ");

        match &ty_def.kind {
            TypeDefKind::Record(r) => {
                w.push_str("struct {\n");
                for f in &r.fields {
                    Self::write_docs(w, &f.docs, "    ");
                    let _ = write!(w, "    {}: ", Self::field_name(&f.name));
                    self.write_ref(w, Some(&f.ty));
                    w.push_str(",\n");
                }
                w.push_str("};\n\n");
            }
            TypeDefKind::Variant(v) => {
                w.push_str("union(enum) {\n");
                for c in &v.cases {
                    Self::write_docs(w, &c.docs, "    ");
                    let _ = write!(w, "    {}", Self::field_name(&c.name));
                    if let Some(t) = &c.ty {
                        w.push_str(": ");
                        self.write_ref(w, Some(t));
                    }
                    w.push_str(",\n");
                }
                w.push_str("};\n\n");
            }
            TypeDefKind::Enum(e) => {
                let tag = if e.cases.len() <= (1 << 8) {
                    "u8"
                } else if e.cases.len() <= (1 << 16) {
                    "u16"
                } else {
                    "u32"
                };
                let _ = writeln!(w, "enum({tag}) {{");
                for c in &e.cases {
                    Self::write_docs(w, &c.docs, "    ");
                    let _ = writeln!(w, "    {},", Self::field_name(&c.name));
                }
                w.push_str("};\n\n");
            }
            TypeDefKind::Flags(f) => {
                let bits = if f.flags.len() <= 8 {
                    8
                } else if f.flags.len() <= 16 {
                    16
                } else {
                    32
                };
                let _ = writeln!(w, "packed struct(u{bits}) {{");
                for flag in &f.flags {
                    Self::write_docs(w, &flag.docs, "    ");
                    let _ = writeln!(w, "    {}: bool = false,", Self::field_name(&flag.name));
                }
                if f.flags.len() != bits {
                    let _ = writeln!(w, "    _: u{} = 0,", bits - f.flags.len());
                }
                w.push_str("};\n\n");
            }
            TypeDefKind::Resource => {
                let drop_id = fmt_id(&format!("[resource-drop]{wit_name}"));
                let _ = write!(
                    w,
                    "enum(u32) {{\n    _,\n\n    pub const resource_drop = &raw.{drop_id};\n\n"
                );
                self.write_methods(w, id, &name, funcs);
                let _ = write!(
                    w,
                    "    /// Keeps the handle alive past the current callback. Release it with `deinit`.\n    pub fn keep(self: {name}) void {{\n        abi.keep({name}, self);\n    }}\n\n    /// Releases the handle now instead of when the callback returns.\n    pub fn deinit(self: {name}) void {{\n        abi.drop({name}, self);\n    }}\n}};\n\n"
                );
            }
            TypeDefKind::Type(_)
            | TypeDefKind::List(_)
            | TypeDefKind::FixedLengthList(_, _)
            | TypeDefKind::Option(_)
            | TypeDefKind::Result(_)
            | TypeDefKind::Tuple(_) => {
                self.write_shape(w, id);
                if self.is_plain_alias(id) {
                    w.push_str(";\n");
                } else {
                    w.push_str(";\n\n");
                }
            }
            TypeDefKind::Handle(_)
            | TypeDefKind::Future(_)
            | TypeDefKind::Stream(_)
            | TypeDefKind::Map(_, _)
            | TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn write_methods(
        &mut self,
        w: &mut String,
        resource: TypeId,
        resource_name: &str,
        funcs: &IndexMap<String, Function>,
    ) {
        let mut scope = IndexSet::new();
        for reserved in ["resource_drop", "keep", "deinit"] {
            scope.insert(reserved.to_string());
        }
        for func in funcs.values() {
            if Self::method_owner(func) == Some(resource) {
                scope.insert(self.func_name(func));
            }
        }
        for func in funcs.values() {
            if Self::method_owner(func) == Some(resource) {
                self.write_func(w, func, "    ", Some(resource_name), &scope);
            }
        }
    }

    fn param_name(&self, param: &Param, scope: &IndexSet<String>) -> String {
        let name = words(&param.name, Style::Snake);
        if scope.contains(&name)
            || name == "abi"
            || name == "raw"
            || self
                .imported
                .iter()
                .any(|&iface| self.module_name(iface) == name)
        {
            return format!("{name}_");
        }
        name
    }

    fn write_func(
        &mut self,
        w: &mut String,
        func: &Function,
        indent: &str,
        self_type: Option<&str>,
        scope: &IndexSet<String>,
    ) {
        let is_method = func.name.starts_with("[method]");
        let names: Vec<String> = func
            .params
            .iter()
            .map(|p| self.param_name(p, scope))
            .collect();

        Self::write_docs(w, &func.docs, indent);
        let mut consumes = false;
        for (p, name) in func.params.iter().zip(&names) {
            if !self.is_own(&p.ty) {
                continue;
            }
            if !consumes && func.docs.contents.is_some() {
                let _ = writeln!(w, "{indent}///");
            }
            consumes = true;
            let _ = writeln!(w, "{indent}/// Consumes `{name}`.");
        }

        let _ = write!(w, "{indent}pub fn {}(", fmt_id(&self.func_name(func)));
        for (i, (p, name)) in func.params.iter().zip(&names).enumerate() {
            if i != 0 {
                w.push_str(", ");
            }
            let _ = write!(w, "{}: ", fmt_id(name));
            if is_method && i == 0 {
                w.push_str(self_type.unwrap());
            } else if let Type::Id(id) = p.ty {
                if let TypeDefKind::Handle(Handle::Borrow(res)) = self.resolve.types[id].kind {
                    self.write_type_id(w, res);
                } else {
                    self.write_ref(w, Some(&p.ty));
                }
            } else {
                self.write_ref(w, Some(&p.ty));
            }
        }
        w.push_str(") ");
        self.write_ref(w, func.result.as_ref());

        let _ = write!(
            w,
            " {{\n{indent}    return abi.call(raw.{}, ",
            fmt_id(&func.name)
        );
        self.write_ref(w, func.result.as_ref());
        w.push_str(if func.params.len() > 1 {
            ", .{ "
        } else {
            ", .{"
        });
        for (i, (p, name)) in func.params.iter().zip(&names).enumerate() {
            if i != 0 {
                w.push_str(", ");
            }
            if self.is_borrow(&p.ty) {
                let _ = write!(w, "abi.borrow({})", fmt_id(name));
            } else {
                let _ = write!(w, "{}", fmt_id(name));
            }
        }
        let _ = write!(
            w,
            "{});\n{indent}}}\n\n",
            if func.params.len() > 1 { " }" } else { "}" }
        );
    }

    fn write_raw(w: &mut String, module: &str, name: &str, sig: &Signature) {
        let _ = write!(w, "    extern \"{module}\" fn {}(", fmt_id(name));
        for (i, p) in sig.params.iter().enumerate() {
            if i != 0 {
                w.push_str(", ");
            }
            w.push_str(p.as_str());
        }
        let _ = writeln!(w, ") {};", sig.result.map_or("void", |r| r.as_str()));
    }

    fn write_imports(&self, w: &mut String) {
        if self.used_root {
            let _ = writeln!(w, "const root = @import(\"root.zig\");");
        }
        for &iface in &self.used {
            let name = self.module_name(iface);
            let _ = writeln!(w, "const {} = @import(\"{name}.zig\");", fmt_id(name));
        }
    }

    fn header() -> String {
        let mut s = Source::default();
        wit_bindgen_core::generated_preamble(&mut s, env!("CARGO_PKG_VERSION"));
        let mut out = s.to_string();
        out.push('\n');
        out
    }

    fn interface_file(&mut self, iface: InterfaceId) -> String {
        self.current = Some(iface);
        self.imported.clear();
        let _ = self.interface_pass(iface);
        self.imported = self.used.clone();
        self.interface_pass(iface)
    }

    fn interface_pass(&mut self, iface: InterfaceId) -> String {
        self.used.clear();
        self.used_root = false;
        let types = &self.resolve.interfaces[iface].types;
        let empty_funcs = IndexMap::new();
        let funcs = if self.imports.contains(&iface) {
            &self.resolve.interfaces[iface].functions
        } else {
            &empty_funcs
        };
        let module = self.import_path(iface);

        let mut scope = IndexSet::new();
        for name in types.keys() {
            scope.insert(words(name, Style::Title));
        }
        for func in funcs.values() {
            if Self::method_owner(func).is_none() {
                scope.insert(self.func_name(func));
            }
        }

        let mut body = String::new();
        let mut in_aliases = false;
        for &id in types.values() {
            let is_alias = self.is_plain_alias(id);
            if in_aliases && !is_alias {
                body.push('\n');
            }
            in_aliases = is_alias;
            self.write_type(&mut body, id, funcs);
        }
        if in_aliases {
            body.push('\n');
        }

        for func in funcs.values() {
            if Self::method_owner(func).is_none() {
                self.write_func(&mut body, func, "", None, &scope);
            }
        }

        let mut raw = String::new();
        for (name, func) in funcs {
            let sig = self.signature(func, Side::Import);
            Self::write_raw(&mut raw, &module, name, &sig);
        }
        for (name, &id) in types {
            if !matches!(self.resolve.types[id].kind, TypeDefKind::Resource) {
                continue;
            }
            let drop = Signature {
                params: vec![Core::I32],
                result: None,
                indirect: false,
            };
            Self::write_raw(&mut raw, &module, &format!("[resource-drop]{name}"), &drop);
        }

        let mut out = Self::header();
        let uses_abi = !raw.is_empty() || body.contains("abi.");
        if uses_abi {
            let _ = writeln!(out, "const abi = @import(\"{}\");", self.abi_path);
        }
        self.write_imports(&mut out);
        if uses_abi || !self.used.is_empty() || self.used_root {
            out.push('\n');
        }
        out.push_str(&body);
        if !raw.is_empty() {
            let _ = write!(out, "const raw = struct {{\n{raw}}};\n");
        }
        format!("{}\n", out.trim_end_matches('\n'))
    }

    fn root_module_file(&mut self) -> String {
        self.current = None;
        self.imported.clear();
        let _ = self.root_module_pass();
        self.imported = self.used.clone();
        self.root_module_pass()
    }

    fn root_module_pass(&mut self) -> String {
        self.used.clear();
        self.used_root = false;
        let world = &self.resolve.worlds[self.world_id];
        let mut types = IndexMap::new();
        let mut funcs = IndexMap::new();
        for (key, item) in &world.imports {
            match item {
                WorldItem::Type { id, .. } => {
                    if !Self::is_interface_alias(self.resolve, *id) {
                        types.insert(self.resolve.name_world_key(key), *id);
                    }
                }
                WorldItem::Function(f) => {
                    funcs.insert(f.name.clone(), f.clone());
                }
                WorldItem::Interface { .. } => {}
            }
        }

        let mut scope = IndexSet::new();
        for name in types.keys() {
            scope.insert(words(name, Style::Title));
        }
        for func in funcs.values() {
            if Self::method_owner(func).is_none() {
                scope.insert(self.func_name(func));
            }
        }

        let mut body = String::new();
        let mut in_aliases = false;
        for &id in types.values() {
            let is_alias = self.is_plain_alias(id);
            if in_aliases && !is_alias {
                body.push('\n');
            }
            in_aliases = is_alias;
            self.write_type(&mut body, id, &funcs);
        }
        if in_aliases {
            body.push('\n');
        }

        for func in funcs.values() {
            if Self::method_owner(func).is_none() {
                self.write_func(&mut body, func, "", None, &scope);
            }
        }

        let mut raw = String::new();
        for (name, func) in &funcs {
            let sig = self.signature(func, Side::Import);
            Self::write_raw(&mut raw, "$root", name, &sig);
        }
        for (name, &id) in &types {
            if !matches!(self.resolve.types[id].kind, TypeDefKind::Resource) {
                continue;
            }
            let drop = Signature {
                params: vec![Core::I32],
                result: None,
                indirect: false,
            };
            Self::write_raw(&mut raw, "$root", &format!("[resource-drop]{name}"), &drop);
        }

        let mut out = Self::header();
        let uses_abi = !raw.is_empty() || body.contains("abi.");
        if uses_abi {
            let _ = writeln!(out, "const abi = @import(\"{}\");", self.abi_path);
        }
        self.write_imports(&mut out);
        if uses_abi || !self.used.is_empty() {
            out.push('\n');
        }
        out.push_str(&body);
        if !raw.is_empty() {
            let _ = write!(out, "const raw = struct {{\n{raw}}};\n");
        }
        format!("{}\n", out.trim_end_matches('\n'))
    }

    fn write_export(&mut self, w: &mut String, export_name: &str, fn_name: &str, func: &Function) {
        let sig = self.signature(func, Side::Export);

        Self::write_docs(w, &func.docs, "");
        let _ = write!(w, "pub fn {}(comptime impl: fn (", fmt_id(fn_name));
        for (i, p) in func.params.iter().enumerate() {
            if i != 0 {
                w.push_str(", ");
            }
            self.write_ref(w, Some(&p.ty));
        }
        w.push_str(") ");
        self.write_ref(w, func.result.as_ref());

        w.push_str(") void {\n    const glue = struct {\n        fn call(");
        for (i, p) in sig.params.iter().enumerate() {
            if i != 0 {
                w.push_str(", ");
            }
            let _ = write!(w, "p{i}: {}", p.as_str());
        }
        let _ = writeln!(
            w,
            ") callconv(.c) {} {{",
            sig.result.map_or("void", |r| r.as_str())
        );

        w.push_str("            abi.beginCall();\n");
        w.push_str("            const args = abi.liftArgs(std.meta.ArgsTuple(@TypeOf(impl)), ");
        w.push_str(if sig.params.len() > 1 { ".{ " } else { ".{" });
        for i in 0..sig.params.len() {
            if i != 0 {
                w.push_str(", ");
            }
            let _ = write!(w, "p{i}");
        }
        w.push_str(if sig.params.len() > 1 {
            " });\n"
        } else {
            "});\n"
        });

        if !sig.indirect {
            w.push_str("            defer abi.endCall();\n");
        }
        w.push_str("            return abi.lowerReturn(");
        self.write_ref(w, func.result.as_ref());
        w.push_str(", @call(.auto, impl, args));\n        }\n");

        if sig.indirect {
            w.push_str(
                "\n        fn post(_: i32) callconv(.c) void {\n            abi.endCall();\n        }\n",
            );
        }
        let _ = writeln!(
            w,
            "    }};\n    @export(&glue.call, .{{ .name = \"{export_name}\" }});"
        );
        if sig.indirect {
            let _ = writeln!(
                w,
                "    @export(&glue.post, .{{ .name = \"cabi_post_{export_name}\" }});"
            );
        }
        w.push_str("}\n\n");
    }

    fn exports_file(&mut self) -> String {
        self.current = None;
        self.used.clear();
        self.used_root = false;
        self.imported.clear();

        let mut scope = IndexSet::new();
        let mut body = String::new();
        let exports = &self.resolve.worlds[self.world_id].exports;
        for (key, item) in exports {
            match item {
                WorldItem::Function(func) => {
                    let name = self.resolve.name_world_key(key);
                    let mut fn_name = words(&name, Style::Camel);
                    while !scope.insert(fn_name.clone()) {
                        fn_name.push('_');
                    }
                    self.write_export(&mut body, &name, &fn_name, func);
                }
                WorldItem::Interface { id, .. } => {
                    let iface_path = self.import_path(*id);
                    let funcs = &self.resolve.interfaces[*id].functions;
                    for (func_name, func) in funcs {
                        let mut fn_name = words(func_name, Style::Camel);
                        while !scope.insert(fn_name.clone()) {
                            fn_name.push('_');
                        }
                        let full = format!("{iface_path}#{func_name}");
                        self.write_export(&mut body, &full, &fn_name, func);
                    }
                }
                WorldItem::Type { .. } => {}
            }
        }

        let mut out = Self::header();
        let _ = writeln!(
            out,
            "const std = @import(\"std\");\nconst abi = @import(\"{}\");",
            self.abi_path
        );
        self.write_imports(&mut out);
        let _ = write!(out, "\n{}\n", body.trim_end_matches('\n'));
        out
    }
}

impl WorldGenerator for Zig {
    fn uses_nominal_type_ids(&self) -> bool {
        false
    }

    fn import_interface(
        &mut self,
        _resolve: &Resolve,
        _name: &WorldKey,
        _iface: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        Ok(())
    }

    fn export_interface(
        &mut self,
        _resolve: &Resolve,
        _name: &WorldKey,
        _iface: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        Ok(())
    }

    fn import_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        Ok(())
    }

    fn export_funcs(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        Ok(())
    }

    fn import_types(
        &mut self,
        _resolve: &Resolve,
        _world: WorldId,
        _types: &[(&str, TypeId)],
        _files: &mut Files,
    ) -> Result<()> {
        Ok(())
    }

    fn finish(&mut self, resolve: &Resolve, world_id: WorldId, files: &mut Files) -> Result<()> {
        let name = self.opts.effective_name();
        let emit_bundled_abi = self.opts.abi_path.is_none();
        let abi_path = self
            .opts
            .abi_path
            .clone()
            .unwrap_or_else(|| "abi.zig".to_string());

        let mut g = Gen::new(resolve, world_id, abi_path)?;

        let mut root_file = Gen::header();
        if emit_bundled_abi {
            files.push(&format!("{name}/abi.zig"), ABI_ZIG.as_bytes());
            let _ = writeln!(root_file, "pub const abi = @import(\"{name}/abi.zig\");");
        }

        if g.has_root_module {
            let data = g.root_module_file();
            files.push(&format!("{name}/root.zig"), data.as_bytes());
            let _ = writeln!(root_file, "pub const root = @import(\"{name}/root.zig\");");
        }

        let interfaces = g.interfaces.clone();
        for iface in interfaces {
            let module = g.module_name(iface).to_string();
            let data = g.interface_file(iface);
            files.push(&format!("{name}/{module}.zig"), data.as_bytes());
            let _ = writeln!(
                root_file,
                "pub const {} = @import(\"{name}/{module}.zig\");",
                fmt_id(&module)
            );
        }

        if !resolve.worlds[world_id].exports.is_empty() {
            let data = g.exports_file();
            files.push(&format!("{name}/exports.zig"), data.as_bytes());
            let _ = writeln!(
                root_file,
                "pub const exports = @import(\"{name}/exports.zig\");"
            );
        }

        files.push(&format!("{name}.zig"), root_file.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn strip_header(s: &str) -> &str {
        match s.find("\n\n") {
            Some(idx) if s.starts_with("// Generated by ") => &s[idx + 2..],
            _ => s,
        }
    }

    #[test]
    fn demo_world() {
        let mut resolve = Resolve::default();
        let (pkg, _) = resolve
            .push_path(Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/testdata/wit/demo.wit"
            )))
            .unwrap();
        let world = resolve.select_world(&[pkg], None).unwrap();

        let mut files = Files::default();
        Opts {
            name: "wit".to_string(),
            abi_path: Some("../abi.zig".to_string()),
        }
        .build()
        .generate(&mut resolve, world, &mut files)
        .unwrap();

        let expected = [
            (
                "wit/types.zig",
                include_str!("../testdata/expected/wit/types.zig"),
            ),
            (
                "wit/host.zig",
                include_str!("../testdata/expected/wit/host.zig"),
            ),
            (
                "wit/guest.zig",
                include_str!("../testdata/expected/wit/guest.zig"),
            ),
            (
                "wit/exports.zig",
                include_str!("../testdata/expected/wit/exports.zig"),
            ),
            ("wit.zig", include_str!("../testdata/expected/wit.zig")),
        ];

        assert_eq!(files.iter().count(), expected.len());
        for (want_path, want_str) in expected {
            let got_bytes = files
                .iter()
                .find_map(|(p, b)| (p == want_path).then_some(b))
                .unwrap_or_else(|| panic!("missing generated file {want_path}"));
            let got_str = std::str::from_utf8(got_bytes).unwrap();
            assert_eq!(
                strip_header(got_str),
                strip_header(want_str),
                "mismatch in {want_path}"
            );
        }
    }

    #[test]
    fn bundled_abi_and_world_items() {
        let mut resolve = Resolve::default();
        let pkg = resolve
            .push_str(
                "world.wit",
                r#"
                package example:test@0.1.0;

                world app {
                    record config {
                        port: u16,
                    }
                    import get-config: func() -> config;
                    export run: func(cfg: config);
                }
                "#,
            )
            .unwrap();
        let world = resolve.select_world(&[pkg], None).unwrap();

        let mut files = Files::default();
        Opts::default()
            .build()
            .generate(&mut resolve, world, &mut files)
            .unwrap();

        let file_names: Vec<&str> = files.iter().map(|(name, _)| name).collect();
        assert_eq!(
            file_names,
            vec!["wit.zig", "wit/abi.zig", "wit/exports.zig", "wit/root.zig"]
        );
    }
}
