mod component_type_object;

use heck::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use std::mem;
use wit_bindgen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Ns, WorldGenerator,
};
use wit_component::StringEncoding;

#[derive(Default)]
struct C {
    src: Source,
    opts: Opts,
    includes: Vec<String>,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
    names: Ns,
    needs_string: bool,
    world: String,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Skip emitting component allocation helper functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub no_helpers: bool,
    /// Set component string encoding
    #[cfg_attr(feature = "clap", arg(long, default_value_t = StringEncoding::default()))]
    pub string_encoding: StringEncoding,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        let mut r = C::default();
        r.opts = self.clone();
        Box::new(r)
    }
}

#[derive(Debug, Default)]
struct Return {
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
    Void,
    OptionBool(Type),
    ResultBool(Option<Type>, Option<Type>),
    Type(Type),
}

impl WorldGenerator for C {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.world = name.to_string();
        self.sizes.fill(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &str,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let prev = self.interface_names.insert(id, name.to_string());
        assert!(prev.is_none());
        let mut gen = self.interface(name, resolve, true);
        gen.interface = Some(id);
        gen.types(id);

        for (i, (_name, func)) in resolve.interfaces[id].functions.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Imported Functions from `{name}`");
            }
            gen.import(name, func);
        }

        gen.finish();

        gen.gen.src.append(&gen.src);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        let mut gen = self.interface(name, resolve, true);

        for (i, (_name, func)) in funcs.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Imported Functions from `{name}`");
            }
            gen.import("$root", func);
        }

        gen.finish();

        gen.gen.src.append(&gen.src);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &str,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        self.interface_names.insert(id, name.to_string());
        let mut gen = self.interface(name, resolve, false);
        gen.interface = Some(id);
        gen.types(id);

        for (i, (_name, func)) in resolve.interfaces[id].functions.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Exported Functions from `{name}`");
            }
            gen.export(func, Some(name));
        }

        gen.finish();

        gen.gen.src.append(&gen.src);
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        let mut gen = self.interface(name, resolve, false);

        for (i, (_name, func)) in funcs.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Exported Functions from `{name}`");
            }
            gen.export(func, None);
        }

        gen.finish();

        gen.gen.src.append(&gen.src);
    }

    fn export_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = &resolve.worlds[world].name;
        let mut gen = self.interface(name, resolve, false);
        for (name, id) in types {
            gen.define_type(name, *id);
        }
        gen.finish();
        gen.gen.src.append(&gen.src);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        let world = &resolve.worlds[id];
        let linking_symbol = component_type_object::linking_symbol(&world.name);
        self.include("<stdlib.h>");
        let snake = world.name.to_snake_case();
        uwrite!(
            self.src.c_adapters,
            "
               extern void {linking_symbol}(void);
               void {linking_symbol}_public_use_in_this_compilation_unit(void) {{
                   {linking_symbol}();
               }}
           ",
        );

        self.print_intrinsics();

        if self.needs_string {
            self.include("<string.h>");
            let (strlen, size) = match self.opts.string_encoding {
                StringEncoding::UTF8 => (format!("strlen(s)"), 1),
                StringEncoding::UTF16 => {
                    self.include("<uchar.h>");
                    uwrite!(
                        self.src.h_helpers,
                        "
                            size_t {snake}_string_len(const char16_t* s);
                        ",
                    );
                    uwrite!(
                        self.src.c_helpers,
                        "
                            size_t {snake}_string_len(const char16_t* s) {{
                                char16_t* c = (char16_t*)s;
                                for (; *c; ++c);
                                return c-s;
                            }}
                        ",
                    );
                    (format!("{snake}_string_len(s)"), 2)
                }
                StringEncoding::CompactUTF16 => unimplemented!(),
            };
            let ty = self.char_type();
            uwrite!(
                self.src.h_helpers,
                "
                   void {snake}_string_set({snake}_string_t *ret, const {ty} *s);
                   void {snake}_string_dup({snake}_string_t *ret, const {ty} *s);
                   void {snake}_string_free({snake}_string_t *ret);\
               ",
            );
            uwrite!(
                self.src.c_helpers,
                "
                   void {snake}_string_set({snake}_string_t *ret, const {ty} *s) {{
                       ret->ptr = ({ty}*) s;
                       ret->len = {strlen};
                   }}

                   void {snake}_string_dup({snake}_string_t *ret, const {ty} *s) {{
                       ret->len = {strlen};
                       ret->ptr = cabi_realloc(NULL, 0, {size}, ret->len * {size});
                       memcpy(ret->ptr, s, ret->len * {size});
                   }}

                   void {snake}_string_free({snake}_string_t *ret) {{
                       if (ret->len > 0) {{
                           free(ret->ptr);
                       }}
                       ret->ptr = NULL;
                       ret->len = 0;
                   }}
               ",
            );
        }

        let mut h_str = wit_bindgen_core::Source::default();

        uwrite!(
            h_str,
            "#ifndef __BINDINGS_{0}_H
            #define __BINDINGS_{0}_H
            #ifdef __cplusplus
            extern \"C\" {{",
            world.name.to_shouty_snake_case(),
        );

        // Deindent the extern C { declaration
        h_str.deindent(1);
        uwriteln!(h_str, "\n#endif\n");

        self.include("<stdint.h>");
        self.include("<stdbool.h>");

        for include in self.includes.iter() {
            uwriteln!(h_str, "#include {include}");
        }

        let mut c_str = wit_bindgen_core::Source::default();
        uwriteln!(c_str, "#include \"{snake}.h\"");
        if c_str.len() > 0 {
            c_str.push_str("\n");
        }
        c_str.push_str(&self.src.c_defs);
        c_str.push_str(&self.src.c_fns);

        if self.needs_string {
            uwriteln!(
                h_str,
                "
                typedef struct {{\n\
                  {ty} *ptr;\n\
                  size_t len;\n\
                }} {snake}_string_t;",
                ty = self.char_type(),
            );
        }
        if self.src.h_defs.len() > 0 {
            h_str.push_str(&self.src.h_defs);
        }

        h_str.push_str(&self.src.h_fns);

        if !self.opts.no_helpers && self.src.h_helpers.len() > 0 {
            uwriteln!(h_str, "\n// Helper Functions");
            h_str.push_str(&self.src.h_helpers);
            h_str.push_str("\n");
        }

        if !self.opts.no_helpers && self.src.c_helpers.len() > 0 {
            uwriteln!(c_str, "\n// Helper Functions");
            c_str.push_str(self.src.c_helpers.as_mut_string());
        }

        uwriteln!(c_str, "\n// Component Adapters");

        // Declare a statically-allocated return area, if needed. We only do
        // this for export bindings, because import bindings allocate their
        // return-area on the stack.
        if self.return_pointer_area_size > 0 {
            // Automatic indentation avoided due to `extern "C" {` declaration
            uwrite!(
                c_str,
                "
                __attribute__((aligned({})))
                static uint8_t RET_AREA[{}];
                ",
                self.return_pointer_area_align,
                self.return_pointer_area_size,
            );
        }
        c_str.push_str(&self.src.c_adapters);

        uwriteln!(
            h_str,
            "
            #ifdef __cplusplus
            }}
            #endif
            #endif"
        );

        files.push(&format!("{snake}.c"), c_str.as_bytes());
        files.push(&format!("{snake}.h"), h_str.as_bytes());
        files.push(
            &format!("{snake}_component_type.o",),
            component_type_object::object(resolve, id, self.opts.string_encoding)
                .unwrap()
                .as_slice(),
        );
    }
}

impl C {
    fn interface<'a>(
        &'a mut self,
        name: &'a str,
        resolve: &'a Resolve,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            name,
            src: Source::default(),
            gen: self,
            resolve,
            interface: None,
            public_anonymous_types: Default::default(),
            private_anonymous_types: Default::default(),
            types: Default::default(),
            in_import,
        }
    }

    fn include(&mut self, s: &str) {
        self.includes.push(s.to_string());
    }

    fn char_type(&self) -> &'static str {
        match self.opts.string_encoding {
            StringEncoding::UTF8 => "char",
            StringEncoding::UTF16 => "char16_t",
            StringEncoding::CompactUTF16 => panic!("Compact UTF16 unsupported"),
        }
    }
}

struct InterfaceGenerator<'a> {
    name: &'a str,
    src: Source,
    in_import: bool,
    gen: &'a mut C,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,

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
    types: HashMap<TypeId, wit_bindgen_core::Source>,
}

impl C {
    fn print_intrinsics(&mut self) {
        // Note that these intrinsics are declared as `weak` so they can be
        // overridden from some other symbol.
        self.src.c_fns(
            r#"
                __attribute__((weak, export_name("cabi_realloc")))
                void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
                    if (new_size == 0) return (void*) align;
                    void *ret = realloc(ptr, new_size);
                    if (!ret) abort();
                    return ret;
                }
            "#,
        );
    }
}

impl Return {
    fn return_single(&mut self, resolve: &Resolve, ty: &Type, orig_ty: &Type) {
        let id = match ty {
            Type::Id(id) => *id,
            Type::String => {
                self.retptrs.push(*orig_ty);
                return;
            }
            _ => {
                self.scalar = Some(Scalar::Type(*orig_ty));
                return;
            }
        };
        match &resolve.types[id].kind {
            TypeDefKind::Type(t) => return self.return_single(resolve, t, orig_ty),

            // Flags are returned as their bare values, and enums are scalars
            TypeDefKind::Flags(_) | TypeDefKind::Enum(_) => {
                self.scalar = Some(Scalar::Type(*orig_ty));
                return;
            }

            // Unpack optional returns where a boolean discriminant is
            // returned and then the actual type returned is returned
            // through a return pointer.
            TypeDefKind::Option(ty) => {
                self.scalar = Some(Scalar::OptionBool(*ty));
                self.retptrs.push(*ty);
                return;
            }

            // Unpack a result as a boolean return type, with two
            // return pointers for ok and err values
            TypeDefKind::Result(r) => {
                if let Some(ok) = r.ok {
                    self.retptrs.push(ok);
                }
                if let Some(err) = r.err {
                    self.retptrs.push(err);
                }
                self.scalar = Some(Scalar::ResultBool(r.ok, r.err));
                return;
            }

            // These types are always returned indirectly.
            TypeDefKind::Tuple(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::List(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {}

            TypeDefKind::Future(_) => todo!("return_single for future"),
            TypeDefKind::Stream(_) => todo!("return_single for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }

        self.retptrs.push(*orig_ty);
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, id: TypeId, name: &str, record: &Record, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        for field in record.fields.iter() {
            self.print_ty(SourceType::HDefs, &field.ty);
            self.src.h_defs(" ");
            self.src.h_defs(&to_c_ident(&field.name));
            self.src.h_defs(";\n");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_tuple(&mut self, id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        for (i, ty) in tuple.types.iter().enumerate() {
            self.print_ty(SourceType::HDefs, ty);
            uwriteln!(self.src.h_defs, " f{i};");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef ");
        let repr = flags_repr(flags);
        self.src.h_defs(int_repr(repr));
        self.src.h_defs(" ");
        self.print_typedef_target(name);

        if flags.flags.len() > 0 {
            self.src.h_defs("\n");
        }
        for (i, flag) in flags.flags.iter().enumerate() {
            uwriteln!(
                self.src.h_defs,
                "#define {}_{}_{} (1 << {})",
                self.name.to_shouty_snake_case(),
                name.to_shouty_snake_case(),
                flag.name.to_shouty_snake_case(),
                i,
            );
        }

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_variant(&mut self, id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs(int_repr(variant.tag()));
        self.src.h_defs(" tag;\n");
        self.src.h_defs("union {\n");
        for case in variant.cases.iter() {
            if let Some(ty) = self.get_nonempty_type(case.ty.as_ref()) {
                self.print_ty(SourceType::HDefs, ty);
                self.src.h_defs(" ");
                self.src.h_defs(&to_c_ident(&case.name));
                self.src.h_defs(";\n");
            }
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        if variant.cases.len() > 0 {
            self.src.h_defs("\n");
        }
        for (i, case) in variant.cases.iter().enumerate() {
            uwriteln!(
                self.src.h_defs,
                "#define {}_{}_{} {}",
                self.name.to_shouty_snake_case(),
                name.to_shouty_snake_case(),
                case.name.to_shouty_snake_case(),
                i,
            );
        }

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_union(&mut self, id: TypeId, name: &str, union: &Union, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs(int_repr(union.tag()));
        self.src.h_defs(" tag;\n");
        self.src.h_defs("union {\n");
        for (i, case) in union.cases.iter().enumerate() {
            self.print_ty(SourceType::HDefs, &case.ty);
            uwriteln!(self.src.h_defs, " f{i};");
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_option(&mut self, id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs("bool is_some;\n");
        if !self.is_empty_type(payload) {
            self.print_ty(SourceType::HDefs, payload);
            self.src.h_defs(" val;\n");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_result(&mut self, id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs("bool is_err;\n");
        self.src.h_defs("union {\n");
        if let Some(ok) = self.get_nonempty_type(result.ok.as_ref()) {
            self.print_ty(SourceType::HDefs, ok);
            self.src.h_defs(" ok;\n");
        }
        if let Some(err) = self.get_nonempty_type(result.err.as_ref()) {
            self.print_ty(SourceType::HDefs, err);
            self.src.h_defs(" err;\n");
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(name);

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        uwrite!(self.src.h_defs, "\n");
        self.docs(docs);
        let int_t = int_repr(enum_.tag());
        uwrite!(self.src.h_defs, "typedef {int_t} ");
        self.print_typedef_target(name);

        if enum_.cases.len() > 0 {
            self.src.h_defs("\n");
        }
        for (i, case) in enum_.cases.iter().enumerate() {
            uwriteln!(
                self.src.h_defs,
                "#define {}_{}_{} {}",
                self.name.to_shouty_snake_case(),
                name.to_shouty_snake_case(),
                case.name.to_shouty_snake_case(),
                i,
            );
        }

        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_alias(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef ");
        self.print_ty(SourceType::HDefs, ty);
        self.src.h_defs(" ");
        self.print_typedef_target(name);
        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_list(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs);
        self.src.h_defs("typedef struct {\n");
        self.print_ty(SourceType::HDefs, ty);
        self.src.h_defs(" *ptr;\n");
        self.src.h_defs("size_t len;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(name);
        self.types
            .insert(id, mem::replace(&mut self.src.h_defs, prev));
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        drop((_id, name, ty, docs));
    }
}

impl InterfaceGenerator<'_> {
    fn import(&mut self, wasm_import_module: &str, func: &Function) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        self.src.c_fns("\n");

        // In the private C file, print a function declaration which is the
        // actual wasm import that we'll be calling, and this has the raw wasm
        // signature.
        uwriteln!(
            self.src.c_fns,
            "__attribute__((import_module(\"{}\"), import_name(\"{}\")))",
            wasm_import_module,
            func.name
        );
        let import_name = self.gen.names.tmp(&format!(
            "__wasm_import_{}_{}",
            self.name.to_snake_case(),
            func.name.to_snake_case()
        ));
        match sig.results.len() {
            0 => self.src.c_fns("void"),
            1 => self.src.c_fns(wasm_type(sig.results[0])),
            _ => unimplemented!("multi-value return not supported"),
        }
        self.src.c_fns(" ");
        self.src.c_fns(&import_name);
        self.src.c_fns("(");
        for (i, param) in sig.params.iter().enumerate() {
            if i > 0 {
                self.src.c_fns(", ");
            }
            self.src.c_fns(wasm_type(*param));
        }
        if sig.params.len() == 0 {
            self.src.c_fns("void");
        }
        self.src.c_fns(");\n");

        // Print the public facing signature into the header, and since that's
        // what we are defining also print it into the C file.
        let c_sig = self.print_sig(func);
        self.src.c_adapters("\n");
        self.src.c_adapters(&c_sig.sig);
        self.src.c_adapters(" {\n");

        // construct optional adapters from maybe pointers to real optional
        // structs internally
        let mut optional_adapters = String::from("");
        for (i, (_, param)) in c_sig.params.iter().enumerate() {
            let ty = &func.params[i].1;
            if let Type::Id(id) = ty {
                if let TypeDefKind::Option(option_ty) = &self.resolve.types[*id].kind {
                    let ty = self.type_string(ty);
                    uwrite!(
                        optional_adapters,
                        "{ty} {param};
                        {param}.is_some = maybe_{param} != NULL;"
                    );
                    if !self.is_empty_type(option_ty) {
                        uwriteln!(
                            optional_adapters,
                            "if (maybe_{param}) {{
                                {param}.val = *maybe_{param};
                            }}",
                        );
                    }
                }
            }
        }

        let mut f = FunctionBindgen::new(self, c_sig, &import_name);
        for (pointer, param) in f.sig.params.iter() {
            f.locals.insert(&param).unwrap();
            if *pointer {
                f.params.push(format!("*{}", param));
            } else {
                f.params.push(param.clone());
            }
        }
        for ptr in f.sig.retptrs.iter() {
            f.locals.insert(ptr).unwrap();
        }
        f.src.push_str(&optional_adapters);
        f.gen.resolve.call(
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            import_return_pointer_area_size,
            import_return_pointer_area_align,
            ..
        } = f;

        if import_return_pointer_area_size > 0 {
            self.src.c_adapters(&format!(
                "\
                    __attribute__((aligned({import_return_pointer_area_align})))
                    uint8_t ret_area[{import_return_pointer_area_size}];
                ",
            ));
        }

        self.src.c_adapters(&String::from(src));
        self.src.c_adapters("}\n");
    }

    fn export(&mut self, func: &Function, interface_name: Option<&str>) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let export_name = func.core_export_name(interface_name);

        // Print the actual header for this function into the header file, and
        // it's what we'll be calling.
        let h_sig = self.print_sig(func);

        // Generate, in the C source file, the raw wasm signature that has the
        // canonical ABI.
        uwriteln!(
            self.src.c_adapters,
            "\n__attribute__((export_name(\"{export_name}\")))"
        );
        let import_name = self.gen.names.tmp(&format!(
            "__wasm_export_{}_{}",
            self.name.to_snake_case(),
            func.name.to_snake_case()
        ));

        let mut f = FunctionBindgen::new(self, h_sig, &import_name);
        match sig.results.len() {
            0 => f.gen.src.c_adapters("void"),
            1 => f.gen.src.c_adapters(wasm_type(sig.results[0])),
            _ => unimplemented!("multi-value return not supported"),
        }
        f.gen.src.c_adapters(" ");
        f.gen.src.c_adapters(&import_name);
        f.gen.src.c_adapters("(");
        for (i, param) in sig.params.iter().enumerate() {
            if i > 0 {
                f.gen.src.c_adapters(", ");
            }
            let name = f.locals.tmp("arg");
            uwrite!(f.gen.src.c_adapters, "{} {}", wasm_type(*param), name);
            f.params.push(name);
        }
        if sig.params.len() == 0 {
            f.gen.src.c_adapters("void");
        }
        f.gen.src.c_adapters(") {\n");

        // Perform all lifting/lowering and append it to our src.
        f.gen.resolve.call(
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );
        let FunctionBindgen { src, .. } = f;
        self.src.c_adapters(&src);
        self.src.c_adapters("}\n");

        if self.resolve.guest_export_needs_post_return(func) {
            uwriteln!(
                self.src.c_fns,
                "__attribute__((weak, export_name(\"cabi_post_{export_name}\")))"
            );
            uwrite!(self.src.c_fns, "void {import_name}_post_return(");

            let mut params = Vec::new();
            let mut c_sig = CSig {
                name: String::from("INVALID"),
                sig: String::from("INVALID"),
                params: Vec::new(),
                ret: Return::default(),
                retptrs: Vec::new(),
            };
            for (i, result) in sig.results.iter().enumerate() {
                let name = format!("arg{i}");
                uwrite!(self.src.c_fns, "{} {name}", wasm_type(*result));
                c_sig.params.push((false, name.clone()));
                params.push(name);
            }
            self.src.c_fns.push_str(") {\n");

            let mut f = FunctionBindgen::new(self, c_sig, &import_name);
            f.params = params;
            f.gen.resolve.post_return(func, &mut f);
            let FunctionBindgen { src, .. } = f;
            self.src.c_fns(&src);
            self.src.c_fns("}\n");
        }
    }

    fn finish(&mut self) {
        // Continuously generate anonymous types while we continue to find more
        //
        // First we take care of the public set of anonymous types. This will
        // iteratively print them and also remove any references from the
        // private set if we happen to also reference them.
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                self.print_anonymous_type(ty);
            }
        }

        // Next we take care of private types. To do this we have basically the
        // same loop as above, after we switch the sets. We record, however,
        // all private types in a local set here to later determine if the type
        // needs to be in the C file or the H file.
        //
        // Note though that we don't re-print a type (and consider it private)
        // if we already printed it above as part of the public set.
        let mut private_types = HashSet::new();
        self.public_anonymous_types = mem::take(&mut self.private_anonymous_types);
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                if self.types.contains_key(&ty) {
                    continue;
                }
                private_types.insert(ty);
                self.print_anonymous_type(ty);
            }
        }

        for (id, _) in self.resolve.types.iter() {
            if let Some(ty) = self.types.get(&id) {
                if private_types.contains(&id) {
                    // It's private; print it in the .c file.
                    self.src.c_defs(ty);
                } else {
                    // It's public; print it in the .h file.
                    self.src.h_defs(ty);
                    self.print_dtor(id);
                }
            }
        }
    }

    fn print_sig(&mut self, func: &Function) -> CSig {
        let name = format!(
            "{}_{}",
            self.name.to_snake_case(),
            func.name.to_snake_case()
        );
        self.gen.names.insert(&name).expect("duplicate symbols");

        let start = self.src.h_fns.len();
        let mut result_rets = false;
        let mut result_rets_has_ok_type = false;

        let ret = self.classify_ret(func);
        match &ret.scalar {
            None | Some(Scalar::Void) => self.src.h_fns("void"),
            Some(Scalar::OptionBool(_id)) => self.src.h_fns("bool"),
            Some(Scalar::ResultBool(ok, _err)) => {
                result_rets = true;
                result_rets_has_ok_type = ok.is_some();
                self.src.h_fns("bool");
            }
            Some(Scalar::Type(ty)) => self.print_ty(SourceType::HFns, ty),
        }
        self.src.h_fns(" ");
        self.src.h_fns(&name);
        self.src.h_fns("(");
        let mut params = Vec::new();
        for (i, (name, ty)) in func.params.iter().enumerate() {
            if i > 0 {
                self.src.h_fns(", ");
            }
            let pointer = self.is_arg_by_pointer(ty);
            // optional param pointer flattening
            let optional_type = if let Type::Id(id) = ty {
                if let TypeDefKind::Option(option_ty) = &self.resolve.types[*id].kind {
                    Some(option_ty)
                } else {
                    None
                }
            } else {
                None
            };
            let (print_ty, print_name) = if let Some(option_ty) = optional_type {
                (option_ty, format!("maybe_{}", to_c_ident(name)))
            } else {
                (ty, to_c_ident(name))
            };
            self.print_ty(SourceType::HFns, print_ty);
            self.src.h_fns(" ");
            if pointer {
                self.src.h_fns("*");
            }
            self.src.h_fns(&print_name);
            params.push((optional_type.is_none() && pointer, to_c_ident(name)));
        }
        let mut retptrs = Vec::new();
        let single_ret = ret.retptrs.len() == 1;
        for (i, ty) in ret.retptrs.iter().enumerate() {
            if i > 0 || func.params.len() > 0 {
                self.src.h_fns(", ");
            }
            self.print_ty(SourceType::HFns, ty);
            self.src.h_fns(" *");
            let name: String = if result_rets {
                assert!(i <= 1);
                if i == 0 && result_rets_has_ok_type {
                    "ret".into()
                } else {
                    "err".into()
                }
            } else if single_ret {
                "ret".into()
            } else {
                format!("ret{}", i)
            };
            self.src.h_fns(&name);
            retptrs.push(name);
        }
        if func.params.len() == 0 && ret.retptrs.len() == 0 {
            self.src.h_fns("void");
        }
        self.src.h_fns(")");

        let sig = self.src.h_fns[start..].to_string();
        self.src.h_fns(";\n");

        CSig {
            sig,
            name,
            params,
            ret,
            retptrs,
        }
    }

    fn classify_ret(&mut self, func: &Function) -> Return {
        let mut ret = Return::default();
        match func.results.len() {
            0 => ret.scalar = Some(Scalar::Void),
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                ret.return_single(self.resolve, ty, ty);
            }
            _ => {
                ret.retptrs.extend(func.results.iter_types().cloned());
            }
        }
        return ret;
    }

    fn is_arg_by_pointer(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.resolve.types[*id].kind {
                TypeDefKind::Type(t) => self.is_arg_by_pointer(t),
                TypeDefKind::Variant(_) => true,
                TypeDefKind::Union(_) => true,
                TypeDefKind::Option(_) => true,
                TypeDefKind::Result(_) => true,
                TypeDefKind::Enum(_) => false,
                TypeDefKind::Flags(_) => false,
                TypeDefKind::Tuple(_) | TypeDefKind::Record(_) | TypeDefKind::List(_) => true,
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                TypeDefKind::Unknown => unreachable!(),
            },
            Type::String => true,
            _ => false,
        }
    }

    fn print_typedef_target(&mut self, name: &str) {
        let iface_snake = self.name.to_snake_case();
        let snake = name.to_snake_case();
        self.print_namespace(SourceType::HDefs);
        self.src.h_defs(&snake);
        self.src.h_defs("_t;\n");
        self.gen
            .names
            .insert(&format!("{iface_snake}_{snake}_t"))
            .unwrap();
    }

    fn print_namespace(&mut self, stype: SourceType) {
        self.src.print(stype, &self.name.to_snake_case());
        self.src.print(stype, "_");
    }

    fn print_ty(&mut self, stype: SourceType, ty: &Type) {
        match ty {
            Type::Bool => self.src.print(stype, "bool"),
            Type::Char => self.src.print(stype, "uint32_t"), // TODO: better type?
            Type::U8 => self.src.print(stype, "uint8_t"),
            Type::S8 => self.src.print(stype, "int8_t"),
            Type::U16 => self.src.print(stype, "uint16_t"),
            Type::S16 => self.src.print(stype, "int16_t"),
            Type::U32 => self.src.print(stype, "uint32_t"),
            Type::S32 => self.src.print(stype, "int32_t"),
            Type::U64 => self.src.print(stype, "uint64_t"),
            Type::S64 => self.src.print(stype, "int64_t"),
            Type::Float32 => self.src.print(stype, "float"),
            Type::Float64 => self.src.print(stype, "double"),
            Type::String => {
                self.src.print(stype, &self.gen.world.to_snake_case());
                self.src.print(stype, "_");
                self.src.print(stype, "string_t");
                self.gen.needs_string = true;
            }
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.name {
                    Some(name) => {
                        match ty.owner {
                            TypeOwner::Interface(owner) => {
                                self.src.print(
                                    stype,
                                    &self.gen.interface_names[&owner].to_snake_case(),
                                );
                                self.src.print(stype, "_");
                            }
                            TypeOwner::World(owner) => {
                                self.src
                                    .print(stype, &self.resolve.worlds[owner].name.to_snake_case());
                                self.src.print(stype, "_");
                            }
                            TypeOwner::None => {}
                        }

                        self.src.print(stype, &name.to_snake_case());
                        self.src.print(stype, "_t");
                    }
                    None => match &ty.kind {
                        TypeDefKind::Type(t) => self.print_ty(stype, t),
                        _ => {
                            self.public_anonymous_types.insert(*id);
                            self.private_anonymous_types.remove(id);
                            self.print_namespace(stype);
                            self.print_ty_name(stype, &Type::Id(*id));
                            self.src.print(stype, "_t");
                        }
                    },
                }
            }
        }
    }

    fn print_ty_name(&mut self, stype: SourceType, ty: &Type) {
        match ty {
            Type::Bool => self.src.print(stype, "bool"),
            Type::Char => self.src.print(stype, "char32"),
            Type::U8 => self.src.print(stype, "u8"),
            Type::S8 => self.src.print(stype, "s8"),
            Type::U16 => self.src.print(stype, "u16"),
            Type::S16 => self.src.print(stype, "s16"),
            Type::U32 => self.src.print(stype, "u32"),
            Type::S32 => self.src.print(stype, "s32"),
            Type::U64 => self.src.print(stype, "u64"),
            Type::S64 => self.src.print(stype, "s64"),
            Type::Float32 => self.src.print(stype, "float32"),
            Type::Float64 => self.src.print(stype, "float64"),
            Type::String => self.src.print(stype, "string"),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                if let Some(name) = &ty.name {
                    return self.src.print(stype, &name.to_snake_case());
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty_name(stype, t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(t) => {
                        self.src.print(stype, "tuple");
                        self.src.print(stype, &t.types.len().to_string());
                        for ty in t.types.iter() {
                            self.src.print(stype, "_");
                            self.print_ty_name(stype, ty);
                        }
                    }
                    TypeDefKind::Option(ty) => {
                        self.src.print(stype, "option_");
                        self.print_ty_name(stype, ty);
                    }
                    TypeDefKind::Result(r) => {
                        self.src.print(stype, "result_");
                        self.print_optional_ty_name(stype, r.ok.as_ref());
                        self.src.print(stype, "_");
                        self.print_optional_ty_name(stype, r.err.as_ref());
                    }
                    TypeDefKind::List(t) => {
                        self.src.print(stype, "list_");
                        self.print_ty_name(stype, t);
                    }
                    TypeDefKind::Future(t) => {
                        self.src.print(stype, "future_");
                        self.print_optional_ty_name(stype, t.as_ref());
                    }
                    TypeDefKind::Stream(s) => {
                        self.src.print(stype, "stream_");
                        self.print_optional_ty_name(stype, s.element.as_ref());
                        self.src.print(stype, "_");
                        self.print_optional_ty_name(stype, s.end.as_ref());
                    }
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
        }
    }

    fn print_optional_ty_name(&mut self, stype: SourceType, ty: Option<&Type>) {
        match ty {
            Some(ty) => self.print_ty_name(stype, ty),
            None => self.src.print(stype, "void"),
        }
    }

    fn docs(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.trim().lines() {
            self.src.h_defs("// ");
            self.src.h_defs(line);
            self.src.h_defs("\n");
        }
    }

    fn is_empty_type(&self, ty: &Type) -> bool {
        let id = match ty {
            Type::Id(id) => *id,
            _ => return false,
        };
        match &self.resolve.types[id].kind {
            TypeDefKind::Type(t) => self.is_empty_type(t),
            TypeDefKind::Record(r) => r.fields.is_empty(),
            TypeDefKind::Tuple(t) => t.types.is_empty(),
            _ => false,
        }
    }

    fn get_nonempty_type<'o>(&self, ty: Option<&'o Type>) -> Option<&'o Type> {
        match ty {
            Some(ty) => {
                if self.is_empty_type(ty) {
                    None
                } else {
                    Some(ty)
                }
            }
            None => None,
        }
    }

    fn type_string(&mut self, ty: &Type) -> String {
        // Getting a type string happens during codegen, and by default means
        // that this is a private type that's being generated. This means we
        // want to keep track of new anonymous types that are *only* mentioned
        // in methods like this, so we can place those types in the C file
        // instead of the header interface file.
        let prev = mem::take(&mut self.src.h_defs);
        let prev_public = mem::take(&mut self.public_anonymous_types);
        let prev_private = mem::take(&mut self.private_anonymous_types);

        // Print the type, which will collect into the fields that we replaced
        // above.
        self.print_ty(SourceType::HDefs, ty);

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

        mem::replace(&mut self.src.h_defs, prev).into()
    }

    fn print_anonymous_type(&mut self, ty: TypeId) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\ntypedef ");
        let kind = &self.resolve.types[ty].kind;
        match kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {
                unreachable!()
            }
            TypeDefKind::Tuple(t) => {
                self.src.h_defs("struct {\n");
                for (i, t) in t.types.iter().enumerate() {
                    self.print_ty(SourceType::HDefs, t);
                    uwriteln!(self.src.h_defs, " f{i};");
                }
                self.src.h_defs("}");
            }
            TypeDefKind::Option(t) => {
                self.src.h_defs("struct {\n");
                self.src.h_defs("bool is_some;\n");
                if !self.is_empty_type(t) {
                    self.print_ty(SourceType::HDefs, t);
                    self.src.h_defs(" val;\n");
                }
                self.src.h_defs("}");
            }
            TypeDefKind::Result(r) => {
                self.src.h_defs(
                    "struct {
                    bool is_err;
                    union {
                ",
                );
                if let Some(ok) = self.get_nonempty_type(r.ok.as_ref()) {
                    self.print_ty(SourceType::HDefs, ok);
                    self.src.h_defs(" ok;\n");
                }
                if let Some(err) = self.get_nonempty_type(r.err.as_ref()) {
                    self.print_ty(SourceType::HDefs, err);
                    self.src.h_defs(" err;\n");
                }
                self.src.h_defs("} val;\n");
                self.src.h_defs("}");
            }
            TypeDefKind::List(t) => {
                self.src.h_defs("struct {\n");
                self.print_ty(SourceType::HDefs, t);
                self.src.h_defs(" *ptr;\n");
                self.src.h_defs("size_t len;\n");
                self.src.h_defs("}");
            }
            TypeDefKind::Future(_) => todo!("print_anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("print_anonymous_type for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
        self.src.h_defs(" ");
        self.print_namespace(SourceType::HDefs);
        self.print_ty_name(SourceType::HDefs, &Type::Id(ty));
        self.src.h_defs("_t;\n");
        let type_source = mem::replace(&mut self.src.h_defs, prev);
        self.types.insert(ty, type_source);
    }

    fn print_dtor(&mut self, id: TypeId) {
        let ty = Type::Id(id);
        if !self.owns_anything(&ty) {
            return;
        }
        let pos = self.src.h_helpers.len();
        self.src.h_helpers("\nvoid ");
        self.print_namespace(SourceType::HHelpers);
        self.print_ty_name(SourceType::HHelpers, &ty);
        self.src.h_helpers("_free(");
        self.print_namespace(SourceType::HHelpers);
        self.print_ty_name(SourceType::HHelpers, &ty);
        self.src.h_helpers("_t *ptr)");

        self.src.c_helpers(&self.src.h_helpers[pos..].to_string());
        self.src.h_helpers(";");
        self.src.c_helpers(" {\n");
        match &self.resolve.types[id].kind {
            TypeDefKind::Type(t) => self.free(t, "ptr"),

            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}

            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    if !self.owns_anything(&field.ty) {
                        continue;
                    }
                    self.free(&field.ty, &format!("&ptr->{}", to_c_ident(&field.name)));
                }
            }

            TypeDefKind::Tuple(t) => {
                for (i, ty) in t.types.iter().enumerate() {
                    if !self.owns_anything(ty) {
                        continue;
                    }
                    self.free(ty, &format!("&ptr->f{i}"));
                }
            }

            TypeDefKind::List(t) => {
                if self.owns_anything(t) {
                    self.src
                        .c_helpers("for (size_t i = 0; i < ptr->len; i++) {\n");
                    self.free(t, "&ptr->ptr[i]");
                    self.src.c_helpers("}\n");
                }
                uwriteln!(self.src.c_helpers, "if (ptr->len > 0) {{");
                uwriteln!(self.src.c_helpers, "free(ptr->ptr);");
                uwriteln!(self.src.c_helpers, "}}");
            }

            TypeDefKind::Variant(v) => {
                self.src.c_helpers("switch ((int32_t) ptr->tag) {\n");
                for (i, case) in v.cases.iter().enumerate() {
                    if let Some(ty) = &case.ty {
                        if !self.owns_anything(ty) {
                            continue;
                        }
                        uwriteln!(self.src.c_helpers, "case {}: {{", i);
                        let expr = format!("&ptr->val.{}", to_c_ident(&case.name));
                        if let Some(ty) = &case.ty {
                            self.free(ty, &expr);
                        }
                        self.src.c_helpers("break;\n");
                        self.src.c_helpers("}\n");
                    }
                }
                self.src.c_helpers("}\n");
            }

            TypeDefKind::Union(u) => {
                self.src.c_helpers("switch ((int32_t) ptr->tag) {\n");
                for (i, case) in u.cases.iter().enumerate() {
                    if !self.owns_anything(&case.ty) {
                        continue;
                    }
                    uwriteln!(self.src.c_helpers, "case {i}: {{");
                    let expr = format!("&ptr->val.f{i}");
                    self.free(&case.ty, &expr);
                    self.src.c_helpers("break;\n");
                    self.src.c_helpers("}\n");
                }
                self.src.c_helpers("}\n");
            }

            TypeDefKind::Option(t) => {
                self.src.c_helpers("if (ptr->is_some) {\n");
                self.free(t, "&ptr->val");
                self.src.c_helpers("}\n");
            }

            TypeDefKind::Result(r) => {
                self.src.c_helpers("if (!ptr->is_err) {\n");
                if let Some(ok) = &r.ok {
                    if self.owns_anything(ok) {
                        self.free(ok, "&ptr->val.ok");
                    }
                }
                if let Some(err) = &r.err {
                    if self.owns_anything(err) {
                        self.src.c_helpers("} else {\n");
                        self.free(err, "&ptr->val.err");
                    }
                }
                self.src.c_helpers("}\n");
            }
            TypeDefKind::Future(_) => todo!("print_dtor for future"),
            TypeDefKind::Stream(_) => todo!("print_dtor for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
        self.src.c_helpers("}\n");
    }

    fn owns_anything(&self, ty: &Type) -> bool {
        let id = match ty {
            Type::Id(id) => *id,
            Type::String => return true,
            _ => return false,
        };
        match &self.resolve.types[id].kind {
            TypeDefKind::Type(t) => self.owns_anything(t),
            TypeDefKind::Record(r) => r.fields.iter().any(|t| self.owns_anything(&t.ty)),
            TypeDefKind::Tuple(t) => t.types.iter().any(|t| self.owns_anything(t)),
            TypeDefKind::Flags(_) => false,
            TypeDefKind::Enum(_) => false,
            TypeDefKind::List(_) => true,
            TypeDefKind::Variant(v) => v
                .cases
                .iter()
                .any(|c| self.optional_owns_anything(c.ty.as_ref())),
            TypeDefKind::Union(v) => v.cases.iter().any(|case| self.owns_anything(&case.ty)),
            TypeDefKind::Option(t) => self.owns_anything(t),
            TypeDefKind::Result(r) => {
                self.optional_owns_anything(r.ok.as_ref())
                    || self.optional_owns_anything(r.err.as_ref())
            }
            TypeDefKind::Future(_) => todo!("owns_anything for future"),
            TypeDefKind::Stream(_) => todo!("owns_anything for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn optional_owns_anything(&self, ty: Option<&Type>) -> bool {
        match ty {
            Some(ty) => self.owns_anything(ty),
            None => false,
        }
    }

    fn free(&mut self, ty: &Type, expr: &str) {
        let prev = mem::take(&mut self.src.h_helpers);
        match ty {
            Type::String => {
                self.src.h_helpers(&self.gen.world.to_snake_case());
                self.src.h_helpers("_");
            }
            _ => {
                self.print_namespace(SourceType::HHelpers);
            }
        }
        self.print_ty_name(SourceType::HHelpers, ty);
        let name = mem::replace(&mut self.src.h_helpers, prev);

        self.src.c_helpers(&name);
        self.src.c_helpers("_free(");
        self.src.c_helpers(expr);
        self.src.c_helpers(");\n");
    }
}

struct FunctionBindgen<'a, 'b> {
    gen: &'a mut InterfaceGenerator<'b>,
    locals: Ns,
    src: wit_bindgen_core::Source,
    sig: CSig,
    func_to_call: &'a str,
    block_storage: Vec<wit_bindgen_core::Source>,
    blocks: Vec<(String, Vec<String>)>,
    payloads: Vec<String>,
    params: Vec<String>,
    wasm_return: Option<String>,
    ret_store_cnt: usize,
    import_return_pointer_area_size: usize,
    import_return_pointer_area_align: usize,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'a mut InterfaceGenerator<'b>,
        sig: CSig,
        func_to_call: &'a str,
    ) -> FunctionBindgen<'a, 'b> {
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
            wasm_return: None,
            ret_store_cnt: 0,
            import_return_pointer_area_size: 0,
            import_return_pointer_area_align: 0,
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

    fn load_ext(&mut self, ty: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        self.load(ty, offset, operands, results);
        let result = results.pop().unwrap();
        results.push(format!("(int32_t) ({})", result));
    }

    fn store(&mut self, ty: &str, offset: i32, operands: &[String]) {
        uwriteln!(
            self.src,
            "*(({}*)({} + {})) = {};",
            ty,
            operands[1],
            offset,
            operands[0]
        );
    }

    fn store_in_retptr(&mut self, operand: &String) {
        self.store_op(
            operand,
            &format!("*{}", self.sig.retptrs[self.ret_store_cnt]),
        );
        self.ret_store_cnt = self.ret_store_cnt + 1;
    }
}

impl Bindgen for FunctionBindgen<'_, '_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.gen.gen.sizes
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

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        let ptr = self.locals.tmp("ptr");

        // Use a stack-based return area for imports, because exports need
        // their return area to be live until the post-return call.
        if self.gen.in_import {
            self.import_return_pointer_area_size = self.import_return_pointer_area_size.max(size);
            self.import_return_pointer_area_align =
                self.import_return_pointer_area_align.max(align);
            uwriteln!(self.src, "int32_t {} = (int32_t) &ret_area;", ptr);
        } else {
            self.gen.gen.return_pointer_area_size = self.gen.gen.return_pointer_area_size.max(size);
            self.gen.gen.return_pointer_area_align =
                self.gen.gen.return_pointer_area_align.max(align);
            // Declare a statically-allocated return area.
            uwriteln!(self.src, "int32_t {} = (int32_t) &RET_AREA;", ptr);
        }

        ptr
    }

    fn is_list_canonical(&self, resolve: &Resolve, ty: &Type) -> bool {
        resolve.all_bits_valid(ty)
    }

    fn emit(
        &mut self,
        _resolve: &Resolve,
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
            Instruction::F32FromFloat32
            | Instruction::F64FromFloat64
            | Instruction::Float32FromF32
            | Instruction::Float64FromF64 => {
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

            Instruction::BoolFromI32 | Instruction::I32FromBool => {
                results.push(operands[0].clone());
            }

            Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for f in record.fields.iter() {
                    results.push(format!("({}).{}", op, to_c_ident(&f.name)));
                }
            }
            Instruction::RecordLift { ty, .. } => {
                let name = self.gen.type_string(&Type::Id(*ty));
                let mut result = format!("({}) {{\n", name);
                for op in operands {
                    uwriteln!(result, "{},", op);
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                for i in 0..tuple.types.len() {
                    results.push(format!("({}).f{}", op, i));
                }
            }
            Instruction::TupleLift { ty, .. } => {
                let name = self.gen.type_string(&Type::Id(*ty));
                let mut result = format!("({}) {{\n", name);
                for op in operands {
                    uwriteln!(result, "{},", op);
                }
                result.push_str("}");
                results.push(result);
            }

            // TODO: checked
            Instruction::FlagsLower { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(operands.pop().unwrap());
                }
                Int::U64 => {
                    let name = self.gen.type_string(&Type::Id(*ty));
                    let tmp = self.locals.tmp("flags");
                    uwriteln!(self.src, "{name} {tmp} = {};", operands[0]);
                    results.push(format!("{tmp} & 0xffffffff"));
                    results.push(format!("({tmp} >> 32) & 0xffffffff"));
                }
            },

            Instruction::FlagsLift { flags, ty, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(operands.pop().unwrap());
                }
                Int::U64 => {
                    let name = self.gen.type_string(&Type::Id(*ty));
                    let op0 = &operands[0];
                    let op1 = &operands[1];
                    results.push(format!("(({name}) ({op0})) | ((({name}) ({op1})) << 32)"));
                }
            },

            Instruction::VariantPayloadName => {
                let name = self.locals.tmp("payload");
                results.push(format!("*{}", name));
                self.payloads.push(name);
            }

            Instruction::VariantLower {
                variant,
                results: result_types,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let payloads = self
                    .payloads
                    .drain(self.payloads.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let mut variant_results = Vec::with_capacity(result_types.len());
                for ty in result_types.iter() {
                    let name = self.locals.tmp("variant");
                    results.push(name.clone());
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    variant_results.push(name);
                }

                let expr_to_match = format!("({}).tag", operands[0]);

                uwriteln!(self.src, "switch ((int32_t) {}) {{", expr_to_match);
                for (i, ((case, (block, block_results)), payload)) in
                    variant.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    uwriteln!(self.src, "case {}: {{", i);
                    if let Some(ty) = self.gen.get_nonempty_type(case.ty.as_ref()) {
                        let ty = self.gen.type_string(ty);
                        uwrite!(
                            self.src,
                            "const {} *{} = &({}).val",
                            ty,
                            payload,
                            operands[0],
                        );
                        self.src.push_str(".");
                        self.src.push_str(&to_c_ident(&case.name));
                        self.src.push_str(";\n");
                    }
                    self.src.push_str(&block);

                    for (name, result) in variant_results.iter().zip(&block_results) {
                        uwriteln!(self.src, "{} = {};", name, result);
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }

            Instruction::VariantLift { variant, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let ty = self.gen.type_string(&Type::Id(*ty));
                let result = self.locals.tmp("variant");
                uwriteln!(self.src, "{} {};", ty, result);
                uwriteln!(self.src, "{}.tag = {};", result, operands[0]);
                uwriteln!(self.src, "switch ((int32_t) {}.tag) {{", result);
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    uwriteln!(self.src, "case {}: {{", i);
                    self.src.push_str(&block);
                    assert!(block_results.len() == (case.ty.is_some() as usize));

                    if let Some(_) = self.gen.get_nonempty_type(case.ty.as_ref()) {
                        let mut dst = format!("{}.val", result);
                        dst.push_str(".");
                        dst.push_str(&to_c_ident(&case.name));
                        self.store_op(&block_results[0], &dst);
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
                results.push(result);
            }

            Instruction::UnionLower {
                union,
                results: result_types,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                let payloads = self
                    .payloads
                    .drain(self.payloads.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let mut union_results = Vec::with_capacity(result_types.len());
                for ty in result_types.iter() {
                    let name = self.locals.tmp("unionres");
                    results.push(name.clone());
                    let ty = wasm_type(*ty);
                    uwriteln!(self.src, "{ty} {name};");
                    union_results.push(name);
                }

                let op0 = &operands[0];
                uwriteln!(self.src, "switch (({op0}).tag) {{");
                for (i, ((case, (block, block_results)), payload)) in
                    union.cases.iter().zip(blocks).zip(payloads).enumerate()
                {
                    uwriteln!(self.src, "case {i}: {{");
                    if !self.gen.is_empty_type(&case.ty) {
                        let ty = self.gen.type_string(&case.ty);
                        uwriteln!(self.src, "const {ty} *{payload} = &({op0}).val.f{i};");
                    }
                    self.src.push_str(&block);

                    for (name, result) in union_results.iter().zip(&block_results) {
                        uwriteln!(self.src, "{name} = {result};");
                    }
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }

            Instruction::UnionLift { union, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let ty = self.gen.type_string(&Type::Id(*ty));
                let result = self.locals.tmp("unionres");
                uwriteln!(self.src, "{} {};", ty, result);
                uwriteln!(self.src, "{}.tag = {};", result, operands[0]);
                uwriteln!(self.src, "switch ((int32_t) {}.tag) {{", result);
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    uwriteln!(self.src, "case {i}: {{");
                    self.src.push_str(&block);

                    assert!(block_results.len() == 1);
                    let dst = format!("{result}.val.f{i}");
                    self.store_op(&block_results[0], &dst);
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
                results.push(result);
            }

            Instruction::OptionLower {
                results: result_types,
                payload,
                ..
            } => {
                let (mut some, some_results) = self.blocks.pop().unwrap();
                let (mut none, none_results) = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let _none_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let name = self.locals.tmp("option");
                    results.push(name.clone());
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    let some_result = &some_results[i];
                    uwriteln!(some, "{name} = {some_result};");
                    let none_result = &none_results[i];
                    uwriteln!(none, "{name} = {none_result};");
                }

                let op0 = &operands[0];
                let ty = self.gen.type_string(payload);
                let bind_some = if self.gen.is_empty_type(payload) {
                    String::new()
                } else {
                    format!("const {ty} *{some_payload} = &({op0}).val;")
                };

                uwrite!(
                    self.src,
                    "\
                    if (({op0}).is_some) {{
                        {bind_some}
                        {some}}} else {{
                        {none}}}
                    "
                );
            }

            Instruction::OptionLift { payload, ty, .. } => {
                let (mut some, some_results) = self.blocks.pop().unwrap();
                let (mut none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.len() == 0);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];

                let ty = self.gen.type_string(&Type::Id(*ty));
                let result = self.locals.tmp("option");
                uwriteln!(self.src, "{ty} {result};");
                let op0 = &operands[0];
                let set_some = if self.gen.is_empty_type(payload) {
                    String::new()
                } else {
                    format!("{result}.val = {some_result};\n")
                };
                if none.len() > 0 {
                    none.push('\n');
                }
                if some.len() > 0 {
                    some.push('\n');
                }
                uwrite!(
                    self.src,
                    "switch ({op0}) {{
                        case 0: {{
                            {result}.is_some = false;
                            {none}\
                            break;
                        }}
                        case 1: {{
                            {result}.is_some = true;
                            {some}\
                            {set_some}\
                            break;
                        }}
                    }}\n"
                );
                results.push(result);
            }

            Instruction::ResultLower {
                results: result_types,
                result,
                ..
            } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                let (mut ok, ok_results) = self.blocks.pop().unwrap();
                let err_payload = self.payloads.pop().unwrap();
                let ok_payload = self.payloads.pop().unwrap();

                for (i, ty) in result_types.iter().enumerate() {
                    let name = self.locals.tmp("result");
                    results.push(name.clone());
                    self.src.push_str(wasm_type(*ty));
                    self.src.push_str(" ");
                    self.src.push_str(&name);
                    self.src.push_str(";\n");
                    let ok_result = &ok_results[i];
                    uwriteln!(ok, "{name} = {ok_result};");
                    let err_result = &err_results[i];
                    uwriteln!(err, "{name} = {err_result};");
                }

                let op0 = &operands[0];
                let bind_ok = if let Some(ok) = self.gen.get_nonempty_type(result.ok.as_ref()) {
                    let ok_ty = self.gen.type_string(ok);
                    format!("const {ok_ty} *{ok_payload} = &({op0}).val.ok;")
                } else {
                    String::new()
                };
                let bind_err = if let Some(err) = self.gen.get_nonempty_type(result.err.as_ref()) {
                    let err_ty = self.gen.type_string(err);
                    format!("const {err_ty} *{err_payload} = &({op0}).val.err;")
                } else {
                    String::new()
                };
                uwrite!(
                    self.src,
                    "\
                    if (({op0}).is_err) {{
                        {bind_err}\
                        {err}\
                    }} else {{
                        {bind_ok}\
                        {ok}\
                    }}
                    "
                );
            }

            Instruction::ResultLift { result, ty, .. } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                assert!(err_results.len() == (result.err.is_some() as usize));
                let (mut ok, ok_results) = self.blocks.pop().unwrap();
                assert!(ok_results.len() == (result.ok.is_some() as usize));

                if err.len() > 0 {
                    err.push_str("\n");
                }
                if ok.len() > 0 {
                    ok.push_str("\n");
                }

                let result_tmp = self.locals.tmp("result");
                let set_ok = if let Some(_) = self.gen.get_nonempty_type(result.ok.as_ref()) {
                    let ok_result = &ok_results[0];
                    format!("{result_tmp}.val.ok = {ok_result};\n")
                } else {
                    String::new()
                };
                let set_err = if let Some(_) = self.gen.get_nonempty_type(result.err.as_ref()) {
                    let err_result = &err_results[0];
                    format!("{result_tmp}.val.err = {err_result};\n")
                } else {
                    String::new()
                };

                let ty = self.gen.type_string(&Type::Id(*ty));
                uwriteln!(self.src, "{ty} {result_tmp};");
                let op0 = &operands[0];
                uwriteln!(
                    self.src,
                    "switch ({op0}) {{
                        case 0: {{
                            {result_tmp}.is_err = false;
                            {ok}\
                            {set_ok}\
                            break;
                        }}
                        case 1: {{
                            {result_tmp}.is_err = true;
                            {err}\
                            {set_err}\
                            break;
                        }}
                    }}"
                );
                results.push(result_tmp);
            }

            Instruction::EnumLower { .. } => results.push(format!("(int32_t) {}", operands[0])),
            Instruction::EnumLift { .. } => results.push(operands.pop().unwrap()),

            Instruction::ListCanonLower { .. } | Instruction::StringLower { .. } => {
                results.push(format!("(int32_t) ({}).ptr", operands[0]));
                results.push(format!("(int32_t) ({}).len", operands[0]));
            }
            Instruction::ListCanonLift { element, ty, .. } => {
                let list_name = self.gen.type_string(&Type::Id(*ty));
                let elem_name = self.gen.type_string(element);
                results.push(format!(
                    "({}) {{ ({}*)({}), (size_t)({}) }}",
                    list_name, elem_name, operands[0], operands[1]
                ));
            }
            Instruction::StringLift { .. } => {
                let list_name = self.gen.type_string(&Type::String);
                results.push(format!(
                    "({}) {{ ({}*)({}), (size_t)({}) }}",
                    list_name,
                    self.gen.gen.char_type(),
                    operands[0],
                    operands[1]
                ));
            }

            Instruction::ListLower { .. } => {
                let _body = self.blocks.pop().unwrap();
                results.push(format!("(int32_t) ({}).ptr", operands[0]));
                results.push(format!("(int32_t) ({}).len", operands[0]));
            }

            Instruction::ListLift { element, ty, .. } => {
                let _body = self.blocks.pop().unwrap();
                let list_name = self.gen.type_string(&Type::Id(*ty));
                let elem_name = self.gen.type_string(element);
                results.push(format!(
                    "({}) {{ ({}*)({}), (size_t)({}) }}",
                    list_name, elem_name, operands[0], operands[1]
                ));
            }
            Instruction::IterElem { .. } => results.push("e".to_string()),
            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm { sig, .. } => {
                match sig.results.len() {
                    0 => {}
                    1 => {
                        self.src.push_str(wasm_type(sig.results[0]));
                        let ret = self.locals.tmp("ret");
                        self.wasm_return = Some(ret.clone());
                        uwrite!(self.src, " {} = ", ret);
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

            Instruction::CallInterface { func } => {
                let mut args = String::new();
                for (i, (op, (byref, _))) in operands.iter().zip(&self.sig.params).enumerate() {
                    if i > 0 {
                        args.push_str(", ");
                    }
                    let ty = &func.params[i].1;
                    if *byref {
                        let name = self.locals.tmp("arg");
                        let ty = self.gen.type_string(ty);
                        uwriteln!(self.src, "{} {} = {};", ty, name, op);
                        args.push_str("&");
                        args.push_str(&name);
                    } else {
                        if !self.gen.in_import {
                            if let Type::Id(id) = ty {
                                if let TypeDefKind::Option(option_ty) =
                                    &self.gen.resolve.types[*id].kind
                                {
                                    if self.gen.is_empty_type(option_ty) {
                                        uwrite!(args, "{op}.is_some ? (void*)1 : NULL");
                                    } else {
                                        uwrite!(args, "{op}.is_some ? &({op}.val) : NULL");
                                    }
                                    continue;
                                }
                            }
                        }
                        args.push_str(op);
                    }
                }
                match &self.sig.ret.scalar {
                    None => {
                        let mut retptrs = Vec::new();
                        for ty in self.sig.ret.retptrs.iter() {
                            let name = self.locals.tmp("ret");
                            let ty = self.gen.type_string(ty);
                            uwriteln!(self.src, "{} {};", ty, name);
                            if args.len() > 0 {
                                args.push_str(", ");
                            }
                            args.push_str("&");
                            args.push_str(&name);
                            retptrs.push(name);
                        }
                        uwriteln!(self.src, "{}({});", self.sig.name, args);
                        results.extend(retptrs);
                    }
                    Some(Scalar::Void) => {
                        uwriteln!(self.src, "{}({});", self.sig.name, args);
                    }
                    Some(Scalar::Type(_)) => {
                        let ret = self.locals.tmp("ret");
                        let ty = self
                            .gen
                            .type_string(func.results.iter_types().next().unwrap());
                        uwriteln!(self.src, "{} {} = {}({});", ty, ret, self.sig.name, args);
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
                        let payload_ty = self.gen.type_string(ty);
                        uwriteln!(self.src, "{} {};", payload_ty, val);
                        uwriteln!(self.src, "bool {} = {}({});", ret, self.sig.name, args);
                        let option_ty = self
                            .gen
                            .type_string(func.results.iter_types().next().unwrap());
                        let option_ret = self.locals.tmp("ret");
                        if !self.gen.is_empty_type(ty) {
                            uwrite!(
                                self.src,
                                "
                                    {ty} {ret};
                                    {ret}.is_some = {tag};
                                    {ret}.val = {val};
                                ",
                                ty = option_ty,
                                ret = option_ret,
                                tag = ret,
                                val = val,
                            );
                        } else {
                            uwrite!(
                                self.src,
                                "
                                    {ty} {ret};
                                    {ret}.is_some = {tag};
                                ",
                                ty = option_ty,
                                ret = option_ret,
                                tag = ret,
                            );
                        }
                        results.push(option_ret);
                    }
                    Some(Scalar::ResultBool(ok, err)) => {
                        let result_ty = self
                            .gen
                            .type_string(func.results.iter_types().next().unwrap());
                        let ret = self.locals.tmp("ret");
                        let mut ret_iter = self.sig.ret.retptrs.iter();
                        uwriteln!(self.src, "{result_ty} {ret};");
                        let ok_name = if ok.is_some() {
                            if let Some(ty) = ret_iter.next() {
                                let val = self.locals.tmp("ok");
                                if args.len() > 0 {
                                    uwrite!(args, ", ");
                                }
                                uwrite!(args, "&{val}");
                                let ty = self.gen.type_string(ty);
                                uwriteln!(self.src, "{} {};", ty, val);
                                Some(val)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let err_name = if let Some(ty) = ret_iter.next() {
                            let val = self.locals.tmp("err");
                            if args.len() > 0 {
                                uwrite!(args, ", ")
                            }
                            uwrite!(args, "&{val}");
                            let ty = self.gen.type_string(ty);
                            uwriteln!(self.src, "{} {};", ty, val);
                            Some(val)
                        } else {
                            None
                        };
                        assert!(ret_iter.next().is_none());
                        uwrite!(self.src, "");
                        uwriteln!(self.src, "{ret}.is_err = !{}({args});", self.sig.name);
                        if self.gen.get_nonempty_type(err.as_ref()).is_some() {
                            if let Some(err_name) = err_name {
                                uwriteln!(
                                    self.src,
                                    "if ({ret}.is_err) {{
                                        {ret}.val.err = {err_name};
                                    }}",
                                );
                            }
                        }
                        if self.gen.get_nonempty_type(ok.as_ref()).is_some() {
                            if let Some(ok_name) = ok_name {
                                uwriteln!(
                                    self.src,
                                    "if (!{ret}.is_err) {{
                                        {ret}.val.ok = {ok_name};
                                    }}"
                                );
                            } else {
                                uwrite!(self.src, "\n");
                            }
                        }
                        results.push(ret);
                    }
                }
            }
            Instruction::Return { .. } if self.gen.in_import => match self.sig.ret.scalar {
                None => {
                    for op in operands.iter() {
                        self.store_in_retptr(op);
                    }
                }
                Some(Scalar::Void) => {
                    assert!(operands.is_empty());
                }
                Some(Scalar::Type(_)) => {
                    assert_eq!(operands.len(), 1);
                    self.src.push_str("return ");
                    self.src.push_str(&operands[0]);
                    self.src.push_str(";\n");
                }
                Some(Scalar::OptionBool(o)) => {
                    assert_eq!(operands.len(), 1);
                    let variant = &operands[0];
                    if !self.gen.is_empty_type(&o) {
                        self.store_in_retptr(&format!("{}.val", variant));
                    }
                    self.src.push_str("return ");
                    self.src.push_str(&variant);
                    self.src.push_str(".is_some;\n");
                }
                Some(Scalar::ResultBool(ok, err)) => {
                    assert_eq!(operands.len(), 1);
                    let variant = &operands[0];
                    assert!(self.sig.retptrs.len() <= 2);
                    uwriteln!(self.src, "if (!{}.is_err) {{", variant);
                    if let Some(_) = self.gen.get_nonempty_type(ok.as_ref()) {
                        if self.sig.retptrs.len() == 2 {
                            self.store_in_retptr(&format!("{}.val.ok", variant));
                        } else if self.sig.retptrs.len() == 1 && ok.is_some() {
                            self.store_in_retptr(&format!("{}.val.ok", variant));
                        }
                    }
                    uwriteln!(
                        self.src,
                        "   return 1;
                            }} else {{"
                    );
                    if let Some(_) = self.gen.get_nonempty_type(err.as_ref()) {
                        if self.sig.retptrs.len() == 2 {
                            self.store_in_retptr(&format!("{}.val.err", variant));
                        } else if self.sig.retptrs.len() == 1 && !ok.is_some() {
                            self.store_in_retptr(&format!("{}.val.err", variant));
                        }
                    }
                    uwriteln!(
                        self.src,
                        "   return 0;
                            }}"
                    );
                }
            },
            Instruction::Return { amt, .. } => {
                assert!(*amt <= 1);
                if *amt == 1 {
                    uwriteln!(self.src, "return {};", operands[0]);
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
            Instruction::I32Store8 { offset } => self.store("int8_t", *offset, operands),
            Instruction::I32Store16 { offset } => self.store("int16_t", *offset, operands),

            Instruction::I32Load8U { offset } => {
                self.load_ext("uint8_t", *offset, operands, results)
            }
            Instruction::I32Load8S { offset } => {
                self.load_ext("int8_t", *offset, operands, results)
            }
            Instruction::I32Load16U { offset } => {
                self.load_ext("uint16_t", *offset, operands, results)
            }
            Instruction::I32Load16S { offset } => {
                self.load_ext("int16_t", *offset, operands, results)
            }

            Instruction::GuestDeallocate { .. } => {
                uwriteln!(self.src, "free((void*) ({}));", operands[0]);
            }
            Instruction::GuestDeallocateString => {
                uwriteln!(self.src, "if (({}) > 0) {{", operands[1]);
                uwriteln!(self.src, "free((void*) ({}));", operands[0]);
                uwriteln!(self.src, "}}");
            }
            Instruction::GuestDeallocateVariant { blocks } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .collect::<Vec<_>>();

                uwriteln!(self.src, "switch ((int32_t) {}) {{", operands[0]);
                for (i, (block, results)) in blocks.into_iter().enumerate() {
                    assert!(results.is_empty());
                    uwriteln!(self.src, "case {}: {{", i);
                    self.src.push_str(&block);
                    self.src.push_str("break;\n}\n");
                }
                self.src.push_str("}\n");
            }
            Instruction::GuestDeallocateList { element } => {
                let (body, results) = self.blocks.pop().unwrap();
                assert!(results.is_empty());
                let ptr = self.locals.tmp("ptr");
                let len = self.locals.tmp("len");
                uwriteln!(self.src, "int32_t {ptr} = {};", operands[0]);
                uwriteln!(self.src, "int32_t {len} = {};", operands[1]);
                let i = self.locals.tmp("i");
                uwriteln!(self.src, "for (int32_t {i} = 0; {i} < {len}; {i}++) {{");
                let size = self.gen.gen.sizes.size(element);
                uwriteln!(self.src, "int32_t base = {ptr} + {i} * {size};");
                uwriteln!(self.src, "(void) base;");
                uwrite!(self.src, "{body}");
                uwriteln!(self.src, "}}");
                uwriteln!(self.src, "if ({len} > 0) {{");
                uwriteln!(self.src, "free((void*) ({ptr}));");
                uwriteln!(self.src, "}}");
            }

            i => unimplemented!("{:?}", i),
        }
    }
}

#[derive(Default, Clone, Copy)]
enum SourceType {
    #[default]
    HDefs,
    HFns,
    HHelpers,
    // CDefs,
    // CFns,
    // CHelpers,
    // CAdapters,
}

#[derive(Default)]
struct Source {
    h_defs: wit_bindgen_core::Source,
    h_fns: wit_bindgen_core::Source,
    h_helpers: wit_bindgen_core::Source,
    c_defs: wit_bindgen_core::Source,
    c_fns: wit_bindgen_core::Source,
    c_helpers: wit_bindgen_core::Source,
    c_adapters: wit_bindgen_core::Source,
}

impl Source {
    fn print(&mut self, stype: SourceType, s: &str) {
        match stype {
            SourceType::HDefs => self.h_defs(s),
            SourceType::HFns => self.h_fns(s),
            SourceType::HHelpers => self.h_helpers(s),
            // SourceType::CDefs => self.c_defs(s),
            // SourceType::CFns => self.c_fns(s),
            // SourceType::CHelpers => self.c_helpers(s),
            // SourceType::CAdapters => self.c_adapters(s),
        }
    }
    fn append(&mut self, append_src: &Source) {
        self.h_defs.push_str(&append_src.h_defs);
        self.h_fns.push_str(&append_src.h_fns);
        self.h_helpers.push_str(&append_src.h_helpers);
        self.c_defs.push_str(&append_src.c_defs);
        self.c_fns.push_str(&append_src.c_fns);
        self.c_helpers.push_str(&append_src.c_helpers);
        self.c_adapters.push_str(&append_src.c_adapters);
    }
    fn h_defs(&mut self, s: &str) {
        self.h_defs.push_str(s);
    }
    fn h_fns(&mut self, s: &str) {
        self.h_fns.push_str(s);
    }
    fn h_helpers(&mut self, s: &str) {
        self.h_helpers.push_str(s);
    }
    fn c_defs(&mut self, s: &str) {
        self.c_defs.push_str(s);
    }
    fn c_fns(&mut self, s: &str) {
        self.c_fns.push_str(s);
    }
    fn c_helpers(&mut self, s: &str) {
        self.c_helpers.push_str(s);
    }
    fn c_adapters(&mut self, s: &str) {
        self.c_adapters.push_str(s);
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

fn flags_repr(f: &Flags) -> Int {
    match f.repr() {
        FlagsRepr::U8 => Int::U8,
        FlagsRepr::U16 => Int::U16,
        FlagsRepr::U32(1) => Int::U32,
        FlagsRepr::U32(2) => Int::U64,
        repr => panic!("unimplemented flags {:?}", repr),
    }
}

pub fn to_c_ident(name: &str) -> String {
    match name {
        // Escape C keywords.
        // Source: https://en.cppreference.com/w/c/keyword
        "auto" => "auto_".into(),
        "else" => "else_".into(),
        "long" => "long_".into(),
        "switch" => "switch_".into(),
        "break" => "break_".into(),
        "enum" => "enum_".into(),
        "register" => "register_".into(),
        "typedef" => "typedef_".into(),
        "case" => "case_".into(),
        "extern" => "extern_".into(),
        "return" => "return_".into(),
        "union" => "union_".into(),
        "char" => "char_".into(),
        "float" => "float_".into(),
        "short" => "short_".into(),
        "unsigned" => "unsigned_".into(),
        "const" => "const_".into(),
        "for" => "for_".into(),
        "signed" => "signed_".into(),
        "void" => "void_".into(),
        "continue" => "continue_".into(),
        "goto" => "goto_".into(),
        "sizeof" => "sizeof_".into(),
        "volatile" => "volatile_".into(),
        "default" => "default_".into(),
        "if" => "if_".into(),
        "static" => "static_".into(),
        "while" => "while_".into(),
        "do" => "do_".into(),
        "int" => "int_".into(),
        "struct" => "struct_".into(),
        "_Packed" => "_Packed_".into(),
        "double" => "double_".into(),
        s => s.to_snake_case(),
    }
}
