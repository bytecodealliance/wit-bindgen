use std::collections::{HashMap, HashSet, BTreeMap, btree_map::Entry};
use witx::*;

pub use witx;

pub trait Generator {
    fn preprocess(&mut self, module: &Module, import: bool) {
        drop((module, import));
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str);
    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str);
    fn type_handle(&mut self, name: &Id, ty: &HandleDatatype, docs: &str);
    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str);
    fn type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str);
    fn type_pointer(&mut self, name: &Id, const_: bool, ty: &TypeRef, docs: &str);
    fn type_builtin(&mut self, name: &Id, ty: BuiltinType, docs: &str);
    fn type_buffer(&mut self, name: &Id, ty: &Buffer, docs: &str);
    fn const_(&mut self, name: &Id, ty: &Id, val: u64, docs: &str);
    fn import(&mut self, module: &Id, func: &Function);
    fn export(&mut self, module: &Id, func: &Function);
    fn finish(&mut self, files: &mut Files);

    fn generate(&mut self, module: &Module, import: bool, files: &mut Files) {
        self.preprocess(module, import);
        for ty in module.typenames() {
            let t = match &ty.tref {
                TypeRef::Name(nt) => {
                    self.type_alias(&ty.name, &nt, &ty.docs);
                    continue;
                }
                TypeRef::Value(t) => t,
            };
            match &**t {
                Type::Record(t) => self.type_record(&ty.name, &t, &ty.docs),
                Type::Variant(t) => self.type_variant(&ty.name, &t, &ty.docs),
                Type::Handle(t) => self.type_handle(&ty.name, &t, &ty.docs),
                Type::List(t) => self.type_list(&ty.name, &t, &ty.docs),
                Type::Pointer(t) => self.type_pointer(&ty.name, false, &t, &ty.docs),
                Type::ConstPointer(t) => self.type_pointer(&ty.name, true, &t, &ty.docs),
                Type::Builtin(t) => self.type_builtin(&ty.name, *t, &ty.docs),
                Type::Buffer(b) => self.type_buffer(&ty.name, &b, &ty.docs),
            }
        }

        for c in module.constants() {
            self.const_(&c.name, &c.ty, c.value, &c.docs);
        }

        for f in module.funcs() {
            if import {
                self.import(&module.name(), &f);
            } else {
                self.export(&module.name(), &f);
            }
        }

        self.finish(files)
    }
}

#[derive(Default)]
pub struct Types {
    type_info: HashMap<Id, TypeInfo>,
    handle_dtors: HashSet<Id>,
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

    /// Whether or not this type (transitively) has an out buffer.
    pub has_out_buffer: bool,

    /// Whether or not this type (transitively) has an in buffer.
    pub has_in_buffer: bool,

    /// Whether or not this type is a handle and has a destructor.
    pub handle_with_dtor: bool,
}

impl std::ops::BitOrAssign for TypeInfo {
    fn bitor_assign(&mut self, rhs: Self) {
        self.param |= rhs.param;
        self.result |= rhs.result;
        self.has_list |= rhs.has_list;
        self.has_handle |= rhs.has_handle;
        self.has_out_buffer |= rhs.has_out_buffer;
        self.has_in_buffer |= rhs.has_in_buffer;
    }
}

impl Types {
    pub fn analyze(&mut self, module: &Module) {
        for t in module.typenames() {
            let info = self.type_ref_info(&t.tref);
            self.type_info.insert(t.name.clone(), info);
        }
        for f in module.funcs() {
            for param in f.params.iter() {
                self.set_param_result_tref(&param.tref, true, false);
            }
            for param in f.results.iter() {
                self.set_param_result_tref(&param.tref, false, true);
            }
            self.maybe_set_dtor(module, &f);
        }
    }

    fn maybe_set_dtor(&mut self, module: &Module, f: &Function) {
        // Dtors only happen when the function has a singular parameter
        if f.params.len() != 1 {
            return;
        }
        let param_ty_name = match &f.params[0].tref {
            TypeRef::Name(n) => &n.name,
            _ => return,
        };

        // Dtors are inferred to be `${type}_close` right now, but we should
        // probably use some sort of configuration/witx attribute for this in
        // the future.
        let name = f.name.as_str();
        let prefix = match name.strip_suffix("_close") {
            Some(prefix) => prefix,
            None => return,
        };

        // The singular parameter type name must be the prefix of this
        // function's own name.
        if param_ty_name.as_str() != prefix {
            return;
        }

        // ... and finally the actual type of this value must be a `(handle)`
        let id = Id::new(prefix);
        let ty = match module.typename(&id) {
            Some(ty) => ty,
            None => return,
        };
        match &**ty.type_() {
            Type::Handle(_) => {}
            _ => return,
        }

        // ... and if we got this far then `id` is a handle type which has a
        // destructor in this module.
        self.type_info.get_mut(&id).unwrap().handle_with_dtor = true;
        self.handle_dtors.insert(f.name.clone());
    }

    pub fn get(&self, id: &Id) -> TypeInfo {
        self.type_info[id]
    }

    pub fn is_dtor_func(&self, func: &Id) -> bool {
        self.handle_dtors.contains(func)
    }

    pub fn type_ref_info(&mut self, ty: &TypeRef) -> TypeInfo {
        match ty {
            TypeRef::Name(nt) => match self.type_info.get(&nt.name) {
                Some(info) => *info,
                None => self.type_ref_info(&nt.tref),
            },
            TypeRef::Value(t) => self.type_info(t),
        }
    }

    pub fn type_info(&mut self, ty: &Type) -> TypeInfo {
        let mut info = TypeInfo::default();
        match ty {
            Type::Builtin(_) => {}
            Type::Handle(_) => info.has_handle = true,
            Type::List(t) => {
                info |= self.type_ref_info(t);
                info.has_list = true;
            }
            Type::Pointer(t) | Type::ConstPointer(t) => info = self.type_ref_info(t),
            Type::Variant(v) => {
                for c in v.cases.iter() {
                    if let Some(ty) = &c.tref {
                        info |= self.type_ref_info(ty);
                    }
                }
            }
            Type::Record(r) => {
                for member in r.members.iter() {
                    info |= self.type_ref_info(&member.tref);
                }
            }
            Type::Buffer(t) => {
                info |= self.type_ref_info(&t.tref);
                if t.out {
                    info.has_out_buffer = true;
                } else {
                    info.has_in_buffer = true;
                }
            }
        }
        info
    }

    fn set_param_result_tref(&mut self, ty: &TypeRef, param: bool, result: bool) {
        match ty {
            TypeRef::Name(nt) => {
                let info = self.type_info.get_mut(&nt.name).unwrap();
                if (param && !info.param) || (result && !info.result) {
                    info.param = info.param || param;
                    info.result = info.result || result;
                    self.set_param_result_tref(&nt.tref, param, result);
                }
            }
            TypeRef::Value(t) => self.set_param_result_ty(t, param, result),
        }
    }

    fn set_param_result_ty(&mut self, ty: &Type, param: bool, result: bool) {
        match ty {
            Type::Builtin(_) => {}
            Type::Handle(_) => {}
            Type::List(t) | Type::Pointer(t) | Type::ConstPointer(t) => {
                self.set_param_result_tref(t, param, result)
            }
            Type::Variant(v) => {
                for c in v.cases.iter() {
                    if let Some(ty) = &c.tref {
                        self.set_param_result_tref(ty, param, result)
                    }
                }
            }
            Type::Record(r) => {
                for member in r.members.iter() {
                    self.set_param_result_tref(&member.tref, param, result)
                }
            }
            Type::Buffer(b) => self.set_param_result_tref(&b.tref, param, result),
        }
    }
}

#[derive(Default)]
pub struct Files {
    files: BTreeMap<String, String>,
}

impl Files {
    pub fn push(&mut self, name: &str, contents: &str) {
        match self.files.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(contents.to_owned());
            }
            Entry::Occupied(ref mut entry) => {
                entry.get_mut().push_str(contents);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, &'_ str)> {
        self.files.iter().map(|p| (p.0.as_str(), p.1.as_str()))
    }
}
