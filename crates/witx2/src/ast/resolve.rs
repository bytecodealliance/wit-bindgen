use super::{Error, Item, Span, Value, ValueKind};
use crate::*;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::mem;

#[derive(Default)]
pub struct Resolver {
    type_lookup: HashMap<String, TypeId>,
    types: Arena<TypeDef>,
    resource_lookup: HashMap<String, ResourceId>,
    resources_copied: HashMap<(String, ResourceId), ResourceId>,
    types_copied: HashMap<(String, TypeId), TypeId>,
    resources: Arena<Resource>,
    anon_types: HashMap<Key, TypeId>,
    functions: Vec<Function>,
    globals: Vec<Global>,
}

#[derive(PartialEq, Eq, Hash)]
enum Key {
    Variant(Vec<(String, Option<Type>)>),
    Record(Vec<(String, Type)>),
    List(Type),
    PushBuffer(Type),
    PullBuffer(Type),
    Pointer(Type),
    ConstPointer(Type),
}

impl Resolver {
    pub(super) fn resolve(
        &mut self,
        name: &str,
        fields: &[Item<'_>],
        deps: &HashMap<String, Interface>,
    ) -> Result<Interface> {
        // First pull in any names from our dependencies
        self.process_use(fields, deps)?;
        // ... then register our own names
        self.register_names(fields)?;

        // With all names registered we can now fully expand and translate all
        // types.
        for field in fields {
            let t = match field {
                Item::TypeDef(t) => t,
                _ => continue,
            };
            let id = self.type_lookup[&*t.name.name];
            let kind = self.resolve_type_def(&t.ty)?;
            self.types.get_mut(id).unwrap().kind = kind;
        }

        // And finally we can resolve all type references in functions/globals
        // and additionally validate that types thesmelves are not recursive
        let mut valid_types = HashSet::new();
        let mut visiting = HashSet::new();
        for field in fields {
            match field {
                Item::Value(v) => self.resolve_value(v)?,
                Item::Resource(r) => self.resolve_resource(r)?,
                Item::TypeDef(t) => {
                    self.validate_type_not_recursive(
                        t.name.span,
                        self.type_lookup[&*t.name.name],
                        &mut visiting,
                        &mut valid_types,
                    )?;
                }
                _ => continue,
            }
        }

        Ok(Interface {
            name: name.to_string(),
            types: mem::take(&mut self.types),
            type_lookup: mem::take(&mut self.type_lookup),
            resources: mem::take(&mut self.resources),
            resource_lookup: mem::take(&mut self.resource_lookup),
            interface_lookup: Default::default(),
            interfaces: Default::default(),
            functions: mem::take(&mut self.functions),
            globals: mem::take(&mut self.globals),
        })
    }

    fn process_use<'a>(
        &mut self,
        fields: &[Item<'a>],
        deps: &'a HashMap<String, Interface>,
    ) -> Result<()> {
        for field in fields {
            let u = match field {
                Item::Use(u) => u,
                _ => continue,
            };
            let mut dep = &deps[&*u.from[0].name];
            let mut prev = &*u.from[0].name;
            for name in u.from[1..].iter() {
                dep = match dep.interface_lookup.get(&*name.name) {
                    Some(i) => &dep.interfaces[*i],
                    None => {
                        return Err(Error {
                            span: name.span,
                            msg: format!("`{}` not defined in `{}`", name.name, prev),
                        }
                        .into())
                    }
                };
                prev = &*name.name;
            }

            let mod_name = &u.from[0];

            match &u.names {
                Some(names) => {
                    for name in names {
                        let (my_name, span) = match &name.as_ {
                            Some(id) => (&id.name, id.span),
                            None => (&name.name.name, name.name.span),
                        };
                        let mut found = false;

                        if let Some(id) = dep.resource_lookup.get(&*name.name.name) {
                            let resource = self.copy_resource(&mod_name.name, dep, *id);
                            self.define_resource(my_name, span, resource)?;
                            found = true;
                        }

                        if let Some(id) = dep.type_lookup.get(&*name.name.name) {
                            let ty = self.copy_type_def(&mod_name.name, dep, *id);
                            self.define_type(my_name, span, ty)?;
                            found = true;
                        }

                        if !found {
                            return Err(Error {
                                span: name.name.span,
                                msg: format!("name not defined in submodule"),
                            }
                            .into());
                        }
                    }
                }
                None => {
                    for (id, resource) in dep.resources.iter() {
                        let id = self.copy_resource(&mod_name.name, dep, id);
                        self.define_resource(&resource.name, mod_name.span, id)?;
                    }
                    let mut names = dep.type_lookup.iter().collect::<Vec<_>>();
                    names.sort(); // produce a stable order by which to add names
                    for (name, id) in names {
                        let ty = self.copy_type_def(&mod_name.name, dep, *id);
                        self.define_type(name, mod_name.span, ty)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn copy_resource(&mut self, dep_name: &str, dep: &Interface, r: ResourceId) -> ResourceId {
        let resources = &mut self.resources;
        *self
            .resources_copied
            .entry((dep_name.to_string(), r))
            .or_insert_with(|| {
                let r = &dep.resources[r];
                let resource = Resource {
                    docs: r.docs.clone(),
                    name: r.name.clone(),
                    foreign_module: Some(r.foreign_module.clone().unwrap_or(dep_name.to_string())),
                };
                resources.alloc(resource)
            })
    }

    fn copy_type_def(&mut self, dep_name: &str, dep: &Interface, dep_id: TypeId) -> TypeId {
        if let Some(id) = self.types_copied.get(&(dep_name.to_string(), dep_id)) {
            return *id;
        }
        let ty = &dep.types[dep_id];

        let ty = TypeDef {
            docs: ty.docs.clone(),
            name: ty.name.clone(),
            foreign_module: Some(ty.foreign_module.clone().unwrap_or(dep_name.to_string())),
            kind: match &ty.kind {
                TypeDefKind::Type(t) => TypeDefKind::Type(self.copy_type(dep_name, dep, *t)),
                TypeDefKind::Record(r) => TypeDefKind::Record(Record {
                    fields: r
                        .fields
                        .iter()
                        .map(|field| Field {
                            docs: field.docs.clone(),
                            name: field.name.clone(),
                            ty: self.copy_type(dep_name, dep, field.ty),
                        })
                        .collect(),
                    kind: r.kind,
                }),
                TypeDefKind::Variant(v) => TypeDefKind::Variant(Variant {
                    cases: v
                        .cases
                        .iter()
                        .map(|case| Case {
                            docs: case.docs.clone(),
                            name: case.name.clone(),
                            ty: case.ty.map(|t| self.copy_type(dep_name, dep, t)),
                        })
                        .collect(),
                    tag: v.tag,
                }),
                TypeDefKind::List(t) => TypeDefKind::List(self.copy_type(dep_name, dep, *t)),
                TypeDefKind::PullBuffer(t) => {
                    TypeDefKind::PullBuffer(self.copy_type(dep_name, dep, *t))
                }
                TypeDefKind::PushBuffer(t) => {
                    TypeDefKind::PushBuffer(self.copy_type(dep_name, dep, *t))
                }
                TypeDefKind::Pointer(t) => TypeDefKind::Pointer(self.copy_type(dep_name, dep, *t)),
                TypeDefKind::ConstPointer(t) => {
                    TypeDefKind::ConstPointer(self.copy_type(dep_name, dep, *t))
                }
            },
        };
        let id = self.types.alloc(ty);
        self.types_copied.insert((dep_name.to_string(), dep_id), id);
        return id;
    }

    fn copy_type(&mut self, dep_name: &str, dep: &Interface, ty: Type) -> Type {
        match ty {
            Type::Id(id) => Type::Id(self.copy_type_def(dep_name, dep, id)),
            Type::Handle(id) => Type::Handle(self.copy_resource(dep_name, dep, id)),
            other => other,
        }
    }

    fn register_names(&mut self, fields: &[Item<'_>]) -> Result<()> {
        let mut values = HashSet::new();
        for field in fields {
            match field {
                Item::Resource(r) => {
                    let docs = self.docs(&r.docs);
                    let id = self.resources.alloc(Resource {
                        docs,
                        name: r.name.name.to_string(),
                        foreign_module: None,
                    });
                    self.define_resource(&r.name.name, r.name.span, id)?;
                    let type_id = self.types.alloc(TypeDef {
                        docs: Docs::default(),
                        kind: TypeDefKind::Type(Type::Handle(id)),
                        name: None,
                        foreign_module: None,
                    });
                    self.define_type(&r.name.name, r.name.span, type_id)?;
                }
                Item::TypeDef(t) => {
                    let docs = self.docs(&t.docs);
                    let id = self.types.alloc(TypeDef {
                        docs,
                        // a dummy kind is used for now which will get filled in
                        // later with the actual desired contents.
                        kind: TypeDefKind::List(Type::U8),
                        name: Some(t.name.name.to_string()),
                        foreign_module: None,
                    });
                    self.define_type(&t.name.name, t.name.span, id)?;
                }
                Item::Value(f) => {
                    if !values.insert(&f.name.name) {
                        return Err(Error {
                            span: f.name.span,
                            msg: format!("{:?} defined twice", f.name.name),
                        }
                        .into());
                    }
                }
                Item::Use(_) => {}

                Item::Interface(_) => unimplemented!(),
            }
        }

        Ok(())
    }

    fn define_resource(&mut self, name: &str, span: Span, id: ResourceId) -> Result<()> {
        if self.resource_lookup.insert(name.to_string(), id).is_some() {
            Err(Error {
                span,
                msg: format!("resource {:?} defined twice", name),
            }
            .into())
        } else {
            Ok(())
        }
    }

    fn define_type(&mut self, name: &str, span: Span, id: TypeId) -> Result<()> {
        if self.type_lookup.insert(name.to_string(), id).is_some() {
            Err(Error {
                span,
                msg: format!("type {:?} defined twice", name),
            }
            .into())
        } else {
            Ok(())
        }
    }

    fn resolve_type_def(&mut self, ty: &super::Type<'_>) -> Result<TypeDefKind> {
        Ok(match ty {
            super::Type::U8 => TypeDefKind::Type(Type::U8),
            super::Type::U16 => TypeDefKind::Type(Type::U16),
            super::Type::U32 => TypeDefKind::Type(Type::U32),
            super::Type::U64 => TypeDefKind::Type(Type::U64),
            super::Type::S8 => TypeDefKind::Type(Type::S8),
            super::Type::S16 => TypeDefKind::Type(Type::S16),
            super::Type::S32 => TypeDefKind::Type(Type::S32),
            super::Type::S64 => TypeDefKind::Type(Type::S64),
            super::Type::F32 => TypeDefKind::Type(Type::F32),
            super::Type::F64 => TypeDefKind::Type(Type::F64),
            super::Type::Char => TypeDefKind::Type(Type::Char),
            super::Type::CChar => TypeDefKind::Type(Type::CChar),
            super::Type::Usize => TypeDefKind::Type(Type::Usize),
            super::Type::Handle(resource) => {
                let id = match self.resource_lookup.get(&*resource.name) {
                    Some(id) => *id,
                    None => {
                        return Err(Error {
                            span: resource.span,
                            msg: format!("no resource named `{}`", resource.name),
                        }
                        .into())
                    }
                };
                TypeDefKind::Type(Type::Handle(id))
            }
            super::Type::Name(name) => {
                let id = match self.type_lookup.get(&*name.name) {
                    Some(id) => *id,
                    None => {
                        return Err(Error {
                            span: name.span,
                            msg: format!("no type named `{}`", name.name),
                        }
                        .into())
                    }
                };
                TypeDefKind::Type(Type::Id(id))
            }
            super::Type::List(list) => {
                let ty = self.resolve_type(list)?;
                TypeDefKind::List(ty)
            }
            super::Type::Pointer(list) => {
                let ty = self.resolve_type(list)?;
                TypeDefKind::Pointer(ty)
            }
            super::Type::ConstPointer(list) => {
                let ty = self.resolve_type(list)?;
                TypeDefKind::ConstPointer(ty)
            }
            super::Type::PushBuffer(ty) => {
                let ty = self.resolve_type(ty)?;
                TypeDefKind::PushBuffer(ty)
            }
            super::Type::PullBuffer(ty) => {
                let ty = self.resolve_type(ty)?;
                TypeDefKind::PullBuffer(ty)
            }
            super::Type::Record(record) => {
                let fields = record
                    .fields
                    .iter()
                    .map(|field| {
                        Ok(Field {
                            docs: self.docs(&field.docs),
                            name: field.name.name.to_string(),
                            ty: self.resolve_type(&field.ty)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                TypeDefKind::Record(Record {
                    kind: if record.tuple_hint {
                        RecordKind::Tuple
                    } else if let Some(hint) = &record.flags_repr {
                        RecordKind::Flags(Some(match &**hint {
                            super::Type::U8 => Int::U8,
                            super::Type::U16 => Int::U16,
                            super::Type::U32 => Int::U32,
                            super::Type::U64 => Int::U64,
                            _ => panic!("unknown explicit flags repr"),
                        }))
                    } else {
                        RecordKind::infer(&self.types, &fields)
                    },
                    fields,
                })
            }
            super::Type::Variant(variant) => {
                if variant.cases.is_empty() {
                    return Err(Error {
                        span: variant.span,
                        msg: format!("empty variant"),
                    }
                    .into());
                }
                let cases = variant
                    .cases
                    .iter()
                    .map(|case| {
                        Ok(Case {
                            docs: self.docs(&case.docs),
                            name: case.name.name.to_string(),
                            ty: match &case.ty {
                                Some(ty) => Some(self.resolve_type(ty)?),
                                None => None,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                TypeDefKind::Variant(Variant {
                    tag: match &variant.tag {
                        Some(ty) => self.get_variant_tag(ty),
                        None => Variant::infer_tag(cases.len()),
                    },
                    cases,
                })
            }
        })
    }

    fn get_variant_tag(&self, tag: &super::Type) -> Int {
        match tag {
            super::Type::U8 => Int::U8,
            super::Type::U16 => Int::U16,
            super::Type::U32 => Int::U32,
            super::Type::U64 => Int::U64,
            super::Type::Name(name) => {
                let ty = self.type_lookup[&*name.name];
                self.get_variant_tag_id(ty)
            }
            _ => panic!("unknown explicit variant tag"),
        }
    }

    fn get_variant_tag_id(&self, ty: TypeId) -> Int {
        match &self.types[ty].kind {
            TypeDefKind::Type(Type::U8) => Int::U8,
            TypeDefKind::Type(Type::U16) => Int::U16,
            TypeDefKind::Type(Type::U32) => Int::U32,
            TypeDefKind::Type(Type::U64) => Int::U64,
            TypeDefKind::Type(Type::Id(id)) => self.get_variant_tag_id(*id),
            TypeDefKind::Variant(v) => v.tag,
            _ => panic!("unknown typedef"),
        }
    }

    fn resolve_type(&mut self, ty: &super::Type<'_>) -> Result<Type> {
        let kind = self.resolve_type_def(ty)?;
        Ok(self.anon_type_def(TypeDef {
            kind,
            name: None,
            docs: Docs::default(),
            foreign_module: None,
        }))
    }

    fn anon_type_def(&mut self, ty: TypeDef) -> Type {
        let key = match &ty.kind {
            TypeDefKind::Type(t) => return *t,
            TypeDefKind::Variant(v) => Key::Variant(
                v.cases
                    .iter()
                    .map(|case| (case.name.clone(), case.ty.clone()))
                    .collect::<Vec<_>>(),
            ),
            TypeDefKind::Record(r) => Key::Record(
                r.fields
                    .iter()
                    .map(|case| (case.name.clone(), case.ty.clone()))
                    .collect::<Vec<_>>(),
            ),
            TypeDefKind::List(ty) => Key::List(*ty),
            TypeDefKind::Pointer(ty) => Key::Pointer(*ty),
            TypeDefKind::ConstPointer(ty) => Key::ConstPointer(*ty),
            TypeDefKind::PushBuffer(ty) => Key::PushBuffer(*ty),
            TypeDefKind::PullBuffer(ty) => Key::PullBuffer(*ty),
        };
        let types = &mut self.types;
        let id = self
            .anon_types
            .entry(key)
            .or_insert_with(|| types.alloc(ty));
        Type::Id(*id)
    }

    fn docs(&mut self, doc: &super::Docs<'_>) -> Docs {
        if doc.docs.is_empty() {
            return Docs { contents: None };
        }
        let mut docs = String::new();
        for doc in doc.docs.iter() {
            if doc.starts_with("//") {
                docs.push_str(&doc[2..].trim_start_matches('/').trim());
                docs.push_str("\n");
            } else {
                assert!(doc.starts_with("/*"));
                assert!(doc.ends_with("*/"));
                for line in doc[2..doc.len() - 2].lines() {
                    docs.push_str(line);
                    docs.push_str("\n");
                }
            }
        }
        Docs {
            contents: Some(docs),
        }
    }

    fn resolve_value(&mut self, value: &Value<'_>) -> Result<()> {
        let docs = self.docs(&value.docs);
        match &value.kind {
            ValueKind::Function {
                abi,
                params,
                results,
            } => {
                let params = params
                    .iter()
                    .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(&ty)?)))
                    .collect::<Result<_>>()?;
                let results = results
                    .iter()
                    .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(&ty)?)))
                    .collect::<Result<_>>()?;
                self.functions.push(Function {
                    abi: *abi,
                    docs,
                    name: value.name.name.to_string(),
                    kind: FunctionKind::Freestanding,
                    params,
                    results,
                });
            }
            ValueKind::Global(ty) => {
                let ty = self.resolve_type(ty)?;
                self.globals.push(Global {
                    docs,
                    name: value.name.name.to_string(),
                    ty,
                });
            }
        }
        Ok(())
    }

    fn resolve_resource(&mut self, resource: &super::Resource<'_>) -> Result<()> {
        let mut names = HashSet::new();
        let id = self.resource_lookup[&*resource.name.name];
        for (statik, value) in resource.values.iter() {
            let (abi, params, results) = match &value.kind {
                ValueKind::Function {
                    abi,
                    params,
                    results,
                } => (*abi, params, results),
                ValueKind::Global(_) => {
                    return Err(Error {
                        span: value.name.span,
                        msg: format!("globals not allowed in resources"),
                    }
                    .into());
                }
            };
            if !names.insert(&value.name.name) {
                return Err(Error {
                    span: value.name.span,
                    msg: format!("{:?} defined twice in this resource", value.name.name),
                }
                .into());
            }
            let docs = self.docs(&value.docs);
            let mut params = params
                .iter()
                .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(&ty)?)))
                .collect::<Result<Vec<_>>>()?;
            let results = results
                .iter()
                .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(&ty)?)))
                .collect::<Result<_>>()?;
            let kind = if *statik {
                FunctionKind::Static(id)
            } else {
                params.insert(0, ("self".to_string(), Type::Handle(id)));
                FunctionKind::Method(id)
            };
            self.functions.push(Function {
                abi,
                docs,
                name: format!("{}::{}", resource.name.name, value.name.name.to_string()),
                kind,
                params,
                results,
            });
        }
        Ok(())
    }

    fn validate_type_not_recursive(
        &self,
        span: Span,
        ty: TypeId,
        visiting: &mut HashSet<TypeId>,
        valid: &mut HashSet<TypeId>,
    ) -> Result<()> {
        if valid.contains(&ty) {
            return Ok(());
        }
        if !visiting.insert(ty) {
            return Err(Error {
                span,
                msg: format!("type can recursively refer to itself"),
            }
            .into());
        }

        match &self.types[ty].kind {
            TypeDefKind::List(Type::Id(id))
            | TypeDefKind::Pointer(Type::Id(id))
            | TypeDefKind::ConstPointer(Type::Id(id))
            | TypeDefKind::PushBuffer(Type::Id(id))
            | TypeDefKind::PullBuffer(Type::Id(id))
            | TypeDefKind::Type(Type::Id(id)) => {
                self.validate_type_not_recursive(span, *id, visiting, valid)?
            }
            TypeDefKind::Variant(v) => {
                for case in v.cases.iter() {
                    if let Some(Type::Id(id)) = case.ty {
                        self.validate_type_not_recursive(span, id, visiting, valid)?;
                    }
                }
            }
            TypeDefKind::Record(r) => {
                for case in r.fields.iter() {
                    if let Type::Id(id) = case.ty {
                        self.validate_type_not_recursive(span, id, visiting, valid)?;
                    }
                }
            }

            TypeDefKind::List(_)
            | TypeDefKind::Pointer(_)
            | TypeDefKind::ConstPointer(_)
            | TypeDefKind::PushBuffer(_)
            | TypeDefKind::PullBuffer(_)
            | TypeDefKind::Type(_) => {}
        }

        valid.insert(ty);
        visiting.remove(&ty);
        Ok(())
    }
}
