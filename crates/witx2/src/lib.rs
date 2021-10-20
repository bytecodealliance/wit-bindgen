use anyhow::{anyhow, bail, Context, Result};
use id_arena::{Arena, Id};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub mod abi;
mod ast;
mod sizealign;
pub use sizealign::*;

pub struct Interface {
    pub name: String,
    pub types: Arena<TypeDef>,
    pub type_lookup: HashMap<String, TypeId>,
    pub resources: Arena<Resource>,
    pub resource_lookup: HashMap<String, ResourceId>,
    pub interfaces: Arena<Interface>,
    pub interface_lookup: HashMap<String, InterfaceId>,
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
}

pub type TypeId = Id<TypeDef>;
pub type ResourceId = Id<Resource>;
pub type InterfaceId = Id<Interface>;

pub struct TypeDef {
    pub docs: Docs,
    pub kind: TypeDefKind,
    pub name: Option<String>,
    /// `None` if this type is originally declared in this instance or
    /// otherwise `Some` if it was originally defined in a different module.
    pub foreign_module: Option<String>,
}

pub enum TypeDefKind {
    Record(Record),
    Variant(Variant),
    List(Type),
    Pointer(Type),
    ConstPointer(Type),
    PushBuffer(Type),
    PullBuffer(Type),
    Type(Type),
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Type {
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    Char,
    CChar,
    Usize,
    Handle(ResourceId),
    Id(TypeId),
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Int {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Debug)]
pub struct Record {
    pub fields: Vec<Field>,
    pub kind: RecordKind,
}

#[derive(Copy, Clone, Debug)]
pub enum RecordKind {
    Other,
    Flags(Option<Int>),
    Tuple,
}

#[derive(Debug)]
pub struct Field {
    pub docs: Docs,
    pub name: String,
    pub ty: Type,
}

impl Record {
    pub fn is_tuple(&self) -> bool {
        match self.kind {
            RecordKind::Tuple => true,
            _ => false,
        }
    }

    pub fn is_flags(&self) -> bool {
        match self.kind {
            RecordKind::Flags(_) => true,
            _ => false,
        }
    }

    pub fn num_i32s(&self) -> usize {
        (self.fields.len() + 31) / 32
    }
}

impl RecordKind {
    fn infer(types: &Arena<TypeDef>, fields: &[Field]) -> RecordKind {
        if fields.len() == 0 {
            return RecordKind::Other;
        }

        // Structs-of-bools are classified to get represented as bitflags.
        if fields.iter().all(|t| is_bool(&t.ty, types)) {
            return RecordKind::Flags(None);
        }

        // fields with consecutive integer names get represented as tuples.
        if fields
            .iter()
            .enumerate()
            .all(|(i, m)| m.name.as_str().parse().ok() == Some(i))
        {
            return RecordKind::Tuple;
        }

        return RecordKind::Other;

        fn is_bool(t: &Type, types: &Arena<TypeDef>) -> bool {
            match t {
                Type::Id(v) => match &types[*v].kind {
                    TypeDefKind::Variant(v) => v.is_bool(),
                    TypeDefKind::Type(t) => is_bool(t, types),
                    _ => false,
                },
                _ => false,
            }
        }
    }
}

#[derive(Debug)]
pub struct Variant {
    pub cases: Vec<Case>,
    /// The bit representation of the width of this variant's tag when the
    /// variant is stored in memory.
    pub tag: Int,
}

#[derive(Debug)]
pub struct Case {
    pub docs: Docs,
    pub name: String,
    pub ty: Option<Type>,
}

impl Variant {
    pub fn infer_tag(cases: usize) -> Int {
        match cases {
            n if n <= u8::max_value() as usize => Int::U8,
            n if n <= u16::max_value() as usize => Int::U16,
            n if n <= u32::max_value() as usize => Int::U32,
            n if n <= u64::max_value() as usize => Int::U64,
            _ => panic!("too many cases to fit in a repr"),
        }
    }

    pub fn is_bool(&self) -> bool {
        self.cases.len() == 2
            && self.cases[0].name == "false"
            && self.cases[1].name == "true"
            && self.cases[0].ty.is_none()
            && self.cases[1].ty.is_none()
    }

    pub fn is_enum(&self) -> bool {
        self.cases.iter().all(|c| c.ty.is_none())
    }

    pub fn as_option(&self) -> Option<&Type> {
        if self.cases.len() != 2 {
            return None;
        }
        if self.cases[0].name != "none" || self.cases[0].ty.is_some() {
            return None;
        }
        if self.cases[1].name != "some" {
            return None;
        }
        self.cases[1].ty.as_ref()
    }

    pub fn as_expected(&self) -> Option<(Option<&Type>, Option<&Type>)> {
        if self.cases.len() != 2 {
            return None;
        }
        if self.cases[0].name != "ok" {
            return None;
        }
        if self.cases[1].name != "err" {
            return None;
        }
        Some((self.cases[0].ty.as_ref(), self.cases[1].ty.as_ref()))
    }
}

#[derive(Clone, Default, Debug)]
pub struct Docs {
    pub contents: Option<String>,
}

pub struct Resource {
    pub docs: Docs,
    pub name: String,
    /// `None` if this resource is defined within the containing instance,
    /// otherwise `Some` if it's defined in an instance named here.
    pub foreign_module: Option<String>,
}

pub struct Global {
    pub docs: Docs,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug)]
pub struct Function {
    pub abi: abi::Abi,
    pub is_async: bool,
    pub docs: Docs,
    pub name: String,
    pub kind: FunctionKind,
    pub params: Vec<(String, Type)>,
    pub results: Vec<(String, Type)>,
}

#[derive(Debug)]
pub enum FunctionKind {
    Freestanding,
    Static { resource: ResourceId, name: String },
    Method { resource: ResourceId, name: String },
}

impl Function {
    pub fn item_name(&self) -> &str {
        match &self.kind {
            FunctionKind::Freestanding => &self.name,
            FunctionKind::Static { name, .. } => name,
            FunctionKind::Method { name, .. } => name,
        }
    }
}

fn unwrap_md(contents: String) -> String {
    let mut witx = String::from("");
    let mut extract_next = false;
    Parser::new_ext(&contents, Options::empty())
        .for_each(|event| match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::Borrowed("witx")))) => {
                extract_next = true;
            },
            Event::Text(text) => {
                if extract_next {
                    witx += &text.to_owned();
                    witx += "\n";
                }
            },
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::Borrowed("witx")))) => {
                extract_next = false;
            },
            _ => { },
        });
    witx
}

impl Interface {
    pub fn parse(name: &str, input: String) -> Result<Interface> {
        Interface::parse_with(name, input, |f| {
            Err(anyhow!("cannot load submodule `{}`", f))
        })
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Interface> {
        let path = path.as_ref();
        let parent = path.parent().unwrap();
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read: {}", path.display()))?;
        Interface::parse_with(path, contents, |path| load_fs(parent, path))
    }

    pub fn parse_with(
        filename: impl AsRef<Path>,
        contents: String,
        mut load: impl FnMut(&str) -> Result<(PathBuf, String)>,
    ) -> Result<Interface> {
        Interface::_parse_with(
            filename.as_ref(),
            contents,
            &mut load,
            &mut HashSet::new(),
            &mut HashMap::new(),
        )
    }

    fn _parse_with(
        filename: &Path,
        contents: String,
        load: &mut dyn FnMut(&str) -> Result<(PathBuf, String)>,
        visiting: &mut HashSet<PathBuf>,
        map: &mut HashMap<String, Interface>,
    ) -> Result<Interface> {
        let contents = if filename.extension().and_then(OsStr::to_str).unwrap_or("witx") == "md" {
            unwrap_md(contents)
        } else {
            contents
        };
        // Parse the `contents `into an AST
        let ast = match ast::Ast::parse(&contents) {
            Ok(ast) => ast,
            Err(mut e) => {
                let file = filename.display().to_string();
                ast::rewrite_error(&mut e, &file, &contents);
                return Err(e);
            }
        };

        // Load up any modules into our `map` that have not yet been parsed.
        if !visiting.insert(filename.to_path_buf()) {
            bail!("file `{}` recursively imports itself", filename.display())
        }
        for item in ast.items.iter() {
            let u = match item {
                ast::Item::Use(u) => u,
                _ => continue,
            };
            if map.contains_key(&*u.from[0].name) {
                continue;
            }
            let (filename, contents) = load(&u.from[0].name)
                // TODO: insert context here about `u.name.span` and `filename`
                ?;
            let instance = Interface::_parse_with(&filename, contents, load, visiting, map)?;
            map.insert(u.from[0].name.to_string(), instance);
        }
        visiting.remove(filename);

        // and finally resolve everything into our final instance
        let name = filename.file_stem().unwrap().to_str().unwrap();
        match ast.resolve(name, map) {
            Ok(i) => Ok(i),
            Err(mut e) => {
                let file = filename.display().to_string();
                ast::rewrite_error(&mut e, &file, &contents);
                Err(e)
            }
        }
    }

    pub fn topological_types(&self) -> Vec<TypeId> {
        let mut ret = Vec::new();
        let mut visited = HashSet::new();
        for (id, _) in self.types.iter() {
            self.topo_visit(id, &mut ret, &mut visited);
        }
        return ret;
    }

    fn topo_visit(&self, id: TypeId, list: &mut Vec<TypeId>, visited: &mut HashSet<TypeId>) {
        if !visited.insert(id) {
            return;
        }
        match &self.types[id].kind {
            TypeDefKind::Type(t)
            | TypeDefKind::List(t)
            | TypeDefKind::PushBuffer(t)
            | TypeDefKind::PullBuffer(t)
            | TypeDefKind::Pointer(t)
            | TypeDefKind::ConstPointer(t) => self.topo_visit_ty(t, list, visited),
            TypeDefKind::Record(r) => {
                for f in r.fields.iter() {
                    self.topo_visit_ty(&f.ty, list, visited);
                }
            }
            TypeDefKind::Variant(v) => {
                for v in v.cases.iter() {
                    if let Some(ty) = &v.ty {
                        self.topo_visit_ty(ty, list, visited);
                    }
                }
            }
        }
        list.push(id);
    }

    fn topo_visit_ty(&self, ty: &Type, list: &mut Vec<TypeId>, visited: &mut HashSet<TypeId>) {
        if let Type::Id(id) = ty {
            self.topo_visit(*id, list, visited);
        }
    }

    pub fn all_bits_valid(&self, ty: &Type) -> bool {
        match ty {
            Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::U64
            | Type::S64
            | Type::F32
            | Type::F64
            | Type::CChar
            | Type::Usize => true,

            Type::Char | Type::Handle(_) => false,

            Type::Id(id) => match &self.types[*id].kind {
                TypeDefKind::List(_)
                | TypeDefKind::Variant(_)
                | TypeDefKind::PushBuffer(_)
                | TypeDefKind::PullBuffer(_) => false,
                TypeDefKind::Type(t) => self.all_bits_valid(t),
                TypeDefKind::Record(r) => r.fields.iter().all(|f| self.all_bits_valid(&f.ty)),
                TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => true,
            },
        }
    }

    pub fn has_preview1_pointer(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.types[*id].kind {
                TypeDefKind::List(t) | TypeDefKind::PushBuffer(t) | TypeDefKind::PullBuffer(t) => {
                    self.has_preview1_pointer(t)
                }
                TypeDefKind::Type(t) => self.has_preview1_pointer(t),
                TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => true,
                TypeDefKind::Record(r) => r.fields.iter().any(|f| self.has_preview1_pointer(&f.ty)),
                TypeDefKind::Variant(v) => v.cases.iter().any(|c| match &c.ty {
                    Some(ty) => self.has_preview1_pointer(ty),
                    None => false,
                }),
            },
            _ => false,
        }
    }
}

fn load_fs(root: &Path, name: &str) -> Result<(PathBuf, String)> {
    let path = root.join(name).with_extension("witx");
    let contents =
        fs::read_to_string(&path).context(format!("failed to read `{}`", path.display()))?;
    Ok((path, contents))
}
