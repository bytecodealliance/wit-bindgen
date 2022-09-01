use anyhow::Result;
use std::collections::{btree_map::Entry, BTreeMap, HashMap};
use std::fmt::{self, Write};
use std::ops::Deref;
use std::path::Path;
use wit_parser::*;

pub use wit_parser;
mod ns;

pub use ns::Ns;

/// This is the direction from the user's perspective. Are we importing
/// functions to call, or defining functions and exporting them to be called?
///
/// This is only used outside of `Generator` implementations. Inside of
/// `Generator` implementations, the `Direction` is translated to an
/// `AbiVariant` instead. The ABI variant is usually the same as the
/// `Direction`, but it's different in the case of the Wasmtime host bindings:
///
/// In a wasm-calling-wasm use case, one wasm module would use the `Import`
/// ABI, the other would use the `Export` ABI, and there would be an adapter
/// layer between the two that translates from one ABI to the other.
///
/// But with wasm-calling-host, we don't go through a separate adapter layer;
/// the binding code we generate on the host side just does everything itself.
/// So when the host is conceptually "exporting" a function to wasm, it uses
/// the `Import` ABI so that wasm can also use the `Import` ABI and import it
/// directly from the host.
///
/// These are all implementation details; from the user perspective, and
/// from the perspective of everything outside of `Generator` implementations,
/// `export` means I'm exporting functions to be called, and `import` means I'm
/// importing functions that I'm going to call, in both wasm modules and host
/// code. The enum here represents this user perspective.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    Import,
    Export,
}

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
    fn type_flags(&mut self, iface: &Interface, id: TypeId, name: &str, flags: &Flags, docs: &Docs);
    fn type_tuple(&mut self, iface: &Interface, id: TypeId, name: &str, flags: &Tuple, docs: &Docs);
    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    );
    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    );
    fn type_expected(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        expected: &Expected,
        docs: &Docs,
    );
    fn type_union(&mut self, iface: &Interface, id: TypeId, name: &str, union: &Union, docs: &Docs);
    fn type_enum(&mut self, iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs);
    fn type_resource(&mut self, iface: &Interface, ty: ResourceId);
    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs);

    fn preprocess_functions(&mut self, iface: &Interface, dir: Direction) {
        drop((iface, dir));
    }
    fn import(&mut self, iface: &Interface, func: &Function);
    fn export(&mut self, iface: &Interface, func: &Function);
    fn finish_functions(&mut self, iface: &Interface, dir: Direction) {
        drop((iface, dir));
    }

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
                TypeDefKind::Flags(flags) => self.type_flags(iface, id, name, flags, &ty.docs),
                TypeDefKind::Tuple(tuple) => self.type_tuple(iface, id, name, tuple, &ty.docs),
                TypeDefKind::Enum(enum_) => self.type_enum(iface, id, name, enum_, &ty.docs),
                TypeDefKind::Variant(variant) => {
                    self.type_variant(iface, id, name, variant, &ty.docs)
                }
                TypeDefKind::Option(t) => self.type_option(iface, id, name, t, &ty.docs),
                TypeDefKind::Expected(e) => self.type_expected(iface, id, name, e, &ty.docs),
                TypeDefKind::Union(u) => self.type_union(iface, id, name, u, &ty.docs),
                TypeDefKind::List(t) => self.type_list(iface, id, name, t, &ty.docs),
                TypeDefKind::Type(t) => self.type_alias(iface, id, name, t, &ty.docs),
                TypeDefKind::Future(_) => todo!("generate for future"),
                TypeDefKind::Stream(_) => todo!("generate for stream"),
            }
        }

        for (id, _resource) in iface.resources.iter() {
            self.type_resource(iface, id);
        }

        self.preprocess_functions(iface, dir);

        for f in iface.functions.iter() {
            match dir {
                Direction::Import => self.import(iface, &f),
                Direction::Export => self.export(iface, &f),
            }
        }

        self.finish_functions(iface, dir);

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
}

impl std::ops::BitOrAssign for TypeInfo {
    fn bitor_assign(&mut self, rhs: Self) {
        self.param |= rhs.param;
        self.result |= rhs.result;
        self.has_list |= rhs.has_list;
        self.has_handle |= rhs.has_handle;
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
            self.set_param_result_ty(iface, &f.result, false, true);
        }
    }

    pub fn get(&self, id: TypeId) -> TypeInfo {
        self.type_info[&id]
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
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    info |= self.type_info(iface, ty);
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    info |= self.type_info(iface, &case.ty);
                }
            }
            TypeDefKind::List(ty) => {
                info = self.type_info(iface, ty);
                info.has_list = true;
            }
            TypeDefKind::Type(ty) => {
                info = self.type_info(iface, ty);
            }
            TypeDefKind::Option(ty) => {
                info = self.type_info(iface, ty);
            }
            TypeDefKind::Expected(e) => {
                info = self.type_info(iface, &e.ok);
                info |= self.type_info(iface, &e.err);
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    info |= self.type_info(iface, &case.ty);
                }
            }
            TypeDefKind::Future(ty) => {
                info = self.type_info(iface, ty);
            }
            TypeDefKind::Stream(stream) => {
                info = self.type_info(iface, &stream.element);
                info |= self.type_info(iface, &stream.end);
            }
        }
        self.type_info.insert(ty, info);
        return info;
    }

    pub fn type_info(&mut self, iface: &Interface, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::Handle(_) => info.has_handle = true,
            Type::String => info.has_list = true,
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
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    self.set_param_result_ty(iface, ty, param, result)
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    self.set_param_result_ty(iface, &case.ty, param, result)
                }
            }
            TypeDefKind::List(ty) | TypeDefKind::Type(ty) | TypeDefKind::Option(ty) => {
                self.set_param_result_ty(iface, ty, param, result)
            }
            TypeDefKind::Expected(e) => {
                self.set_param_result_ty(iface, &e.ok, param, result);
                self.set_param_result_ty(iface, &e.err, param, result);
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    self.set_param_result_ty(iface, &case.ty, param, result)
                }
            }
            TypeDefKind::Future(ty) => self.set_param_result_ty(iface, ty, param, result),
            TypeDefKind::Stream(stream) => {
                self.set_param_result_ty(iface, &stream.element, param, result);
                self.set_param_result_ty(iface, &stream.end, param, result);
            }
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

impl Write for Source {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
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

/// Calls [`write!`] with the passed arguments and unwraps the result.
///
/// Useful for writing to things with infallible `Write` implementations like
/// `Source` and `String`.
///
/// [`write!`]: std::write
#[macro_export]
macro_rules! uwrite {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).unwrap()
    };
}

/// Calls [`writeln!`] with the passed arguments and unwraps the result.
///
/// Useful for writing to things with infallible `Write` implementations like
/// `Source` and `String`.
///
/// [`writeln!`]: std::writeln
#[macro_export]
macro_rules! uwriteln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
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
