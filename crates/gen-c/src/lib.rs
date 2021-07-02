#![allow(unused_variables)] // TODO
#![allow(unused_imports)] // TODO
#![allow(dead_code)] // TODO

use heck::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::mem;
use witx_bindgen_gen_core::witx2::abi::{
    Bindgen, Bitcast, Direction, Instruction, LiftLower, WasmType, WitxInstruction,
};
use witx_bindgen_gen_core::{witx2::*, Files, Generator, Ns};

#[derive(Default)]
pub struct C {
    src: Source,
    in_import: bool,
    opts: Opts,
    imports: HashMap<String, Vec<Import>>,
    exports: HashMap<String, Exports>,
    i64_return_pointer_area_size: usize,
    sizes: SizeAlign,
    names: Ns,

    // The set of types that are considered public (aka need to be in the
    // header file) which are anonymous and we're effectively monomorphizing.
    // This is discovered lazily when printing type names.
    public_anonymous_types: BTreeSet<TypeId>,

    // This is similar to `public_anonymous_types` where it's discovered
    // lazily, but the set here are for private types only used in the
    // implementation of functions. These types go in the implementation file,
    // not the header file.
    private_anonymous_types: BTreeSet<TypeId>,

    // Type definitions for the given `TypeId`. This is printed topologically
    // at the end.
    types: HashMap<TypeId, witx_bindgen_gen_core::Source>,

    intrinsics: HashMap<Intrinsic, String>,
    needs_string: bool,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Intrinsic {}

struct Import {
    name: String,
    src: Source,
}

#[derive(Default)]
struct Exports {
    funcs: Vec<Source>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(self) -> C {
        let mut r = C::new();
        r.opts = self;
        r
    }
}

#[derive(Debug)]
struct Return {
    splat_tuple: bool,
    scalar: Option<Scalar>,
    retptrs: Vec<Type>,
}

struct CSig {
    name: String,
    sig: String,
    params: Vec<(bool, String)>,
    ret: Return,
    retptrs: Vec<String>,
}

#[derive(Debug)]
enum Scalar {
    OptionBool(Type),
    ExpectedEnum {
        err: TypeId,
        ok: Option<Type>,
        max_err: usize,
    },
    Type(Type),
}

impl C {
    pub fn new() -> C {
        C::default()
    }

    fn classify_ret(&mut self, iface: &Interface, func: &Function) -> Return {
        let mut ret = Return {
            splat_tuple: false,
            scalar: None,
            retptrs: Vec::new(),
        };
        match func.results.len() {
            0 => {}
            1 => ret.return_single(iface, &func.results[0].1, &func.results[0].1),
            _ => ret.retptrs.extend(func.results.iter().map(|p| p.1)),
        }
        return ret;
    }

    fn print_sig(&mut self, iface: &Interface, func: &Function) -> CSig {
        let name = format!(
            "{}_{}",
            iface.name.to_snake_case(),
            func.name.to_snake_case()
        );
        self.names.insert(&name).expect("duplicate symbols");
        let start = self.src.header.len();

        let ret = self.classify_ret(iface, func);
        match &ret.scalar {
            None => self.src.h("void"),
            Some(Scalar::OptionBool(_id)) => self.src.h("bool"),
            Some(Scalar::ExpectedEnum { err, .. }) => self.print_ty(iface, &Type::Id(*err)),
            Some(Scalar::Type(ty)) => self.print_ty(iface, ty),
        }
        self.src.h(" ");
        self.src.h(&name);
        self.src.h("(");
        let mut params = Vec::new();
        for (i, (name, ty)) in func.params.iter().enumerate() {
            if i > 0 {
                self.src.h(", ");
            }
            self.print_ty(iface, ty);
            self.src.h(" ");
            let pointer = self.is_arg_by_pointer(iface, ty);
            if pointer {
                self.src.h("*");
            }
            let name = name.to_snake_case();
            self.src.h(&name);
            params.push((pointer, name));
        }
        let mut retptrs = Vec::new();
        for (i, ty) in ret.retptrs.iter().enumerate() {
            if i > 0 || func.params.len() > 0 {
                self.src.h(", ");
            }
            self.print_ty(iface, ty);
            self.src.h(" *");
            let name = format!("ret{}", i);
            self.src.h(&name);
            retptrs.push(name);
        }
        if func.params.len() == 0 && ret.retptrs.len() == 0 {
            self.src.h("void");
        }
        self.src.h(")");

        let sig = self.src.header[start..].to_string();
        self.src.h(";\n");

        CSig {
            sig,
            name,
            params,
            ret,
            retptrs,
        }
    }

    fn is_arg_by_pointer(&self, iface: &Interface, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Type(t) => self.is_arg_by_pointer(iface, t),
                TypeDefKind::Variant(v) => !v.is_enum(),
                TypeDefKind::Pointer(t) => false,
                TypeDefKind::ConstPointer(t) => false,
                TypeDefKind::List(Type::Char) => false,
                TypeDefKind::Record(r) if r.is_flags() => false,
                TypeDefKind::Record(_)
                | TypeDefKind::List(_)
                | TypeDefKind::PushBuffer(_)
                | TypeDefKind::PullBuffer(_) => true,
            },
            _ => false,
        }
    }

    fn type_string(&mut self, iface: &Interface, ty: &Type) -> String {
        // Getting a type string happens during codegen, and by default means
        // that this is a private type that's being generated. This means we
        // want to keep track of new anonymous types that are *only* mentioned
        // in methods like this, so we can place those types in the C file
        // instead of the header interface file.
        let prev = mem::take(&mut self.src.header);
        let prev_public = mem::take(&mut self.public_anonymous_types);
        let prev_private = mem::take(&mut self.private_anonymous_types);

        // Print the type, which will collect into the fields that we replaced
        // above.
        self.print_ty(iface, ty);

        // Reset our public/private sets back to what they were beforehand.
        // Note that `print_ty` always adds to the public set, so we're
        // inverting the meaning here by interpreting those as new private
        // types.
        let new_private = mem::replace(&mut self.public_anonymous_types, prev_public);
        assert!(self.private_anonymous_types.is_empty());
        self.private_anonymous_types = prev_private;

        // For all new private types found while we printed this type, if the
        // type isn't already public then it's a new private type.
        for id in new_private {
            if !self.public_anonymous_types.contains(&id) {
                self.private_anonymous_types.insert(id);
            }
        }

        mem::replace(&mut self.src.header, prev).into()
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type) {
        match ty {
            Type::Char => self.src.h("uint32_t"), // TODO: better type?
            Type::U8 => self.src.h("uint8_t"),
            Type::S8 => self.src.h("int8_t"),
            Type::U16 => self.src.h("uint16_t"),
            Type::S16 => self.src.h("int16_t"),
            Type::U32 => self.src.h("uint32_t"),
            Type::S32 => self.src.h("int32_t"),
            Type::U64 => self.src.h("uint64_t"),
            Type::S64 => self.src.h("int64_t"),
            Type::CChar => self.src.h("char"),
            Type::F32 => self.src.h("float"),
            Type::F64 => self.src.h("double"),
            Type::Usize => self.src.h("size_t"),
            Type::Handle(id) => {
                self.src.h(&iface.resources[*id].name.to_snake_case());
                self.src.h("_t");
            }
            Type::Id(id) => {
                let ty = &iface.types[*id];
                if let Some(name) = &ty.name {
                    self.src.h(&name.to_snake_case());
                    self.src.h("_t");
                    return;
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(iface, t),
                    TypeDefKind::Variant(v) => {
                        if v.is_bool() {
                            return self.src.h("bool");
                        }
                        self.public_anonymous_types.insert(*id);
                        self.private_anonymous_types.remove(id);
                        self.print_ty_name(iface, &Type::Id(*id));
                        self.src.h("_t");
                    }
                    TypeDefKind::Pointer(t) => {
                        self.print_ty(iface, t);
                        self.src.h("*");
                    }
                    TypeDefKind::ConstPointer(t) => {
                        self.src.h("const ");
                        self.print_ty(iface, t);
                        self.src.h("*");
                    }
                    TypeDefKind::List(Type::Char) => {
                        self.src
                            .h(&format!("{}_string_t", iface.name.to_snake_case()));
                        self.needs_string = true;
                    }
                    TypeDefKind::Record(_)
                    | TypeDefKind::List(_)
                    | TypeDefKind::PushBuffer(_)
                    | TypeDefKind::PullBuffer(_) => {
                        self.public_anonymous_types.insert(*id);
                        self.private_anonymous_types.remove(id);
                        self.print_ty_name(iface, &Type::Id(*id));
                        self.src.h("_t");
                    }
                }
            }
        }
    }

    fn print_ty_name(&mut self, iface: &Interface, ty: &Type) {
        match ty {
            Type::Char => self.src.h("char32"),
            Type::U8 => self.src.h("u8"),
            Type::S8 => self.src.h("s8"),
            Type::U16 => self.src.h("u16"),
            Type::S16 => self.src.h("s16"),
            Type::U32 => self.src.h("u32"),
            Type::S32 => self.src.h("s32"),
            Type::U64 => self.src.h("u64"),
            Type::S64 => self.src.h("s64"),
            Type::CChar => self.src.h("char"),
            Type::F32 => self.src.h("f32"),
            Type::F64 => self.src.h("f64"),
            Type::Usize => self.src.h("usize"),
            Type::Handle(id) => unimplemented!(),
            Type::Id(id) => {
                let ty = &iface.types[*id];
                if let Some(name) = &ty.name {
                    return self.src.h(&name.to_snake_case());
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty_name(iface, t),
                    TypeDefKind::Record(r) => {
                        assert!(r.is_tuple());
                        self.src.h("tuple");
                        self.src.h(&r.fields.len().to_string());
                        for field in r.fields.iter() {
                            self.src.h("_");
                            self.print_ty_name(iface, &field.ty);
                        }
                    }
                    TypeDefKind::Variant(v) => {
                        if let Some(ty) = v.as_option() {
                            self.src.h("option_");
                            self.print_ty_name(iface, ty);
                        } else if let Some((ok, err)) = v.as_expected() {
                            self.src.h("expected_");
                            match ok {
                                Some(t) => self.print_ty_name(iface, t),
                                None => self.src.h("void"),
                            }
                            self.src.h("_");
                            match err {
                                Some(t) => self.print_ty_name(iface, t),
                                None => self.src.h("void"),
                            }
                        } else if v.is_bool() {
                            self.src.h("bool");
                        } else {
                            unimplemented!();
                        }
                    }
                    TypeDefKind::Pointer(t) => {
                        self.src.h("ptr_");
                        self.print_ty_name(iface, t);
                    }
                    TypeDefKind::ConstPointer(t) => {
                        self.src.h("const_ptr_ ");
                        self.print_ty_name(iface, t);
                    }
                    TypeDefKind::List(Type::Char) => self.src.h("string"),
                    TypeDefKind::List(t) => {
                        self.src.h("list_");
                        self.print_ty_name(iface, t);
                    }
                    TypeDefKind::PushBuffer(t) => {
                        self.src.h("push_buffer_");
                        self.print_ty_name(iface, t);
                    }
                    TypeDefKind::PullBuffer(t) => {
                        self.src.h("pull_buffer_");
                        self.print_ty_name(iface, t);
                    }
                }
            }
        }
    }

    fn print_anonymous_type(&mut self, iface: &Interface, ty: TypeId) {
        let prev = mem::take(&mut self.src.header);
        self.src.h("typedef ");
        match &iface.types[ty].kind {
            TypeDefKind::Type(_) | TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => {
                unreachable!()
            }
            TypeDefKind::Record(r) => {
                assert!(r.is_tuple());
                self.src.h("struct {\n");
                for (i, f) in r.fields.iter().enumerate() {
                    self.print_ty(iface, &f.ty);
                    self.src.h(" ");
                    self.src.h(&format!("f{};\n", i));
                }
                self.src.h("}");
            }
            TypeDefKind::Variant(v) => {
                if let Some(t) = v.as_option() {
                    self.src.h("struct {\n");
                    self.src.h("\
                        // `true` if `val` is present, `false` otherwise
                        bool tag;
                    ");
                    self.print_ty(iface, t);
                    self.src.h(" val;\n");
                    self.src.h("}");
                } else if let Some((ok, err)) = v.as_expected() {
                    if ok.is_none() && err.is_none() {
                        self.src.h("uint8_t");
                    } else {
                        self.src.h("struct {
                            // `true` if `val` is `ok`, `false` otherwise
                            bool tag;
                            union {
                        ");
                        if let Some(ok) = ok {
                            self.print_ty(iface, ok);
                            self.src.h(" ok;\n");
                        }
                        if let Some(err) = err {
                            self.print_ty(iface, err);
                            self.src.h(" err;\n");
                        }
                        self.src.h("} val;\n");
                        self.src.h("}");
                    }
                } else {
                    unimplemented!();
                }
            }
            TypeDefKind::List(t) => {
                self.src.h("struct {\n");
                self.print_ty(iface, t);
                self.src.h(" *ptr;\n");
                self.src.h("size_t len;\n");
                self.src.h("}");
            }
            TypeDefKind::PushBuffer(t) => {
                unimplemented!();
            }
            TypeDefKind::PullBuffer(t) => {
                unimplemented!();
            }
        }
        self.src.h(" ");
        self.print_ty_name(iface, &Type::Id(ty));
        self.src.h("_t;\n");
        self.types
            .insert(ty, mem::replace(&mut self.src.header, prev));
    }

    fn is_empty_type(&self, iface: &Interface, ty: &Type) -> bool {
        let id = match ty {
            Type::Id(id) => *id,
            _ => return false,
        };
        match &iface.types[id].kind {
            TypeDefKind::Type(t) => self.is_empty_type(iface, t),
            TypeDefKind::Record(r) => r.fields.is_empty(),
            _ => false,
        }
    }

    fn intrinsic(&mut self, i: Intrinsic) -> String {
        match i {}
        // if let Some(name) = self.intrinsics.get(&i) {
        //     return name.clone();
        // }
        // let name = match i {
        //     // Intrinsic::StringRealloc => "__string_realloc",
        // };
        // let name = self.names.tmp(name);
        // self.intrinsics.insert(i, name.clone());
        // name
    }

    fn print_intrinsics(&mut self) {
        for (i, name) in self.intrinsics.drain() {
            match i {
                // Intrinsic::StringRealloc => {
                //     self.src.c(&format!(
                //         "
                //             static char *{}(int32_t ptr, int32_t len) {{
                //                 if (len == 0) {{
                //                     return strdup(\"\");
                //                 }}
                //                 char *ret = realloc((char*) ptr, len + 1);
                //                 ret[len] = 0;
                //                 return ret;
                //             }}
                //         ",
                //         name,
                //     ));
                // }
            }
        }
    }
}

impl Return {
    fn return_single(&mut self, iface: &Interface, ty: &Type, orig_ty: &Type) {
        let id = match ty {
            Type::Id(id) => *id,
            other => {
                self.scalar = Some(Scalar::Type(*orig_ty));
                return;
            }
        };
        match &iface.types[id].kind {
            TypeDefKind::Type(t) => self.return_single(iface, t, orig_ty),

            // record returns may become many return pointers with tuples
            TypeDefKind::Record(_) => self.splat_tuples(iface, ty, orig_ty),

            // other records/lists/buffers always go to return pointers
            TypeDefKind::List(_) | TypeDefKind::PushBuffer(_) | TypeDefKind::PullBuffer(_) => {
                self.retptrs.push(*orig_ty)
            }

            // pointers are scalars
            TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => {
                self.scalar = Some(Scalar::Type(*orig_ty));
            }

            // Enums are scalars (this includes bools)
            TypeDefKind::Variant(v) if v.is_enum() => {
                self.scalar = Some(Scalar::Type(*orig_ty));
            }

            TypeDefKind::Variant(r) => {
                // Unpack optional returns where a boolean discriminant is
                // returned and then the actual type returned is returned
                // through a return pointer.
                if let Some(ty) = r.as_option() {
                    self.scalar = Some(Scalar::OptionBool(*ty));
                    self.retptrs.push(*ty);
                    return;
                }

                // Unpack `expected<T, E>` returns where `E` looks like an enum
                // so we can return that in the scalar return and have `T` get
                // returned through the normal returns.
                if let Some((ok, err)) = r.as_expected() {
                    if let Some(Type::Id(err)) = err {
                        if let TypeDefKind::Variant(e) = &iface.types[*err].kind {
                            if e.is_enum() {
                                self.scalar = Some(Scalar::ExpectedEnum {
                                    ok: ok.cloned(),
                                    err: *err,
                                    max_err: e.cases.len(),
                                });
                                if let Some(ok) = ok {
                                    self.splat_tuples(iface, ok, ok);
                                }
                                return;
                            }
                        }
                    }
                }

                // If all that failed then just return the variant via a normal
                // return pointer
                self.retptrs.push(*orig_ty);
            }
        }
    }

    fn splat_tuples(&mut self, iface: &Interface, ty: &Type, orig_ty: &Type) {
        let id = match ty {
            Type::Id(id) => *id,
            other => {
                self.retptrs.push(*orig_ty);
                return;
            }
        };
        match &iface.types[id].kind {
            TypeDefKind::Record(r) if r.is_tuple() => {
                self.splat_tuple = true;
                self.retptrs.extend(r.fields.iter().map(|f| f.ty));
            }
            _ => self.retptrs.push(*orig_ty),
        }
    }
}

impl Generator for C {
    fn preprocess(&mut self, iface: &Interface, dir: Direction) {
        self.sizes.fill(dir, iface);
        self.in_import = dir == Direction::Import;

        for func in iface.functions.iter() {
            let sig = iface.wasm_signature(dir, func);
            if let Some(results) = sig.retptr {
                self.i64_return_pointer_area_size =
                    self.i64_return_pointer_area_size.max(results.len());
            }
        }
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        let prev = mem::take(&mut self.src.header);
        self.names.insert(&name.to_snake_case()).unwrap();
        if record.is_flags() {
            self.src.h("typedef ");
            let repr = iface
                .flags_repr(record)
                .expect("unsupported number of flags");
            self.src.h(int_repr(repr));
            self.src.h(" ");
            self.src.h(&name.to_snake_case());
            self.src.h("_t;\n");

            for (i, field) in record.fields.iter().enumerate() {
                self.src.h(&format!(
                    "#define {}_{} (1 << {})\n",
                    name.to_shouty_snake_case(),
                    field.name.to_shouty_snake_case(),
                    i,
                ));
            }
        } else {
            self.src.h("typedef struct {\n");
            for field in record.fields.iter() {
                self.print_ty(iface, &field.ty);
                self.src.h(" ");
                self.src.h(&field.name.to_snake_case());
                self.src.h(";\n");
            }
            self.src.h("} ");
            self.src.h(&name.to_snake_case());
            self.src.h("_t;\n");
        }

        self.types
            .insert(id, mem::replace(&mut self.src.header, prev));
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        let prev = mem::take(&mut self.src.header);
        self.names.insert(&name.to_snake_case()).unwrap();
        if variant.is_enum() {
            self.src.h("typedef ");
            self.src.h(int_repr(variant.tag));
            self.src.h(" ");
            self.src.h(&name.to_snake_case());
            self.src.h("_t;\n");
        } else {
            self.src.h("typedef struct {\n");
            self.src.h(int_repr(variant.tag));
            self.src.h(" tag;\n");
            self.src.h("union {\n");
            for case in variant.cases.iter() {
                if let Some(ty) = &case.ty {
                    self.print_ty(iface, ty);
                    self.src.h(" ");
                    self.src.h(&case_field_name(case));
                    self.src.h(";\n");
                }
            }
            self.src.h("} val;\n");
            self.src.h("} ");
            self.src.h(&name.to_snake_case());
            self.src.h("_t;\n");
        }
        for (i, case) in variant.cases.iter().enumerate() {
            self.src.h(&format!(
                "#define {}_{} {}\n",
                name.to_shouty_snake_case(),
                case.name.to_shouty_snake_case(),
                i,
            ));
        }

        self.types
            .insert(id, mem::replace(&mut self.src.header, prev));
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        drop((iface, ty));
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.header);
        self.src.h("typedef ");
        self.print_ty(iface, ty);
        self.src.h(" ");
        self.src.h(&name.to_snake_case());
        self.src.h("_t;\n");
        self.types
            .insert(id, mem::replace(&mut self.src.header, prev));
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.header);
        self.src.h("typedef struct {\n");
        self.print_ty(iface, ty);
        self.src.h(" *ptr;\n");
        self.src.h("size_t len;\n");
        self.src.h("} ");
        self.src.h(&name.to_snake_case());
        self.src.h("_t;\n");
        self.types
            .insert(id, mem::replace(&mut self.src.header, prev));
    }

    fn type_pointer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        const_: bool,
        ty: &Type,
        docs: &Docs,
    ) {
        drop((iface, _id, name, const_, ty, docs));
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        drop((iface, _id, name, ty, docs));
    }

    fn type_push_buffer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
    }

    fn type_pull_buffer(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        ty: &Type,
        docs: &Docs,
    ) {
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);
        let sig = iface.wasm_signature(Direction::Import, func);

        self.src.c(&format!(
            "__attribute__((import_module(\"{}\"), import_name(\"{}\")))\n",
            iface.name, func.name
        ));
        let import_name = self.names.tmp(&format!(
            "__wasm_import_{}_{}",
            iface.name.to_snake_case(),
            func.name.to_snake_case()
        ));
        match sig.results.len() {
            0 => self.src.c("void"),
            1 => self.src.c(wasm_type(sig.results[0])),
            _ => unimplemented!("multi-value return not supported"),
        }
        self.src.c(" ");
        self.src.c(&import_name);
        self.src.c("(");
        for (i, param) in sig.params.iter().enumerate() {
            if i > 0 {
                self.src.c(", ");
            }
            self.src.c(wasm_type(*param));
        }
        if sig.params.len() == 0 {
            self.src.c("void");
        }
        self.src.c(");\n");

        let c_sig = self.print_sig(iface, func);
        self.src.c(&c_sig.sig);
        self.src.c(" {\n");

        let mut f = FunctionBindgen::new(self, c_sig, &import_name);
        for (pointer, param) in f.sig.params.iter() {
            f.locals.insert(param).unwrap();

            if *pointer {
                f.params.push(format!("*{}", param));
            } else {
                f.params.push(param.clone());
            }
        }
        for ptr in f.sig.retptrs.iter() {
            f.locals.insert(ptr).unwrap();
        }
        iface.call(
            Direction::Import,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );

        let FunctionBindgen { src, .. } = f;

        self.src.c(&String::from(src));
        self.src.c("}\n");

        let src = mem::replace(&mut self.src, prev);
        self.imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.to_string(),
                src: src,
            });
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);
        let sig = iface.wasm_signature(Direction::Export, func);

        self.src.c(&format!(
            "__attribute__((export_name(\"{}\")))\n",
            func.name
        ));
        let import_name = self.names.tmp(&format!(
            "__wasm_export_{}_{}",
            iface.name.to_snake_case(),
            func.name.to_snake_case()
        ));

        let c_sig = self.print_sig(iface, func);
        let mut f = FunctionBindgen::new(self, c_sig, &import_name);
        match sig.results.len() {
            0 => f.gen.src.c("void"),
            1 => f.gen.src.c(wasm_type(sig.results[0])),
            _ => unimplemented!("multi-value return not supported"),
        }
        f.gen.src.c(" ");
        f.gen.src.c(&import_name);
        f.gen.src.c("(");
        for (i, param) in sig.params.iter().enumerate() {
            if i > 0 {
                f.gen.src.c(", ");
            }
            let name = f.locals.tmp("arg");
            f.gen.src.c(&format!("{} {}", wasm_type(*param), name));
            f.params.push(name);
        }
        if sig.params.len() == 0 {
            f.gen.src.c("void");
        }
        f.gen.src.c(") {\n");

        iface.call(
            Direction::Export,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );

        let FunctionBindgen { src, .. } = f;

        self.src.c(&String::from(src));
        self.src.c("}\n");

        let src = mem::replace(&mut self.src, prev);
        self.imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.to_string(),
                src: src,
            });
    }

    fn finish(&mut self, iface: &Interface, files: &mut Files) {
        self.src.h("#include <stdint.h>\n");
        self.src.h("#include <stdbool.h>\n");
        self.src.c("#include <bindings.h>\n");
        self.src.c("#include <stdlib.h>\n");

        for (_, resource) in iface.resources.iter() {
            self.src.h(&format!(
                "
                    typedef struct {{
                        uint32_t idx;
                    }} {}_t;
                ",
                resource.name.to_snake_case()
            ));
        }

        // Continuously generate anonymous types while we continue to find more
        //
        // First we take care of the public set of anonymous types. This will
        // iteratively print them and also remove any references from the
        // private set if we happen to also reference them.
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                self.print_anonymous_type(iface, ty);
            }
        }

        // Next we take care of private types. To do this we have basically the
        // same loop as above, after we switch the sets. We record, however,
        // all private types in a local set here to later determine if the type
        // needs to be in the C file or the H file.
        let mut private_types = HashSet::new();
        self.public_anonymous_types = mem::take(&mut self.private_anonymous_types);
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                private_types.insert(ty);
                self.print_anonymous_type(iface, ty);
            }
        }

        if self.needs_string {
            self.src.h(&format!(
                "
                    typedef struct {{
                        uint8_t *ptr;
                        size_t len;
                    }} {0}_string_t;

                    void {0}_string_init({0}_string_t *ret, const char *s);
                ",
                iface.name.to_snake_case(),
            ));
            self.src.c("#include <string.h>\n");
            self.src.c(&format!(
                "
                    void {0}_string_init({0}_string_t *ret, const char *s) {{
                        ret->ptr = (uint8_t*) s;
                        ret->len = strlen(s);
                    }}
                ",
                iface.name.to_snake_case(),
            ));
        }

        // Afterwards print all types. Note that this print must be in a
        // topological order, so we
        for id in iface.topological_types() {
            if let Some(ty) = self.types.get(&id) {
                if private_types.contains(&id) {
                    self.src.c(ty);
                } else {
                    self.src.h(ty);
                }
            }
        }

        if self.i64_return_pointer_area_size > 0 {
            self.src.c(&format!(
                "static int64_t RET_AREA[{}];\n",
                self.i64_return_pointer_area_size,
            ));
        }

        self.print_intrinsics();

        for (module, funcs) in mem::take(&mut self.imports) {
            for func in funcs {
                self.src.h(&func.src.header);
                self.src.c(&func.src.src);
            }
        }

        //for (module, exports) in mem::take(&mut self.exports) {
        //    let module = module.to_camel_case();
        //    self.src.ts(&format!("export class {} {{\n", module));
        //    self.src.js(&format!("export class {} {{\n", module));

        //    self.src.ts("
        //        // The WebAssembly instance that this class is operating with.
        //        // This is only available after the `instantiate` method has
        //        // been called.
        //        instance: WebAssembly.Instance;
        //    ");

        //    self.src.ts("
        //        // Constructs a new instance with internal state necessary to
        //        // manage a wasm instance.
        //        //
        //        // Note that this does not actually instantiate the WebAssembly
        //        // instance or module, you'll need to call the `instantiate`
        //        // method below to \"activate\" this class.
        //        constructor();
        //    ");
        //    if self.exported_resources.len() > 0 {
        //        self.src.js("constructor() {\n");
        //        for r in self.exported_resources.iter() {
        //            self.src
        //                .js(&format!("this._resource{}_slab = new Slab();\n", r.index()));
        //        }
        //        self.src.js("}\n");
        //    }

        //    self.src.ts("
        //        // This is a low-level method which can be used to add any
        //        // intrinsics necessary for this instance to operate to an
        //        // import object.
        //        //
        //        // The `import` object given here is expected to be used later
        //        // to actually instantiate the module this class corresponds to.
        //        // If the `instantiate` method below actually does the
        //        // instantiation then there's no need to call this method, but
        //        // if you're instantiating manually elsewhere then this can be
        //        // used to prepare the import object for external instantiation.
        //        add_to_imports(imports: any);
        //    ");
        //    self.src.js("add_to_imports(imports) {\n");
        //    if self.exported_resources.len() > 0 {
        //        self.src
        //            .js("if (!(\"canonical_abi\" in imports)) imports[\"canonical_abi\"] = {};\n");
        //    }
        //    for r in self.exported_resources.iter() {
        //        self.src.js(&format!(
        //            "
        //                imports.canonical_abi['resource_drop_{name}'] = i => {{
        //                    this._resource{idx}_slab.remove(i).drop();
        //                }};
        //                imports.canonical_abi['resource_clone_{name}'] = i => {{
        //                    const obj = this._resource{idx}_slab.get(i);
        //                    return this._resource{idx}_slab.insert(obj.clone())
        //                }};
        //                imports.canonical_abi['resource_get_{name}'] = i => {{
        //                    return this._resource{idx}_slab.get(i)._wasm_val;
        //                }};
        //                imports.canonical_abi['resource_new_{name}'] = i => {{
        //                    const dtor = this._exports['canonical_abi_drop_{name}'];
        //                    const registry = this._registry{idx};
        //                    return this._resource{idx}_slab.insert(new {class}(i, dtor, registry));
        //                }};
        //            ",
        //            name = iface.resources[*r].name,
        //            idx = r.index(),
        //            class = iface.resources[*r].name.to_camel_case(),
        //        ));
        //    }
        //    self.src.js("}\n");

        //    self.src.ts(&format!(
        //        "
        //            // Initializes this object with the provided WebAssembly
        //            // module/instance.
        //            //
        //            // This is intended to be a flexible method of instantiating
        //            // and completion of the initialization of this class. This
        //            // method must be called before interacting with the
        //            // WebAssembly object.
        //            //
        //            // The first argument to this method is where to get the
        //            // wasm from. This can be a whole bunch of different types,
        //            // for example:
        //            //
        //            // * A precompiled `WebAssembly.Module`
        //            // * A typed array buffer containing the wasm bytecode.
        //            // * A `Promise` of a `Response` which is used with
        //            //   `instantiateStreaming`
        //            // * A `Response` itself used with `instantiateStreaming`.
        //            // * An already instantiated `WebAssembly.Instance`
        //            //
        //            // If necessary the module is compiled, and if necessary the
        //            // module is instantiated. Whether or not it's necessary
        //            // depends on the type of argument provided to
        //            // instantiation.
        //            //
        //            // If instantiation is performed then the `imports` object
        //            // passed here is the list of imports used to instantiate
        //            // the instance. This method may add its own intrinsics to
        //            // this `imports` object too.
        //            instantiate(
        //                module: WebAssembly.Module | BufferSource | Promise<Response> | Response | WebAssembly.Instance,
        //                imports?: any,
        //            ): Promise<void>;
        //        ",
        //    ));
        //    self.src.js("
        //        async instantiate(module, imports) {
        //            imports = imports || {};
        //            this.add_to_imports(imports);
        //    ");

        //    // With intrinsics prep'd we can now instantiate the module. JS has
        //    // a ... variety of methods of instantiation, so we basically just
        //    // try to be flexible here.
        //    self.src.js("
        //        if (module instanceof WebAssembly.Instance) {
        //            this.instance = module;
        //        } else if (module instanceof WebAssembly.Module) {
        //            this.instance = await WebAssembly.instantiate(module, imports);
        //        } else if (module instanceof ArrayBuffer || module instanceof Uint8Array) {
        //            const { instance } = await WebAssembly.instantiate(module, imports);
        //            this.instance = instance;
        //        } else {
        //            const { instance } = await WebAssembly.instantiateStreaming(module, imports);
        //            this.instance = instance;
        //        }
        //        this._exports = this.instance.exports;
        //    ");

        //    // Exported resources all get a finalization registry, and we
        //    // created them after instantiation so we can pass the raw wasm
        //    // export as the destructor callback.
        //    for r in self.exported_resources.iter() {
        //        self.src.js(&format!(
        //            "this._registry{} = new FinalizationRegistry(this._exports['canonical_abi_drop_{}']);\n",
        //            r.index(),
        //            iface.resources[*r].name,
        //        ));
        //    }
        //    self.src.js("}\n");

        //    for func in exports.funcs.iter() {
        //        self.src.js(&func.js);
        //        self.src.ts(&func.ts);
        //    }
        //    self.src.ts("}\n");
        //    self.src.js("}\n");
        //}

        files.push("bindings.c", self.src.src.as_bytes());
        files.push("bindings.h", self.src.header.as_bytes());
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut C,
    locals: Ns,
    // tmp: usize,
    src: witx_bindgen_gen_core::Source,
    sig: CSig,
    func_to_call: &'a str,
    block_storage: Vec<witx_bindgen_gen_core::Source>,
    blocks: Vec<(String, Vec<String>)>,
    payloads: Vec<String>,
    params: Vec<String>,
}

impl<'a> FunctionBindgen<'a> {
    fn new(gen: &'a mut C, sig: CSig, func_to_call: &'a str) -> FunctionBindgen<'a> {
        FunctionBindgen {
            gen,
            sig,
            locals: Default::default(),
            src: Default::default(),
            func_to_call,
            block_storage: Vec::new(),
            blocks: Vec::new(),
            payloads: Vec::new(),
            params: Vec::new(),
        }
    }

    fn store_op(&mut self, op: &str, loc: &str) {
        self.src.push_str(loc);
        self.src.push_str(" = ");
        self.src.push_str(op);
        self.src.push_str(";\n");
    }

    fn load(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        results.push(format!("*(({}*) ({} + {}))", ty, operands[0], offset));
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        self.src.push_str(&format!(
            "*(({}*)({} + {})) = {};\n",
            ty, operands[1], offset, operands[0]
        ));
    }

    fn store_in_retptrs(&mut self, operands: &[String]) {
        if self.sig.ret.splat_tuple {
            assert_eq!(operands.len(), 1);
            let op = &operands[0];
            for (i, ptr) in self.sig.retptrs.clone().into_iter().enumerate() {
                self.store_op(&format!("{}.f{}", op, i), &format!("*{}", ptr));
            }
            // ...
        } else {
            assert_eq!(operands.len(), self.sig.retptrs.len());
            for (op, ptr) in operands.iter().zip(self.sig.retptrs.clone()) {
                self.store_op(op, &format!("*{}", ptr));
            }
        }
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, to_restore);
        self.blocks.push((src.into(), mem::take(operands)));
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> String {
        unimplemented!()
    }

    fn i64_return_pointer_area(&mut self, amt: usize) -> String {
        assert!(amt <= self.gen.i64_return_pointer_area_size);
        let ptr = self.locals.tmp("ptr");
        self.src
            .push_str(&format!("int32_t {} = (int32_t) &RET_AREA;\n", ptr));
        ptr
    }

    fn is_list_canonical(&self, iface: &Interface, ty: &Type) -> bool {
        iface.all_bits_valid(ty)
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::ConstZero { tys } => {
                for _ in tys.iter() {
                    results.push("0".to_string());
                }
            }

            // TODO: checked?
            Instruction::U8FromI32 => results.push(format!("(uint8_t) ({})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("(int8_t) ({})", operands[0])),
            Instruction::U16FromI32 => results.push(format!("(uint16_t) ({})", operands[0])),
            Instruction::S16FromI32 => results.push(format!("(int16_t) ({})", operands[0])),
            Instruction::U32FromI32 => results.push(format!("(uint32_t) ({})", operands[0])),
            Instruction::S32FromI32 | Instruction::S64FromI64 => results.push(operands[0].clone()),
            Instruction::U64FromI64 => results.push(format!("(uint64_t) ({})", operands[0])),

            Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32 => {
                results.push(format!("(int32_t) ({})", operands[0]));
            }
            Instruction::I32FromS32 | Instruction::I64FromS64 => results.push(operands[0].clone()),
            Instruction::I64FromU64 => {
                results.push(format!("(int64_t) ({})", operands[0]));
            }

            // f32/f64 have the same representation in the import type and in C,
            // so no conversions necessary.
            Instruction::F32FromIf32
            | Instruction::F64FromIf64
            | Instruction::If32FromF32
            | Instruction::If64FromF64 => {
                results.push(operands[0].clone());
            }

            // TODO: checked
            Instruction::CharFromI32 => {
                results.push(format!("(uint32_t) ({})", operands[0]));
            }
            Instruction::I32FromChar => {
                results.push(format!("(int32_t) ({})", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    let op = op;
                    match cast {
                        Bitcast::I32ToF32 | Bitcast::I64ToF32 => {
                            results
                                .push(format!("((union {{ int32_t a; float b; }}){{ {} }}).b", op));
                        }
                        Bitcast::F32ToI32 | Bitcast::F32ToI64 => {
                            results
                                .push(format!("((union {{ float a; int32_t b; }}){{ {} }}).b", op));
                        }
                        Bitcast::F32ToF64 | Bitcast::F64ToF32 => results.push(op.to_string()),
                        Bitcast::I64ToF64 => {
                            results.push(format!(
                                "((union {{ int64_t a; double b; }}){{ {} }}).b",
                                op
                            ));
                        }
                        Bitcast::F64ToI64 => {
                            results.push(format!(
                                "((union {{ double a; int64_t b; }}){{ {} }}).b",
                                op
                            ));
                        }
                        Bitcast::I32ToI64 => {
                            results.push(format!("(int64_t) {}", op));
                        }
                        Bitcast::I64ToI32 => {
                            results.push(format!("(int32_t) {}", op));
                        }
                        Bitcast::None => results.push(op.to_string()),
                    }
                }
            }

            Instruction::I32FromOwnedHandle { .. } | Instruction::I32FromBorrowedHandle { .. } => {
                results.push(format!("({}).idx", operands[0]));
            }

            Instruction::HandleBorrowedFromI32 { ty, .. }
            | Instruction::HandleOwnedFromI32 { ty, .. } => {
                results.push(format!(
                    "({}_t){{ {} }}",
                    iface.resources[*ty].name.to_snake_case(),
                    operands[0],
                ));
            }

            Instruction::RecordLower { record, .. } => {
                if record.is_tuple() {
                    let op = &operands[0];
                    for i in 0..record.fields.len() {
                        results.push(format!("({}).f{}", op, i));
                    }
                } else {
                    let op = &operands[0];
                    for f in record.fields.iter() {
                        results.push(format!("({}).{}", op, f.name.to_snake_case()));
                    }
                }
            }
            Instruction::RecordLift {
                record, name, ty, ..
            } => {
                let name = self.gen.type_string(iface, &Type::Id(*ty));
                let mut result = format!("({}) {{", name);
                for op in operands {
                    result.push_str(&format!("{},", op));
                }
                result.push_str("}");
                results.push(result);
            }

            // TODO: checked
            Instruction::FlagsLower { record, .. } | Instruction::FlagsLift { record, .. } => {
                match record.num_i32s() {
                    0 | 1 => results.push(operands.pop().unwrap()),
                    _ => panic!("unsupported bitflags"),
                }
            }
            Instruction::FlagsLower64 { record, .. } | Instruction::FlagsLift64 { record, .. } => {
                results.push(operands.pop().unwrap());
            }

            Instruction::VariantPayloadName => {
                let name = self.locals.tmp("payload");
                results.push(format!("*{}", name));
                self.payloads.push(name);
            }
            Instruction::VariantLower {
                variant,
                results: result_types,
                name,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let payload = self.payloads.pop().unwrap();

                if results.len() == 1 && variant.is_enum() {
                    results.push(format!("(int32_t) {}", operands[0]));
                    return;
                }

                let mut variant_results = Vec::new();
                for ty in result_types.iter() {
                    let name = self.locals.tmp("variant");
                    results.push(name.clone());
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    variant_results.push(name);
                }

                let expr_to_match = if variant.is_enum() {
                    operands[0].to_string()
                } else {
                    format!("({}).tag", operands[0])
                };

                self.src
                    .push_str(&format!("switch ((int32_t) {}) {{\n", expr_to_match));
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    self.src.push_str(&format!("case {}: {{\n", i));
                    if let Some(ty) = &case.ty {
                        if !self.gen.is_empty_type(iface, ty) {
                            let ty = self.gen.type_string(iface, ty);
                            self.src.push_str(&format!(
                                "const {} *{} = &({}).val",
                                ty, payload, operands[0],
                            ));
                            if !variant.as_option().is_some() {
                                self.src.push_str(".");
                                self.src.push_str(&case_field_name(case));
                            }
                            self.src.push_str(";\n");
                        }
                    }
                    self.src.push_str(&block);

                    for (name, result) in variant_results.iter().zip(&block_results) {
                        self.src.push_str(&format!("{} = {};\n", name, result));
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }

            Instruction::VariantLift {
                variant, name, ty, ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                if variant.is_enum() {
                    return results.push(operands.pop().unwrap());
                }

                let ty = self.gen.type_string(iface, &Type::Id(*ty));
                let result = self.locals.tmp("variant");
                self.src.push_str(&format!("{} {};\n", ty, result));
                self.src
                    .push_str(&format!("{}.tag = {};\n", result, operands[0]));
                self.src
                    .push_str(&format!("switch ((int32_t) {}.tag) {{\n", result));
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    self.src.push_str(&format!("case {}: {{\n", i));
                    self.src.push_str(&block);

                    if case.ty.is_some() {
                        assert!(block_results.len() == 1);
                        let mut dst = format!("{}.val", result);
                        if !variant.as_option().is_some() {
                            dst.push_str(".");
                            dst.push_str(&case_field_name(case));
                        }
                        self.store_op(&block_results[0], &dst);
                    } else {
                        assert!(block_results.is_empty());
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
                results.push(result);
            }

            Instruction::ListCanonLower { element, .. } => {
                results.push(format!("(int32_t) ({}).ptr", operands[0]));
                results.push(format!("(int32_t) ({}).len", operands[0]));
            }
            Instruction::ListCanonLift { element, ty, .. } => {
                let list_name = self.gen.type_string(iface, &Type::Id(*ty));
                let elem_name = match element {
                    Type::Char => "uint8_t".into(),
                    _ => self.gen.type_string(iface, element),
                };
                results.push(format!(
                    "({}) {{ ({}*)({}), (size_t)({}) }}",
                    list_name, elem_name, operands[0], operands[1]
                ));
            }

            Instruction::ListLower { element, .. } => {
                let _body = self.blocks.pop().unwrap();
                results.push(format!("(int32_t) ({}).ptr", operands[0]));
                results.push(format!("(int32_t) ({}).len", operands[0]));
            }

            Instruction::ListLift { element, free, ty } => {
                let _body = self.blocks.pop().unwrap();
                let list_name = self.gen.type_string(iface, &Type::Id(*ty));
                let elem_name = match element {
                    Type::Char => "uint8_t".into(),
                    _ => self.gen.type_string(iface, element),
                };
                results.push(format!(
                    "({}) {{ ({}*)({}), (size_t)({}) }}",
                    list_name, elem_name, operands[0], operands[1]
                ));
            }
            Instruction::IterElem => results.push("e".to_string()),
            Instruction::IterBasePointer => results.push("base".to_string()),

            // Instruction::BufferLiftPtrLen { push, ty } => {
            //     let (block, block_results) = self.blocks.pop().unwrap();
            //     // assert_eq!(block_results.len(), 1);
            //     let tmp = self.tmp();
            //     self.needs_memory = true;
            //     self.src
            //         .js(&format!("const ptr{} = {};\n", tmp, operands[1]));
            //     self.src
            //         .js(&format!("const len{} = {};\n", tmp, operands[2]));
            //     if let Some(ty) = self.gen.array_ty(iface, ty) {
            //         results.push(format!("new {}(memory.buffer, ptr{}, len{1})", ty, tmp));
            //     } else {
            //         let size = self.gen.sizes.size(ty);
            //         if *push {
            //             self.gen.needs_push_buffer = true;
            //             assert!(block_results.is_empty());
            //             results.push(format!(
            //                 "new PushBuffer(ptr{}, len{0}, {}, (e, base) => {{
            //                     {}
            //                 }})",
            //                 tmp, size, block
            //             ));
            //         } else {
            //             self.gen.needs_pull_buffer = true;
            //             assert_eq!(block_results.len(), 1);
            //             results.push(format!(
            //                 "new PullBuffer(ptr{}, len{0}, {}, (base) => {{
            //                     {}
            //                     return {};
            //                 }})",
            //                 tmp, size, block, block_results[0],
            //             ));
            //         }
            //     }
            // }

            //    Instruction::BufferLowerHandle { push, ty } => {
            //        let block = self.blocks.pop().unwrap();
            //        let size = self.sizes.size(ty);
            //        let tmp = self.tmp();
            //        let handle = format!("handle{}", tmp);
            //        let closure = format!("closure{}", tmp);
            //        self.needs_buffer_transaction = true;
            //        if iface.all_bits_valid(ty) {
            //            let method = if *push { "push_out_raw" } else { "push_in_raw" };
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.{}({}) }};\n",
            //                handle, method, operands[0],
            //            ));
            //        } else if *push {
            //            self.closures.push_str(&format!(
            //                "let {} = |memory: &wasmtime::Memory, base: i32| {{
            //                    Ok(({}, {}))
            //                }};\n",
            //                closure, block, size,
            //            ));
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.push_out({}, &{}) }};\n",
            //                handle, operands[0], closure,
            //            ));
            //        } else {
            //            let start = self.src.len();
            //            self.print_ty(iface, ty, TypeMode::AllBorrowed("'_"));
            //            let ty = self.src[start..].to_string();
            //            self.src.truncate(start);
            //            self.closures.push_str(&format!(
            //                "let {} = |memory: &wasmtime::Memory, base: i32, e: {}| {{
            //                    {};
            //                    Ok({})
            //                }};\n",
            //                closure, ty, block, size,
            //            ));
            //            self.push_str(&format!(
            //                "let {} = unsafe {{ buffer_transaction.push_in({}, &{}) }};\n",
            //                handle, operands[0], closure,
            //            ));
            //        }
            //        results.push(format!("{}", handle));
            //    }
            Instruction::CallWasm {
                module: _,
                name,
                sig,
            } => {
                match sig.results.len() {
                    0 => {}
                    1 => {
                        self.src.push_str(wasm_type(sig.results[0]));
                        let ret = self.locals.tmp("ret");
                        self.src.push_str(&format!(" {} = ", ret));
                        results.push(ret);
                    }
                    _ => unimplemented!(),
                }
                self.src.push_str(self.func_to_call);
                self.src.push_str("(");
                for (i, op) in operands.iter().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.src.push_str(op);
                }
                self.src.push_str(");\n");
            }

            Instruction::CallInterface { module: _, func } => {
                let mut args = String::new();
                for (i, (op, (byref, _))) in operands.iter().zip(&self.sig.params).enumerate() {
                    if i > 0 {
                        args.push_str(", ");
                    }
                    if *byref {
                        let name = self.locals.tmp("arg");
                        let ty = self.gen.type_string(iface, &func.params[i].1);
                        self.src.push_str(&format!("{} {} = {};\n", ty, name, op));
                        args.push_str("&");
                        args.push_str(&name);
                    } else {
                        args.push_str(op);
                    }
                }
                match &self.sig.ret.scalar {
                    None => {
                        let mut retptrs = Vec::new();
                        for ty in self.sig.ret.retptrs.iter() {
                            let name = self.locals.tmp("ret");
                            let ty = self.gen.type_string(iface, ty);
                            self.src.push_str(&format!("{} {};\n", ty, name));
                            if args.len() > 0 {
                                args.push_str(", ");
                            }
                            args.push_str("&");
                            args.push_str(&name);
                            retptrs.push(name);
                        }
                        self.src
                            .push_str(&format!("{}({});\n", self.sig.name, args));
                        if self.sig.ret.splat_tuple {
                            let ty = self.gen.type_string(iface, &func.results[0].1);
                            results.push(format!("({}){{ {} }}", ty, retptrs.join(", ")));
                        } else if self.sig.retptrs.len() > 0 {
                            results.extend(retptrs);
                        }
                    }
                    Some(Scalar::Type(_)) => {
                        assert_eq!(func.results.len(), 1);
                        let ret = self.locals.tmp("ret");
                        let ty = self.gen.type_string(iface, &func.results[0].1);
                        self.src
                            .push_str(&format!("{} {} = {}({});\n", ty, ret, self.sig.name, args));
                        results.push(ret);
                    }
                    Some(Scalar::OptionBool(ty)) => {
                        let ret = self.locals.tmp("ret");
                        let val = self.locals.tmp("val");
                        if args.len() > 0 {
                            args.push_str(", ");
                        }
                        args.push_str("&");
                        args.push_str(&val);
                        let payload_ty = self.gen.type_string(iface, ty);
                        self.src.push_str(&format!("{} {};\n", payload_ty, val));
                        self.src
                            .push_str(&format!("bool {} = {}({});\n", ret, self.sig.name, args));
                        let option_ty = self.gen.type_string(iface, &func.results[0].1);
                        let option_ret = self.locals.tmp("ret");
                        self.src.push_str(&format!(
                            "
                                {ty} {ret};
                                {ret}.tag = {tag};
                                {ret}.val = {val};
                            ",
                            ty = option_ty,
                            ret = option_ret,
                            tag = ret,
                            val = val,
                        ));
                        results.push(option_ret);
                    }
                    Some(Scalar::ExpectedEnum { ok, err, max_err }) => {
                        let ret = self.locals.tmp("ret");
                        let mut ok_names = Vec::new();
                        for ty in self.sig.ret.retptrs.iter() {
                            let val = self.locals.tmp("ok");
                            if args.len() > 0 {
                                args.push_str(", ");
                            }
                            args.push_str("&");
                            args.push_str(&val);
                            let ty = self.gen.type_string(iface, ty);
                            self.src.push_str(&format!("{} {};\n", ty, val));
                            ok_names.push(val);
                        }
                        let err_ty = self.gen.type_string(iface, &Type::Id(*err));
                        self.src.push_str(&format!(
                            "{} {} = {}({});\n",
                            err_ty, ret, self.sig.name, args,
                        ));
                        let expected_ty = self.gen.type_string(iface, &func.results[0].1);
                        let expected_ret = self.locals.tmp("ret");
                        self.src.push_str(&format!(
                            "
                                {ty} {ret};
                                if ({tag} <= {max}) {{
                                    {ret}.tag = 1;
                                    {ret}.val.err = {tag};
                                }} else {{
                                    {ret}.tag = 0;
                                    {set_ok}
                                }}
                            ",
                            ty = expected_ty,
                            ret = expected_ret,
                            tag = ret,
                            max = max_err,
                            set_ok = if self.sig.ret.retptrs.len() == 0 {
                                String::new()
                            } else if self.sig.ret.splat_tuple {
                                let mut s = String::new();
                                for (i, name) in ok_names.iter().enumerate() {
                                    s.push_str(&format!(
                                        "{}.val.ok.f{} = {};\n",
                                        expected_ret, i, name,
                                    ));
                                }
                                s
                            } else {
                                let name = ok_names.pop().unwrap();
                                format!("{}.val.ok = {};", expected_ret, name)
                            },
                        ));
                        results.push(expected_ret);
                    }
                }
            }
            Instruction::Return { amt, func } if self.gen.in_import => match self.sig.ret.scalar {
                None => self.store_in_retptrs(operands),
                Some(Scalar::Type(_)) => {
                    assert_eq!(operands.len(), 1);
                    self.src.push_str("return ");
                    self.src.push_str(&operands[0]);
                    self.src.push_str(";\n");
                }
                Some(Scalar::OptionBool(_)) => {
                    assert_eq!(operands.len(), 1);
                    let variant = &operands[0];
                    self.store_in_retptrs(&[format!("{}.val", variant)]);
                    self.src.push_str("return ");
                    self.src.push_str(&variant);
                    self.src.push_str(".tag;\n");
                }
                Some(Scalar::ExpectedEnum { .. }) => {
                    assert_eq!(operands.len(), 1);
                    let variant = &operands[0];
                    if self.sig.retptrs.len() > 0 {
                        self.store_in_retptrs(&[format!("{}.val.ok", variant)]);
                    }
                    self.src
                        .push_str(&format!("return {}.tag ? {0}.val.err : -1;\n", variant,));
                }
            },
            Instruction::Return { amt, .. } => {
                assert!(*amt <= 1);
                if *amt == 1 {
                    self.src.push_str(&format!("return {};\n", operands[0]));
                }
            }

            Instruction::I32Load { offset } => self.load("int32_t", *offset, operands, results),
            Instruction::I64Load { offset } => self.load("int64_t", *offset, operands, results),
            Instruction::F32Load { offset } => self.load("float", *offset, operands, results),
            Instruction::F64Load { offset } => self.load("double", *offset, operands, results),
            Instruction::I32Store { offset } => self.store("int32_t", *offset, operands),
            Instruction::I64Store { offset } => self.store("int64_t", *offset, operands),
            Instruction::F32Store { offset } => self.store("float", *offset, operands),
            Instruction::F64Store { offset } => self.store("double", *offset, operands),

            Instruction::I32Load8U { offset }
            | Instruction::I32Load8S { offset }
            | Instruction::I32Load16U { offset }
            | Instruction::I32Load16S { offset } => {
                drop(offset);
                self.src.push_str("INVALID");
                results.push("INVALID".to_string());
            }
            Instruction::I32Store8 { offset } | Instruction::I32Store16 { offset } => {
                drop(offset);
                self.src.push_str("INVALID");
            }

            // Instruction::Witx { instr } => match instr {
            //     WitxInstruction::PointerFromI32 { .. } => results.push(operands[0].clone()),
            //     i => unimplemented!("{:?}", i),
            // },
            i => unimplemented!("{:?}", i),
        }
    }
}

#[derive(Default)]
struct Source {
    header: witx_bindgen_gen_core::Source,
    src: witx_bindgen_gen_core::Source,
}

impl Source {
    fn c(&mut self, s: &str) {
        self.src.push_str(s);
    }
    fn h(&mut self, s: &str) {
        self.header.push_str(s);
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int32_t",
        WasmType::I64 => "int64_t",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
    }
}

fn int_repr(ty: Int) -> &'static str {
    match ty {
        Int::U8 => "uint8_t",
        Int::U16 => "uint16_t",
        Int::U32 => "uint32_t",
        Int::U64 => "uint64_t",
    }
}

fn case_field_name(case: &Case) -> String {
    if case.name.parse::<u32>().is_ok() {
        format!("f{}", case.name)
    } else {
        case.name.to_snake_case()
    }
}
