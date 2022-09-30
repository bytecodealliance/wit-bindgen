use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    iter, mem,
    ops::Deref,
};
use wit_bindgen_core::{
    uwrite, uwriteln,
    wit_parser::{
        abi::{AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType},
        Case, Docs, Enum, Field, Flags, FlagsRepr, Function, FunctionKind, Int, Interface, Record,
        ResourceId, Result_, SizeAlign, Tuple, Type, TypeDefKind, TypeId, Union, Variant,
    },
    Direction, Files, Generator, Ns,
};

#[derive(Default)]
pub struct TeaVmJava {
    opts: Opts,
    src: String,
    stub: String,
    aliases: HashMap<TypeId, Type>,
    list_aliases: HashMap<TypeId, Type>,
    sizes: SizeAlign,
    anonymous_types: HashSet<TypeId>,
    return_area_size: usize,
    return_area_align: usize,
    needs_cleanup: bool,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub generate_stub: bool,
}

impl Opts {
    pub fn build(&self) -> TeaVmJava {
        TeaVmJava {
            opts: self.clone(),
            ..TeaVmJava::default()
        }
    }
}

impl TeaVmJava {
    fn type_name(&mut self, iface: &Interface, ty: &Type) -> String {
        self.type_name_with_qualifier(iface, ty, None)
    }

    fn type_name_with_qualifier(
        &mut self,
        iface: &Interface,
        ty: &Type,
        qualifier: Option<&str>,
    ) -> String {
        match ty {
            Type::Bool => "boolean".into(),
            Type::U8 | Type::S8 => "byte".into(),
            Type::U16 | Type::S16 => "short".into(),
            Type::U32 | Type::S32 | Type::Char => "int".into(),
            Type::U64 | Type::S64 => "long".into(),
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::Handle(_) => todo!("resources"),
            Type::String => "String".into(),
            Type::Id(id) => {
                // TODO: can we rely on all relevant type aliases being known by the time we reach here?  If not,
                // we'll need to refactor to ensure `type_name` isn't called until all aliases are known.
                if let Some(ty) = self.aliases.get(id).copied() {
                    self.type_name_with_qualifier(iface, &ty, qualifier)
                } else if let Some(ty) = self.list_aliases.get(id).copied() {
                    format!("{}[]", self.type_name_with_qualifier(iface, &ty, qualifier))
                } else {
                    let ty = &iface.types[*id];
                    match &ty.name {
                        Some(name) => {
                            format!("{}{}", qualifier.unwrap_or(""), name.to_upper_camel_case())
                        }
                        None => match &ty.kind {
                            TypeDefKind::Type(ty) => {
                                self.type_name_with_qualifier(iface, ty, qualifier)
                            }
                            TypeDefKind::List(ty) => {
                                format!("{}[]", self.type_name_with_qualifier(iface, ty, qualifier))
                            }
                            _ => {
                                self.anonymous_types.insert(*id);
                                format!(
                                    "{}{}",
                                    qualifier.unwrap_or(""),
                                    encoded_type_name(iface, &Type::Id(*id))
                                )
                            }
                        },
                    }
                }
            }
        }
    }

    fn print_docs(&mut self, docs: &Docs) {
        if let Some(docs) = &docs.contents {
            let lines = docs
                .trim()
                .lines()
                .map(|line| format!("* {line}"))
                .collect::<Vec<_>>()
                .join("\n");

            uwrite!(
                self.src,
                "
                /**
                 {lines}
                 */
                "
            )
        }
    }

    fn print_anonymous_type(&mut self, iface: &Interface, ty: TypeId) {
        match &iface.types[ty].kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_)
            | TypeDefKind::List(_) => unreachable!(),

            TypeDefKind::Tuple(tuple) => self.type_tuple(
                iface,
                ty,
                &encoded_type_name(iface, &Type::Id(ty)),
                tuple,
                &Docs::default(),
            ),

            TypeDefKind::Option(payload) => self.type_option(
                iface,
                ty,
                &encoded_type_name(iface, &Type::Id(ty)),
                payload,
                &Docs::default(),
            ),

            TypeDefKind::Result(result) => self.type_result(
                iface,
                ty,
                &encoded_type_name(iface, &Type::Id(ty)),
                result,
                &Docs::default(),
            ),

            TypeDefKind::Future(_) => todo!("print_anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("print_anonymous_type for stream"),
        }
    }

    fn non_empty_type<'a>(&self, iface: &'a Interface, ty: Option<&'a Type>) -> Option<&'a Type> {
        if let Some(ty) = ty {
            let id = match ty {
                Type::Id(id) => *id,
                _ => return Some(ty),
            };
            match &iface.types[id].kind {
                TypeDefKind::Type(t) => self.non_empty_type(iface, Some(t)).map(|_| ty),
                TypeDefKind::Record(r) => (!r.fields.is_empty()).then_some(ty),
                TypeDefKind::Tuple(t) => (!t.types.is_empty()).then_some(ty),
                _ => Some(ty),
            }
        } else {
            None
        }
    }

    fn sig_string(
        &mut self,
        iface: &Interface,
        func: &Function,
        qualifier: Option<&str>,
    ) -> String {
        let name = func.name.to_lower_camel_case();

        let result_type = match func.results.len() {
            0 => "void".into(),
            1 => self.type_name_with_qualifier(
                iface,
                func.results.iter_types().next().unwrap(),
                qualifier,
            ),
            _ => todo!("multiple return values"),
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name_with_qualifier(iface, ty, qualifier);
                let name = name.to_lower_camel_case();
                format!("{ty} {name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("public static {result_type} {name}({params})")
    }
}

impl Generator for TeaVmJava {
    fn preprocess_one(&mut self, iface: &Interface, _dir: Direction) {
        let package = format!("wit_{}", iface.name.to_snake_case());
        let name = iface.name.to_upper_camel_case();

        uwrite!(
            self.src,
            "package {package};

             import java.nio.charset.StandardCharsets;
             import java.util.ArrayList;

             import org.teavm.interop.Memory;
             import org.teavm.interop.Address;
             import org.teavm.interop.Import;
             import org.teavm.interop.Export;

             public final class {name} {{
                private {name}() {{}}
            "
        );

        if self.opts.generate_stub {
            uwrite!(
                self.stub,
                "package {package};

                 public class {name}Impl {{
                "
            );
        }

        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let parameters = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{} {}",
                    self.type_name(iface, &field.ty),
                    field.name.to_lower_camel_case()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let assignments = record
            .fields
            .iter()
            .map(|field| {
                let name = field.name.to_lower_camel_case();
                format!("this.{name} = {name};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let fields = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "public final {} {};",
                    self.type_name(iface, &field.ty),
                    field.name.to_lower_camel_case()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static final class {name} {{
                {fields}

                public {name}({parameters}) {{
                    {assignments}
                }}
            }}
            "
        );
    }

    fn type_flags(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let ty = match flags.repr() {
            FlagsRepr::U8 => "byte",
            FlagsRepr::U16 => "short",
            FlagsRepr::U32(1) => "int",
            FlagsRepr::U32(2) => "long",
            repr => todo!("flags {repr:?}"),
        };

        let flags = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let name = flag.name.to_shouty_snake_case();
                let suffix = if matches!(flags.repr(), FlagsRepr::U32(2)) {
                    "L"
                } else {
                    ""
                };
                format!("public static final {ty} {name} = ({ty}) (1{suffix} << {i});")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static final class {name} {{
                public final {ty} value;

                public {name}({ty} value) {{
                    this.value = value;
                }}

                {flags}
            }}
            "
        );
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        self.type_record(
            iface,
            id,
            name,
            &Record {
                fields: tuple
                    .types
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| Field {
                        docs: Docs::default(),
                        name: format!("f{i}"),
                        ty: *ty,
                    })
                    .collect(),
            },
            docs,
        )
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();
        let tag_type = int_type(variant.tag());

        let constructors = variant
            .cases
            .iter()
            .map(|case| {
                let case_name = case.name.to_lower_camel_case();
                let tag = case.name.to_shouty_snake_case();
                let (parameter, argument) =
                    if let Some(ty) = self.non_empty_type(iface, case.ty.as_ref()) {
                        (
                            format!("{} {case_name}", self.type_name(iface, ty)),
                            case_name.deref(),
                        )
                    } else {
                        (String::new(), "null")
                    };

                format!(
                    "public static {name} {case_name}({parameter}) {{
                         return new {name}({tag}, {argument});
                     }}
                    "
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let accessors = variant
            .cases
            .iter()
            .filter_map(|case| {
                self.non_empty_type(iface, case.ty.as_ref()).map(|ty| {
                    let case_name = case.name.to_upper_camel_case();
                    let tag = case.name.to_shouty_snake_case();
                    let ty = self.type_name(iface, ty);
                    format!(
                        r#"public {ty} get{case_name}() {{
                               if (this.tag == {tag}) {{
                                   return ({ty}) this.value;
                               }} else {{
                                   throw new RuntimeException("expected {tag}, got " + this.tag);
                               }}
                           }}
                        "#
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n");

        let tags = variant
            .cases
            .iter()
            .enumerate()
            .map(|(i, case)| {
                let tag = case.name.to_shouty_snake_case();
                format!("public static final {tag_type} {tag} = {i};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static final class {name} {{
                public final {tag_type} tag;
                private final Object value;

                private {name}({tag_type} tag, Object value) {{
                    this.tag = tag;
                    this.value = value;
                }}

                {constructors}
                {accessors}
                {tags}
            }}
            "
        );
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        self.type_variant(
            iface,
            id,
            name,
            &Variant {
                cases: vec![
                    Case {
                        docs: Docs::default(),
                        name: "none".into(),
                        ty: None,
                    },
                    Case {
                        docs: Docs::default(),
                        name: "some".into(),
                        ty: Some(*payload),
                    },
                ],
            },
            docs,
        )
    }

    fn type_result(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        result: &Result_,
        docs: &Docs,
    ) {
        self.type_variant(
            iface,
            id,
            name,
            &Variant {
                cases: vec![
                    Case {
                        docs: Docs::default(),
                        name: "ok".into(),
                        ty: result.ok,
                    },
                    Case {
                        docs: Docs::default(),
                        name: "err".into(),
                        ty: result.err,
                    },
                ],
            },
            docs,
        )
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        self.type_variant(
            iface,
            id,
            name,
            &Variant {
                cases: union
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| Case {
                        docs: case.docs.clone(),
                        name: format!("f{i}"),
                        ty: Some(case.ty),
                    })
                    .collect(),
            },
            docs,
        )
    }

    fn type_enum(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        enum_: &Enum,
        docs: &Docs,
    ) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let cases = enum_
            .cases
            .iter()
            .map(|case| case.name.to_shouty_snake_case())
            .collect::<Vec<_>>()
            .join(", ");

        uwrite!(
            self.src,
            "
            public static enum {name} {{
                {cases}
            }}
            "
        );
    }

    fn type_resource(&mut self, _iface: &Interface, _ty: ResourceId) {
        todo!("resources")
    }

    fn type_alias(&mut self, _iface: &Interface, id: TypeId, _name: &str, ty: &Type, _docs: &Docs) {
        self.aliases.insert(id, *ty);
    }

    fn type_list(&mut self, _iface: &Interface, id: TypeId, _name: &str, ty: &Type, _docs: &Docs) {
        self.list_aliases.insert(id, *ty);
    }

    fn type_builtin(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        _name: &str,
        _ty: &Type,
        _docs: &Docs,
    ) {
        unimplemented!();
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        if func.kind != FunctionKind::Freestanding {
            todo!("resources");
        }

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            func.params.iter().map(|(name, _)| name.clone()).collect(),
        );

        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
        );

        let src = bindgen.src;

        let cleanup_list = if bindgen.needs_cleanup_list {
            self.needs_cleanup = true;

            "ArrayList<Cleanup> cleanupList = new ArrayList<>();\n"
        } else {
            ""
        };

        let module = &iface.name;
        let name = &func.name;

        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);

        let result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => todo!("multi-value return"),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let sig = self.sig_string(iface, func, None);

        uwrite!(
            self.src,
            r#"@Import(name = "{name}", module = "{module}")
               private static native {result_type} wasmImport{camel_name}({params});

               {sig} {{
                   {cleanup_list} {src}
               }}
            "#
        );
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        let sig = iface.wasm_signature(AbiVariant::GuestExport, func);

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        );

        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );

        assert!(!bindgen.needs_cleanup_list);

        let src = bindgen.src;
        let name = &func.name;

        let result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => todo!("multi-value return"),
        };

        let camel_name = func.name.to_upper_camel_case();

        let params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        uwrite!(
            self.src,
            r#"
            @Export(name = "{name}")
            private static {result_type} wasmExport{camel_name}({params}) {{
                {src}
            }}
            "#
        );

        if iface.guest_export_needs_post_return(func) {
            let name = &func.name;

            let params = sig
                .results
                .iter()
                .enumerate()
                .map(|(i, param)| {
                    let ty = wasm_type(*param);
                    format!("{ty} p{i}")
                })
                .collect::<Vec<_>>()
                .join(", ");

            let mut bindgen = FunctionBindgen::new(
                self,
                "INVALID",
                (0..sig.results.len()).map(|i| format!("p{i}")).collect(),
            );

            iface.post_return(func, &mut bindgen);

            let src = bindgen.src;

            uwrite!(
                self.src,
                r#"
                @Export(name = "cabi_post_{name}")
                private static void wasmExport{camel_name}PostReturn({params}) {{
                    {src}
                }}
                "#
            );
        }

        if self.opts.generate_stub {
            let class = iface.name.to_upper_camel_case();
            let sig = self.sig_string(iface, func, Some(&format!("{class}.")));

            uwrite!(
                self.stub,
                r#"
                {sig} {{
                    throw new RuntimeException("todo");
                }}
                "#
            );
        }
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        let mut printed = HashSet::new();
        while !self.anonymous_types.is_empty() {
            for ty in mem::take(&mut self.anonymous_types) {
                let name = encoded_type_name(iface, &Type::Id(ty));
                if !printed.contains(&name) {
                    printed.insert(name);
                    self.print_anonymous_type(iface, ty);
                }
            }
        }

        if self.needs_cleanup {
            uwrite!(
                self.src,
                "
                private static final class Cleanup {{
                    public final int address;
                    public final int size;
                    public final int align;

                    public Cleanup(int address, int size, int align) {{
                        this.address = address;
                        this.size = size;
                        this.align = align;
                    }}
                }}
                "
            );
        }

        if self.return_area_align > 0 {
            let size = self.return_area_size;
            let align = self.return_area_align;

            uwriteln!(
                self.src,
                "private static final int RETURN_AREA = Memory.malloc({size}, {align}).toInt();",
            );
        }

        self.src.push_str("}\n");

        files.push(
            &format!("{}.java", iface.name.to_upper_camel_case()),
            indent(&self.src).as_bytes(),
        );

        if self.opts.generate_stub {
            self.stub.push_str("}\n");

            files.push(
                &format!("{}Impl.java", iface.name.to_upper_camel_case()),
                indent(&self.stub).as_bytes(),
            );
        }
    }
}

struct Block {
    body: String,
    results: Vec<String>,
    element: String,
    base: String,
}

struct Cleanup {
    address: String,
    size: usize,
    align: usize,
}

struct BlockStorage {
    body: String,
    element: String,
    base: String,
    cleanup: Vec<Cleanup>,
}

struct FunctionBindgen<'a> {
    gen: &'a mut TeaVmJava,
    func_name: &'a str,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    payloads: Vec<String>,
    cleanup: Vec<Cleanup>,
    needs_cleanup_list: bool,
}

impl<'a> FunctionBindgen<'a> {
    fn new(
        gen: &'a mut TeaVmJava,
        func_name: &'a str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a> {
        Self {
            gen,
            func_name,
            params,
            src: String::new(),
            locals: Ns::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            payloads: Vec::new(),
            cleanup: Vec::new(),
            needs_cleanup_list: false,
        }
    }

    fn lower_variant(
        &mut self,
        types: &[Option<Type>],
        lowered_types: &[WasmType],
        iface: &Interface,
        op: &str,
        results: &mut Vec<String>,
    ) {
        let blocks = self
            .blocks
            .drain(self.blocks.len() - types.len()..)
            .collect::<Vec<_>>();

        let payloads = self
            .payloads
            .drain(self.payloads.len() - types.len()..)
            .collect::<Vec<_>>();

        let lowered = lowered_types
            .iter()
            .map(|_| self.locals.tmp("lowered"))
            .collect::<Vec<_>>();

        results.extend(lowered.iter().cloned());

        let declarations = lowered
            .iter()
            .zip(lowered_types)
            .map(|(lowered, ty)| format!("{} {lowered};", wasm_type(*ty)))
            .collect::<Vec<_>>()
            .join("\n");

        let cases = types
            .iter()
            .zip(blocks)
            .zip(payloads)
            .enumerate()
            .map(|(i, ((ty, Block { body, results, .. }), payload))| {
                let payload = if let Some(ty) = self.gen.non_empty_type(iface, ty.as_ref()) {
                    let ty = self.gen.type_name(iface, ty);

                    format!("{ty} {payload} = ({ty}) ({op}).value;")
                } else {
                    String::new()
                };

                let assignments = lowered
                    .iter()
                    .zip(&results)
                    .map(|(lowered, result)| format!("{lowered} = {result};\n"))
                    .collect::<Vec<_>>()
                    .concat();

                format!(
                    "case {i}: {{
                         {payload}
                         {body}
                         {assignments}
                         break;
                     }}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            r#"
            {declarations}

            switch (({op}).tag) {{
                {cases}

                default: throw new AssertionError("invalid discriminant: " + ({op}).tag);
            }}
            "#
        );
    }

    fn lift_variant(
        &mut self,
        ty: &Type,
        cases: &[(&str, Option<Type>)],
        iface: &Interface,
        op: &str,
        results: &mut Vec<String>,
    ) {
        let blocks = self
            .blocks
            .drain(self.blocks.len() - cases.len()..)
            .collect::<Vec<_>>();

        let ty = self.gen.type_name(iface, ty);
        let lifted = self.locals.tmp("lifted");

        let cases = cases
            .iter()
            .zip(blocks)
            .enumerate()
            .map(|(i, ((case_name, case_ty), Block { body, results, .. }))| {
                let payload = if self.gen.non_empty_type(iface, case_ty.as_ref()).is_some() {
                    results.into_iter().next().unwrap()
                } else {
                    String::new()
                };

                let method = case_name.to_lower_camel_case();

                format!(
                    "case {i}: {{
                         {body}
                         {lifted} = {ty}.{method}({payload});
                         break;
                     }}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            r#"
            {ty} {lifted};

            switch ({op}) {{
                {cases}

                default: throw new AssertionError("invalid discriminant: " + ({op}));
            }}
            "#
        );

        results.push(lifted);
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

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
            Instruction::ConstZero { tys } => results.extend(tys.iter().map(|ty| {
                match ty {
                    WasmType::I32 => "0",
                    WasmType::I64 => "0L",
                    WasmType::F32 => "0.0F",
                    WasmType::F64 => "0.0D",
                }
                .to_owned()
            })),

            // TODO: checked
            Instruction::U8FromI32 => results.push(format!("(byte) ({})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("(byte) ({})", operands[0])),
            Instruction::U16FromI32 => results.push(format!("(short) ({})", operands[0])),
            Instruction::S16FromI32 => results.push(format!("(short) ({})", operands[0])),

            Instruction::I32FromU8 => results.push(format!("((int) ({})) & 0xFF", operands[0])),
            Instruction::I32FromU16 => results.push(format!("((int) ({})) & 0xFFFF", operands[0])),

            Instruction::I32FromS8 | Instruction::I32FromS16 => {
                results.push(format!("(int) ({})", operands[0]))
            }

            Instruction::CharFromI32
            | Instruction::I32FromChar
            | Instruction::U32FromI32
            | Instruction::S32FromI32
            | Instruction::S64FromI64
            | Instruction::U64FromI64
            | Instruction::I32FromU32
            | Instruction::I32FromS32
            | Instruction::I64FromS64
            | Instruction::I64FromU64
            | Instruction::F32FromFloat32
            | Instruction::F64FromFloat64
            | Instruction::Float32FromF32
            | Instruction::Float64FromF64 => results.push(operands[0].clone()),

            Instruction::Bitcasts { casts } => {
                results.extend(casts.iter().zip(operands).map(|(cast, op)| match cast {
                    Bitcast::I32ToF32 => format!("Float.intBitsToFloat({op})"),
                    Bitcast::I64ToF32 => format!("Float.intBitsToFloat((int) ({op}))"),
                    Bitcast::F32ToI32 => format!("Float.floatToIntBits({op})"),
                    Bitcast::F32ToI64 => format!("(long) Float.floatToIntBits({op})"),
                    Bitcast::I64ToF64 => format!("Double.longBitsToDouble({op})"),
                    Bitcast::F64ToI64 => format!("Double.doubleToLongBits({op})"),
                    Bitcast::I32ToI64 => format!("(long) ({op})"),
                    Bitcast::I64ToI32 => format!("(int) ({op})"),
                    Bitcast::None => op.to_owned(),
                }))
            }

            Instruction::I32FromBool => {
                results.push(format!("({} ? 1 : 0)", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            // handles in exports
            Instruction::I32FromOwnedHandle { .. } => todo!("resources"),
            Instruction::HandleBorrowedFromI32 { .. } => todo!("resources"),

            // handles in imports
            Instruction::I32FromBorrowedHandle { .. } => todo!("resources"),
            Instruction::HandleOwnedFromI32 { .. } => todo!("resources"),

            // TODO: checked
            Instruction::FlagsLower { flags, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(format!("({}).value", operands[0]));
                }
                Int::U64 => {
                    let op = &operands[0];
                    results.push(format!("(int) (({op}).value & 0xffffffffL)"));
                    results.push(format!("(int) ((({op}).value >>> 32) & 0xffffffffL)"));
                }
            },

            Instruction::FlagsLift { name, flags, .. } => match flags_repr(flags) {
                Int::U8 | Int::U16 | Int::U32 => {
                    results.push(format!(
                        "new {}(({}) {})",
                        name.to_upper_camel_case(),
                        int_type(flags_repr(flags)),
                        operands[0]
                    ));
                }
                Int::U64 => {
                    results.push(format!(
                        "new {}(((long) ({})) | (((long) ({})) << 32))",
                        name.to_upper_camel_case(),
                        operands[0],
                        operands[1]
                    ));
                }
            },

            Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for field in record.fields.iter() {
                    results.push(format!("({op}).{}", field.name.to_lower_camel_case()));
                }
            }
            Instruction::RecordLift { ty, .. } | Instruction::TupleLift { ty, .. } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!(
                    "new {}({ops})",
                    self.gen.type_name(iface, &Type::Id(*ty))
                ));
            }

            Instruction::TupleLower { tuple, .. } => {
                let op = &operands[0];
                for i in 0..tuple.types.len() {
                    results.push(format!("({op}).f{i}"));
                }
            }

            Instruction::VariantPayloadName => {
                let payload = self.locals.tmp("payload");
                results.push(payload.clone());
                self.payloads.push(payload);
            }

            Instruction::VariantLower {
                variant,
                results: lowered_types,
                ..
            } => self.lower_variant(
                &variant.cases.iter().map(|case| case.ty).collect::<Vec<_>>(),
                lowered_types,
                iface,
                &operands[0],
                results,
            ),

            Instruction::VariantLift { variant, ty, .. } => self.lift_variant(
                &Type::Id(*ty),
                &variant
                    .cases
                    .iter()
                    .map(|case| (case.name.deref(), case.ty))
                    .collect::<Vec<_>>(),
                iface,
                &operands[0],
                results,
            ),

            Instruction::UnionLower {
                union,
                results: lowered_types,
                ..
            } => self.lower_variant(
                &union
                    .cases
                    .iter()
                    .map(|case| Some(case.ty))
                    .collect::<Vec<_>>(),
                lowered_types,
                iface,
                &operands[0],
                results,
            ),

            Instruction::UnionLift { union, ty, .. } => {
                let cases = union
                    .cases
                    .iter()
                    .enumerate()
                    .map(|(i, case)| (format!("f{i}"), case.ty))
                    .collect::<Vec<_>>();

                self.lift_variant(
                    &Type::Id(*ty),
                    &cases
                        .iter()
                        .map(|(name, ty)| (name.deref(), Some(*ty)))
                        .collect::<Vec<_>>(),
                    iface,
                    &operands[0],
                    results,
                )
            }

            Instruction::OptionLower {
                results: lowered_types,
                payload,
                ..
            } => self.lower_variant(
                &[None, Some(**payload)],
                lowered_types,
                iface,
                &operands[0],
                results,
            ),

            Instruction::OptionLift { payload, ty } => self.lift_variant(
                &Type::Id(*ty),
                &[("none", None), ("some", Some(**payload))],
                iface,
                &operands[0],
                results,
            ),

            Instruction::ResultLower {
                results: lowered_types,
                result,
                ..
            } => self.lower_variant(
                &[result.ok, result.err],
                lowered_types,
                iface,
                &operands[0],
                results,
            ),

            Instruction::ResultLift { result, ty } => self.lift_variant(
                &Type::Id(*ty),
                &[("ok", result.ok), ("err", result.err)],
                iface,
                &operands[0],
                results,
            ),

            Instruction::EnumLower { .. } => results.push(format!("{}.ordinal()", operands[0])),

            Instruction::EnumLift { name, .. } => results.push(format!(
                "{}.values()[{}]",
                name.to_upper_camel_case(),
                operands[0]
            )),

            Instruction::ListCanonLower { element, realloc } => {
                let op = &operands[0];
                let (size, ty) = list_element_info(element);

                // Note that we can only reliably use `Address.ofData` for elements with alignment <= 4 because as
                // of this writing TeaVM does not guarantee 64 bit items are aligned on 8 byte boundaries.
                if realloc.is_none() && size <= 4 {
                    results.push(format!("Address.ofData({op}).toInt()"));
                } else {
                    let address = self.locals.tmp("address");
                    let ty = ty.to_upper_camel_case();

                    uwrite!(
                        self.src,
                        "
                        Address {address} = Memory.malloc({size} * ({op}).length, {size});
                        Memory.put{ty}s({address}, {op}, 0, ({op}).length);
                        "
                    );

                    if realloc.is_none() {
                        self.cleanup.push(Cleanup {
                            address: format!("{address}.toInt()"),
                            size,
                            align: size,
                        });
                    }

                    results.push(format!("{address}.toInt()"));
                }
                results.push(format!("({op}).length"));
            }

            Instruction::ListCanonLift { element, .. } => {
                let (_, ty) = list_element_info(element);
                let ty_upper = ty.to_upper_camel_case();
                let array = self.locals.tmp("array");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    {ty}[] {array} = new {ty}[{length}];
                    Memory.get{ty_upper}s(Address.fromInt({address}), {array}, 0, ({array}).length);
                    "
                );

                results.push(array);
            }

            Instruction::StringLower { realloc } => {
                let op = &operands[0];
                let bytes = self.locals.tmp("bytes");
                uwriteln!(
                    self.src,
                    "byte[] {bytes} = ({op}).getBytes(StandardCharsets.UTF_8);"
                );

                if realloc.is_none() {
                    results.push(format!("Address.ofData({bytes}).toInt()"));
                } else {
                    let address = self.locals.tmp("address");

                    uwrite!(
                        self.src,
                        "
                        Address {address} = Memory.malloc({bytes}.length, 1);
                        Memory.putBytes({address}, {bytes}, 0, {bytes}.length);
                        "
                    );

                    results.push(format!("{address}.toInt()"));
                }
                results.push(format!("{bytes}.length"));
            }

            Instruction::StringLift { .. } => {
                let bytes = self.locals.tmp("bytes");
                let address = &operands[0];
                let length = &operands[1];

                uwrite!(
                    self.src,
                    "
                    byte[] {bytes} = new byte[{length}];
                    Memory.getBytes(Address.fromInt({address}), {bytes}, 0, {length});
                    "
                );

                results.push(format!("new String({bytes}, StandardCharsets.UTF_8)"));
            }

            Instruction::ListLower { element, realloc } => {
                let Block {
                    body,
                    results: block_results,
                    element: block_element,
                    base,
                } = self.blocks.pop().unwrap();
                assert!(block_results.is_empty());

                let op = &operands[0];
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let address = self.locals.tmp("address");
                let ty = self.gen.type_name(iface, element);
                let index = self.locals.tmp("index");

                uwrite!(
                    self.src,
                    "
                    int {address} = Memory.malloc(({op}).length * {size}, {align}).toInt();
                    for (int {index} = 0; {index} < ({op}).length; ++{index}) {{
                        {ty} {block_element} = ({op})[{index}];
                        int {base} = {address} + ({index} * {size});
                        {body}
                    }}
                    "
                );

                if realloc.is_none() {
                    self.cleanup.push(Cleanup {
                        address: address.clone(),
                        size,
                        align,
                    });
                }

                results.push(address);
                results.push(format!("({op}).length"));
            }

            Instruction::ListLift { element, .. } => {
                let Block {
                    body,
                    results: block_results,
                    base,
                    ..
                } = self.blocks.pop().unwrap();
                let address = &operands[0];
                let length = &operands[1];
                let array = self.locals.tmp("array");
                let ty = self.gen.type_name(iface, element);
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let index = self.locals.tmp("index");

                let result = match &block_results[..] {
                    [result] => result,
                    _ => todo!("result count == {}", results.len()),
                };

                let new = if let Some(position) = ty.find('[') {
                    let (name, brackets) = ty.split_at(position);
                    format!("new {name}[{length}]{brackets}")
                } else {
                    format!("new {ty}[{length}]")
                };

                uwrite!(
                    self.src,
                    "
                    {ty}[] {array} = {new};
                    for (int {index} = 0; {index} < {array}.length; ++{index}) {{
                        int {base} = ({address}) + ({index} * {size});
                        {body}
                        {array}[{index}] = {result};
                    }}
                    Memory.free(Address.fromInt({address}), ({length}) * {size}, {align});
                    "
                );

                results.push(array);
            }

            Instruction::IterElem { .. } => {
                results.push(self.block_storage.last().unwrap().element.clone())
            }

            Instruction::IterBasePointer => {
                results.push(self.block_storage.last().unwrap().base.clone())
            }

            Instruction::CallWasm { sig, .. } => {
                let assignment = match &sig.results[..] {
                    [result] => {
                        let ty = wasm_type(*result);
                        let result = self.locals.tmp("result");
                        let assignment = format!("{ty} {result} = ");
                        results.push(result);
                        assignment
                    }

                    [] => String::new(),

                    _ => unreachable!(),
                };

                let func_name = self.func_name.to_upper_camel_case();

                let operands = operands.join(", ");

                uwriteln!(self.src, "{assignment} wasmImport{func_name}({operands});");
            }

            Instruction::CallInterface { module, func } => {
                let assignment = match func.results.len() {
                    0 => String::new(),
                    1 => {
                        let ty = self
                            .gen
                            .type_name(iface, func.results.iter_types().next().unwrap());
                        let result = self.locals.tmp("result");
                        let assignment = format!("{ty} {result} = ");
                        results.push(result);
                        assignment
                    }
                    _ => todo!("multiple return values"),
                };

                let module = module.to_upper_camel_case();
                let name = func.name.to_lower_camel_case();

                let args = operands.join(", ");

                uwrite!(
                    self.src,
                    "
                    {assignment}{module}Impl.{name}({args});
                    "
                );
            }

            Instruction::Return { amt, .. } => {
                for Cleanup {
                    address,
                    size,
                    align,
                } in &self.cleanup
                {
                    uwriteln!(
                        self.src,
                        "Memory.free(Address.fromInt({address}), {size}, {align});"
                    );
                }

                if self.needs_cleanup_list {
                    uwrite!(
                        self.src,
                        "
                        for (Cleanup cleanup : cleanupList) {{
                            Memory.free(Address.fromInt(cleanup.address), cleanup.size, cleanup.align);
                        }}
                        "
                        );
                }

                match amt {
                    0 => (),
                    1 => uwriteln!(self.src, "return {};", operands[0]),
                    _ => unreachable!(),
                }
            }

            Instruction::I32Load { offset } => results.push(format!(
                "Address.fromInt(({}) + {offset}).getInt()",
                operands[0]
            )),

            Instruction::I32Load8U { offset } => results.push(format!(
                "(((int) Address.fromInt(({}) + {offset}).getByte()) & 0xFF)",
                operands[0]
            )),

            Instruction::I32Load8S { offset } => results.push(format!(
                "((int) Address.fromInt(({}) + {offset}).getByte())",
                operands[0]
            )),

            Instruction::I32Load16U { offset } => results.push(format!(
                "(((int) Address.fromInt(({}) + {offset}).getShort()) & 0xFFFF)",
                operands[0]
            )),

            Instruction::I32Load16S { offset } => results.push(format!(
                "((int) Address.fromInt(({}) + {offset}).getShort())",
                operands[0]
            )),

            Instruction::I64Load { offset } => results.push(format!(
                "Address.fromInt(({}) + {offset}).getLong()",
                operands[0]
            )),

            Instruction::F32Load { offset } => results.push(format!(
                "Address.fromInt(({}) + {offset}).getFloat()",
                operands[0]
            )),

            Instruction::F64Load { offset } => results.push(format!(
                "Address.fromInt(({}) + {offset}).getDouble()",
                operands[0]
            )),

            Instruction::I32Store { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putInt({});",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store8 { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putByte((byte) ({}));",
                operands[1],
                operands[0]
            ),

            Instruction::I32Store16 { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putShort((short) ({}));",
                operands[1],
                operands[0]
            ),

            Instruction::I64Store { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putLong({});",
                operands[1],
                operands[0]
            ),

            Instruction::F32Store { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putFloat({});",
                operands[1],
                operands[0]
            ),

            Instruction::F64Store { offset } => uwriteln!(
                self.src,
                "Address.fromInt(({}) + {offset}).putDouble({});",
                operands[1],
                operands[0]
            ),

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { size, align } => {
                uwriteln!(
                    self.src,
                    "Memory.free(Address.fromInt({}), {size}, {align});",
                    operands[0]
                )
            }

            Instruction::GuestDeallocateString => uwriteln!(
                self.src,
                "Memory.free(Address.fromInt({}), {}, 1);",
                operands[0],
                operands[1]
            ),

            Instruction::GuestDeallocateVariant { blocks } => {
                let cases = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .enumerate()
                    .map(|(i, Block { body, results, .. })| {
                        assert!(results.is_empty());

                        format!(
                            "case {i}: {{
                                 {body}
                                 break;
                             }}"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let op = &operands[0];

                uwrite!(
                    self.src,
                    "
                    switch ({op}) {{
                        {cases}
                    }}
                    "
                );
            }

            Instruction::GuestDeallocateList { element } => {
                let Block {
                    body,
                    results,
                    base,
                    ..
                } = self.blocks.pop().unwrap();
                assert!(results.is_empty());

                let address = &operands[0];
                let length = &operands[1];

                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                if !body.trim().is_empty() {
                    let index = self.locals.tmp("index");

                    uwrite!(
                        self.src,
                        "
                        for (int {index} = 0; {index} < ({length}); ++{index}) {{
                            int {base} = ({address}) + ({index} * {size});
                            {body}
                        }}
                        "
                    );
                }

                uwriteln!(
                    self.src,
                    "Memory.free(Address.fromInt({address}), ({length}) * {size}, {align});"
                );
            }
        }
    }

    fn return_pointer(&mut self, _iface: &Interface, size: usize, align: usize) -> String {
        self.gen.return_area_size = self.gen.return_area_size.max(size);
        self.gen.return_area_align = self.gen.return_area_align.max(align);
        "RETURN_AREA".into()
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            element: self.locals.tmp("element"),
            base: self.locals.tmp("base"),
            cleanup: mem::take(&mut self.cleanup),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage {
            body,
            element,
            base,
            cleanup,
        } = self.block_storage.pop().unwrap();

        if !self.cleanup.is_empty() {
            self.needs_cleanup_list = true;

            for Cleanup {
                address,
                size,
                align,
            } in &self.cleanup
            {
                uwriteln!(
                    self.src,
                    "cleanupList.add(new Cleanup({address}, {size}, {align}));"
                );
            }
        }

        self.cleanup = cleanup;

        self.blocks.push(Block {
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
            element,
            base,
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn is_list_canonical(&self, _iface: &Interface, element: &Type) -> bool {
        matches!(
            element,
            Type::U8
                | Type::S8
                | Type::U16
                | Type::S16
                | Type::U32
                | Type::S32
                | Type::U64
                | Type::S64
                | Type::Float32
                | Type::Float64
        )
    }
}

fn int_type(int: Int) -> &'static str {
    match int {
        Int::U8 => "byte",
        Int::U16 => "short",
        Int::U32 => "int",
        Int::U64 => "long",
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int",
        WasmType::I64 => "long",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
    }
}

fn flags_repr(flags: &Flags) -> Int {
    match flags.repr() {
        FlagsRepr::U8 => Int::U8,
        FlagsRepr::U16 => Int::U16,
        FlagsRepr::U32(1) => Int::U32,
        FlagsRepr::U32(2) => Int::U64,
        repr => panic!("unimplemented flags {repr:?}"),
    }
}

fn list_element_info(ty: &Type) -> (usize, &'static str) {
    match ty {
        Type::U8 | Type::S8 => (1, "byte"),
        Type::U16 | Type::S16 => (2, "short"),
        Type::U32 | Type::S32 => (4, "int"),
        Type::U64 | Type::S64 => (8, "long"),
        Type::Float32 => (4, "float"),
        Type::Float64 => (8, "double"),
        _ => unreachable!(),
    }
}

fn encoded_type_name(iface: &Interface, ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".into(),
        Type::Char => "Char32".into(),
        Type::U8 => "Unsigned8".into(),
        Type::S8 => "Signed8".into(),
        Type::U16 => "Unsigned16".into(),
        Type::S16 => "Signed16".into(),
        Type::U32 => "Unsigned32".into(),
        Type::S32 => "Signed32".into(),
        Type::U64 => "Unsigned64".into(),
        Type::S64 => "Signed64".into(),
        Type::Float32 => "Float32".into(),
        Type::Float64 => "Float64".into(),
        Type::Handle(id) => iface.resources[*id].name.to_upper_camel_case(),
        Type::String => "String".into(),
        Type::Id(id) => {
            let ty = &iface.types[*id];
            if let Some(name) = &ty.name {
                name.to_upper_camel_case()
            } else {
                match &ty.kind {
                    TypeDefKind::Type(t) => encoded_type_name(iface, t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(tuple) => {
                        let length = tuple.types.len();
                        let types = tuple
                            .types
                            .iter()
                            .map(|ty| encoded_type_name(iface, ty))
                            .collect::<Vec<_>>()
                            .concat();

                        format!("Tuple{length}{types}")
                    }
                    TypeDefKind::Option(ty) => {
                        format!("Option{}", encoded_type_name(iface, ty))
                    }
                    TypeDefKind::Result(result) => format!(
                        "Result{}{}",
                        optional_encoded_type_name(iface, result.ok.as_ref()),
                        optional_encoded_type_name(iface, result.err.as_ref())
                    ),
                    TypeDefKind::List(ty) => {
                        format!("List{}", encoded_type_name(iface, ty))
                    }
                    TypeDefKind::Future(ty) => {
                        format!("Future{}", optional_encoded_type_name(iface, ty.as_ref()))
                    }
                    TypeDefKind::Stream(stream) => format!(
                        "Stream{}{}",
                        optional_encoded_type_name(iface, stream.element.as_ref()),
                        optional_encoded_type_name(iface, stream.end.as_ref())
                    ),
                }
            }
        }
    }
}

fn optional_encoded_type_name(iface: &Interface, ty: Option<&Type>) -> String {
    if let Some(ty) = ty {
        encoded_type_name(iface, ty)
    } else {
        "Void".into()
    }
}

fn indent(code: &str) -> String {
    let mut indented = String::with_capacity(code.len());
    let mut indent = 0;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            indent -= 1;
        }
        indented.extend(iter::repeat(' ').take(indent * 4));
        indented.push_str(trimmed);
        if trimmed.ends_with('{') {
            indent += 1;
        }
        indented.push('\n');
    }
    indented
}
