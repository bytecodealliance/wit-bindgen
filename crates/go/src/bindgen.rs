use std::fmt::Write as _;

use heck::{ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_c::{flags_repr, int_repr};
use wit_bindgen_core::wit_parser::Handle::{Borrow, Own};
use wit_bindgen_core::wit_parser::{Field, Function, Type, TypeDefKind};
use wit_bindgen_core::{dealias, uwriteln, Direction, Source};

use super::avoid_keyword;
use crate::interface;

pub(crate) struct FunctionBindgen<'a, 'b> {
    pub(crate) interface: &'a mut interface::InterfaceGenerator<'b>,
    pub(crate) func: &'a Function,
    pub(crate) c_args: Vec<String>,
    pub(crate) args: Vec<String>,
    pub(crate) lower_src: Source,
    pub(crate) lift_src: Source,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    pub(crate) fn new(
        interface: &'a mut interface::InterfaceGenerator<'b>,
        func: &'a Function,
    ) -> Self {
        Self {
            interface,
            func,
            c_args: Vec::new(),
            args: Vec::new(),
            lower_src: Source::default(),
            lift_src: Source::default(),
        }
    }

    pub(crate) fn process_args(&mut self) {
        self.func
            .params
            .iter()
            .for_each(|(name, ty)| match self.interface.direction {
                Direction::Import => self.lower(&avoid_keyword(&name.to_snake_case()), ty),
                Direction::Export => self.lift(&avoid_keyword(&name.to_snake_case()), ty),
            });
    }

    pub(crate) fn process_returns(&mut self) {
        match self.func.results.len() {
            0 => {}
            1 => {
                let ty = self.func.results.iter_types().next().unwrap();
                match self.interface.direction {
                    Direction::Import => self.lift("ret", ty),
                    Direction::Export => self.lower("result", ty),
                }
            }
            _ => {
                for (i, ty) in self.func.results.iter_types().enumerate() {
                    match self.interface.direction {
                        Direction::Import => self.lift(&format!("ret{i}"), ty),
                        Direction::Export => self.lower(&format!("result{i}"), ty),
                    }
                }
            }
        };
    }

    pub(crate) fn lower(&mut self, name: &str, ty: &Type) {
        let lower_name = format!("lower_{name}");
        self.lower_value(name, ty, lower_name.as_ref());

        // Check whether or not the C variable needs to be freed.
        // If this variable is in export function, which will be returned to host to use.
        //    There is no need to free return variables.
        // If this variable does not own anything, it does not need to be freed.
        // If this variable is in inner node of the recursive call, no need to be freed.
        //    This is because the root node's call to free will recursively free the whole tree.
        // Otherwise, free this variable.
        //
        // TODO: should test if free is necessary
        if matches!(self.interface.direction, Direction::Import) && false {
            self.lower_src
                .push_str(&self.interface.free_c_arg(ty, &format!("&{lower_name}")));
        }
        self.c_args.push(lower_name);
    }

    pub(crate) fn lower_list_value(&mut self, param: &str, l: &Type, lower_name: &str) {
        let list_ty = self.interface.gen.get_c_ty(l);
        uwriteln!(
                self.lower_src,
                "if len({param}) == 0 {{
                {lower_name}.ptr = nil
                {lower_name}.len = 0
            }} else {{
                var empty_{lower_name} {list_ty}
                {lower_name}.ptr = (*{list_ty})(C.malloc(C.size_t(len({param})) * C.size_t(unsafe.Sizeof(empty_{lower_name}))))
                {lower_name}.len = C.size_t(len({param}))"
            );

        uwriteln!(self.lower_src, "for {lower_name}_i := range {param} {{");
        uwriteln!(self.lower_src,
                "{lower_name}_ptr := (*{list_ty})(unsafe.Pointer(uintptr(unsafe.Pointer({lower_name}.ptr)) +
            uintptr({lower_name}_i)*unsafe.Sizeof(empty_{lower_name})))"
            );

        let param = &format!("{param}[{lower_name}_i]");
        let lower_name = &format!("{lower_name}_ptr");

        if let Some(inner) = self.interface.extract_list_ty(l) {
            self.lower_list_value(param, &inner.clone(), lower_name);
        } else {
            self.lower_value(param, l, &format!("{lower_name}_value"));
            uwriteln!(self.lower_src, "*{lower_name} = {lower_name}_value");
        }

        uwriteln!(self.lower_src, "}}");
        uwriteln!(self.lower_src, "}}");
    }

    pub(crate) fn lower_result_value(
        &mut self,
        param: &str,
        ty: &Type,
        lower_name: &str,
        lower_inner_name1: &str,
        lower_inner_name2: &str,
    ) {
        // lower_inner_name could be {lower_name}.val if it's used in import
        // else, it could be either ret or err

        let (ok, err) = self.interface.extract_result_ty(ty);
        uwriteln!(self.lower_src, "if {param}.IsOk() {{");
        if let Some(ok_inner) = ok {
            self.interface.gen.with_import_unsafe(true);
            let c_target_name = self.interface.gen.get_c_ty(&ok_inner);
            uwriteln!(
                self.lower_src,
                "{lower_name}_ptr := (*{c_target_name})(unsafe.Pointer({lower_inner_name1}))"
            );
            self.lower_value(
                &format!("{param}.Unwrap()"),
                &ok_inner,
                &format!("{lower_name}_val"),
            );
            uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
        }
        self.lower_src.push_str("} else {\n");
        if let Some(err_inner) = err {
            self.interface.gen.with_import_unsafe(true);
            let c_target_name = self.interface.gen.get_c_ty(&err_inner);
            uwriteln!(
                self.lower_src,
                "{lower_name}_ptr := (*{c_target_name})(unsafe.Pointer({lower_inner_name2}))"
            );
            self.lower_value(
                &format!("{param}.UnwrapErr()"),
                &err_inner,
                &format!("{lower_name}_val"),
            );
            uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
        }
        self.lower_src.push_str("}\n");
    }

    /// Lower a value to a string representation.
    ///
    /// # Parameters
    ///
    /// * `param` - The string representation of the parameter of a function
    /// * `ty` - A reference to a `Type` that specifies the type of the value.
    /// * `lower_name` - A reference to a string that represents the name to be used for the lower value.
    pub(crate) fn lower_value(&mut self, param: &str, ty: &Type, lower_name: &str) {
        match ty {
            Type::Bool => {
                uwriteln!(self.lower_src, "{lower_name} := {param}",);
            }
            Type::String => {
                self.interface.gen.with_import_unsafe(true);
                uwriteln!(
                    self.lower_src,
                    "var {lower_name} {value}",
                    value = self.interface.gen.get_c_ty(ty),
                );
                uwriteln!(
                    self.lower_src,
                    "
                    // use unsafe.Pointer to avoid copy
                    {lower_name}.ptr = (*uint8)(unsafe.Pointer(C.CString({param})))
                    {lower_name}.len = C.size_t(len({param}))"
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.resolve.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for field in r.fields.iter() {
                            let c_field_name = &self.get_c_field_name(field);
                            let field_name = &self.get_go_field_name(field);

                            self.lower_value(
                                &format!("{param}.{field_name}"),
                                &field.ty,
                                &format!("{lower_name}_{c_field_name}"),
                            );
                            uwriteln!(
                                self.lower_src,
                                "{lower_name}.{c_field_name} = {lower_name}_{c_field_name}"
                            )
                        }
                    }

                    TypeDefKind::Flags(f) => {
                        let int_repr = int_repr(flags_repr(f));
                        uwriteln!(self.lower_src, "{lower_name} := C.{int_repr}({param})");
                    }
                    TypeDefKind::Tuple(t) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, ty) in t.types.iter().enumerate() {
                            self.lower_value(
                                &format!("{param}.F{i}"),
                                ty,
                                &format!("{lower_name}_f{i}"),
                            );
                            uwriteln!(self.lower_src, "{lower_name}.f{i} = {lower_name}_f{i}");
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        uwriteln!(self.lower_src, "if {param}.IsSome() {{");
                        self.lower_value(
                            &format!("{param}.Unwrap()"),
                            o,
                            &format!("{lower_name}_val"),
                        );
                        uwriteln!(self.lower_src, "{lower_name}.val = {lower_name}_val");
                        uwriteln!(self.lower_src, "{lower_name}.is_some = true");
                        self.lower_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));

                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        uwriteln!(self.lower_src, "{lower_name}.is_err = {param}.IsErr()");
                        let inner_name = format!("&{lower_name}.val");
                        self.lower_result_value(
                            param,
                            &Type::Id(*id),
                            lower_name,
                            &inner_name,
                            &inner_name,
                        );
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.with_import_unsafe(true);
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));

                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        self.lower_list_value(param, l, lower_name);
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lower_src,
                            "var {lower_name} {value}",
                            value = self.interface.gen.get_c_ty(t),
                        );
                        self.lower_value(param, t, &format!("{lower_name}_val"));
                        uwriteln!(self.lower_src, "{lower_name} = {lower_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.with_import_unsafe(true);

                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            if let Some(ty) = case.ty.as_ref() {
                                let name = self.interface.gen.get_c_ty(ty);
                                uwriteln!(
                                        self.lower_src,
                                        "
                                    {lower_name}.tag = {i}
                                    {lower_name}_ptr := (*{name})(unsafe.Pointer(&{lower_name}.val))"
                                    );
                                self.lower_value(
                                    &format!("{param}.Get{case_name}()"),
                                    ty,
                                    &format!("{lower_name}_val"),
                                );
                                uwriteln!(self.lower_src, "*{lower_name}_ptr = {lower_name}_val");
                            } else {
                                uwriteln!(self.lower_src, "{lower_name}.tag = {i}");
                            }
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lower_src, "var {lower_name} {c_typedef_target}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(
                                self.lower_src,
                                "if {param}.Kind() == {ty}Kind{case_name} {{"
                            );
                            uwriteln!(self.lower_src, "{lower_name} = {i}");
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(h) => {
                        match self.interface.direction {
                            Direction::Import => {
                                let c_typedef_target = self.interface.gen.get_c_ty(&Type::Id(*id));
                                uwriteln!(
                                    self.lower_src,
                                    "var {lower_name} {c_typedef_target}
                                        {lower_name}.__handle = C.int32_t({param})"
                                )
                            }
                            Direction::Export => {
                                let resource = dealias(
                                    self.interface.resolve,
                                    *match h {
                                        Borrow(resource) => resource,
                                        Own(resource) => resource,
                                    },
                                );
                                let ns = self.interface.c_namespace_of_resource(resource);
                                let snake = self.interface.resolve.types[resource]
                                    .name
                                    .as_ref()
                                    .unwrap()
                                    .to_snake_case();
                                // If the resource is exported, then `type_resource` have created a
                                // internal bookkeeping map for this resource. We will need to
                                // use the map to get the resource interface. Otherwise, this resource
                                // is imported, and we will use the generated i32 handle directly.
                                if self.interface.gen.exported_resources.contains(&resource) {
                                    let resource = dealias(self.interface.resolve, resource);
                                    let c_typedef_target =
                                        self.interface.gen.get_c_ty(&Type::Id(resource));
                                    let ty_name = self.interface.gen.type_names[&resource].clone();
                                    let private_type_name = ty_name.to_snake_case();
                                    self.interface.gen.with_import_unsafe(true);
                                    uwriteln!(self.lower_src,
                                            "{private_type_name}_mu.Lock()
                                        {private_type_name}_next_id += 1
                                        {private_type_name}_pointers[{private_type_name}_next_id] = {param}
                                        {private_type_name}_mu.Unlock()
                                        {lower_name}_c := (*{c_typedef_target})(unsafe.Pointer(C.malloc(C.size_t(unsafe.Sizeof({c_typedef_target}{{}})))))
                                        {lower_name}_c.__handle = C.int32_t({private_type_name}_next_id)
                                        {lower_name} := C.{ns}_{snake}_new({lower_name}_c) // pass the pointer directly
                                        set{ty_name}OwningHandler({param}, int32({lower_name}.__handle))"
                                        );
                                } else {
                                    // need to construct either an own or borrowed C handle type.

                                    let c_typedef_target = match h {
                                        Own(_) => {
                                            let mut own = ns.clone();
                                            own.push_str("_own_");
                                            own.push_str(&snake);
                                            own.push_str("_t");
                                            own
                                        }
                                        Borrow(_) => {
                                            let mut borrow = ns.clone();
                                            borrow.push_str("_borrow_");
                                            borrow.push_str(&snake);
                                            borrow.push_str("_t");
                                            borrow
                                        }
                                    };
                                    uwriteln!(
                                        self.lower_src,
                                        "var {lower_name} C.{c_typedef_target}
                                        {lower_name}.__handle = C.int32_t({param})"
                                    );
                                }
                            }
                        }
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
            a => {
                uwriteln!(
                    self.lower_src,
                    "{lower_name} := {c_type_name}({param_name})",
                    c_type_name = self.interface.gen.get_c_ty(a),
                    param_name = param,
                );
            }
        }
    }

    pub(crate) fn lift(&mut self, name: &str, ty: &Type) {
        let lift_name = format!("lift_{name}");
        self.lift_value(name, ty, lift_name.as_str());
        self.args.push(lift_name);
    }

    pub(crate) fn lift_value(&mut self, param: &str, ty: &Type, lift_name: &str) {
        match ty {
            Type::Bool => {
                uwriteln!(self.lift_src, "{lift_name} := {param}");
            }
            Type::String => {
                self.interface.gen.with_import_unsafe(true);
                uwriteln!(
                        self.lift_src,
                        "var {name} {value}
                    {lift_name} = C.GoStringN((*C.char)(unsafe.Pointer({param}.ptr)), C.int({param}.len))",
                        name = lift_name,
                        value = self.interface.get_ty(ty),
                    );
            }
            Type::Id(id) => {
                let ty = &self.interface.resolve.types[*id]; // receive type
                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for field in r.fields.iter() {
                            let field_name = &self.get_go_field_name(field);
                            let c_field_name = &self.get_c_field_name(field);
                            self.lift_value(
                                &format!("{param}.{c_field_name}"),
                                &field.ty,
                                &format!("{lift_name}_{field_name}"),
                            );
                            uwriteln!(
                                self.lift_src,
                                "{lift_name}.{field_name} = {lift_name}_{field_name}"
                            );
                        }
                    }
                    TypeDefKind::Flags(_f) => {
                        let field = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}
                            {lift_name} = {field}({param})",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                    }
                    TypeDefKind::Tuple(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {ty_name}",
                            name = lift_name,
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for (i, t) in t.types.iter().enumerate() {
                            self.lift_value(
                                &format!("{param}.f{i}"),
                                t,
                                &format!("{lift_name}_F{i}"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.F{i} = {lift_name}_F{i}");
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        uwriteln!(self.lift_src, "if {param}.is_some {{");
                        self.lift_value(&format!("{param}.val"), o, &format!("{lift_name}_val"));

                        uwriteln!(self.lift_src, "{lift_name}.Set({lift_name}_val)");
                        self.lift_src.push_str("} else {\n");
                        uwriteln!(self.lift_src, "{lift_name}.Unset()");
                        self.lift_src.push_str("}\n");
                    }
                    TypeDefKind::Result(_) => {
                        self.interface.gen.with_result_option(true);
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        let (ok, err) = self.interface.extract_result_ty(&Type::Id(*id));

                        // normal result route
                        uwriteln!(self.lift_src, "if {param}.is_err {{");
                        if let Some(err_inner) = err {
                            let err_inner_name = self.interface.gen.get_c_ty(&err_inner);
                            self.interface.gen.with_import_unsafe(true);
                            uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{err_inner_name})(unsafe.Pointer(&{param}.val))");
                            self.lift_value(
                                &format!("{lift_name}_ptr"),
                                &err_inner,
                                &format!("{lift_name}_val"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.SetErr({lift_name}_val)")
                        } else {
                            uwriteln!(self.lift_src, "{lift_name}.SetErr(struct{{}}{{}})")
                        }
                        uwriteln!(self.lift_src, "}} else {{");
                        if let Some(ok_inner) = ok {
                            let ok_inner_name = self.interface.gen.get_c_ty(&ok_inner);
                            self.interface.gen.with_import_unsafe(true);
                            uwriteln!(self.lift_src, "{lift_name}_ptr := *(*{ok_inner_name})(unsafe.Pointer(&{param}.val))");
                            self.lift_value(
                                &format!("{lift_name}_ptr"),
                                &ok_inner,
                                &format!("{lift_name}_val"),
                            );
                            uwriteln!(self.lift_src, "{lift_name}.Set({lift_name}_val)")
                        }
                        uwriteln!(self.lift_src, "}}");
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.with_import_unsafe(true);
                        let list_ty = self.interface.get_ty(&Type::Id(*id));
                        let c_ty_name = self.interface.gen.get_c_ty(l);
                        uwriteln!(self.lift_src, "var {lift_name} {list_ty}",);
                        uwriteln!(self.lift_src, "{lift_name} = make({list_ty}, {param}.len)");
                        uwriteln!(self.lift_src, "if {param}.len > 0 {{");
                        uwriteln!(self.lift_src, "for {lift_name}_i := 0; {lift_name}_i < int({param}.len); {lift_name}_i++ {{");
                        uwriteln!(self.lift_src, "var empty_{lift_name} {c_ty_name}");
                        uwriteln!(
                                self.lift_src,
                                "{lift_name}_ptr := *(*{c_ty_name})(unsafe.Pointer(uintptr(unsafe.Pointer({param}.ptr)) +
                            uintptr({lift_name}_i)*unsafe.Sizeof(empty_{lift_name})))"
                            );

                        // If l is an empty tuple, set _ = {lift_name}_ptr
                        // this is a special case needs to be handled
                        if self.interface.is_empty_tuple_ty(l) {
                            uwriteln!(self.lift_src, "_ = {lift_name}_ptr");
                        }

                        self.lift_value(
                            &format!("{lift_name}_ptr"),
                            l,
                            &format!("list_{lift_name}"),
                        );

                        uwriteln!(
                            self.lift_src,
                            "{lift_name}[{lift_name}_i] = list_{lift_name}"
                        );
                        self.lift_src.push_str("}\n");
                        self.lift_src.push_str("}\n");
                        // TODO: don't forget to free `ret`
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {lift_name} {ty_name}",
                            ty_name = self.interface.get_ty(&Type::Id(*id)),
                        );
                        self.lift_value(param, t, &format!("{lift_name}_val"));
                        uwriteln!(self.lift_src, "{lift_name} = {lift_name}_val");
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.with_import_unsafe(true);
                        let ty_name: String = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            if let Some(ty) = case.ty.as_ref() {
                                let c_ty_name = self.interface.gen.get_c_ty(ty);
                                uwriteln!(
                                        self.lift_src,
                                        "{lift_name}_ptr := *(*{c_ty_name})(unsafe.Pointer(&{param}.val))"
                                    );
                                self.lift_value(
                                    &format!("{lift_name}_ptr"),
                                    ty,
                                    &format!("{lift_name}_val"),
                                );
                                uwriteln!(
                                    self.lift_src,
                                    "{lift_name} = {ty_name}{case_name}({lift_name}_val)"
                                )
                            } else {
                                uwriteln!(self.lift_src, "{lift_name} = {ty_name}{case_name}()");
                            }
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let ty_name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {ty_name}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            uwriteln!(self.lift_src, "if {param} == {i} {{");
                            uwriteln!(self.lift_src, "{lift_name} = {ty_name}{case_name}()");
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    TypeDefKind::Resource => todo!("impl resource"),
                    TypeDefKind::Handle(h) => {
                        match self.interface.direction {
                            Direction::Import => {
                                let ty_name = self.interface.get_ty(&Type::Id(*id));
                                uwriteln!(
                                    self.lift_src,
                                    "var {lift_name} {ty_name}
                                    {lift_name} = {ty_name}({param}.__handle)
                                    "
                                );
                            }
                            Direction::Export => {
                                // If the resource is exported, then `type_resource` have created a
                                // internal bookkeeping map for this resource. We will need to
                                // use the map to get the resource interface. Otherwise, this resource
                                // is imported, and we will use the generated i32 handle directly.
                                let resource = dealias(
                                    self.interface.resolve,
                                    *match h {
                                        Borrow(resource) => resource,
                                        Own(resource) => resource,
                                    },
                                );
                                // we want to get the namespace of the dealias resource since
                                // only the `rep` and `new` functions are generated in the namespace
                                // see issue: https://github.com/bytecodealliance/wit-bindgen/issues/763
                                let ns = self.interface.c_namespace_of_resource(resource);
                                let snake = self.interface.resolve.types[resource]
                                    .name
                                    .as_ref()
                                    .unwrap()
                                    .to_snake_case();
                                if self.interface.gen.exported_resources.contains(&resource) {
                                    let resource_name: String =
                                        self.interface.get_ty(&Type::Id(resource)).to_snake_case();
                                    match h {
                                        Own(_) => {
                                            uwriteln!(
                                                    self.lift_src,
                                                    "{lift_name}_rep := C.{ns}_{snake}_rep({param})
                                                {lift_name}_handle := int32({lift_name}_rep.__handle)"
                                                );
                                        }
                                        Borrow(_) => {
                                            uwriteln!(
                                                self.lift_src,
                                                "{lift_name}_handle := int32({param}.__handle)"
                                            );
                                        }
                                    }
                                    uwriteln!(
                                            self.lift_src,
                                            "{lift_name}, ok := {resource_name}_pointers[{lift_name}_handle]
                                        if !ok {{
                                            panic(\"internal error: invalid handle\")
                                        }}"
                                        );
                                } else {
                                    let resource_name = self.interface.get_ty(&Type::Id(resource));
                                    uwriteln!(
                                        self.lift_src,
                                        "{lift_name} := {resource_name}({param}.__handle)",
                                    );
                                }
                            }
                        }
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
            a => {
                let target_name = self.interface.get_ty(a);

                uwriteln!(self.lift_src, "var {lift_name} {target_name}",);
                uwriteln!(self.lift_src, "{lift_name} = {target_name}({param})",);
            }
        }
    }

    pub(crate) fn get_c_field_name(&mut self, field: &Field) -> String {
        avoid_keyword(field.name.to_snake_case().as_str())
    }

    pub(crate) fn get_go_field_name(&mut self, field: &Field) -> String {
        let name = &self.interface.field_name(field);
        avoid_keyword(name)
    }
}
