mod component_type_object;

use anyhow::Result;
use heck::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use std::mem;
use wit_bindgen_core::abi::{self, AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Ns, WorldGenerator,
};
use wit_component::StringEncoding;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
enum Direction {
    #[default]
    Import,
    Export,
}

#[derive(Default)]
struct ResourceInfo {
    direction: Direction,
    borrow: Option<TypeId>,
    own: Option<TypeId>,
}

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

    // Known names for interfaces as they're seen in imports and exports.
    //
    // This is subsequently used to generate a namespace for each type that's
    // used, but only in the case that the interface itself doesn't already have
    // an original name.
    interface_names: HashMap<InterfaceId, WorldKey>,

    // Interfaces who have had their types printed.
    //
    // This is used to guard against printing the types for an interface twice.
    // The same interface can be both imported and exported in which case only
    // one set of types is generated and all bindings for both imports and
    // exports use that set of types.
    interfaces_with_types_printed: HashSet<InterfaceId>,

    // Type definitions for the given `TypeId`. This is printed topologically
    // at the end.
    types: HashMap<TypeId, wit_bindgen_core::Source>,

    resources: HashMap<TypeId, ResourceInfo>,

    // The set of types that are considered public (aka need to be in the
    // header file) which are anonymous and we're effectively monomorphizing.
    // This is discovered lazily when printing type names.
    public_anonymous_types: BTreeSet<TypeId>,

    // This is similar to `public_anonymous_types` where it's discovered
    // lazily, but the set here are for private types only used in the
    // implementation of functions. These types go in the implementation file,
    // not the header file.
    private_anonymous_types: BTreeSet<TypeId>,
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
    // Skip optional null pointer and boolean result argument signature flattening
    #[cfg_attr(feature = "clap", arg(long, default_value_t = false))]
    pub no_sig_flattening: bool,
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
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let prev = self.interface_names.insert(id, name.clone());
        assert!(prev.is_none());
        let mut gen = self.interface(resolve, true);
        gen.interface = Some(id);
        if gen.gen.interfaces_with_types_printed.insert(id) {
            gen.types(id);
        }

        for (i, (_name, func)) in resolve.interfaces[id].functions.iter().enumerate() {
            if i == 0 {
                let name = resolve.name_world_key(name);
                uwriteln!(gen.src.h_fns, "\n// Imported Functions from `{name}`");
            }
            gen.import(Some(name), func);
        }

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
        let mut gen = self.interface(resolve, true);

        for (i, (_name, func)) in funcs.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Imported Functions from `{name}`");
            }
            gen.import(None, func);
        }

        gen.gen.src.append(&gen.src);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, false);
        gen.interface = Some(id);
        if gen.gen.interfaces_with_types_printed.insert(id) {
            gen.types(id);
        } else {
            let iface = &resolve.interfaces[id];
            for id in iface.types.values() {
                if let TypeDefKind::Resource = &resolve.types[*id].kind {
                    // This will require a substantial refactor so we can
                    // generate two sets of types and helper functions, one for
                    // the imported types and one for the exported types.
                    todo!("importing and exporting the same interface containing a resource not yet supported");
                }
            }
        }

        for (i, (_name, func)) in resolve.interfaces[id].functions.iter().enumerate() {
            if i == 0 {
                let name = resolve.name_world_key(name);
                uwriteln!(gen.src.h_fns, "\n// Exported Functions from `{name}`");
            }
            gen.export(func, Some(name));
        }

        gen.gen.src.append(&gen.src);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = &resolve.worlds[world].name;
        let mut gen = self.interface(resolve, false);

        for (i, (_name, func)) in funcs.iter().enumerate() {
            if i == 0 {
                uwriteln!(gen.src.h_fns, "\n// Exported Functions from `{name}`");
            }
            gen.export(func, None);
        }

        gen.gen.src.append(&gen.src);
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(resolve, false);
        for (name, id) in types {
            gen.define_type(name, *id);
        }
        gen.gen.src.append(&gen.src);
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        self.finish_types(resolve);

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
        let version = env!("CARGO_PKG_VERSION");
        let mut h_str = wit_bindgen_core::Source::default();

        wit_bindgen_core::generated_preamble(&mut h_str, version);

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
        wit_bindgen_core::generated_preamble(&mut c_str, version);
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

        self.finish_resources(resolve, &mut h_str, &mut c_str);

        uwriteln!(c_str, "\n// Component Adapters");

        // Declare a statically-allocated return area, if needed. We only do
        // this for export bindings, because import bindings allocate their
        // return-area on the stack.
        if self.return_pointer_area_size > 0 {
            // Automatic indentation avoided due to `extern "C" {` declaration
            uwrite!(
                c_str,
                "
                __attribute__((__aligned__({})))
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
        resolve: &'a Resolve,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: Source::default(),
            gen: self,
            resolve,
            interface: None,
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

    fn finish_types(&mut self, resolve: &Resolve) {
        let handles = resolve
            .types
            .iter()
            .filter_map(|(id, ty)| {
                if let TypeDefKind::Handle(handle) = &ty.kind {
                    Some((handle, id))
                } else {
                    None
                }
            })
            .collect();

        // Continuously generate anonymous types while we continue to find more
        //
        // First we take care of the public set of anonymous types. This will
        // iteratively print them and also remove any references from the
        // private set if we happen to also reference them.
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                self.print_anonymous_type(resolve, ty, &handles);
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
                self.print_anonymous_type(resolve, ty, &handles);
            }
        }

        for (id, _) in resolve.types.iter() {
            if let Some(ty) = self.types.get(&id) {
                if private_types.contains(&id) {
                    // It's private; print it in the .c file.
                    self.src.c_defs(ty);
                } else {
                    // It's public; print it in the .h file.
                    self.src.h_defs(ty);
                    self.print_dtor(resolve, id);
                }
            }
        }
    }

    fn finish_resources(
        &self,
        resolve: &Resolve,
        h_str: &mut wit_bindgen_core::Source,
        c_str: &mut wit_bindgen_core::Source,
    ) {
        // Workaround for the space-swallowing `Source::push_str` state machine:
        let space = " ";

        for (id, info) in &self.resources {
            let namespace = self.owner_namespace(resolve, *id);
            let name = resolve.types[*id].name.as_deref().unwrap();
            let snake = name.to_snake_case();
            let module = self.owner_wasm_namespace(resolve, *id);

            for s in [&mut *h_str, &mut *c_str] {
                uwriteln!(
                    s,
                    "\n// Functions for working with resource `{module}/{name}`"
                );
            }

            if let (Direction::Import, Some(own), Some(borrow)) =
                (&info.direction, &info.own, &info.borrow)
            {
                let own_namespace = self.owner_namespace(resolve, *own);
                let own_name = format!("{own_namespace}_own_{snake}_t");
                let borrow_namespace = self.owner_namespace(resolve, *borrow);
                let borrow_name = format!("{borrow_namespace}_borrow_{snake}_t");

                uwriteln!(
                    h_str,
                    "{borrow_name} {namespace}_borrow_{snake}({own_name});"
                );

                uwriteln!(
                    c_str,
                    "{borrow_name} {namespace}_borrow_{snake}({own_name}{space}arg) {{
                         return ({borrow_name}) {{ arg.__handle }};
                     }}"
                );
            }

            if let Some(own) = &info.own {
                let own_namespace = self.owner_namespace(resolve, *own);
                let own_name = format!("{own_namespace}_own_{snake}_t");

                uwriteln!(h_str, "void {namespace}_{snake}_drop_own({own_name});");

                uwriteln!(
                    c_str,
                    r#"__attribute__((__import_module__("{module}"), __import_name__("[resource-drop]{name}")))
                       void __wasm_import_{namespace}_{snake}_drop_own(int32_t);

                       void {namespace}_{snake}_drop_own({own_name}{space}arg) {{
                           __wasm_import_{namespace}_{snake}_drop_own(arg.__handle);
                       }}"#
                );
            }

            if let (Direction::Import, Some(borrow)) = (&info.direction, &info.borrow) {
                let borrow_namespace = self.owner_namespace(resolve, *borrow);
                let borrow_name = format!("{borrow_namespace}_borrow_{snake}_t");

                uwriteln!(
                    h_str,
                    "void {borrow_namespace}_{snake}_drop_borrow({borrow_name});"
                );

                uwriteln!(
                    c_str,
                    r#"__attribute__((__import_module__("{module}"), __import_name__("[resource-drop]{name}")))
                       void __wasm_import_{borrow_namespace}_{snake}_drop_borrow(int32_t);

                       void {borrow_namespace}_{snake}_drop_borrow({borrow_name}{space}arg) {{
                           __wasm_import_{borrow_namespace}_{snake}_drop_borrow(arg.__handle);
                       }}"#
                );
            }

            if let (Direction::Export, Some(own)) = (&info.direction, &info.own) {
                let own_namespace = self.owner_namespace(resolve, *own);
                let own_name = format!("{own_namespace}_own_{snake}_t");

                uwriteln!(
                    h_str,
                    "{own_name} {namespace}_{snake}_new({namespace}_{snake}_t*);"
                );

                uwriteln!(
                    c_str,
                    r#"__attribute__((__import_module__("{module}"), __import_name__("[resource-new]{name}")))
                       int32_t __wasm_import_{namespace}_{snake}_new(int32_t);

                       {own_name} {namespace}_{snake}_new({namespace}_{snake}_t* arg) {{
                           return ({own_name}) {{ __wasm_import_{namespace}_{snake}_new((int32_t) arg) }};
                       }}"#
                );

                uwriteln!(
                    h_str,
                    "{namespace}_{snake}_t* {namespace}_{snake}_rep({own_name});"
                );

                uwriteln!(
                    c_str,
                    r#"__attribute__((__import_module__("{module}"), __import_name__("[resource-rep]{snake}")))
                       int32_t __wasm_import_{namespace}_{snake}_rep(int32_t);

                       {namespace}_{snake}_t* {namespace}_{snake}_rep({own_name}{space}arg) {{
                           return ({namespace}_{snake}_t*) __wasm_import_{namespace}_{snake}_rep(arg.__handle);
                       }}"#
                );

                uwriteln!(
                    h_str,
                    "void {namespace}_{snake}_destructor({namespace}_{snake}_t*);"
                );

                uwriteln!(
                    c_str,
                    r#"__attribute__((__export_name__("{module}#[dtor]{snake}")))
                       void __wasm_export_{namespace}_{snake}_dtor({namespace}_{snake}_t* arg) {{
                           {namespace}_{snake}_destructor(arg);
                       }}"#
                );
            }
        }
    }

    fn find_handle_alias_target(
        &mut self,
        resolve: &Resolve,
        handle: &Handle,
        handles: &HashMap<&Handle, TypeId>,
    ) -> Option<TypeId> {
        let mut target = match handle {
            Handle::Borrow(target) => target,
            Handle::Own(target) => target,
        };

        loop {
            if let TypeDefKind::Type(Type::Id(id)) = &resolve.types[*target].kind {
                let target_handle = match handle {
                    Handle::Borrow(_) => Handle::Borrow(*id),
                    Handle::Own(_) => Handle::Own(*id),
                };
                if let Some(ty) = handles.get(&target_handle) {
                    break Some(*ty);
                } else {
                    target = id;
                }
            } else {
                break None;
            }
        }
    }

    fn print_anonymous_type(
        &mut self,
        resolve: &Resolve,
        ty: TypeId,
        handles: &HashMap<&Handle, TypeId>,
    ) {
        // If this anonymous type is already defined then it was referred to
        // twice from multiple various locations, so skip the second set of
        // bindings as they'll be the same as the first.
        let mut name = self.owner_namespace(resolve, ty);
        name.push_str("_");
        push_ty_name(
            resolve,
            &Type::Id(ty),
            &self.interface_names,
            &self.world,
            &mut name,
        );
        name.push_str("_t");
        if self.names.insert(&name).is_err() {
            return;
        }

        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\ntypedef ");
        let kind = &resolve.types[ty].kind;
        match kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Resource
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {
                unreachable!()
            }
            TypeDefKind::Handle(handle) => {
                // If this is an alias to a handle _and_ a corresponding handle type exists for the target of the
                // alias, generate a typedef alias.  Otherwise, generate an independent type.
                if let Some(ty) = self.find_handle_alias_target(resolve, handle, handles) {
                    let mut name = self.owner_namespace(resolve, ty);
                    name.push_str("_");
                    push_ty_name(
                        resolve,
                        &Type::Id(ty),
                        &self.interface_names,
                        &self.world,
                        &mut name,
                    );
                    name.push_str("_t");
                    self.src.h_defs(&name);
                } else {
                    self.src.h_defs("struct {\nint32_t __handle;\n}");
                }
            }
            TypeDefKind::Tuple(t) => {
                self.src.h_defs("struct {\n");
                for (i, t) in t.types.iter().enumerate() {
                    let ty = self.type_name(resolve, t);
                    uwriteln!(self.src.h_defs, "{ty} f{i};");
                }
                self.src.h_defs("}");
            }
            TypeDefKind::Option(t) => {
                self.src.h_defs("struct {\n");
                self.src.h_defs("bool is_some;\n");
                if !is_empty_type(resolve, t) {
                    let ty = self.type_name(resolve, t);
                    uwriteln!(self.src.h_defs, "{ty} val;");
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
                if let Some(ok) = get_nonempty_type(resolve, r.ok.as_ref()) {
                    let ty = self.type_name(resolve, ok);
                    uwriteln!(self.src.h_defs, "{ty} ok;");
                }
                if let Some(err) = get_nonempty_type(resolve, r.err.as_ref()) {
                    let ty = self.type_name(resolve, err);
                    uwriteln!(self.src.h_defs, "{ty} err;");
                }
                self.src.h_defs("} val;\n");
                self.src.h_defs("}");
            }
            TypeDefKind::List(t) => {
                self.src.h_defs("struct {\n");
                let ty = self.type_name(resolve, t);
                uwriteln!(self.src.h_defs, "{ty} *ptr;");
                self.src.h_defs("size_t len;\n");
                self.src.h_defs("}");
            }
            TypeDefKind::Future(_) => todo!("print_anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("print_anonymous_type for stream"),
            TypeDefKind::Unknown => unreachable!(),
        }
        self.src.h_defs(" ");
        self.src.h_defs(&name);
        self.src.h_defs(";\n");
        let type_source = mem::replace(&mut self.src.h_defs, prev);
        self.types.insert(ty, type_source);
    }

    fn print_dtor(&mut self, resolve: &Resolve, id: TypeId) {
        fn is_local_resource<'a>(this: &'a C) -> impl Fn(&Resolve, TypeId) -> bool + 'a {
            move |resolve, resource| {
                matches!(
                    this.resources
                        .get(&dealias(resolve, resource))
                        .map(|info| &info.direction),
                    Some(Direction::Export)
                )
            }
        }

        let ty = Type::Id(id);
        if !owns_anything(resolve, &ty, &is_local_resource(self)) {
            return;
        }
        let pos = self.src.h_helpers.len();
        self.src.h_helpers("\nvoid ");
        let ns = self.owner_namespace(resolve, id);
        self.src.h_helpers(&ns);
        self.src.h_helpers("_");
        self.src
            .h_helpers
            .print_ty_name(&self.interface_names, &self.world, resolve, &ty);
        self.src.h_helpers("_free(");
        self.src.h_helpers(&ns);
        self.src.h_helpers("_");
        self.src
            .h_helpers
            .print_ty_name(&self.interface_names, &self.world, resolve, &ty);
        self.src.h_helpers("_t *ptr)");

        self.src.c_helpers(&self.src.h_helpers[pos..].to_string());
        self.src.h_helpers(";");
        self.src.c_helpers(" {\n");
        match &resolve.types[id].kind {
            TypeDefKind::Type(t) => self.free(resolve, t, "ptr"),

            TypeDefKind::Flags(_) => {}
            TypeDefKind::Enum(_) => {}

            TypeDefKind::Record(r) => {
                for field in r.fields.iter() {
                    if !owns_anything(resolve, &field.ty, &is_local_resource(self)) {
                        continue;
                    }
                    self.free(
                        resolve,
                        &field.ty,
                        &format!("&ptr->{}", to_c_ident(&field.name)),
                    );
                }
            }

            TypeDefKind::Tuple(t) => {
                for (i, ty) in t.types.iter().enumerate() {
                    if !owns_anything(resolve, ty, &is_local_resource(self)) {
                        continue;
                    }
                    self.free(resolve, ty, &format!("&ptr->f{i}"));
                }
            }

            TypeDefKind::List(t) => {
                if owns_anything(resolve, t, &is_local_resource(self)) {
                    self.src
                        .c_helpers("for (size_t i = 0; i < ptr->len; i++) {\n");
                    self.free(resolve, t, "&ptr->ptr[i]");
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
                        if !owns_anything(resolve, ty, &is_local_resource(self)) {
                            continue;
                        }
                        uwriteln!(self.src.c_helpers, "case {}: {{", i);
                        let expr = format!("&ptr->val.{}", to_c_ident(&case.name));
                        if let Some(ty) = &case.ty {
                            self.free(resolve, ty, &expr);
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
                    if !owns_anything(resolve, &case.ty, &is_local_resource(self)) {
                        continue;
                    }
                    uwriteln!(self.src.c_helpers, "case {i}: {{");
                    let expr = format!("&ptr->val.f{i}");
                    self.free(resolve, &case.ty, &expr);
                    self.src.c_helpers("break;\n");
                    self.src.c_helpers("}\n");
                }
                self.src.c_helpers("}\n");
            }

            TypeDefKind::Option(t) => {
                self.src.c_helpers("if (ptr->is_some) {\n");
                self.free(resolve, t, "&ptr->val");
                self.src.c_helpers("}\n");
            }

            TypeDefKind::Result(r) => {
                self.src.c_helpers("if (!ptr->is_err) {\n");
                if let Some(ok) = &r.ok {
                    if owns_anything(resolve, ok, &is_local_resource(self)) {
                        self.free(resolve, ok, "&ptr->val.ok");
                    }
                }
                if let Some(err) = &r.err {
                    if owns_anything(resolve, err, &is_local_resource(self)) {
                        self.src.c_helpers("} else {\n");
                        self.free(resolve, err, "&ptr->val.err");
                    }
                }
                self.src.c_helpers("}\n");
            }
            TypeDefKind::Future(_) => todo!("print_dtor for future"),
            TypeDefKind::Stream(_) => todo!("print_dtor for stream"),
            TypeDefKind::Resource => unreachable!(),
            TypeDefKind::Handle(handle) => {
                let handle_namespace = self.owner_namespace(resolve, id);
                let resource = dealias(
                    resolve,
                    *match handle {
                        Handle::Borrow(resource) => resource,
                        Handle::Own(resource) => resource,
                    },
                );
                let resource_namespace = self.owner_namespace(resolve, resource);
                let name = resolve.types[resource].name.as_deref().unwrap();
                let snake = name.to_snake_case();
                match handle {
                    Handle::Borrow(_) => uwriteln!(
                        self.src.c_helpers,
                        "{handle_namespace}_{snake}_drop_borrow(*ptr);"
                    ),
                    Handle::Own(_) => {
                        uwriteln!(
                            self.src.c_helpers,
                            "{resource_namespace}_{snake}_drop_own(*ptr);"
                        )
                    }
                }
            }
            TypeDefKind::Unknown => unreachable!(),
        }
        self.src.c_helpers("}\n");
    }

    fn free(&mut self, resolve: &Resolve, ty: &Type, expr: &str) {
        let prev = mem::take(&mut self.src.h_helpers);
        match ty {
            Type::Id(id) => {
                let ns = self.owner_namespace(resolve, *id);
                self.src.h_helpers(&ns);
            }
            _ => {
                self.src.h_helpers(&self.world.to_snake_case());
            }
        }
        self.src.h_helpers("_");
        self.src
            .h_helpers
            .print_ty_name(&self.interface_names, &self.world, resolve, ty);
        let name = mem::replace(&mut self.src.h_helpers, prev);

        self.src.c_helpers(&name);
        self.src.c_helpers("_free(");
        self.src.c_helpers(expr);
        self.src.c_helpers(");\n");
    }

    fn owner_namespace(&self, resolve: &Resolve, id: TypeId) -> String {
        owner_namespace(resolve, id, &self.interface_names).unwrap_or_else(|| {
            // Namespace everything else under the "default" world being
            // generated to avoid putting too much into the root namespace in C.
            self.world.to_snake_case()
        })
    }

    fn owner_wasm_namespace(&self, resolve: &Resolve, id: TypeId) -> String {
        let ty = &resolve.types[id];
        match ty.owner {
            TypeOwner::Interface(owner) => resolve.name_world_key(&self.interface_names[&owner]),

            TypeOwner::World(owner) => resolve.worlds[owner].name.clone(),

            // Namespace everything else under the "default" world being
            // generated to avoid putting too much into the root namespace in C.
            TypeOwner::None => self.world.clone(),
        }
    }

    fn type_name(&mut self, resolve: &Resolve, ty: &Type) -> String {
        let mut name = String::new();
        self.push_type_name(resolve, ty, &mut name);
        name
    }

    fn push_type_name(&mut self, resolve: &Resolve, ty: &Type, dst: &mut String) {
        match ty {
            Type::Bool => dst.push_str("bool"),
            Type::Char => dst.push_str("uint32_t"), // TODO: better type?
            Type::U8 => dst.push_str("uint8_t"),
            Type::S8 => dst.push_str("int8_t"),
            Type::U16 => dst.push_str("uint16_t"),
            Type::S16 => dst.push_str("int16_t"),
            Type::U32 => dst.push_str("uint32_t"),
            Type::S32 => dst.push_str("int32_t"),
            Type::U64 => dst.push_str("uint64_t"),
            Type::S64 => dst.push_str("int64_t"),
            Type::Float32 => dst.push_str("float"),
            Type::Float64 => dst.push_str("double"),
            Type::String => {
                dst.push_str(&self.world.to_snake_case());
                dst.push_str("_");
                dst.push_str("string_t");
                self.needs_string = true;
            }
            Type::Id(id) => {
                let ty = &resolve.types[*id];
                let ns = self.owner_namespace(resolve, *id);
                match &ty.name {
                    Some(name) => {
                        dst.push_str(&ns);
                        dst.push_str("_");
                        dst.push_str(&name.to_snake_case());
                        dst.push_str("_t");
                    }
                    None => match &ty.kind {
                        TypeDefKind::Type(t) => self.push_type_name(resolve, t, dst),
                        TypeDefKind::Handle(Handle::Borrow(resource))
                            if matches!(
                                self.resources
                                    .get(&dealias(resolve, *resource))
                                    .map(|info| &info.direction),
                                Some(Direction::Export)
                            ) =>
                        {
                            let resource = dealias(resolve, *resource);
                            self.resources.entry(resource).or_default().borrow = Some(*id);
                            self.push_type_name(resolve, &Type::Id(resource), dst);
                            dst.push_str("*");
                        }
                        _ => {
                            match &ty.kind {
                                TypeDefKind::Handle(Handle::Borrow(resource)) => {
                                    self.resources
                                        .entry(dealias(resolve, *resource))
                                        .or_default()
                                        .borrow = Some(*id);
                                }
                                TypeDefKind::Handle(Handle::Own(resource)) => {
                                    self.resources
                                        .entry(dealias(resolve, *resource))
                                        .or_default()
                                        .own = Some(*id);
                                }
                                _ => {}
                            }
                            self.public_anonymous_types.insert(*id);
                            self.private_anonymous_types.remove(id);
                            dst.push_str(&ns);
                            dst.push_str("_");
                            push_ty_name(
                                resolve,
                                &Type::Id(*id),
                                &self.interface_names,
                                &self.world,
                                dst,
                            );
                            dst.push_str("_t");
                        }
                    },
                }
            }
        }
    }
}

pub fn push_ty_name(
    resolve: &Resolve,
    ty: &Type,
    interface_names: &HashMap<InterfaceId, WorldKey>,
    world: &str,
    src: &mut String,
) {
    match ty {
        Type::Bool => src.push_str("bool"),
        Type::Char => src.push_str("char32"),
        Type::U8 => src.push_str("u8"),
        Type::S8 => src.push_str("s8"),
        Type::U16 => src.push_str("u16"),
        Type::S16 => src.push_str("s16"),
        Type::U32 => src.push_str("u32"),
        Type::S32 => src.push_str("s32"),
        Type::U64 => src.push_str("u64"),
        Type::S64 => src.push_str("s64"),
        Type::Float32 => src.push_str("float32"),
        Type::Float64 => src.push_str("float64"),
        Type::String => src.push_str("string"),
        Type::Id(id) => {
            let ty = &resolve.types[*id];
            if let Some(name) = &ty.name {
                return src.push_str(&name.to_snake_case());
            }
            match &ty.kind {
                TypeDefKind::Type(t) => push_ty_name(resolve, t, interface_names, world, src),
                TypeDefKind::Record(_)
                | TypeDefKind::Resource
                | TypeDefKind::Flags(_)
                | TypeDefKind::Enum(_)
                | TypeDefKind::Variant(_)
                | TypeDefKind::Union(_) => {
                    unimplemented!()
                }
                TypeDefKind::Tuple(t) => {
                    src.push_str("tuple");
                    src.push_str(&t.types.len().to_string());
                    for ty in t.types.iter() {
                        src.push_str("_");
                        push_optional_owner_namespace(ty, resolve, interface_names, src);
                        push_ty_name(resolve, ty, interface_names, world, src);
                    }
                }
                TypeDefKind::Option(ty) => {
                    src.push_str("option_");
                    push_optional_owner_namespace(ty, resolve, interface_names, src);
                    push_ty_name(resolve, ty, interface_names, world, src);
                }
                TypeDefKind::Result(r) => {
                    src.push_str("result_");
                    push_optional_ty_name(resolve, r.ok.as_ref(), interface_names, world, src);
                    src.push_str("_");
                    push_optional_ty_name(resolve, r.err.as_ref(), interface_names, world, src);
                }
                TypeDefKind::List(ty) => {
                    src.push_str("list_");
                    push_optional_owner_namespace(ty, resolve, interface_names, src);
                    push_ty_name(resolve, ty, interface_names, world, src);
                }
                TypeDefKind::Future(ty) => {
                    src.push_str("future_");
                    push_optional_ty_name(resolve, ty.as_ref(), interface_names, world, src);
                }
                TypeDefKind::Stream(s) => {
                    src.push_str("stream_");
                    push_optional_ty_name(resolve, s.element.as_ref(), interface_names, world, src);
                    src.push_str("_");
                    push_optional_ty_name(resolve, s.end.as_ref(), interface_names, world, src);
                }
                TypeDefKind::Handle(handle) => {
                    push_handle_name(resolve, handle, interface_names, world, src);
                }
                TypeDefKind::Unknown => unreachable!(),
            }
        }
    }
}

fn push_handle_name(
    resolve: &Resolve,
    handle: &Handle,
    interface_names: &HashMap<InterfaceId, WorldKey>,
    world: &str,
    src: &mut String,
) {
    let (resource, prefix) = match handle {
        Handle::Own(resource) => (resource, "own_"),
        Handle::Borrow(resource) => (resource, "borrow_"),
    };
    src.push_str(prefix);
    push_ty_name(resolve, &Type::Id(*resource), interface_names, world, src);
}

fn push_optional_ty_name(
    resolve: &Resolve,
    ty: Option<&Type>,
    interface_names: &HashMap<InterfaceId, WorldKey>,
    world: &str,
    dst: &mut String,
) {
    match ty {
        Some(ty) => {
            push_optional_owner_namespace(ty, resolve, interface_names, dst);
            push_ty_name(resolve, ty, interface_names, world, dst)
        }
        None => dst.push_str("void"),
    }
}

// If the type is referenced through an id, prepend the owner namespace to ensure disambiguation
fn push_optional_owner_namespace(
    ty: &Type,
    resolve: &Resolve,
    interface_names: &HashMap<InterfaceId, WorldKey>,
    dst: &mut String,
) {
    if let Type::Id(i) = ty {
        let namespace = owner_namespace(resolve, *i, interface_names);
        if let Some(namespace) = namespace {
            dst.push_str(&namespace);
            dst.push_str("_");
        }
    }
}

pub fn owner_namespace(
    resolve: &Resolve,
    id: TypeId,
    interface_names: &HashMap<InterfaceId, WorldKey>,
) -> Option<String> {
    let ty = &resolve.types[id];
    match ty.owner {
        TypeOwner::Interface(owner) => {
            Some(interface_identifier(&interface_names[&owner], resolve))
        }
        TypeOwner::World(owner) => Some(resolve.worlds[owner].name.to_snake_case()),
        TypeOwner::None => None,
    }
}

fn interface_identifier(interface_id: &WorldKey, resolve: &Resolve) -> String {
    match interface_id {
        WorldKey::Name(name) => name.to_snake_case(),
        WorldKey::Interface(id) => {
            let mut ns = String::new();
            let iface = &resolve.interfaces[*id];
            let pkg = &resolve.packages[iface.package.unwrap()];
            ns.push_str(&pkg.name.namespace.to_snake_case());
            ns.push_str("_");
            ns.push_str(&pkg.name.name.to_snake_case());
            ns.push_str("_");
            ns.push_str(&iface.name.as_ref().unwrap().to_snake_case());
            ns
        }
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    in_import: bool,
    gen: &'a mut C,
    resolve: &'a Resolve,
    interface: Option<InterfaceId>,
}

impl C {
    fn print_intrinsics(&mut self) {
        // Note that these intrinsics are declared as `weak` so they can be
        // overridden from some other symbol.
        self.src.c_fns(
            r#"
                __attribute__((__weak__, __export_name__("cabi_realloc")))
                void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
                    (void) old_size;
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
    fn return_single(
        &mut self,
        resolve: &Resolve,
        ty: &Type,
        orig_ty: &Type,
        sig_flattening: bool,
    ) {
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
            TypeDefKind::Type(t) => return self.return_single(resolve, t, orig_ty, sig_flattening),

            // Flags are returned as their bare values, and enums and handles are scalars
            TypeDefKind::Flags(_) | TypeDefKind::Enum(_) | TypeDefKind::Handle(_) => {
                self.scalar = Some(Scalar::Type(*orig_ty));
                return;
            }

            // Unpack optional returns where a boolean discriminant is
            // returned and then the actual type returned is returned
            // through a return pointer.
            TypeDefKind::Option(ty) => {
                if sig_flattening {
                    self.scalar = Some(Scalar::OptionBool(*ty));
                    self.retptrs.push(*ty);
                    return;
                }
            }

            // Unpack a result as a boolean return type, with two
            // return pointers for ok and err values
            TypeDefKind::Result(r) => {
                if sig_flattening {
                    if let Some(ok) = r.ok {
                        self.retptrs.push(ok);
                    }
                    if let Some(err) = r.err {
                        self.retptrs.push(err);
                    }
                    self.scalar = Some(Scalar::ResultBool(r.ok, r.err));
                    return;
                }
            }

            // These types are always returned indirectly.
            TypeDefKind::Tuple(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::List(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {}

            TypeDefKind::Future(_) => todo!("return_single for future"),
            TypeDefKind::Stream(_) => todo!("return_single for stream"),
            TypeDefKind::Resource => todo!("return_single for resource"),
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
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        for field in record.fields.iter() {
            self.docs(&field.docs, SourceType::HDefs);
            self.print_ty(SourceType::HDefs, &field.ty);
            self.src.h_defs(" ");
            self.src.h_defs(&to_c_ident(&field.name));
            self.src.h_defs(";\n");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        self.finish_ty(id, prev);
    }

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);

        if !self.in_import {
            self.src.h_defs("\n");
            self.docs(docs, SourceType::HDefs);
            self.src.h_defs("typedef struct ");
            let ns = self.gen.owner_namespace(self.resolve, id).to_snake_case();
            let snake = name.to_snake_case();
            self.src.h_defs(&ns);
            self.src.h_defs("_");
            self.src.h_defs(&snake);
            self.src.h_defs("_t ");
            self.src.h_defs(&ns);
            self.src.h_defs("_");
            self.src.h_defs(&snake);
            self.src.h_defs("_t;\n");
            self.gen.names.insert(&format!("{ns}_{snake}_t")).unwrap();
        }

        self.gen.resources.entry(id).or_default().direction = if self.in_import {
            Direction::Import
        } else {
            Direction::Export
        };

        self.finish_ty(id, prev);
    }

    fn type_tuple(&mut self, id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        for (i, ty) in tuple.types.iter().enumerate() {
            self.print_ty(SourceType::HDefs, ty);
            uwriteln!(self.src.h_defs, " f{i};");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        self.finish_ty(id, prev);
    }

    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef ");
        let repr = flags_repr(flags);
        self.src.h_defs(int_repr(repr));
        self.src.h_defs(" ");
        self.print_typedef_target(id, name);

        if flags.flags.len() > 0 {
            self.src.h_defs("\n");
        }
        let ns = self
            .gen
            .owner_namespace(self.resolve, id)
            .to_shouty_snake_case();
        for (i, flag) in flags.flags.iter().enumerate() {
            self.docs(&flag.docs, SourceType::HDefs);
            uwriteln!(
                self.src.h_defs,
                "#define {ns}_{}_{} (1 << {i})",
                name.to_shouty_snake_case(),
                flag.name.to_shouty_snake_case(),
            );
        }

        self.finish_ty(id, prev);
    }

    fn type_variant(&mut self, id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs(int_repr(variant.tag()));
        self.src.h_defs(" tag;\n");
        self.src.h_defs("union {\n");
        for case in variant.cases.iter() {
            if let Some(ty) = get_nonempty_type(self.resolve, case.ty.as_ref()) {
                self.print_ty(SourceType::HDefs, ty);
                self.src.h_defs(" ");
                self.src.h_defs(&to_c_ident(&case.name));
                self.src.h_defs(";\n");
            }
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        if variant.cases.len() > 0 {
            self.src.h_defs("\n");
        }
        let ns = self
            .gen
            .owner_namespace(self.resolve, id)
            .to_shouty_snake_case();
        for (i, case) in variant.cases.iter().enumerate() {
            self.docs(&case.docs, SourceType::HDefs);
            uwriteln!(
                self.src.h_defs,
                "#define {ns}_{}_{} {i}",
                name.to_shouty_snake_case(),
                case.name.to_shouty_snake_case(),
            );
        }

        self.finish_ty(id, prev);
    }

    fn type_union(&mut self, id: TypeId, name: &str, union: &Union, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs(int_repr(union.tag()));
        self.src.h_defs(" tag;\n");
        self.src.h_defs("union {\n");
        for (i, case) in union.cases.iter().enumerate() {
            self.docs(&case.docs, SourceType::HDefs);
            self.print_ty(SourceType::HDefs, &case.ty);
            uwriteln!(self.src.h_defs, " f{i};");
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        self.finish_ty(id, prev);
    }

    fn type_option(&mut self, id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs("bool is_some;\n");
        if !is_empty_type(self.resolve, payload) {
            self.print_ty(SourceType::HDefs, payload);
            self.src.h_defs(" val;\n");
        }
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        self.finish_ty(id, prev);
    }

    fn type_result(&mut self, id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        self.src.h_defs("bool is_err;\n");
        self.src.h_defs("union {\n");
        if let Some(ok) = get_nonempty_type(self.resolve, result.ok.as_ref()) {
            self.print_ty(SourceType::HDefs, ok);
            self.src.h_defs(" ok;\n");
        }
        if let Some(err) = get_nonempty_type(self.resolve, result.err.as_ref()) {
            self.print_ty(SourceType::HDefs, err);
            self.src.h_defs(" err;\n");
        }
        self.src.h_defs("} val;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);

        self.finish_ty(id, prev);
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        uwrite!(self.src.h_defs, "\n");
        self.docs(docs, SourceType::HDefs);
        let int_t = int_repr(enum_.tag());
        uwrite!(self.src.h_defs, "typedef {int_t} ");
        self.print_typedef_target(id, name);

        if enum_.cases.len() > 0 {
            self.src.h_defs("\n");
        }
        let ns = self
            .gen
            .owner_namespace(self.resolve, id)
            .to_shouty_snake_case();
        for (i, case) in enum_.cases.iter().enumerate() {
            self.docs(&case.docs, SourceType::HDefs);
            uwriteln!(
                self.src.h_defs,
                "#define {ns}_{}_{} {i}",
                name.to_shouty_snake_case(),
                case.name.to_shouty_snake_case(),
            );
        }

        self.finish_ty(id, prev);
    }

    fn type_alias(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);

        let target = dealias(self.resolve, id);
        if !matches!(&self.resolve.types[target].kind,
                     TypeDefKind::Resource if self.gen.resources[&target].direction == Direction::Import)
        {
            self.src.h_defs("\n");
            self.docs(docs, SourceType::HDefs);
            self.src.h_defs("typedef ");
            self.print_ty(SourceType::HDefs, ty);
            self.src.h_defs(" ");
            self.print_typedef_target(id, name);
        }

        self.finish_ty(id, prev);
    }

    fn type_list(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let prev = mem::take(&mut self.src.h_defs);
        self.src.h_defs("\n");
        self.docs(docs, SourceType::HDefs);
        self.src.h_defs("typedef struct {\n");
        self.print_ty(SourceType::HDefs, ty);
        self.src.h_defs(" *ptr;\n");
        self.src.h_defs("size_t len;\n");
        self.src.h_defs("} ");
        self.print_typedef_target(id, name);
        self.finish_ty(id, prev);
    }

    fn type_builtin(&mut self, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        let _ = (id, name, ty, docs);
    }
}

impl InterfaceGenerator<'_> {
    fn c_func_name(&self, interface_name: Option<&WorldKey>, func: &Function) -> String {
        let mut name = String::new();
        match interface_name {
            Some(WorldKey::Name(k)) => name.push_str(&k.to_snake_case()),
            Some(WorldKey::Interface(id)) => {
                if !self.in_import {
                    name.push_str("exports_");
                }
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                name.push_str(&pkg.name.namespace.to_snake_case());
                name.push_str("_");
                name.push_str(&pkg.name.name.to_snake_case());
                name.push_str("_");
                name.push_str(&iface.name.as_ref().unwrap().to_snake_case());
            }
            None => name.push_str(&self.gen.world.to_snake_case()),
        }
        name.push_str("_");
        name.push_str(&func.name.to_snake_case().replace('.', "_"));
        name
    }

    fn import(&mut self, interface_name: Option<&WorldKey>, func: &Function) {
        self.docs(&func.docs, SourceType::HFns);
        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        self.src.c_fns("\n");

        // In the private C file, print a function declaration which is the
        // actual wasm import that we'll be calling, and this has the raw wasm
        // signature.
        uwriteln!(
            self.src.c_fns,
            "__attribute__((__import_module__(\"{}\"), __import_name__(\"{}\")))",
            match interface_name {
                Some(name) => self.resolve.name_world_key(name),
                None => "$root".to_string(),
            },
            func.name
        );
        let name = self.c_func_name(interface_name, func);
        let import_name = self.gen.names.tmp(&format!("__wasm_import_{name}",));
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
        let c_sig = self.print_sig(interface_name, func, !self.gen.opts.no_sig_flattening);
        self.src.c_adapters("\n");
        self.src.c_adapters(&c_sig.sig);
        self.src.c_adapters(" {\n");

        // construct optional adapters from maybe pointers to real optional
        // structs internally
        let mut optional_adapters = String::from("");
        if !self.gen.opts.no_sig_flattening {
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
                        if !is_empty_type(self.resolve, option_ty) {
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
        abi::call(
            f.gen.resolve,
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
                    __attribute__((__aligned__({import_return_pointer_area_align})))
                    uint8_t ret_area[{import_return_pointer_area_size}];
                ",
            ));
        }

        self.src.c_adapters(&String::from(src));
        self.src.c_adapters("}\n");
    }

    fn export(&mut self, func: &Function, interface_name: Option<&WorldKey>) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let core_module_name = interface_name.map(|s| self.resolve.name_world_key(s));
        let export_name = func.core_export_name(core_module_name.as_deref());

        // Print the actual header for this function into the header file, and
        // it's what we'll be calling.
        let h_sig = self.print_sig(interface_name, func, !self.gen.opts.no_sig_flattening);

        // Generate, in the C source file, the raw wasm signature that has the
        // canonical ABI.
        uwriteln!(
            self.src.c_adapters,
            "\n__attribute__((__export_name__(\"{export_name}\")))"
        );
        let name = self.c_func_name(interface_name, func);
        let import_name = self.gen.names.tmp(&format!("__wasm_export_{name}"));

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
        abi::call(
            f.gen.resolve,
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );
        let FunctionBindgen { src, .. } = f;
        self.src.c_adapters(&src);
        self.src.c_adapters("}\n");

        if abi::guest_export_needs_post_return(self.resolve, func) {
            uwriteln!(
                self.src.c_fns,
                "__attribute__((__weak__, __export_name__(\"cabi_post_{export_name}\")))"
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
            abi::post_return(f.gen.resolve, func, &mut f);
            let FunctionBindgen { src, .. } = f;
            self.src.c_fns(&src);
            self.src.c_fns("}\n");
        }
    }

    fn print_sig(
        &mut self,
        interface_name: Option<&WorldKey>,
        func: &Function,
        sig_flattening: bool,
    ) -> CSig {
        let name = self.c_func_name(interface_name, func);
        self.gen.names.insert(&name).expect("duplicate symbols");

        let start = self.src.h_fns.len();
        let mut result_rets = false;
        let mut result_rets_has_ok_type = false;

        let ret = self.classify_ret(func, sig_flattening);
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
            let pointer = is_arg_by_pointer(self.resolve, ty);
            // optional param pointer sig_flattening
            let optional_type = if let Type::Id(id) = ty {
                if let TypeDefKind::Option(option_ty) = &self.resolve.types[*id].kind {
                    if sig_flattening {
                        Some(option_ty)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            let (print_ty, print_name) = if sig_flattening {
                if let Some(option_ty) = optional_type {
                    (option_ty, format!("maybe_{}", to_c_ident(name)))
                } else {
                    (ty, to_c_ident(name))
                }
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

    fn classify_ret(&mut self, func: &Function, sig_flattening: bool) -> Return {
        let mut ret = Return::default();
        match func.results.len() {
            0 => ret.scalar = Some(Scalar::Void),
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                ret.return_single(self.resolve, ty, ty, sig_flattening);
            }
            _ => {
                ret.retptrs.extend(func.results.iter_types().cloned());
            }
        }
        return ret;
    }

    fn print_typedef_target(&mut self, id: TypeId, name: &str) {
        let ns = self.gen.owner_namespace(self.resolve, id).to_snake_case();
        let snake = name.to_snake_case();
        self.src.h_defs(&ns);
        self.src.h_defs("_");
        self.src.h_defs(&snake);
        self.src.h_defs("_t;\n");
        self.gen.names.insert(&format!("{ns}_{snake}_t")).unwrap();
    }

    fn print_ty(&mut self, stype: SourceType, ty: &Type) {
        self.gen
            .push_type_name(self.resolve, ty, self.src.src(stype).as_mut_string());
    }

    fn docs(&mut self, docs: &Docs, stype: SourceType) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        let src = self.src.src(stype);
        for line in docs.trim().lines() {
            src.push_str("// ");
            src.push_str(line);
            src.push_str("\n");
        }
    }

    fn type_string(&mut self, ty: &Type) -> String {
        // Getting a type string happens during codegen, and by default means
        // that this is a private type that's being generated. This means we
        // want to keep track of new anonymous types that are *only* mentioned
        // in methods like this, so we can place those types in the C file
        // instead of the header interface file.
        let prev = mem::take(&mut self.src.h_defs);
        let prev_public = mem::take(&mut self.gen.public_anonymous_types);
        let prev_private = mem::take(&mut self.gen.private_anonymous_types);

        // Print the type, which will collect into the fields that we replaced
        // above.
        self.print_ty(SourceType::HDefs, ty);

        // Reset our public/private sets back to what they were beforehand.
        // Note that `print_ty` always adds to the public set, so we're
        // inverting the meaning here by interpreting those as new private
        // types.
        let new_private = mem::replace(&mut self.gen.public_anonymous_types, prev_public);
        assert!(self.gen.private_anonymous_types.is_empty());
        self.gen.private_anonymous_types = prev_private;

        // For all new private types found while we printed this type, if the
        // type isn't already public then it's a new private type.
        for id in new_private {
            if !self.gen.public_anonymous_types.contains(&id) {
                self.gen.private_anonymous_types.insert(id);
            }
        }

        mem::replace(&mut self.src.h_defs, prev).into()
    }

    fn finish_ty(&mut self, id: TypeId, orig_h_defs: wit_bindgen_core::Source) {
        let prev = self
            .gen
            .types
            .insert(id, mem::replace(&mut self.src.h_defs, orig_h_defs));
        assert!(prev.is_none());
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

    fn empty_return_value(&mut self) {
        // Empty types have no state, so we don't emit stores for them. But we
        // do need to keep track of which return variable we're looking at.
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
        resolve: &Resolve,
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

            Instruction::HandleLower { .. } => {
                let op = &operands[0];
                results.push(format!("({op}).__handle"))
            }

            Instruction::HandleLift { handle, ty, .. } => match handle {
                Handle::Borrow(resource)
                    if matches!(
                        self.gen.gen.resources[&dealias(resolve, *resource)].direction,
                        Direction::Export
                    ) =>
                {
                    // Here we've received a borrow of a resource which we've exported ourselves, so we can treat
                    // it as a raw pointer rather than an opaque handle.
                    let op = &operands[0];
                    let name = self.gen.type_string(&Type::Id(dealias(resolve, *resource)));
                    results.push(format!("(({name}*) {op})"))
                }
                _ => {
                    let op = &operands[0];
                    let name = self.gen.type_string(&Type::Id(*ty));
                    results.push(format!("({name}) {{ {op} }}"))
                }
            },

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
                    if let Some(ty) = get_nonempty_type(self.gen.resolve, case.ty.as_ref()) {
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

                    if let Some(_) = get_nonempty_type(self.gen.resolve, case.ty.as_ref()) {
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
                    if !is_empty_type(self.gen.resolve, &case.ty) {
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
                let bind_some = if is_empty_type(self.gen.resolve, payload) {
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
                let set_some = if is_empty_type(self.gen.resolve, payload) {
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
                let bind_ok =
                    if let Some(ok) = get_nonempty_type(self.gen.resolve, result.ok.as_ref()) {
                        let ok_ty = self.gen.type_string(ok);
                        format!("const {ok_ty} *{ok_payload} = &({op0}).val.ok;")
                    } else {
                        String::new()
                    };
                let bind_err =
                    if let Some(err) = get_nonempty_type(self.gen.resolve, result.err.as_ref()) {
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
                let set_ok =
                    if let Some(_) = get_nonempty_type(self.gen.resolve, result.ok.as_ref()) {
                        let ok_result = &ok_results[0];
                        format!("{result_tmp}.val.ok = {ok_result};\n")
                    } else {
                        String::new()
                    };
                let set_err =
                    if let Some(_) = get_nonempty_type(self.gen.resolve, result.err.as_ref()) {
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
                                    if is_empty_type(self.gen.resolve, option_ty) {
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
                        if !is_empty_type(self.gen.resolve, ty) {
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
                        if get_nonempty_type(self.gen.resolve, err.as_ref()).is_some() {
                            if let Some(err_name) = err_name {
                                uwriteln!(
                                    self.src,
                                    "if ({ret}.is_err) {{
                                        {ret}.val.err = {err_name};
                                    }}",
                                );
                            }
                        }
                        if get_nonempty_type(self.gen.resolve, ok.as_ref()).is_some() {
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
                    if !is_empty_type(self.gen.resolve, &o) {
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
                    if ok.is_some() {
                        if get_nonempty_type(self.gen.resolve, ok.as_ref()).is_some() {
                            self.store_in_retptr(&format!("{}.val.ok", variant));
                        } else {
                            self.empty_return_value();
                        }
                    }
                    uwriteln!(
                        self.src,
                        "   return 1;
                            }} else {{"
                    );
                    if err.is_some() {
                        if get_nonempty_type(self.gen.resolve, err.as_ref()).is_some() {
                            self.store_in_retptr(&format!("{}.val.err", variant));
                        } else {
                            self.empty_return_value();
                        }
                    }
                    uwriteln!(
                        self.src,
                        "   return 0;
                            }}"
                    );
                    assert_eq!(self.ret_store_cnt, self.sig.retptrs.len());
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
    // HHelpers,
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
    fn src(&mut self, stype: SourceType) -> &mut wit_bindgen_core::Source {
        match stype {
            SourceType::HDefs => &mut self.h_defs,
            SourceType::HFns => &mut self.h_fns,
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

trait SourceExt {
    fn as_source(&mut self) -> &mut wit_bindgen_core::Source;

    fn print_ty_name(
        &mut self,
        interface_names: &HashMap<InterfaceId, WorldKey>,
        world: &str,
        resolve: &Resolve,
        ty: &Type,
    ) {
        push_ty_name(
            resolve,
            ty,
            interface_names,
            world,
            self.as_source().as_mut_string(),
        );
    }
}

impl SourceExt for wit_bindgen_core::Source {
    fn as_source(&mut self) -> &mut wit_bindgen_core::Source {
        self
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

pub fn int_repr(ty: Int) -> &'static str {
    match ty {
        Int::U8 => "uint8_t",
        Int::U16 => "uint16_t",
        Int::U32 => "uint32_t",
        Int::U64 => "uint64_t",
    }
}

pub fn flags_repr(f: &Flags) -> Int {
    match f.repr() {
        FlagsRepr::U8 => Int::U8,
        FlagsRepr::U16 => Int::U16,
        FlagsRepr::U32(1) => Int::U32,
        FlagsRepr::U32(2) => Int::U64,
        repr => panic!("unimplemented flags {:?}", repr),
    }
}

pub fn is_arg_by_pointer(resolve: &Resolve, ty: &Type) -> bool {
    match ty {
        Type::Id(id) => match resolve.types[*id].kind {
            TypeDefKind::Type(t) => is_arg_by_pointer(resolve, &t),
            TypeDefKind::Variant(_) => true,
            TypeDefKind::Union(_) => true,
            TypeDefKind::Option(_) => true,
            TypeDefKind::Result(_) => true,
            TypeDefKind::Enum(_) => false,
            TypeDefKind::Flags(_) => false,
            TypeDefKind::Handle(_) => false,
            TypeDefKind::Tuple(_) | TypeDefKind::Record(_) | TypeDefKind::List(_) => true,
            TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
            TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
            TypeDefKind::Resource => todo!("is_arg_by_pointer for resource"),
            TypeDefKind::Unknown => unreachable!(),
        },
        Type::String => true,
        _ => false,
    }
}

pub fn is_empty_type(resolve: &Resolve, ty: &Type) -> bool {
    let id = match ty {
        Type::Id(id) => *id,
        _ => return false,
    };
    match &resolve.types[id].kind {
        TypeDefKind::Type(t) => is_empty_type(resolve, t),
        TypeDefKind::Record(r) => r.fields.is_empty(),
        TypeDefKind::Tuple(t) => t.types.is_empty(),
        _ => false,
    }
}

pub fn get_nonempty_type<'o>(resolve: &Resolve, ty: Option<&'o Type>) -> Option<&'o Type> {
    match ty {
        Some(ty) => {
            if is_empty_type(resolve, ty) {
                None
            } else {
                Some(ty)
            }
        }
        None => None,
    }
}

pub fn owns_anything(
    resolve: &Resolve,
    ty: &Type,
    is_local_resource: &dyn Fn(&Resolve, TypeId) -> bool,
) -> bool {
    let id = match ty {
        Type::Id(id) => *id,
        Type::String => return true,
        _ => return false,
    };
    match &resolve.types[id].kind {
        TypeDefKind::Type(t) => owns_anything(resolve, t, is_local_resource),
        TypeDefKind::Record(r) => r
            .fields
            .iter()
            .any(|t| owns_anything(resolve, &t.ty, is_local_resource)),
        TypeDefKind::Tuple(t) => t
            .types
            .iter()
            .any(|t| owns_anything(resolve, t, is_local_resource)),
        TypeDefKind::Flags(_) => false,
        TypeDefKind::Enum(_) => false,
        TypeDefKind::List(_) => true,
        TypeDefKind::Variant(v) => v
            .cases
            .iter()
            .any(|c| optional_owns_anything(resolve, c.ty.as_ref(), is_local_resource)),
        TypeDefKind::Union(v) => v
            .cases
            .iter()
            .any(|case| owns_anything(resolve, &case.ty, is_local_resource)),
        TypeDefKind::Option(t) => owns_anything(resolve, t, is_local_resource),
        TypeDefKind::Result(r) => {
            optional_owns_anything(resolve, r.ok.as_ref(), is_local_resource)
                || optional_owns_anything(resolve, r.err.as_ref(), is_local_resource)
        }
        TypeDefKind::Future(_) => todo!("owns_anything for future"),
        TypeDefKind::Stream(_) => todo!("owns_anything for stream"),
        TypeDefKind::Resource => false,
        TypeDefKind::Handle(Handle::Borrow(resource)) if is_local_resource(resolve, *resource) => {
            false
        }
        TypeDefKind::Handle(_) => true,
        TypeDefKind::Unknown => unreachable!(),
    }
}

pub fn optional_owns_anything(
    resolve: &Resolve,
    ty: Option<&Type>,
    is_local_resource: &dyn Fn(&Resolve, TypeId) -> bool,
) -> bool {
    match ty {
        Some(ty) => owns_anything(resolve, ty, is_local_resource),
        None => false,
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

fn dealias(resolve: &Resolve, mut id: TypeId) -> TypeId {
    loop {
        match &resolve.types[id].kind {
            TypeDefKind::Type(Type::Id(that_id)) => id = *that_id,
            _ => break id,
        }
    }
}
