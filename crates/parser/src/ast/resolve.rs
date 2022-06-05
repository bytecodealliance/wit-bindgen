use super::{Error, Item, Span, Value, ValueKind};
use crate::*;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::mem;

#[derive(Default)]
pub struct Resolver {
    // Note: this should only ever point to named types, since an anonymous type can't have a name to be looked up with.
    type_lookup: HashMap<String, TypeId>,
    types: Arena<CustomType>,
    resource_lookup: HashMap<String, ResourceId>,
    resources_copied: HashMap<(String, ResourceId), ResourceId>,
    types_copied: HashMap<(String, TypeId), TypeId>,
    resources: Arena<Resource>,
    anon_types: HashMap<AnonymousType, TypeId>,
    functions: Vec<Function>,
    globals: Vec<Global>,
}

impl Resolver {
    pub(super) fn resolve(
        &mut self,
        name: &str,
        items: &[Item<'_>],
        deps: &HashMap<String, Interface>,
    ) -> Result<Interface> {
        // First pull in any names from our dependencies
        self.process_use(items, deps)?;
        // ... then register our own names
        self.register_names(items)?;

        // With all names registered we can now fully expand and translate all
        // types.
        for field in items {
            let t = match field {
                Item::TypeDef(t) => t,
                _ => continue,
            };
            let id = self.type_lookup[&*t.name.name];
            let kind = self.resolve_named_type(&t.kind)?;
            match self.types.get_mut(id).unwrap() {
                CustomType::Named(ty) => ty.kind = kind,
                // An anonymous type can't have a name.
                CustomType::Anonymous(_) => unreachable!(),
            }
        }

        // And finally we can resolve all type references in functions/globals
        // and additionally validate that types thesmelves are not recursive
        let mut valid_types = HashSet::new();
        let mut visiting = HashSet::new();
        for field in items {
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
                            let ty = self.copy_custom_type(&mod_name.name, dep, *id);
                            self.define_type(my_name, span, ty)?;
                            found = true;
                        }

                        if !found {
                            return Err(Error {
                                span: name.name.span,
                                msg: "name not defined in submodule".to_string(),
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
                        let ty = self.copy_custom_type(&mod_name.name, dep, *id);
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
                    foreign_module: Some(
                        r.foreign_module
                            .clone()
                            .unwrap_or_else(|| dep_name.to_string()),
                    ),
                };
                resources.alloc(resource)
            })
    }

    fn copy_custom_type(&mut self, dep_name: &str, dep: &Interface, dep_id: TypeId) -> TypeId {
        if let Some(id) = self.types_copied.get(&(dep_name.to_string(), dep_id)) {
            return *id;
        }
        let ty = &dep.types[dep_id];

        let ty = match ty {
            CustomType::Anonymous(ty) => CustomType::Anonymous(match ty {
                AnonymousType::Option(t) => {
                    AnonymousType::Option(self.copy_type(dep_name, dep, *t))
                }
                AnonymousType::Expected(e) => AnonymousType::Expected(Expected {
                    ok: self.copy_type(dep_name, dep, e.ok),
                    err: self.copy_type(dep_name, dep, e.err),
                }),
                AnonymousType::Tuple(t) => AnonymousType::Tuple(Tuple {
                    types: t
                        .types
                        .iter()
                        .map(|ty| self.copy_type(dep_name, dep, *ty))
                        .collect(),
                }),
                AnonymousType::List(t) => AnonymousType::List(self.copy_type(dep_name, dep, *t)),
            }),
            CustomType::Named(ty) => CustomType::Named(NamedType {
                docs: ty.docs.clone(),
                name: ty.name.clone(),
                foreign_module: Some(
                    ty.foreign_module
                        .clone()
                        .unwrap_or_else(|| dep_name.to_string()),
                ),
                kind: match &ty.kind {
                    NamedTypeKind::Alias(t) => {
                        NamedTypeKind::Alias(self.copy_type(dep_name, dep, *t))
                    }
                    NamedTypeKind::Record(r) => NamedTypeKind::Record(Record {
                        fields: r
                            .fields
                            .iter()
                            .map(|field| Field {
                                docs: field.docs.clone(),
                                name: field.name.clone(),
                                ty: self.copy_type(dep_name, dep, field.ty),
                            })
                            .collect(),
                    }),
                    NamedTypeKind::Flags(f) => NamedTypeKind::Flags(f.clone()),
                    NamedTypeKind::Variant(v) => NamedTypeKind::Variant(Variant {
                        cases: v
                            .cases
                            .iter()
                            .map(|case| Case {
                                docs: case.docs.clone(),
                                name: case.name.clone(),
                                ty: self.copy_type(dep_name, dep, case.ty),
                            })
                            .collect(),
                    }),
                    NamedTypeKind::Enum(e) => NamedTypeKind::Enum(Enum {
                        cases: e.cases.clone(),
                    }),
                    NamedTypeKind::Union(u) => NamedTypeKind::Union(Union {
                        cases: u
                            .cases
                            .iter()
                            .map(|c| UnionCase {
                                docs: c.docs.clone(),
                                ty: self.copy_type(dep_name, dep, c.ty),
                            })
                            .collect(),
                    }),
                },
            }),
        };
        let id = self.types.alloc(ty);
        self.types_copied.insert((dep_name.to_string(), dep_id), id);
        id
    }

    fn copy_type(&mut self, dep_name: &str, dep: &Interface, ty: Type) -> Type {
        match ty {
            Type::Id(id) => Type::Id(self.copy_custom_type(dep_name, dep, id)),
            Type::Handle(id) => Type::Handle(self.copy_resource(dep_name, dep, id)),
            other => other,
        }
    }

    /// Register all of the named types in `items` in `self.types` without actually initializing them.
    ///
    /// This is done so that types can still reference one another when they're defined out of order.
    fn register_names(&mut self, items: &[Item<'_>]) -> Result<()> {
        let mut values = HashSet::new();
        for field in items {
            match field {
                Item::Resource(r) => {
                    let docs = self.docs(&r.docs);
                    let id = self.resources.alloc(Resource {
                        docs,
                        name: r.name.name.to_string(),
                        foreign_module: None,
                    });
                    self.define_resource(&r.name.name, r.name.span, id)?;
                }
                Item::TypeDef(t) => {
                    let docs = self.docs(&t.docs);
                    let id = self.types.alloc(CustomType::Named(NamedType {
                        docs,
                        // a dummy kind is used for now which will get filled in
                        // later with the actual desired contents.
                        kind: NamedTypeKind::Record(Record { fields: vec![] }),
                        name: t.name.name.to_string(),
                        foreign_module: None,
                    }));
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

    /// Convert a `TypeDefKind` to a `NamedTypeKind`.
    fn resolve_named_type(&mut self, ty: &super::TypeDefKind<'_>) -> Result<NamedTypeKind> {
        Ok(match ty {
            super::TypeDefKind::Alias(alias) => {
                let ty = self.resolve_type(&alias.target)?;
                NamedTypeKind::Alias(ty)
            }
            super::TypeDefKind::Record(record) => {
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
                NamedTypeKind::Record(Record { fields })
            }
            super::TypeDefKind::Flags(flags) => {
                let flags = flags
                    .flags
                    .iter()
                    .map(|flag| Flag {
                        docs: self.docs(&flag.docs),
                        name: flag.name.name.to_string(),
                    })
                    .collect::<Vec<_>>();
                NamedTypeKind::Flags(Flags { flags })
            }
            super::TypeDefKind::Variant(variant) => {
                if variant.cases.is_empty() {
                    return Err(Error {
                        span: variant.span,
                        msg: "empty variant".to_string(),
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
                                Some(ty) => self.resolve_type(ty)?,
                                None => Type::Unit,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                NamedTypeKind::Variant(Variant { cases })
            }
            super::TypeDefKind::Enum(e) => {
                if e.cases.is_empty() {
                    return Err(Error {
                        span: e.span,
                        msg: "empty enum".to_string(),
                    }
                    .into());
                }
                let cases = e
                    .cases
                    .iter()
                    .map(|case| {
                        Ok(EnumCase {
                            docs: self.docs(&case.docs),
                            name: case.name.name.to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                NamedTypeKind::Enum(Enum { cases })
            }
            super::TypeDefKind::Union(e) => {
                if e.cases.is_empty() {
                    return Err(Error {
                        span: e.span,
                        msg: "empty union".to_string(),
                    }
                    .into());
                }
                let cases = e
                    .cases
                    .iter()
                    .map(|case| {
                        Ok(UnionCase {
                            docs: self.docs(&case.docs),
                            ty: self.resolve_type(&case.ty)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                NamedTypeKind::Union(Union { cases })
            }
        })
    }

    fn resolve_type(&mut self, ty: &super::Type<'_>) -> Result<Type> {
        Ok(match ty {
            super::Type::Unit => Type::Unit,
            super::Type::Bool => Type::Bool,
            super::Type::U8 => Type::U8,
            super::Type::U16 => Type::U16,
            super::Type::U32 => Type::U32,
            super::Type::U64 => Type::U64,
            super::Type::S8 => Type::S8,
            super::Type::S16 => Type::S16,
            super::Type::S32 => Type::S32,
            super::Type::S64 => Type::S64,
            super::Type::Float32 => Type::Float32,
            super::Type::Float64 => Type::Float64,
            super::Type::Char => Type::Char,
            super::Type::String => Type::String,
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
                Type::Handle(id)
            }
            super::Type::Name(name) => match self.type_lookup.get(&*name.name) {
                Some(id) => Type::Id(*id),
                None => match self.resource_lookup.get(&*name.name) {
                    Some(id) => Type::Handle(*id),
                    _ => {
                        return Err(Error {
                            span: name.span,
                            msg: format!("no type named `{}`", name.name),
                        }
                        .into())
                    }
                },
            },

            super::Type::Option(ty) => {
                let ty = self.resolve_type(ty)?;
                self.resolve_anon_type(AnonymousType::Option(ty))
            }
            super::Type::Expected(expected) => {
                let ok = self.resolve_type(&expected.ok)?;
                let err = self.resolve_type(&expected.err)?;
                self.resolve_anon_type(AnonymousType::Expected(Expected { ok, err }))
            }
            super::Type::Tuple(types) => {
                let types = types
                    .iter()
                    .map(|ty| self.resolve_type(ty))
                    .collect::<Result<_>>()?;
                self.resolve_anon_type(AnonymousType::Tuple(Tuple { types }))
            }
            super::Type::List(ty) => {
                let ty = self.resolve_type(ty)?;
                self.resolve_anon_type(AnonymousType::List(ty))
            }
        })
    }

    fn resolve_anon_type(&mut self, ty: AnonymousType) -> Type {
        let types = &mut self.types;
        let id = self
            .anon_types
            .entry(ty.clone())
            .or_insert_with(|| types.alloc(CustomType::Anonymous(ty)));
        Type::Id(*id)
    }

    fn docs(&mut self, doc: &super::Docs<'_>) -> Docs {
        if doc.docs.is_empty() {
            return Docs { contents: None };
        }
        let mut docs = String::new();
        for doc in doc.docs.iter() {
            // Comments which are not doc-comments are silently ignored
            if let Some(doc) = doc.strip_prefix("///") {
                docs.push_str(doc.trim_start_matches('/').trim());
                docs.push('\n');
            } else if let Some(doc) = doc.strip_prefix("/**") {
                assert!(doc.ends_with("*/"));
                for line in doc[..doc.len() - 2].lines() {
                    docs.push_str(line);
                    docs.push('\n');
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
                is_async,
                params,
                result,
            } => {
                let params = params
                    .iter()
                    .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(ty)?)))
                    .collect::<Result<_>>()?;
                let result = self.resolve_type(result)?;
                self.functions.push(Function {
                    docs,
                    name: value.name.name.to_string(),
                    kind: FunctionKind::Freestanding,
                    params,
                    result,
                    is_async: *is_async,
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
            let (is_async, params, result) = match &value.kind {
                ValueKind::Function {
                    is_async,
                    params,
                    result,
                } => (*is_async, params, result),
                ValueKind::Global(_) => {
                    return Err(Error {
                        span: value.name.span,
                        msg: "globals not allowed in resources".to_string(),
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
                .map(|(name, ty)| Ok((name.name.to_string(), self.resolve_type(ty)?)))
                .collect::<Result<Vec<_>>>()?;
            let result = self.resolve_type(result)?;
            let kind = if *statik {
                FunctionKind::Static {
                    resource: id,
                    name: value.name.name.to_string(),
                }
            } else {
                params.insert(0, ("self".to_string(), Type::Handle(id)));
                FunctionKind::Method {
                    resource: id,
                    name: value.name.name.to_string(),
                }
            };
            self.functions.push(Function {
                is_async,
                docs,
                name: format!("{}::{}", resource.name.name, value.name.name),
                kind,
                params,
                result,
            });
        }
        Ok(())
    }

    /// Checks that a type isn't recursive (which would be invalid).
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
                msg: "type can recursively refer to itself".to_string(),
            }
            .into());
        }

        match &self.types[ty] {
            CustomType::Anonymous(ty) => match ty {
                AnonymousType::Option(t) => {
                    if let Type::Id(id) = *t {
                        self.validate_type_not_recursive(span, id, visiting, valid)?
                    }
                }
                AnonymousType::Expected(e) => {
                    if let Type::Id(id) = e.ok {
                        self.validate_type_not_recursive(span, id, visiting, valid)?
                    }
                    if let Type::Id(id) = e.err {
                        self.validate_type_not_recursive(span, id, visiting, valid)?
                    }
                }
                AnonymousType::Tuple(t) => {
                    for ty in t.types.iter() {
                        if let Type::Id(id) = *ty {
                            self.validate_type_not_recursive(span, id, visiting, valid)?;
                        }
                    }
                }
                AnonymousType::List(ty) => {
                    if let Type::Id(id) = ty {
                        self.validate_type_not_recursive(span, *id, visiting, valid)?
                    }
                }
            },
            CustomType::Named(ty) => match &ty.kind {
                NamedTypeKind::Alias(Type::Id(id)) => {
                    self.validate_type_not_recursive(span, *id, visiting, valid)?
                }
                NamedTypeKind::Variant(v) => {
                    for case in v.cases.iter() {
                        if let Type::Id(id) = case.ty {
                            self.validate_type_not_recursive(span, id, visiting, valid)?;
                        }
                    }
                }
                NamedTypeKind::Record(r) => {
                    for case in r.fields.iter() {
                        if let Type::Id(id) = case.ty {
                            self.validate_type_not_recursive(span, id, visiting, valid)?;
                        }
                    }
                }
                NamedTypeKind::Union(u) => {
                    for c in u.cases.iter() {
                        if let Type::Id(id) = c.ty {
                            self.validate_type_not_recursive(span, id, visiting, valid)?
                        }
                    }
                }

                NamedTypeKind::Flags(_) | NamedTypeKind::Alias(_) | NamedTypeKind::Enum(_) => {}
            },
        }

        valid.insert(ty);
        visiting.remove(&ty);
        Ok(())
    }
}
