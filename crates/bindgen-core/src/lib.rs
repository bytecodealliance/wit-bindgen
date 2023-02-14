use std::collections::{btree_map::Entry, BTreeMap, HashMap};
use std::fmt::{self, Write};
use std::ops::Deref;
use wit_parser::*;

pub use wit_parser;
mod ns;

pub use ns::Ns;

#[derive(Default)]
pub struct Types {
    type_info: HashMap<TypeId, TypeInfo>,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct TypeInfo {
    /// Whether or not this type is ever used (transitively) within the
    /// parameter of a function.
    pub param: bool,

    /// Whether or not this type is ever used (transitively) within the
    /// result of a function.
    pub result: bool,

    /// Whether or not this type is ever used (transitively) within the
    /// error case in the result of a function.
    pub error: bool,

    /// Whether or not this type (transitively) has a list.
    pub has_list: bool,
}

impl std::ops::BitOrAssign for TypeInfo {
    fn bitor_assign(&mut self, rhs: Self) {
        self.param |= rhs.param;
        self.result |= rhs.result;
        self.error |= rhs.error;
        self.has_list |= rhs.has_list;
    }
}

impl Types {
    pub fn analyze(&mut self, resolve: &Resolve) {
        for (t, _) in resolve.types.iter() {
            self.type_id_info(resolve, t);
        }
        for (_, iface) in resolve.interfaces.iter() {
            for (_, f) in iface.functions.iter() {
                self.type_info_func(resolve, f);
            }
        }
        for (_, world) in resolve.worlds.iter() {
            for (_, item) in world.imports.iter().chain(&world.exports) {
                match item {
                    WorldItem::Function(f) => {
                        self.type_info_func(resolve, f);
                    }
                    WorldItem::Interface(_) | WorldItem::Type(_) => {}
                }
            }
        }
    }

    fn type_info_func(&mut self, resolve: &Resolve, f: &Function) {
        for (_, ty) in f.params.iter() {
            self.set_param_result_ty(resolve, ty, true, false, false);
        }
        for ty in f.results.iter_types() {
            self.set_param_result_ty(resolve, ty, false, true, false);
        }
    }

    pub fn get(&self, id: TypeId) -> TypeInfo {
        self.type_info[&id]
    }

    pub fn type_id_info(&mut self, resolve: &Resolve, ty: TypeId) -> TypeInfo {
        if let Some(info) = self.type_info.get(&ty) {
            return *info;
        }
        let mut info = TypeInfo::default();
        match &resolve.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    info |= self.type_info(resolve, &field.ty);
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    info |= self.type_info(resolve, ty);
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    info |= self.optional_type_info(resolve, case.ty.as_ref());
                }
            }
            TypeDefKind::List(ty) => {
                info = self.type_info(resolve, ty);
                info.has_list = true;
            }
            TypeDefKind::Type(ty) => {
                info = self.type_info(resolve, ty);
            }
            TypeDefKind::Option(ty) => {
                info = self.type_info(resolve, ty);
            }
            TypeDefKind::Result(r) => {
                info = self.optional_type_info(resolve, r.ok.as_ref());
                info |= self.optional_type_info(resolve, r.err.as_ref());
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    info |= self.type_info(resolve, &case.ty);
                }
            }
            TypeDefKind::Future(ty) => {
                info = self.optional_type_info(resolve, ty.as_ref());
            }
            TypeDefKind::Stream(stream) => {
                info = self.optional_type_info(resolve, stream.element.as_ref());
                info |= self.optional_type_info(resolve, stream.end.as_ref());
            }
            TypeDefKind::Unknown => unreachable!(),
        }
        self.type_info.insert(ty, info);
        info
    }

    pub fn type_info(&mut self, resolve: &Resolve, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::String => info.has_list = true,
            Type::Id(id) => return self.type_id_info(resolve, *id),
            _ => {}
        }
        info
    }

    fn optional_type_info(&mut self, resolve: &Resolve, ty: Option<&Type>) -> TypeInfo {
        match ty {
            Some(ty) => self.type_info(resolve, ty),
            None => TypeInfo::default(),
        }
    }

    fn set_param_result_id(
        &mut self,
        resolve: &Resolve,
        ty: TypeId,
        param: bool,
        result: bool,
        error: bool,
    ) {
        match &resolve.types[ty].kind {
            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    self.set_param_result_ty(resolve, &field.ty, param, result, error)
                }
            }
            TypeDefKind::Tuple(t) => {
                for ty in t.types.iter() {
                    self.set_param_result_ty(resolve, ty, param, result, error)
                }
            }
            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    self.set_param_result_optional_ty(
                        resolve,
                        case.ty.as_ref(),
                        param,
                        result,
                        error,
                    )
                }
            }
            TypeDefKind::List(ty) | TypeDefKind::Type(ty) | TypeDefKind::Option(ty) => {
                self.set_param_result_ty(resolve, ty, param, result, error)
            }
            TypeDefKind::Result(r) => {
                self.set_param_result_optional_ty(resolve, r.ok.as_ref(), param, result, error);
                self.set_param_result_optional_ty(resolve, r.err.as_ref(), param, result, result);
            }
            TypeDefKind::Union(u) => {
                for case in u.cases.iter() {
                    self.set_param_result_ty(resolve, &case.ty, param, result, error)
                }
            }
            TypeDefKind::Future(ty) => {
                self.set_param_result_optional_ty(resolve, ty.as_ref(), param, result, error)
            }
            TypeDefKind::Stream(stream) => {
                self.set_param_result_optional_ty(
                    resolve,
                    stream.element.as_ref(),
                    param,
                    result,
                    error,
                );
                self.set_param_result_optional_ty(
                    resolve,
                    stream.end.as_ref(),
                    param,
                    result,
                    error,
                );
            }
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn set_param_result_ty(
        &mut self,
        resolve: &Resolve,
        ty: &Type,
        param: bool,
        result: bool,
        error: bool,
    ) {
        match ty {
            Type::Id(id) => {
                self.type_id_info(resolve, *id);
                let info = self.type_info.get_mut(id).unwrap();
                if (param && !info.param) || (result && !info.result) || (error && !info.error) {
                    info.param = info.param || param;
                    info.result = info.result || result;
                    info.error = info.error || error;
                    self.set_param_result_id(resolve, *id, param, result, error);
                }
            }
            _ => {}
        }
    }

    fn set_param_result_optional_ty(
        &mut self,
        resolve: &Resolve,
        ty: Option<&Type>,
        param: bool,
        result: bool,
        error: bool,
    ) {
        match ty {
            Some(ty) => self.set_param_result_ty(resolve, ty, param, result, error),
            None => (),
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

    pub fn get_size(&mut self, name: &str) -> Option<usize> {
        match self.files.get(name) {
            Some(data) => Some(data.len()),
            None => None,
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Vec<u8>> {
        return self.files.remove(name);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, &'_ [u8])> {
        self.files.iter().map(|p| (p.0.as_str(), p.1.as_slice()))
    }
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
            if trimmed.starts_with('}') && self.s.ends_with("  ") {
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
                // Note that a `saturating_sub` is used here to prevent a panic
                // here in the case of invalid code being generated in debug
                // mode. It's typically easier to debug those issues through
                // looking at the source code rather than getting a panic.
                self.indent = self.indent.saturating_sub(1);
            }
            if i != lines.len() - 1 || src.ends_with('\n') {
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
        self.s.push('\n');
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
    use super::Source;

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
}

pub trait WorldGenerator {
    fn generate(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        let world = &resolve.worlds[id];
        self.preprocess(resolve, id);

        let mut funcs = Vec::new();
        let mut types = Vec::new();
        for (name, import) in world.imports.iter() {
            match import {
                WorldItem::Function(f) => funcs.push((name.as_str(), f)),
                WorldItem::Interface(id) => self.import_interface(resolve, name, *id, files),
                WorldItem::Type(id) => types.push((name.as_str(), *id)),
            }
        }
        if !types.is_empty() {
            self.export_types(resolve, id, &types, files);
        }
        if !funcs.is_empty() {
            self.import_funcs(resolve, id, &funcs, files);
        }
        funcs.clear();

        for (name, export) in world.exports.iter() {
            match export {
                WorldItem::Function(f) => funcs.push((name.as_str(), f)),
                WorldItem::Interface(id) => self.export_interface(resolve, name, *id, files),
                WorldItem::Type(_) => unreachable!(),
            }
        }
        if !funcs.is_empty() {
            self.export_funcs(resolve, id, &funcs, files);
        }
        self.finish(resolve, id, files);
    }

    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        drop(resolve);
        drop(world);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &str,
        iface: InterfaceId,
        files: &mut Files,
    );
    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &str,
        iface: InterfaceId,
        files: &mut Files,
    );
    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        files: &mut Files,
    );
    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        files: &mut Files,
    );
    fn export_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        files: &mut Files,
    );
    fn finish(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files);
}

/// This is a possible replacement for the `Generator` trait above, currently
/// only used by the JS bindings for generating bindings for a component.
///
/// The current plan is to see how things shake out with worlds and various
/// other generators to see if everything can be updated to a less
/// per-`*.wit`-file centric interface in the future. Even this will probably
/// change for JS though. In any case it's something that was useful for JS and
/// is suitable to replace otherwise at any time.
pub trait InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve;

    fn type_record(&mut self, id: TypeId, name: &str, record: &Record, docs: &Docs);
    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs);
    fn type_tuple(&mut self, id: TypeId, name: &str, flags: &Tuple, docs: &Docs);
    fn type_variant(&mut self, id: TypeId, name: &str, variant: &Variant, docs: &Docs);
    fn type_option(&mut self, id: TypeId, name: &str, payload: &Type, docs: &Docs);
    fn type_result(&mut self, id: TypeId, name: &str, result: &Result_, docs: &Docs);
    fn type_union(&mut self, id: TypeId, name: &str, union: &Union, docs: &Docs);
    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs);
    fn type_alias(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_list(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs);
    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs);

    fn types(&mut self, iface: InterfaceId) {
        let iface = &self.resolve().interfaces[iface];
        for (name, id) in iface.types.iter() {
            self.define_type(name, *id);
        }
    }

    fn define_type(&mut self, name: &str, id: TypeId) {
        let ty = &self.resolve().types[id];
        match &ty.kind {
            TypeDefKind::Record(record) => self.type_record(id, name, record, &ty.docs),
            TypeDefKind::Flags(flags) => self.type_flags(id, name, flags, &ty.docs),
            TypeDefKind::Tuple(tuple) => self.type_tuple(id, name, tuple, &ty.docs),
            TypeDefKind::Enum(enum_) => self.type_enum(id, name, enum_, &ty.docs),
            TypeDefKind::Variant(variant) => self.type_variant(id, name, variant, &ty.docs),
            TypeDefKind::Option(t) => self.type_option(id, name, t, &ty.docs),
            TypeDefKind::Result(r) => self.type_result(id, name, r, &ty.docs),
            TypeDefKind::Union(u) => self.type_union(id, name, u, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(_) => todo!("generate for future"),
            TypeDefKind::Stream(_) => todo!("generate for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }
}
