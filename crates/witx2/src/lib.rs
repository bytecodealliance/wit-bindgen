use anyhow::{anyhow, bail, Context, Result};
use id_arena::{Arena, Id};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

mod ast;

pub struct Instance {
    pub types: Arena<TypeDef>,
    pub type_lookup: HashMap<String, TypeId>,
    pub resources: Arena<Resource>,
    pub resource_lookup: HashMap<String, ResourceId>,
    pub functions: Vec<Function>,
}

pub type TypeId = Id<TypeDef>;
pub type ResourceId = Id<Resource>;

pub struct TypeDef {
    pub docs: Documentation,
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
    PushBuffer(Type),
    PullBuffer(Type),
    Type(Type),
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
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
    Handle(ResourceId),
    Id(TypeId),
}

pub struct Record {
    pub fields: Vec<Field>,
}

pub struct Field {
    pub docs: Documentation,
    pub name: String,
    pub ty: Type,
}

pub struct Variant {
    pub cases: Vec<Case>,
}

pub struct Case {
    pub docs: Documentation,
    pub name: String,
    pub ty: Option<Type>,
}

#[derive(Clone, Default)]
pub struct Documentation {
    pub contents: Option<String>,
}

pub struct Resource {
    pub docs: Documentation,
    pub name: String,
    /// `None` if this resource is defined within the containing instance,
    /// otherwise `Some` if it's defined in an instance named here.
    pub foreign_module: Option<String>,
}

pub struct Function {
    pub docs: Documentation,
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub results: Vec<Type>,
}

impl Instance {
    pub fn parse(input: &str) -> Result<Instance> {
        Instance::parse_with("<anon>", input, |f| {
            Err(anyhow!("cannot load submodule `{}`", f))
        })
    }

    pub fn parse_file(path: impl AsRef<Path>) -> Result<Instance> {
        let path = path.as_ref();
        let parent = path.parent().unwrap();
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read: {}", path.display()))?;
        Instance::parse_with(path, &contents, |path| load_fs(parent, path))
    }

    pub fn parse_with(
        filename: impl AsRef<Path>,
        contents: &str,
        mut load: impl FnMut(&str) -> Result<(PathBuf, String)>,
    ) -> Result<Instance> {
        Instance::_parse_with(
            filename.as_ref(),
            contents,
            &mut load,
            &mut HashSet::new(),
            &mut HashMap::new(),
        )
    }

    fn _parse_with(
        filename: &Path,
        contents: &str,
        load: &mut dyn FnMut(&str) -> Result<(PathBuf, String)>,
        visiting: &mut HashSet<PathBuf>,
        map: &mut HashMap<String, Instance>,
    ) -> Result<Instance> {
        // Parse the `contents `into an AST
        let ast = match ast::Ast::parse(&contents) {
            Ok(ast) => ast,
            Err(mut e) => {
                let file = filename.display().to_string();
                ast::rewrite_error(&mut e, &file, contents);
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
            if map.contains_key(&*u.from.name) {
                continue;
            }
            let (filename, contents) = load(&u.from.name)
                // TODO: insert context here about `u.name.span` and `filename`
                ?;
            let instance = Instance::_parse_with(&filename, &contents, load, visiting, map)?;
            map.insert(u.from.name.to_string(), instance);
        }
        visiting.remove(filename);

        // and finally resolve everything into our final instance
        match ast.resolve(map) {
            Ok(i) => Ok(i),
            Err(mut e) => {
                let file = filename.display().to_string();
                ast::rewrite_error(&mut e, &file, contents);
                Err(e)
            }
        }
    }
}

fn load_fs(root: &Path, name: &str) -> Result<(PathBuf, String)> {
    let path = root.join(name).with_extension("witx");
    let contents =
        fs::read_to_string(&path).context(format!("failed to read `{}`", path.display()))?;
    Ok((path, contents))
}
