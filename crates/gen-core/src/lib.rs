use anyhow::Result;
use std::collections::{btree_map::Entry, BTreeMap, HashMap, HashSet};
use std::ops::Deref;
use std::path::Path;
use witx2::abi::{Abi, Direction};
use witx2::*;

// pub use witx;
pub use witx2;
mod ns;

pub use ns::Ns;

pub trait Generator {
    fn preprocess_all(&mut self, imports: &[Interface], exports: &[Interface]) {
        drop((imports, exports));
    }

    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        drop((iface, dir));
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    );
    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    );
    fn type_resource(&mut self, iface: &Interface, ty: ResourceId);
    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_pointer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        const_: bool,
        ty: &Type,
        docs: &Docs,
    );
    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    );
    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    );
    // fn const_(&mut self, iface: &Interface, name: &str, ty: &str, val: u64, docs: &Docs);
    fn import(&mut self, iface: &Interface, func: &Function);
    fn export(&mut self, iface: &Interface, func: &Function);

    fn finish_one(&mut self, iface: &Interface, files: &mut Files);

    fn finish_all(&mut self, files: &mut Files) {
        drop(files);
    }

    fn generate_one(&mut self, iface: &Interface, dir: Direction, files: &mut Files) {
        self.preprocess_one(iface, dir);

        for (id, ty) in iface.types.iter() {
            // assert!(ty.foreign_module.is_none()); // TODO
            let name = match &ty.name {
                Some(name) => name,
                None => continue,
            };
            match &ty.kind {
                TypeDefKind::Record(record) => self.type_record(iface, id, name, record, &ty.docs),
                TypeDefKind::Variant(variant) => {
                    self.type_variant(iface, id, name, variant, &ty.docs)
                }
                TypeDefKind::List(t) => self.type_list(iface, id, name, t, &ty.docs),
                TypeDefKind::PushBuffer(t) => self.type_push_buffer(iface, id, name, t, &ty.docs),
                TypeDefKind::PullBuffer(t) => self.type_pull_buffer(iface, id, name, t, &ty.docs),
                TypeDefKind::Type(t) => self.type_alias(iface, id, name, t, &ty.docs),
                TypeDefKind::Pointer(t) => self.type_pointer(iface, id, name, false, t, &ty.docs),
                TypeDefKind::ConstPointer(t) => {
                    self.type_pointer(iface, id, name, true, t, &ty.docs)
                }
            }
        }

        for (id, _resource) in iface.resources.iter() {
            self.type_resource(iface, id);
        }

        // for c in module.constants() {
        //     self.const_(&c.name, &c.ty, c.value, &c.docs);
        // }

        for f in iface.functions.iter() {
            match dir {
                Direction::Import => self.import(iface, &f),
                Direction::Export => self.export(iface, &f),
            }
        }

        self.finish_one(iface, files)
    }

    fn generate_all(&mut self, imports: &[Interface], exports: &[Interface], files: &mut Files) {
        self.preprocess_all(imports, exports);

        for imp in imports {
            self.generate_one(imp, Direction::Import, files);
        }

        for exp in exports {
            self.generate_one(exp, Direction::Export, files);
        }

        self.finish_all(files);
    }
}

#[derive(Default)]
pub struct Types {
    type_info: HashMap<TypeId, TypeInfo>,
    handle_dtors: HashSet<ResourceId>,
    dtor_funcs: HashSet<String>,
}

#[derive(Default, Clone, Copy)]
pub struct TypeInfo {
    /// Whether or not this type is ever used (transitively) within the
    /// parameter of a function.
    pub param: bool,

    /// Whether or not this type is ever used (transitively) within the
    /// result of a function.
    pub result: bool,

    /// Whether or not this type (transitively) has a list.
    pub has_list: bool,

    /// Whether or not this type (transitively) has a handle.
    pub has_handle: bool,

    /// Whether or not this type (transitively) has a push buffer.
    pub has_push_buffer: bool,

    /// Whether or not this type (transitively) has a pull buffer.
    pub has_pull_buffer: bool,
}

impl std::ops::BitOrAssign for TypeInfo {
    fn bitor_assign(&mut self, rhs: Self) {
        self.param |= rhs.param;
        self.result |= rhs.result;
        self.has_list |= rhs.has_list;
        self.has_handle |= rhs.has_handle;
        self.has_push_buffer |= rhs.has_push_buffer;
        self.has_pull_buffer |= rhs.has_pull_buffer;
    }
}

impl Types {
    pub fn analyze(&mut self, iface: &Interface) {
        for (t, _) in iface.types.iter() {
            self.type_id_info(iface, t);
        }
        for f in iface.functions.iter() {
            for (_, ty) in f.params.iter() {
                self.set_param_result_ty(iface, ty, true, false);
            }
            for (_, ty) in f.results.iter() {
                self.set_param_result_ty(iface, ty, false, true);
            }
            self.maybe_set_preview1_dtor(iface, f);
        }
    }

    fn maybe_set_preview1_dtor(&mut self, iface: &Interface, f: &Function) {
        match f.abi {
            Abi::Preview1 => {}
            _ => return,
        }

        // Dtors only happen when the function has a singular parameter
        if f.params.len() != 1 {
            return;
        }

        // Dtors are inferred to be `${type}_close` right now.
        let name = f.name.as_str();
        let prefix = match name.strip_suffix("_close") {
            Some(prefix) => prefix,
            None => return,
        };

        // The singular parameter type name must be the prefix of this
        // function's own name.
        let resource = match find_handle(iface, &f.params[0].1) {
            Some(id) => id,
            None => return,
        };
        if iface.resources[resource].name != prefix {
            return;
        }

        self.handle_dtors.insert(resource);
        self.dtor_funcs.insert(f.name.to_string());

        fn find_handle(iface: &Interface, ty: &Type) -> Option<ResourceId> {
            match ty {
                Type::Handle(r) => Some(*r),
                Type::Id(id) => match &iface.types[*id].kind {
                    TypeDefKind::Type(t) => find_handle(iface, t),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub fn get(&self, id: TypeId) -> TypeInfo {
        self.type_info[&id]
    }

    pub fn has_preview1_dtor(&self, resource: ResourceId) -> bool {
        self.handle_dtors.contains(&resource)
    }

    pub fn is_preview1_dtor_func(&self, func: &Function) -> bool {
        self.dtor_funcs.contains(&func.name)
    }

    pub fn type_id_info(&mut self, iface: &Interface, ty: TypeId) -> TypeInfo {
        if let Some(info) = self.type_info.get(&ty) {
            return *info;
        }
        let mut info = TypeInfo::default();
        match &iface.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    info |= self.type_info(iface, &field.ty);
                }
            }
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    if let Some(ty) = &case.ty {
                        info |= self.type_info(iface, ty);
                    }
                }
            }
            TypeDefKind::List(ty) => {
                info = self.type_info(iface, ty);
                info.has_list = true;
            }
            TypeDefKind::PushBuffer(ty) => {
                info = self.type_info(iface, ty);
                info.has_push_buffer = true;
            }
            TypeDefKind::PullBuffer(ty) => {
                info = self.type_info(iface, ty);
                info.has_pull_buffer = true;
            }
            TypeDefKind::ConstPointer(ty) | TypeDefKind::Pointer(ty) | TypeDefKind::Type(ty) => {
                info = self.type_info(iface, ty)
            }
        }
        self.type_info.insert(ty, info);
        return info;
    }

    pub fn type_info(&mut self, iface: &Interface, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::Handle(_) => info.has_handle = true,
            Type::Id(id) => return self.type_id_info(iface, *id),
            _ => {}
        }
        info
    }

    fn set_param_result_id(&mut self, iface: &Interface, ty: TypeId, param: bool, result: bool) {
        match &iface.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    self.set_param_result_ty(iface, &field.ty, param, result)
                }
            }
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    if let Some(ty) = &case.ty {
                        self.set_param_result_ty(iface, ty, param, result)
                    }
                }
            }
            TypeDefKind::List(ty)
            | TypeDefKind::PushBuffer(ty)
            | TypeDefKind::PullBuffer(ty)
            | TypeDefKind::Pointer(ty)
            | TypeDefKind::ConstPointer(ty) => self.set_param_result_ty(iface, ty, param, result),
            TypeDefKind::Type(ty) => self.set_param_result_ty(iface, ty, param, result),
        }
    }

    fn set_param_result_ty(&mut self, iface: &Interface, ty: &Type, param: bool, result: bool) {
        match ty {
            Type::Id(id) => {
                self.type_id_info(iface, *id);
                let info = self.type_info.get_mut(id).unwrap();
                if (param && !info.param) || (result && !info.result) {
                    info.param = info.param || param;
                    info.result = info.result || result;
                    self.set_param_result_id(iface, *id, param, result);
                }
            }
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct Files {
    files: BTreeMap<String, Vec<u8>>,
}

impl Files {
    pub fn push(&mut self, name: &str, contents: &[u8]) {
        match self.files.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(contents.to_owned());
            }
            Entry::Occupied(ref mut entry) => {
                entry.get_mut().extend_from_slice(contents);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, &'_ [u8])> {
        self.files.iter().map(|p| (p.0.as_str(), p.1.as_slice()))
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<Interface> {
    Interface::parse_file(path)
}

#[derive(Default)]
pub struct Source {
    s: String,
    indent: usize,
}

impl Source {
    pub fn push_str(&mut self, src: &str) {
        let lines = src.lines().collect::<Vec<_>>();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("}") && self.s.ends_with("  ") {
                self.s.pop();
                self.s.pop();
            }
            self.s.push_str(if lines.len() == 1 {
                line
            } else {
                line.trim_start()
            });
            if trimmed.ends_with('{') {
                self.indent += 1;
            }
            if trimmed.starts_with('}') {
                self.indent -= 1;
            }
            if i != lines.len() - 1 || src.ends_with("\n") {
                self.newline();
            }
        }
    }

    pub fn indent(&mut self, amt: usize) {
        self.indent += amt;
    }

    pub fn deindent(&mut self, amt: usize) {
        self.indent -= amt;
    }

    fn newline(&mut self) {
        self.s.push_str("\n");
        for _ in 0..self.indent {
            self.s.push_str("  ");
        }
    }

    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.s
    }
}

impl Deref for Source {
    type Target = str;
    fn deref(&self) -> &str {
        &self.s
    }
}

impl From<Source> for String {
    fn from(s: Source) -> String {
        s.s
    }
}

#[cfg(test)]
mod tests {
    use super::{Generator, Source};

    #[test]
    fn simple_append() {
        let mut s = Source::default();
        s.push_str("x");
        assert_eq!(s.s, "x");
        s.push_str("y");
        assert_eq!(s.s, "xy");
        s.push_str("z ");
        assert_eq!(s.s, "xyz ");
        s.push_str(" a ");
        assert_eq!(s.s, "xyz  a ");
        s.push_str("\na");
        assert_eq!(s.s, "xyz  a \na");
    }

    #[test]
    fn newline_remap() {
        let mut s = Source::default();
        s.push_str("function() {\n");
        s.push_str("y\n");
        s.push_str("}\n");
        assert_eq!(s.s, "function() {\n  y\n}\n");
    }

    #[test]
    fn if_else() {
        let mut s = Source::default();
        s.push_str("if() {\n");
        s.push_str("y\n");
        s.push_str("} else if () {\n");
        s.push_str("z\n");
        s.push_str("}\n");
        assert_eq!(s.s, "if() {\n  y\n} else if () {\n  z\n}\n");
    }

    #[test]
    fn trim_ws() {
        let mut s = Source::default();
        s.push_str(
            "function() {
                x
        }",
        );
        assert_eq!(s.s, "function() {\n  x\n}");
    }

    #[test]
    fn generator_is_object_safe() {
        fn _assert(_: &dyn Generator) {}
    }
}
