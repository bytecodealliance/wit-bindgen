mod component_type_object;

use anyhow::Result;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    iter, mem,
    ops::Deref,
};
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Instruction, LiftLower, WasmType},
    Direction,
};
use wit_bindgen_core::{
    uwrite, uwriteln,
    wit_parser::{
        Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Int, InterfaceId, Record, Resolve,
        Result_, SizeAlign, Tuple, Type, TypeDef, TypeDefKind, TypeId, TypeOwner, Variant, WorldId,
        WorldKey,
    },
    Files, InterfaceGenerator as _, Ns, WorldGenerator,
};
use wit_component::StringEncoding;
mod csproj;
pub use csproj::CSProject;

//cargo run c-sharp --out-dir testing-csharp tests/codegen/floats.wit

//TODO remove unused
const CSHARP_IMPORTS: &str = "\
using System;
using System.Runtime.CompilerServices;
using System.Collections;
using System.Runtime.InteropServices;
using System.Text;

";

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long, default_value_t = StringEncoding::default()))]
    pub string_encoding: StringEncoding,
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(CSharp {
            opts: self.clone(),
            ..CSharp::default()
        })
    }
}

struct InterfaceFragment {
    csharp_src: String,
    csharp_interop_src: String,
    stub: String,
}

pub struct InterfaceTypeAndFragments {
    is_export: bool,
    interface_fragments: Vec<InterfaceFragment>,
}

impl InterfaceTypeAndFragments {
    pub fn new(is_export: bool) -> Self {
        InterfaceTypeAndFragments {
            is_export: is_export,
            interface_fragments: Vec::<InterfaceFragment>::new(),
        }
    }
}

#[derive(Default)]
pub struct CSharp {
    opts: Opts,
    name: String,
    return_area_size: usize,
    return_area_align: usize,
    tuple_counts: HashSet<usize>,
    needs_result: bool,
    needs_interop_string: bool,
    interface_fragments: HashMap<String, InterfaceTypeAndFragments>,
    world_fragments: Vec<InterfaceFragment>,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, String>,
}

impl CSharp {
    fn qualifier(&self) -> String {
        let world = self.name.to_upper_camel_case();
        format!("{world}World.")
    }

    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a str,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            csharp_interop_src: String::new(),
            stub: String::new(),
            gen: self,
            resolve,
            name,
            in_import,
        }
    }

    fn get_class_name_from_qualified_name(qualified_type: String) -> String {
        let parts: Vec<&str> = qualified_type.split('.').collect();
        if let Some(last_part) = parts.last() {
            last_part.to_string()
        } else {
            String::new()
        }
    }
}

impl WorldGenerator for CSharp {
    fn preprocess(&mut self, resolve: &Resolve, world: WorldId) {
        let name = &resolve.worlds[world].name;
        self.name = name.to_string();
        self.sizes.fill(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let name = interface_name(resolve, key, Direction::Import);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, true);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(&resolve.name_world_key(key), func);
        }

        gen.add_import_return_area();
        gen.add_interface_fragment(false);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name, true);

        for (_, func) in funcs {
            gen.import(name, func);
        }

        gen.add_world_fragment();
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> Result<()> {
        let name = interface_name(resolve, key, Direction::Export);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, false);
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(func, Some(key));
        }

        gen.add_export_return_area();
        gen.add_interface_fragment(true);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> Result<()> {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name, false);

        for (_, func) in funcs {
            gen.export(func, None);
        }

        gen.add_world_fragment();
        Ok(())
    }

    fn import_types(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let name = &format!("{}-world", resolve.worlds[world].name);
        let mut gen = self.interface(resolve, name, true);

        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) {
        let world = &resolve.worlds[id];
        let snake = world.name.to_snake_case();
        let namespace = format!("wit_{snake}");
        let name = world.name.to_upper_camel_case();

        let version = env!("CARGO_PKG_VERSION");
        let mut src = String::new();
        uwriteln!(src, "// Generated by `wit-bindgen` {version}. DO NOT EDIT!");

        uwrite!(
            src,
            "{CSHARP_IMPORTS}

            namespace {namespace} {{

             public interface I{name}World {{
            "
        );

        src.push_str(
            &self
                .world_fragments
                .iter()
                .map(|f| f.csharp_src.deref())
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        if self.needs_result {
            src.push_str(
                r#"
                using System.Runtime.InteropServices;

                namespace Wit.Interop;

                [StructLayout(LayoutKind.Sequential)]
                public readonly struct Result<Ok, Err>
                {
                    public readonly byte Tag;
                    private readonly object _value;

                    private Result(byte tag, object value)
                    {
                        Tag = tag;
                        _value = value;
                    }

                    public static Result<Ok, Err> ok(Ok ok)
                    {
                        return new Result<Ok, Err>(OK, ok!);
                    }

                    public static Result<Ok, Err> err(Err err)
                    {
                        return new Result<Ok, Err>(ERR, err!);
                    }

                    public bool IsOk => Tag == OK;
                    public bool IsErr => Tag == ERR;

                    public Ok AsOk 
                    { 
                        get 
                        {
                            if (Tag == OK) 
                                return (Ok)_value;
                            else 
                                throw new ArgumentException("expected OK, got " + Tag);
                        } 
                    }

                    public Err AsErr
                    {
                        get
                        {
                            if (Tag == ERR)
                                return (Err)_value;
                            else
                                throw new ArgumentException("expected ERR, got " + Tag);
                        }
                    }

                    public const byte OK = 0;
                    public const byte ERR = 1;
                }
                "#,
            )
        }
        src.push_str("}\n");

        if self.needs_interop_string {
            src.push_str(
                r#"
                public static class InteropString
                {
                    public static IntPtr FromString(string input, out int length)
                    {
                        var utf8Bytes = Encoding.UTF8.GetBytes(input);
                        length = utf8Bytes.Length;
                        var gcHandle = GCHandle.Alloc(utf8Bytes, GCHandleType.Pinned);
                        return gcHandle.AddrOfPinnedObject();
                    }
                }
                "#,
            )
        }

        if !&self.world_fragments.is_empty() {
            src.push_str("\n");

            src.push_str("namespace exports {\n");
            src.push_str(&format!("public static class {name}World\n"));
            src.push_str("{");

            // Declare a statically-allocated return area, if needed. We only do
            // this for export bindings, because import bindings allocate their
            // return-area on the stack.
            if self.return_area_size > 0 {
                let mut ret_area_str = String::new();

                uwrite!(
                    ret_area_str,
                    "
                    [InlineArray({})]
                    [StructLayout(LayoutKind.Sequential, Pack = {})]
                    private struct ReturnArea
                    {{
                        private byte buffer;
    
                        private int GetS32(int offset)
                        {{
                            ReadOnlySpan<byte> span = this;

                            return BitConverter.ToInt32(span.Slice(offset, 4));
                        }}
                        
                        public void SetS32(int offset, int value)
                        {{
                            Span<byte> span = this;

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        internal unsafe int AddrOfBuffer()
                        {{
                            fixed(byte* ptr = &buffer)
                            {{
                                return (int)ptr;
                            }}
                        }}

                        public unsafe string GetUTF8String(int p0, int p1)
                        {{
                            return Encoding.UTF8.GetString((byte*)p0, p1);
                        }}
                    }}
    
                    [ThreadStatic]
                    private static ReturnArea returnArea = default;
                    ",
                    self.return_area_size,
                    self.return_area_align,
                );

                src.push_str(&ret_area_str);
            }

            for fragement in &self.world_fragments {
                src.push_str("\n");

                src.push_str(&fragement.csharp_interop_src);
            }
            src.push_str("}\n");
            src.push_str("}\n");
        }

        src.push_str("\n");
        src.push_str(
            r#"
                internal static class Intrinsics
                {
                    [UnmanagedCallersOnly(EntryPoint = "cabi_realloc")]
                    internal static IntPtr cabi_realloc(IntPtr ptr, uint old_size, uint align, uint new_size)
                    {
                        if (new_size == 0)
                        {
                            if(old_size != 0)
                            {
                                Marshal.Release(ptr);
                            }
                            return new IntPtr((int)align);
                        }
                
                        if (new_size > int.MaxValue)
                        {
                            throw new ArgumentException("Cannot allocate more that int.MaxValue", nameof(new_size));
                        }
                        
                        if(old_size != 0)
                        {
                            return Marshal.ReAllocHGlobal(ptr, (int)new_size);
                        }

                        return Marshal.AllocHGlobal((int)new_size);
                    }
                }
            "#,
        );
        src.push_str("}\n");

        files.push(&format!("{name}.cs"), indent(&src).as_bytes());

        let generate_stub = |name: String, files: &mut Files, stubs: Stubs| {
            let stub_file_name = format!("{name}Impl");
            let interface_name = CSharp::get_class_name_from_qualified_name(name.clone());
            let stub_class_name = format!("{interface_name}Impl");

            let (fragments, fully_qaulified_namespace) = match stubs {
                Stubs::World(fragments) => {
                    let fully_qaulified_namespace = format!("{namespace}");
                    (fragments, fully_qaulified_namespace)
                }
                Stubs::Interface(fragments) => {
                    let fully_qaulified_namespace = format!("{namespace}.{name}");
                    (fragments, fully_qaulified_namespace)
                }
            };

            let body = fragments
                .iter()
                .map(|f| f.stub.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                {CSHARP_IMPORTS}

                namespace {fully_qaulified_namespace};

                 public partial class {stub_class_name} : I{interface_name} {{
                    {body}
                 }}
                "
            );

            files.push(&format!("{stub_file_name}.cs"), indent(&body).as_bytes());
        };

        if self.opts.generate_stub {
            generate_stub(
                format!("{name}World"),
                files,
                Stubs::World(&self.world_fragments),
            );
        }

        files.push(
            &format!("{snake}_component_type.o",),
            component_type_object::object(resolve, id, self.opts.string_encoding)
                .unwrap()
                .as_slice(),
        );

        for (name, interface_type_and_fragments) in &self.interface_fragments {
            let fragments = &interface_type_and_fragments.interface_fragments;

            let interface_name = &CSharp::get_class_name_from_qualified_name(name.to_string());

            // C#
            let body = fragments
                .iter()
                .map(|f| f.csharp_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            if body.len() > 0 {
                let body = format!(
                    "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                    {CSHARP_IMPORTS}

                    namespace {namespace}.{name};
        
                    public interface I{interface_name} {{
                        {body}
                    }}
                    "
                );

                files.push(&format!("{name}.cs"), indent(&body).as_bytes());
            }

            // C# Interop
            let body = fragments
                .iter()
                .map(|f| f.csharp_interop_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                {CSHARP_IMPORTS}

                namespace {namespace}.{name};

                public static class {interface_name}Interop {{
                    {body}
                }}
                "
            );

            files.push(&format!("{name}Interop.cs"), indent(&body).as_bytes());

            if interface_type_and_fragments.is_export && self.opts.generate_stub {
                generate_stub(format!("{name}"), files, Stubs::Interface(fragments));
            }
        }
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    csharp_interop_src: String,
    stub: String,
    gen: &'a mut CSharp,
    resolve: &'a Resolve,
    name: &'a str,
    in_import: bool,
}

impl InterfaceGenerator<'_> {
    fn qualifier(&self, when: bool, ty: &TypeDef) -> String {
        if let TypeOwner::Interface(id) = &ty.owner {
            if let Some(name) = self.gen.interface_names.get(id) {
                if name != self.name {
                    return format!("{}.", name);
                }
            }
        }

        if when {
            let name = self.name;
            format!("{name}.")
        } else {
            String::new()
        }
    }

    fn add_interface_fragment(self, is_export: bool) {
        self.gen
            .interface_fragments
            .entry(self.name.to_string())
            .or_insert_with(|| InterfaceTypeAndFragments::new(is_export))
            .interface_fragments
            .push(InterfaceFragment {
                csharp_src: self.src,
                csharp_interop_src: self.csharp_interop_src,
                stub: self.stub,
            });
    }

    fn add_import_return_area(&mut self) {
        let mut ret_struct_type = String::new();
        if self.gen.return_area_size > 0 {
            uwrite!(
                ret_struct_type,
                r#"
                private unsafe struct ReturnArea
                {{
                    private int GetS32(IntPtr ptr, int offset)
                    {{
                        var span = new Span<byte>((void*)ptr, {});

                        return BitConverter.ToInt32(span.Slice(offset, 4));
                    }}

                    public string GetUTF8String(IntPtr ptr)
                    {{
                        return Encoding.UTF8.GetString((byte*)GetS32(ptr, 0), GetS32(ptr, 4));
                    }}

                }}

                [ThreadStatic]
                [FixedAddressValueType]
                private static ReturnArea returnArea;
            "#,
                self.gen.return_area_size
            );
        }

        uwrite!(
            self.csharp_interop_src,
            r#"
                {ret_struct_type}
            "#
        );
    }

    fn add_export_return_area(&mut self) {
        // Declare a statically-allocated return area, if needed. We only do
        // this for export bindings, because import bindings allocate their
        // return-area on the stack.
        if self.gen.return_area_size > 0 {
            let mut ret_area_str = String::new();

            uwrite!(
                ret_area_str,
                "
                    [InlineArray({})]
                    [StructLayout(LayoutKind.Sequential, Pack = {})]
                    private struct ReturnArea
                    {{
                        private byte buffer;
    
                        private int GetS32(int offset)
                        {{
                            ReadOnlySpan<byte> span = this;

                            return BitConverter.ToInt32(span.Slice(offset, 4));
                        }}
                        
                        public void SetS32(int offset, int value)
                        {{
                            Span<byte> span = this;

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        internal unsafe int AddrOfBuffer()
                        {{
                            fixed(byte* ptr = &buffer)
                            {{
                                return (int)ptr;
                            }}
                        }}

                        public unsafe string GetUTF8String(int p0, int p1)
                        {{
                            return Encoding.UTF8.GetString((byte*)p0, p1);
                        }}
                    }}
    
                    [ThreadStatic]
                    private static ReturnArea returnArea = default;
                    ",
                self.gen.return_area_size,
                self.gen.return_area_align,
            );

            self.csharp_interop_src.push_str(&ret_area_str);
        }
    }

    fn add_world_fragment(self) {
        self.gen.world_fragments.push(InterfaceFragment {
            csharp_src: self.src,
            csharp_interop_src: self.csharp_interop_src,
            stub: self.stub,
        });
    }

    fn import(&mut self, _module: &String, func: &Function) {
        if func.kind != FunctionKind::Freestanding {
            todo!("resources");
        }

        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        let wasm_result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let result_type: String = match func.results.len() {
            0 => "void".to_string(),
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                self.type_name(ty)
            }
            _ => unreachable!(), //TODO
        };

        let camel_name = func.name.to_upper_camel_case();

        let wasm_params = sig
            .params
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
            &func.name,
            func.params
                .iter()
                .map(|(name, _)| name.to_csharp_ident())
                .collect(),
        );

        abi::call(
            bindgen.gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
        );

        let src = bindgen.src;

        let params = func
            .params
            .iter()
            .enumerate()
            .map(|(_i, param)| {
                let ty = self.type_name(&param.1);
                let param_name = &param.0;
                format!("{ty} {param_name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let import_name = &func.name;
        uwrite!(
            self.csharp_interop_src,
            r#"
            internal static class {camel_name}WasmInterop
            {{
                [DllImport("*", EntryPoint = "{import_name}")]
                internal static extern {wasm_result_type} wasmImport{camel_name}({wasm_params});
            }}
            "#
        );

        uwrite!(
            self.csharp_interop_src,
            r#"
                internal static unsafe {result_type} {camel_name}({params})
                {{
                    {src}
                    //TODO: free alloc handle (interopString) if exists
                }}
            "#
        );
    }

    fn export(&mut self, func: &Function, interface_name: Option<&WorldKey>) {
        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.name,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
        );

        abi::call(
            bindgen.gen.resolve,
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
        );

        assert!(!bindgen.needs_cleanup_list);

        let src = bindgen.src;

        let wasm_result_type = match &sig.results[..] {
            [] => "void",
            [result] => wasm_type(*result),
            _ => unreachable!(),
        };

        let result_type = match func.results.len() {
            0 => "void".to_owned(),
            1 => self.type_name(func.results.iter_types().next().unwrap()),
            _ => func
                .results
                .iter_types()
                .map(|ty| self.type_name(ty))
                .collect::<Vec<String>>()
                .join(", "),
        };

        let camel_name = func.name.to_upper_camel_case();

        let wasm_params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name(ty);

                format!("{ty} {name}")
            })
            .collect::<Vec<String>>()
            .join(", ");

        let interop_name = format!("wasmExport{camel_name}");
        let core_module_name = interface_name.map(|s| self.resolve.name_world_key(s));
        let export_name = func.core_export_name(core_module_name.as_deref());

        uwrite!(
            self.csharp_interop_src,
            r#"
            [UnmanagedCallersOnly(EntryPoint = "{export_name}")]
            public static {wasm_result_type} {interop_name}({wasm_params}) {{
                {src}
            }}
            "#
        );

        if !sig.results.is_empty() {
            uwrite!(
                self.csharp_interop_src,
                r#"
                [UnmanagedCallersOnly(EntryPoint = "cabi_post_{export_name}")]
                public static void cabi_post_{interop_name}({wasm_result_type} returnValue) {{
                    Console.WriteLine("cabi_post_{export_name}");
                }}
                "#
            );
        }

        uwrite!(
            self.src,
            r#"static abstract {result_type} {camel_name}({params});

            "#
        );

        if self.gen.opts.generate_stub {
            let sig = self.sig_string(func, true);

            uwrite!(
                self.stub,
                r#"
                {sig} {{
                    throw new NotImplementedException();
                }}
                "#
            );
        }
    }

    fn type_name(&mut self, ty: &Type) -> String {
        self.type_name_with_qualifier(ty, false)
    }

    fn type_name_with_qualifier(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "bool".to_owned(),
            Type::U8 => "byte".to_owned(),
            Type::U16 => "ushort".to_owned(),
            Type::U32 => "uint".to_owned(),
            Type::U64 => "ulong".to_owned(),
            Type::S8 => "sbyte".to_owned(),
            Type::S16 => "short".to_owned(),
            Type::S32 => "int".to_owned(),
            Type::S64 => "long".to_owned(),
            Type::Float32 => "float".to_owned(),
            Type::Float64 => "double".to_owned(),
            Type::Char => "uint".to_owned(),
            Type::String => "string".to_owned(),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.type_name_with_qualifier(ty, qualifier),
                    TypeDefKind::List(ty) => {
                        if is_primitive(ty) {
                            format!("{}[]", self.type_name(ty))
                        } else {
                            format!("List<{}>", self.type_name_boxed(ty, qualifier))
                        }
                    }
                    TypeDefKind::Tuple(tuple) => {
                        let count = tuple.types.len();
                        self.gen.tuple_counts.insert(count);

                        let params = if count == 0 {
                            String::new()
                        } else {
                            format!(
                                "({})",
                                tuple
                                    .types
                                    .iter()
                                    .map(|ty| self.type_name_boxed(ty, qualifier))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };

                        params
                    }
                    TypeDefKind::Option(ty) => self.type_name_boxed(ty, qualifier),
                    TypeDefKind::Result(result) => {
                        self.gen.needs_result = true;
                        let mut name = |ty: &Option<Type>| {
                            ty.as_ref()
                                .map(|ty| self.type_name_boxed(ty, qualifier))
                                .unwrap_or_else(|| "void".to_owned())
                        };
                        let ok = name(&result.ok);
                        let err = name(&result.err);

                        format!("{}Result<{ok}, {err}>", self.gen.qualifier())
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(qualifier, ty),
                                name.to_upper_camel_case()
                            )
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    fn type_name_boxed(&mut self, ty: &Type, qualifier: bool) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "byte".into(),
            Type::U16 => "ushort".into(),
            Type::U32 => "uint".into(),
            Type::U64 => "ulong".into(),
            Type::S8 => "sbyte".into(),
            Type::S16 => "short".into(),
            Type::S32 => "int".into(),
            Type::S64 => "long".into(),
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::Char => "uint".into(),
            Type::Id(id) => {
                let def = &self.resolve.types[*id];
                match &def.kind {
                    TypeDefKind::Type(ty) => self.type_name_boxed(ty, qualifier),
                    _ => self.type_name_with_qualifier(ty, qualifier),
                }
            }
            _ => self.type_name_with_qualifier(ty, qualifier),
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

    fn non_empty_type<'a>(&self, ty: Option<&'a Type>) -> Option<&'a Type> {
        if let Some(ty) = ty {
            let id = match ty {
                Type::Id(id) => *id,
                _ => return Some(ty),
            };
            match &self.resolve.types[id].kind {
                TypeDefKind::Type(t) => self.non_empty_type(Some(t)).map(|_| ty),
                TypeDefKind::Record(r) => (!r.fields.is_empty()).then_some(ty),
                TypeDefKind::Tuple(t) => (!t.types.is_empty()).then_some(ty),
                _ => Some(ty),
            }
        } else {
            None
        }
    }

    fn sig_string(&mut self, func: &Function, qualifier: bool) -> String {
        let name = func.name.to_csharp_ident();

        let result_type = match func.results.len() {
            0 => "void".into(),
            1 => {
                self.type_name_with_qualifier(func.results.iter_types().next().unwrap(), qualifier)
            }
            count => {
                self.gen.tuple_counts.insert(count);
                format!(
                    "({})",
                    func.results
                        .iter_types()
                        .map(|ty| self.type_name_boxed(ty, qualifier))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        };

        let params = func
            .params
            .iter()
            .map(|(name, ty)| {
                let ty = self.type_name_with_qualifier(ty, qualifier);
                let name = name.to_csharp_ident();
                format!("{ty} {name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let camel_case = name.to_upper_camel_case();
        format!("public static {result_type} {camel_case}({params})")
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let parameters = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{} {}",
                    self.type_name(&field.ty),
                    field.name.to_csharp_ident()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let assignments = record
            .fields
            .iter()
            .map(|field| {
                let name = field.name.to_csharp_ident();
                format!("this.{name} = {name};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let fields = if record.fields.is_empty() {
            format!("public const {name} INSTANCE = new {name}();")
        } else {
            record
                .fields
                .iter()
                .map(|field| {
                    format!(
                        "public readonly {} {};",
                        self.type_name(&field.ty),
                        field.name.to_csharp_ident()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        uwrite!(
            self.src,
            "
            public static class {name} {{
                {fields}

                public {name}({parameters}) {{
                    {assignments}
                }}
            }}
            "
        );
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();

        let ty = match flags.repr() {
            FlagsRepr::U8 => "byte",
            FlagsRepr::U16 => "ushort",
            FlagsRepr::U32(1) => "uint",
            FlagsRepr::U32(2) => "ulong",
            repr => todo!("flags {repr:?}"),
        };

        let flags = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let flag_name = flag.name.to_shouty_snake_case();
                let suffix = if matches!(flags.repr(), FlagsRepr::U32(2)) {
                    "L"
                } else {
                    ""
                };
                format!(
                    "public static readonly {name} {flag_name} = new {name}(({ty}) (1{suffix} << {i}));"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static class {name} {{
                public readonly {ty} value;

                public {name}({ty} value) {{
                    this.value = value;
                }}

                {flags}
            }}
            "
        );
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, _tuple: &Tuple, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.print_docs(docs);

        let name = name.to_upper_camel_case();
        let tag_type = int_type(variant.tag());

        let constructors = variant
            .cases
            .iter()
            .map(|case| {
                let case_name = case.name.to_csharp_ident();
                let tag = case.name.to_shouty_snake_case();
                let (parameter, argument) = if let Some(ty) = self.non_empty_type(case.ty.as_ref())
                {
                    (
                        format!("{} {case_name}", self.type_name(ty)),
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
                self.non_empty_type(case.ty.as_ref()).map(|ty| {
                    let case_name = case.name.to_upper_camel_case();
                    let tag = case.name.to_shouty_snake_case();
                    let ty = self.type_name(ty);
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
                format!("public readonly {tag_type} {tag} = {i};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            public static class {name} {{
                public readonly {tag_type} tag;
                private readonly Object value;

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

    fn type_option(&mut self, id: TypeId, _name: &str, _payload: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_result(&mut self, id: TypeId, _name: &str, _result: &Result_, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
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

    fn type_alias(&mut self, id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_list(&mut self, id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_builtin(&mut self, _id: TypeId, _name: &str, _ty: &Type, _docs: &Docs) {
        unimplemented!();
    }

    fn define_type(&mut self, name: &str, id: TypeId) {
        let ty = &self.resolve().types[id];
        match &ty.kind {
            TypeDefKind::Record(record) => self.type_record(id, name, record, &ty.docs),
            TypeDefKind::Flags(flags) => self.type_flags(id, name, flags, &ty.docs),
            TypeDefKind::Tuple(tuple) => self.type_tuple(id, name, tuple, &ty.docs),
            TypeDefKind::Enum(enum_) => self.type_enum(id, name, enum_, &ty.docs),
            TypeDefKind::Variant(variant) => self.type_variant(id, name, variant, &ty.docs),
            TypeDefKind::Option(t) => self.type_option(id, name, t, &ty.docs),
            TypeDefKind::Result(r) => self.type_result(id, name, r, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(_) => todo!("generate for future"),
            TypeDefKind::Stream(_) => todo!("generate for stream"),
            TypeDefKind::Resource => todo!("generate for resource"),
            TypeDefKind::Handle(_) => todo!("generate for handle"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn type_resource(&mut self, _id: TypeId, _name: &str, _docs: &Docs) {
        todo!()
    }
}

enum Stubs<'a> {
    World(&'a Vec<InterfaceFragment>),
    Interface(&'a Vec<InterfaceFragment>),
}

struct Block {
    _body: String,
    _results: Vec<String>,
    _xelement: String,
    _xbase: String,
}

struct BlockStorage {
    body: String,
    element: String,
    base: String,
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    func_name: &'b str,
    params: Box<[String]>,
    src: String,
    locals: Ns,
    block_storage: Vec<BlockStorage>,
    blocks: Vec<Block>,
    needs_cleanup_list: bool,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(
        gen: &'b mut InterfaceGenerator<'a>,
        func_name: &'b str,
        params: Box<[String]>,
    ) -> FunctionBindgen<'a, 'b> {
        Self {
            gen,
            func_name,
            params,
            src: String::new(),
            locals: Ns::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            needs_cleanup_list: false,
        }
    }
}

impl Bindgen for FunctionBindgen<'_, '_> {
    type Operand = String;

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
            Instruction::ConstZero { tys } => results.extend(tys.iter().map(|ty| {
                match ty {
                    WasmType::I32 => "0",
                    WasmType::I64 => "0L",
                    WasmType::F32 => "0.0F",
                    WasmType::F64 => "0.0D",
                }
                .to_owned()
            })),

            Instruction::I32Load { offset } => results.push(format!("returnArea.GetS32({offset})")),
            Instruction::I32Load8U { offset } => {
                results.push(format!("returnArea.GetU8({offset})"))
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!("returnArea.GetS8({offset})"))
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!("returnArea.GetU16({offset})"))
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!("returnArea.GetS16({offset})"))
            }
            Instruction::I64Load { offset } => results.push(format!("returnArea.GetS64({offset})")),
            Instruction::F32Load { offset } => results.push(format!("returnArea.GetF32({offset})")),
            Instruction::F64Load { offset } => results.push(format!("returnArea.GetF64({offset})")),

            Instruction::I32Store { offset } => {
                uwriteln!(self.src, "returnArea.SetS32({}, {});", offset, operands[0])
            }
            Instruction::I32Store8 { .. } => todo!("I32Store8"),
            Instruction::I32Store16 { .. } => todo!("I32Store16"),
            Instruction::I64Store { .. } => todo!("I64Store"),
            Instruction::F32Store { .. } => todo!("F32Store"),
            Instruction::F64Store { .. } => todo!("F64Store"),

            Instruction::I64FromU64 => results.push(format!("unchecked((long)({}))", operands[0])),
            Instruction::I32FromChar => results.push(format!("((int){})", operands[0])),
            Instruction::I32FromU32 => results.push(format!("unchecked((int)({}))", operands[0])),
            Instruction::U8FromI32 => results.push(format!("((byte){})", operands[0])),
            Instruction::S8FromI32 => results.push(format!("((sbyte){})", operands[0])),
            Instruction::U16FromI32 => results.push(format!("((ushort){})", operands[0])),
            Instruction::S16FromI32 => results.push(format!("((short){})", operands[0])),
            Instruction::U32FromI32 => results.push(format!("unchecked((uint)({}))", operands[0])),
            Instruction::U64FromI64 => results.push(format!("unchecked((ulong)({}))", operands[0])),
            Instruction::CharFromI32 => results.push(format!("unchecked((uint)({}))", operands[0])),
            Instruction::I64FromS64
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromS32
            | Instruction::F32FromFloat32
            | Instruction::F64FromFloat64
            | Instruction::S32FromI32
            | Instruction::S64FromI64
            | Instruction::Float32FromF32
            | Instruction::Float64FromF64 => results.push(operands[0].clone()),

            Instruction::Bitcasts { .. } => todo!("Bitcasts"),

            Instruction::I32FromBool => {
                results.push(format!("({} ? 1 : 0)", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            Instruction::FlagsLower { .. } => todo!("FlagsLower"),

            Instruction::FlagsLift { .. } => todo!("FlagsLift"),

            Instruction::RecordLower { .. } => todo!("RecordLower"),
            Instruction::RecordLift { .. } => todo!("RecordLift"),
            Instruction::TupleLift { .. } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!("({ops})"));
            }

            Instruction::TupleLower { tuple: _, ty } => {
                let ops = operands
                    .iter()
                    .map(|op| op.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                results.push(format!("({ops})"));
                results.push(format!("({:?})", ty));
            }

            Instruction::VariantPayloadName => {
                todo!("VariantPayloadName");
            }

            Instruction::VariantLower { .. } => todo!("VariantLift"),

            Instruction::VariantLift { .. } => todo!("VariantLift"),

            Instruction::OptionLower { .. } => todo!("OptionLower"),

            Instruction::OptionLift { .. } => todo!("OptionLift"),

            Instruction::ResultLower { .. } => todo!("ResultLower"),

            Instruction::ResultLift { .. } => todo!("ResultLift"),

            Instruction::EnumLower { .. } => todo!("EnumLower"),

            Instruction::EnumLift { .. } => todo!("EnumLift"),

            Instruction::ListCanonLower { .. } => todo!("ListCanonLower"),

            Instruction::ListCanonLift { .. } => todo!("ListCanonLift"),

            Instruction::StringLower { realloc } => {
                let op = &operands[0];
                let interop_string = self.locals.tmp("interopString");
                let result_var = self.locals.tmp("result");
                uwriteln!(
                    self.src,
                    "
                    var {result_var} = {op};
                    IntPtr {interop_string} = InteropString.FromString({result_var}, out int length{result_var});"
                );

                //TODO: Oppertunity to optimize and not reallocate every call
                if realloc.is_none() {
                    results.push(format!("{interop_string}.ToInt32()"));
                } else {
                    results.push(format!("{interop_string}.ToInt32()"));
                }
                results.push(format!("length{result_var}"));

                self.gen.gen.needs_interop_string = true;
            }

            Instruction::StringLift { .. } => {
                if self.gen.in_import {
                    results.push(format!("returnArea.GetUTF8String(ptr)"));
                } else {
                    let address = &operands[0];
                    let length = &operands[1];

                    results.push(format!("returnArea.GetUTF8String({address}, {length})"));
                }
            }

            Instruction::ListLower { .. } => todo!("ListLower"),

            Instruction::ListLift { .. } => todo!("ListLift"),

            Instruction::IterElem { .. } => todo!("IterElem"),

            Instruction::IterBasePointer => todo!("IterBasePointer"),

            Instruction::CallWasm { sig, name } => {
                //TODO: Use base_name instead?
                let assignment = match &sig.results[..] {
                    [_] => {
                        let result = self.locals.tmp("result");
                        let assignment = format!("var {result} = ");
                        results.push(result);
                        assignment
                    }

                    [] => String::new(),

                    _ => unreachable!(),
                };

                let func_name = self.func_name.to_upper_camel_case();
                let name = name.to_upper_camel_case();

                let operands = operands.join(", ");

                uwriteln!(
                    self.src,
                    "{assignment} {name}WasmInterop.wasmImport{func_name}({operands});"
                );
            }

            Instruction::CallInterface { func } => {
                let module = self.gen.name.to_string();
                let func_name = self.func_name.to_upper_camel_case();
                let class_name =
                    CSharp::get_class_name_from_qualified_name(module).to_upper_camel_case();
                let mut oper = String::new();

                for (i, param) in operands.iter().enumerate() {
                    oper.push_str(&format!("({param})"));

                    if i < operands.len() && operands.len() != i + 1 {
                        oper.push_str(", ");
                    }
                }

                match func.results.len() {
                    0 => self
                        .src
                        .push_str(&format!("{class_name}Impl.{func_name}({oper});")),
                    1 => results.push(format!("{class_name}Impl.{func_name}({oper})")),
                    _ => results.push(format!("{class_name}Impl.{func_name}({oper})")),
                }
            }

            Instruction::Return { amt: _, func } => match func.results.len() {
                0 => (),
                1 => uwriteln!(self.src, "return {};", operands[0]),
                _ => {
                    let results = operands.join(", ");
                    let sig = self
                        .gen
                        .resolve()
                        .wasm_signature(AbiVariant::GuestExport, func);
                    let cast = sig
                        .results
                        .into_iter()
                        .map(|ty| wasm_type(ty))
                        .collect::<Vec<&str>>()
                        .join(", ");
                    uwriteln!(self.src, "return ({cast})({results});")
                }
            },

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { .. } => todo!("GuestDeallocate"),

            Instruction::GuestDeallocateString => todo!("GuestDeallocateString"),

            Instruction::GuestDeallocateVariant { .. } => todo!("GuestDeallocateString"),

            Instruction::GuestDeallocateList { .. } => todo!("GuestDeallocateList"),
            Instruction::HandleLower {
                handle: _,
                name: _,
                ty: _,
            } => todo!(),
            Instruction::HandleLift {
                handle: _,
                name: _,
                ty: _dir,
            } => todo!("HandleLeft"),
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        let ptr = self.locals.tmp("ptr");

        // Use a stack-based return area for imports, because exports need
        // their return area to be live until the post-return call.
        if self.gen.in_import {
            self.gen.gen.return_area_size = size;
            self.gen.gen.return_area_align = align;

            uwrite!(
                self.src,
                "
                void* buffer = stackalloc int[{} + {} - 1];
                var {} = ((int)buffer) + ({} - 1) & -{};
                ",
                size,
                align,
                ptr,
                align,
                align,
            );
        } else {
            self.gen.gen.return_area_size = self.gen.gen.return_area_size.max(size);
            self.gen.gen.return_area_align = self.gen.gen.return_area_align.max(align);

            uwrite!(
                self.src,
                "
                var {} = returnArea.AddrOfBuffer();
                ",
                ptr,
            );
        }

        format!("{ptr}")
    }

    fn push_block(&mut self) {
        self.block_storage.push(BlockStorage {
            body: mem::take(&mut self.src),
            element: self.locals.tmp("element"),
            base: self.locals.tmp("base"),
        });
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let BlockStorage {
            body,
            element,
            base,
        } = self.block_storage.pop().unwrap();

        self.blocks.push(Block {
            _body: mem::replace(&mut self.src, body),
            _results: mem::take(operands),
            _xelement: element,
            _xbase: base,
        });
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.gen.sizes
    }

    fn is_list_canonical(&self, _resolve: &Resolve, element: &Type) -> bool {
        is_primitive(element)
    }
}

fn int_type(int: Int) -> &'static str {
    match int {
        Int::U8 => "byte",
        Int::U16 => "ushort",
        Int::U32 => "uint",
        Int::U64 => "ulong",
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

//TODO: Implement Flags
//fn flags_repr(flags: &Flags) -> Int {
//    match flags.repr() {
//        FlagsRepr::U8 => Int::U8,
//        FlagsRepr::U16 => Int::U16,
//        FlagsRepr::U32(1) => Int::U32,
//        FlagsRepr::U32(2) => Int::U64,
//        repr => panic!("unimplemented flags {repr:?}"),
//    }
//}

//fn list_element_info(ty: &Type) -> (usize, &'static str) {
//    match ty {
//        Type::S8 => (1, "sbyte"),
//        Type::S16 => (2, "short"),
//        Type::S32 => (4, "int"),
//        Type::S64 => (8, "long"),
//        Type::U8 => (1, "byte"),
//        Type::U16 => (2, "ushort"),
//        Type::U32 => (4, "uint"),
//        Type::U64 => (8, "ulong"),
//        Type::Float32 => (4, "float"),
//        Type::Float64 => (8, "double"),
//        _ => unreachable!(),
//    }
//}

fn indent(code: &str) -> String {
    let mut indented = String::with_capacity(code.len());
    let mut indent = 0;
    let mut was_empty = false;
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if was_empty {
                continue;
            }
            was_empty = true;
        } else {
            was_empty = false;
        }

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

// fn world_name(resolve: &Resolve, world: WorldId) -> String {
//     format!(
//         "wit.worlds.{}",
//         resolve.worlds[world].name.to_upper_camel_case()
//     )
// }

fn interface_name(resolve: &Resolve, name: &WorldKey, direction: Direction) -> String {
    let pkg = match name {
        WorldKey::Name(_) => None,
        WorldKey::Interface(id) => {
            let pkg = resolve.interfaces[*id].package.unwrap();
            Some(resolve.packages[pkg].name.clone())
        }
    };

    let name = match name {
        WorldKey::Name(name) => name,
        WorldKey::Interface(id) => resolve.interfaces[*id].name.as_ref().unwrap(),
    }
    .to_upper_camel_case();

    let namespace = match &pkg {
        Some(name) => {
            let mut ns = format!(
                "{}.{}.",
                name.namespace.to_csharp_ident(),
                name.name.to_csharp_ident()
            );

            if let Some(version) = &name.version {
                let v = version
                    .to_string()
                    .replace('.', "_")
                    .replace('-', "_")
                    .replace('+', "_");
                ns = format!("{}v{}.", ns, &v);
            }
            ns
        }
        None => String::new(),
    };

    format!(
        "wit.{}.{}{name}",
        match direction {
            Direction::Import => "imports",
            Direction::Export => "exports",
        },
        namespace
    )
}

fn is_primitive(ty: &Type) -> bool {
    matches!(
        ty,
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

trait ToCSharpIdent: ToOwned {
    fn to_csharp_ident(&self) -> Self::Owned;
}

impl ToCSharpIdent for str {
    fn to_csharp_ident(&self) -> String {
        // Escape C# keywords
        // Source: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/

        //TODO: Repace with actual keywords
        match self {
            "abstract" | "continue" | "for" | "new" | "switch" | "assert" | "default" | "goto"
            | "namespace" | "synchronized" | "boolean" | "do" | "if" | "private" | "this"
            | "break" | "double" | "implements" | "protected" | "throw" | "byte" | "else"
            | "import" | "public" | "throws" | "case" | "enum" | "instanceof" | "return"
            | "transient" | "catch" | "extends" | "int" | "short" | "try" | "char" | "final"
            | "interface" | "static" | "void" | "class" | "finally" | "long" | "strictfp"
            | "volatile" | "const" | "float" | "super" | "while" => format!("{self}_"),
            _ => self.to_lower_camel_case(),
        }
    }
}
