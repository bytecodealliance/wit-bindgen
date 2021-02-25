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
