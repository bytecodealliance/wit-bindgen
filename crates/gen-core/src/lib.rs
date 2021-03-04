use std::collections::HashMap;
use witx::*;

pub use witx;

pub trait Generator {
    fn preprocess(&mut self, doc: &Document) {
        drop(doc);
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str);
    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str);
    fn type_handle(&mut self, name: &Id, ty: &HandleDatatype, docs: &str);
    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str);
    fn type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str);
    fn type_pointer(&mut self, name: &Id, const_: bool, ty: &TypeRef, docs: &str);
    fn type_builtin(&mut self, name: &Id, ty: BuiltinType, docs: &str);
    fn const_(&mut self, name: &Id, ty: &Id, val: u64, docs: &str);
    fn import(&mut self, module: &Id, func: &InterfaceFunc);
    fn export(&mut self, module: &Id, func: &InterfaceFunc);
    fn finish(&mut self) -> Files;

    fn generate(&mut self, doc: &Document, import: bool) -> Files {
        self.preprocess(doc);
        for ty in doc.typenames() {
            let t = match &ty.tref {
                TypeRef::Name(nt) => {
                    self.type_alias(&ty.name, nt, &ty.docs);
                    continue;
                }
                TypeRef::Value(t) => t,
            };
            match &**t {
                Type::Record(t) => self.type_record(&ty.name, t, &ty.docs),
                Type::Variant(t) => self.type_variant(&ty.name, t, &ty.docs),
                Type::Handle(t) => self.type_handle(&ty.name, t, &ty.docs),
                Type::List(t) => self.type_list(&ty.name, t, &ty.docs),
                Type::Pointer(t) => self.type_pointer(&ty.name, false, t, &ty.docs),
                Type::ConstPointer(t) => self.type_pointer(&ty.name, true, t, &ty.docs),
                Type::Builtin(t) => self.type_builtin(&ty.name, *t, &ty.docs),
            }
        }

        for c in doc.constants() {
            self.const_(&c.name, &c.ty, c.value, &c.docs);
        }

        for m in doc.modules() {
            for f in m.funcs() {
                if import {
                    self.import(&m.name, &f);
                } else {
                    self.export(&m.name, &f);
                }
            }
        }

        self.finish()
    }
}

#[derive(Default)]
pub struct Types {
    type_info: HashMap<Id, TypeInfo>,
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

impl Types {
    pub fn analyze(&mut self, doc: &Document) {
        for t in doc.typenames() {
            self.type_info.insert(t.name.clone(), TypeInfo::default());
        }
        for t in doc.typenames() {
            self.register_type_info(&TypeRef::Name(t.clone()), false, false);
        }
        for m in doc.modules() {
            for f in m.funcs() {
                for param in f.params.iter() {
                    self.register_type_info(&param.tref, true, false);
                }
                for param in f.results.iter() {
                    self.register_type_info(&param.tref, false, true);
                }
            }
        }
    }

    pub fn get(&self, id: &Id) -> TypeInfo {
        self.type_info[id]
    }

    fn register_type_info(&mut self, ty: &TypeRef, param: bool, result: bool) -> (bool, bool) {
        let ty = match ty {
            TypeRef::Name(nt) => {
                let (list, handle) = self.register_type_info(&nt.tref, param, result);
                let info = self.type_info.get_mut(&nt.name).unwrap();
                info.param = info.param || param;
                info.result = info.result || result;
                info.has_list = info.has_list || list;
                info.has_handle = info.has_handle || handle;
                return (list, handle);
            }
            TypeRef::Value(t) => &**t,
        };
        match ty {
            Type::Handle(_) | Type::Builtin(_) => (false, false),
            Type::List(t) => {
                let (_list, handle) = self.register_type_info(t, param, result);
                (true, handle)
            }
            Type::Pointer(t) | Type::ConstPointer(t) => self.register_type_info(t, param, result),
            Type::Variant(v) => {
                let mut list = false;
                let mut handle = false;
                for c in v.cases.iter() {
                    if let Some(ty) = &c.tref {
                        let pair = self.register_type_info(ty, param, result);
                        list = list || pair.0;
                        handle = handle || pair.1;
                    }
                }
                (list, handle)
            }
            Type::Record(r) => {
                let mut list = false;
                let mut handle = false;
                for member in r.members.iter() {
                    let pair = self.register_type_info(&member.tref, param, result);
                    list = list || pair.0;
                    handle = handle || pair.1;
                }
                (list, handle)
            }
        }
    }
}

#[derive(Default)]
pub struct Files {
    files: Vec<(String, String)>,
}

impl Files {
    pub fn push(&mut self, name: &str, contents: &str) {
        self.files.push((name.to_string(), contents.to_string()));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, &'_ str)> {
        self.files.iter().map(|p| (p.0.as_str(), p.1.as_str()))
    }
}
