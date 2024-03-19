mod component_type_object;

use anyhow::Result;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToUpperCamelCase};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    iter, mem,
    ops::Deref,
};
use wit_bindgen_core::{
    abi::{self, AbiVariant, Bindgen, Instruction, LiftLower, WasmType},
    wit_parser::LiveTypes,
    Direction,
};
use wit_bindgen_core::{
    uwrite, uwriteln,
    wit_parser::{
        Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Int, InterfaceId, Record, Resolve,
        Result_, SizeAlign, Tuple, Type, TypeDefKind, TypeId, TypeOwner, Variant, WorldId,
        WorldKey,
    },
    Files, InterfaceGenerator as _, Ns, WorldGenerator,
};
use wit_component::StringEncoding;
mod csproj;
pub use csproj::CSProject;

//TODO remove unused
const CSHARP_IMPORTS: &str = "\
using System;
using System.Runtime.CompilerServices;
using System.Collections;
using System.Runtime.InteropServices;
using System.Text;
using System.Diagnostics;

";

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long, default_value_t = StringEncoding::default()))]
    pub string_encoding: StringEncoding,
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,

    // TODO: This should only temporarily needed until mono and native aot aligns.
    #[cfg_attr(feature = "clap", arg(short, long, value_enum))]
    pub runtime: CSharpRuntime,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(CSharp {
            opts: self.clone(),
            ..CSharp::default()
        })
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum CSharpRuntime {
    #[default]
    NativeAOT,
    Mono,
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

/// Indicates if we are generating for functions in an interface or free standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionLevel {
    Interface,
    FreeStanding,
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
    anonymous_type_owners: HashMap<TypeId, TypeOwner>,
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
        direction: Direction,
        function_level: FunctionLevel,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            csharp_interop_src: String::new(),
            stub: String::new(),
            gen: self,
            resolve,
            name,
            direction,
            function_level,
        }
    }

    // returns the qualifier and last part
    fn get_class_name_from_qualified_name(qualified_type: String) -> (String, String) {
        let parts: Vec<&str> = qualified_type.split('.').collect();
        if let Some(last_part) = parts.last() {
            let mut qualifier = qualified_type.strip_suffix(last_part);
            if qualifier.is_some() {
                qualifier = qualifier.unwrap().strip_suffix(".");
            }
            (qualifier.unwrap_or("").to_string(), last_part.to_string())
        } else {
            (String::new(), String::new())
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
        let name = interface_name(self, resolve, key, Direction::Import);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, Direction::Import, FunctionLevel::Interface);

        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.import(&resolve.name_world_key(key), func);
        }

        // for anonymous types
        gen.define_interface_types(id);

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
        let mut gen = self.interface(
            resolve,
            name,
            Direction::Import,
            FunctionLevel::FreeStanding,
        );

        for (import_module_name, func) in funcs {
            gen.import(import_module_name, func);
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
        let name = interface_name(self, resolve, key, Direction::Export);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, Direction::Export, FunctionLevel::Interface);

        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.export(func, Some(key));
        }

        // for anonymous types
        gen.define_interface_types(id);

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
        let mut gen = self.interface(
            resolve,
            name,
            Direction::Export,
            FunctionLevel::FreeStanding,
        );

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
        let mut gen = self.interface(resolve, name, Direction::Import, FunctionLevel::Interface);

        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> Result<()> {
        let world = &resolve.worlds[id];
        let world_namespace = self.qualifier();
        let world_namespace = world_namespace.strip_suffix(".").unwrap();
        let namespace = format!("{world_namespace}");
        let name = world.name.to_upper_camel_case();

        let version = env!("CARGO_PKG_VERSION");
        let mut src = String::new();
        uwriteln!(src, "// Generated by `wit-bindgen` {version}. DO NOT EDIT!");

        uwrite!(
            src,
            "{CSHARP_IMPORTS}

            namespace {world_namespace} {{

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
                    [InlineArray({0})]
                    [StructLayout(LayoutKind.Sequential, Pack = {1})]
                    private struct ReturnArea
                    {{
                        private byte buffer;

                        private int GetS32(int offset)
                        {{
                            ReadOnlySpan<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            return BitConverter.ToInt32(span.Slice(offset, 4));
                        }}

                        public void SetS32(int offset, int value)
                        {{
                            Span<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        public void SetF32(int offset, float value)
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
                    self.return_area_align
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

        src.push_str("}\n");

        files.push(&format!("{name}.cs"), indent(&src).as_bytes());

        let mut cabi_relloc_src = String::new();

        cabi_relloc_src.push_str(
            r#"
                #include <stdlib.h>

                /* Done in C so we can avoid initializing the dotnet runtime and hence WASI libc */
                /* It would be preferable to do this in C# but the constrainst of cabi_realloc and the demands */
                /* of WASI libc prevent us doing so. */
                /* See https://github.com/bytecodealliance/wit-bindgen/issues/777  */
                /* and https://github.com/WebAssembly/wasi-libc/issues/452 */
                /* The component model `start` function might be an alternative to this depending on whether it */
                /* has the same constraints as `cabi_realloc` */
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
        files.push(
            &format!("{name}World_cabi_realloc.c"),
            indent(&cabi_relloc_src).as_bytes(),
        );

        let generate_stub = |name: String, files: &mut Files, stubs: Stubs| {
            let (stub_namespace, interface_or_class_name) =
                CSharp::get_class_name_from_qualified_name(name.clone());

            let stub_class_name = format!(
                "{}Impl",
                match interface_or_class_name.starts_with("I") {
                    true => interface_or_class_name
                        .strip_prefix("I")
                        .unwrap()
                        .to_string(),
                    false => interface_or_class_name.clone(),
                }
            );

            let stub_file_name = match stub_namespace.len() {
                0 => stub_class_name.clone(),
                _ => format!("{stub_namespace}.{stub_class_name}"),
            };

            let (fragments, fully_qualified_namespace) = match stubs {
                Stubs::World(fragments) => {
                    let fully_qualified_namespace = format!("{namespace}");
                    (fragments, fully_qualified_namespace)
                }
                Stubs::Interface(fragments) => {
                    let fully_qualified_namespace = format!("{stub_namespace}");
                    (fragments, fully_qualified_namespace)
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

                namespace {fully_qualified_namespace};

                 public partial class {stub_class_name} : {interface_or_class_name} {{
                    {body}
                 }}
                "
            );

            files.push(&format!("{stub_file_name}.cs"), indent(&body).as_bytes());
        };

        if self.opts.generate_stub {
            generate_stub(
                format!("I{name}World"),
                files,
                Stubs::World(&self.world_fragments),
            );
        }

        //TODO: This is currently neede for mono even if it's built as a library.
        if self.opts.runtime == CSharpRuntime::Mono {
            files.push(
                &format!("MonoEntrypoint.cs",),
                indent(
                    r#"
                public class MonoEntrypoint() {
                    public static void Main() {
                    }
                }
                "#,
                )
                .as_bytes(),
            );
        }

        files.push(
            &format!("{world_namespace}_component_type.o",),
            component_type_object::object(resolve, id, self.opts.string_encoding)
                .unwrap()
                .as_slice(),
        );

        // TODO: remove when we switch to dotnet 9
        let mut wasm_import_linakge_src = String::new();

        wasm_import_linakge_src.push_str(
            r#"
            // temporarily add this attribute until it is available in dotnet 9
            namespace System.Runtime.InteropServices
            {
                internal partial class WasmImportLinkageAttribute : Attribute {}
            }
            "#,
        );
        files.push(
            &format!("{world_namespace}_wasm_import_linkage_attribute.cs"),
            indent(&wasm_import_linakge_src).as_bytes(),
        );

        for (name, interface_type_and_fragments) in &self.interface_fragments {
            let fragments = &interface_type_and_fragments.interface_fragments;

            let (namespace, interface_name) =
                &CSharp::get_class_name_from_qualified_name(name.to_string());

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

                    namespace {namespace};

                    public interface {interface_name} {{
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

            let class_name = interface_name.strip_prefix("I").unwrap();
            let body = format!(
                "// Generated by `wit-bindgen` {version}. DO NOT EDIT!
                {CSHARP_IMPORTS}

                namespace {namespace}
                {{
                  public static class {class_name}Interop {{
                      {body}
                  }}
                }}
                "
            );

            files.push(
                &format!("{namespace}.{class_name}Interop.cs"),
                indent(&body).as_bytes(),
            );

            if interface_type_and_fragments.is_export && self.opts.generate_stub {
                generate_stub(name.to_string(), files, Stubs::Interface(fragments));
            }
        }

        Ok(())
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    csharp_interop_src: String,
    stub: String,
    gen: &'a mut CSharp,
    resolve: &'a Resolve,
    name: &'a str,
    direction: Direction,
    function_level: FunctionLevel,
}

impl InterfaceGenerator<'_> {
    fn define_interface_types(&mut self, id: InterfaceId) {
        let mut live = LiveTypes::default();
        live.add_interface(self.resolve, id);
        self.define_live_types(live, id);
    }

    //TODO: we probably need this for anonymous types outside of an interface...
    // fn define_function_types(&mut self, funcs: &[(&str, &Function)]) {
    //     let mut live = LiveTypes::default();
    //     for (_, func) in funcs {
    //         live.add_func(self.resolve, func);
    //     }
    //     self.define_live_types(live);
    // }

    fn define_live_types(&mut self, live: LiveTypes, id: InterfaceId) {
        let mut type_names = HashMap::new();

        for ty in live.iter() {
            // just create c# types for wit anonymous types
            let type_def = &self.resolve.types[ty];
            if type_names.contains_key(&ty) || type_def.name.is_some() {
                continue;
            }

            let typedef_name = self.type_name(&Type::Id(ty));

            let prev = type_names.insert(ty, typedef_name.clone());
            assert!(prev.is_none());

            // workaround for owner not set on anonymous types, maintain or own map to the owner
            self.gen
                .anonymous_type_owners
                .insert(ty, TypeOwner::Interface(id));

            self.define_anonymous_type(ty, &typedef_name)
        }
    }

    fn define_anonymous_type(&mut self, type_id: TypeId, typedef_name: &str) {
        let type_def = &self.resolve().types[type_id];
        let kind = &type_def.kind;

        // TODO Does c# need this exit?
        // // skip `typedef handle_x handle_y` where `handle_x` is the same as `handle_y`
        // if let TypeDefKind::Handle(handle) = kind {
        //     let resource = match handle {
        //         Handle::Borrow(id) | Handle::Own(id) => id,
        //     };
        //     let origin = dealias(self.resolve, *resource);
        //     if origin == *resource {
        //         return;
        //     }
        // }

        //TODO: what other TypeDefKind do we need here?
        match kind {
            TypeDefKind::Tuple(t) => self.type_tuple(type_id, typedef_name, t, &type_def.docs),
            TypeDefKind::Option(t) => self.type_option(type_id, typedef_name, t, &type_def.docs),
            TypeDefKind::Record(t) => self.type_record(type_id, typedef_name, t, &type_def.docs),
            _ => unreachable!(),
        }
    }

    fn qualifier(&self, when: bool, ty: &TypeId) -> String {
        // anonymous types dont get an owner from wit-parser, so assume they are part of an interface here.
        let owner = if let Some(owner_type) = self.gen.anonymous_type_owners.get(ty) {
            *owner_type
        } else {
            let type_def = &self.resolve.types[*ty];
            type_def.owner
        };

        if let TypeOwner::Interface(id) = owner {
            if let Some(name) = self.gen.interface_names.get(&id) {
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
                    public static byte GetU8(IntPtr ptr)
                    {{
                        var span = new Span<byte>((void*)ptr, 1);

                        return span[0];
                    }}

                    public static ushort GetU16(IntPtr ptr)
                    {{
                        var span = new Span<byte>((void*)ptr, 2);

                        return BitConverter.ToUInt16(span);
                    }}

                    public static int GetS32(IntPtr ptr)
                    {{
                        var span = new Span<byte>((void*)ptr, 4);

                        return BitConverter.ToInt32(span);
                    }}

                    internal static float GetF32(IntPtr ptr, int offset)
                    {{
                        var span = new Span<byte>((void*)ptr, 4);
                        return BitConverter.ToSingle(span.Slice(offset, 4));
                    }}

                    public static string GetUTF8String(IntPtr ptr)
                    {{
                        return Encoding.UTF8.GetString((byte*)GetS32(ptr), GetS32(ptr + 4));
                    }}

                }}
            "#
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
                    [InlineArray({0})]
                    [StructLayout(LayoutKind.Sequential, Pack = {1})]
                    private struct ReturnArea
                    {{
                        private byte buffer;

                        private int GetS32(int offset)
                        {{
                            ReadOnlySpan<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            return BitConverter.ToInt32(span.Slice(offset, 4));
                        }}

                        public void SetS8(int offset, int value)
                        {{
                            Span<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        public void SetS16(int offset, short value)
                        {{
                            Span<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        public void SetS32(int offset, int value)
                        {{
                            Span<byte> span = MemoryMarshal.CreateSpan(ref buffer, {0});

                            BitConverter.TryWriteBytes(span.Slice(offset), value);
                        }}

                        public void SetF32(int offset, float value)
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
                self.gen.return_area_align
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

    fn import(&mut self, import_module_name: &str, func: &Function) {
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
                self.type_name_with_qualifier(ty, true)
            }
            _ => {
                let types = func
                    .results
                    .iter_types()
                    .map(|ty| self.type_name_with_qualifier(ty, true))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", types)
            }
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
                let ty = self.type_name_with_qualifier(&param.1, true);
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
                [DllImport("{import_module_name}", EntryPoint = "{import_name}"), WasmImportLinkage]
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
            _ => {
                let types = func
                    .results
                    .iter_types()
                    .map(|ty| self.type_name(ty))
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("({})", types)
            }
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

    // We use a global:: prefix to avoid conflicts with namespace clashes on partial namespace matches
    fn global_if_user_type(&mut self, ty: &Type) -> String {
        match ty {
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Tuple(_tuple) => "".to_owned(),
                    TypeDefKind::Type(inner_type) => self.global_if_user_type(inner_type),
                    _ => "global::".to_owned(),
                }
            }
            _ => "".to_owned(),
        }
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

                        let params = match count {
                            0 => String::new(),
                            1 => self.type_name_boxed(tuple.types.first().unwrap(), qualifier),
                            _ => format!(
                                "({})",
                                tuple
                                    .types
                                    .iter()
                                    .map(|ty| self.type_name_boxed(ty, qualifier))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        };

                        params
                    }
                    TypeDefKind::Option(base_ty) => {
                        // TODO: investigate a generic Option<> class.
                        if let Some(_name) = &ty.name {
                            format!(
                                "{}Option{}",
                                self.qualifier(qualifier, id),
                                self.type_name_with_qualifier(base_ty, false)
                            )
                        } else {
                            format!(
                                "{}Option_{}",
                                self.qualifier(qualifier, id),
                                self.type_name_with_qualifier(base_ty, false)
                            )
                        }
                    }
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
                                self.qualifier(qualifier, id),
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
                let global_prefix =
                    self.global_if_user_type(func.results.iter_types().next().unwrap());
                format!(
                    "{}{}",
                    global_prefix,
                    self.type_name_with_qualifier(
                        func.results.iter_types().next().unwrap(),
                        qualifier
                    )
                )
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
                let global_prefix = self.global_if_user_type(ty);
                let ty = self.type_name_with_qualifier(ty, qualifier);
                let name = name.to_csharp_ident();
                format!("{global_prefix}{ty} {name}")
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
            public class {name} {{
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

        let enum_elements = flags
            .flags
            .iter()
            .enumerate()
            .map(|(i, flag)| {
                let flag_name = flag.name.to_shouty_snake_case();
                let suffix = if matches!(flags.repr(), FlagsRepr::U32(2)) {
                    "UL"
                } else {
                    ""
                };
                format!("{flag_name} = 1{suffix} << {i},")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let enum_type = match flags.repr() {
            FlagsRepr::U32(2) => ": ulong",
            FlagsRepr::U16 => ": ushort",
            FlagsRepr::U8 => ": byte",
            _ => "",
        };

        uwrite!(
            self.src,
            "
            public enum {name} {enum_type} {{
                {enum_elements}
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

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, _docs: &Docs) {
        let payload_type_name = self.type_name(payload);
        let name = &self.type_name(&Type::Id(id));

        uwrite!(
            self.src,
            "
            public class {0} {{
                private static {0} none = new ();

                private {0}()
                {{
                    HasValue = false;
                }}

                public {0}({1} v)
                {{
                    HasValue = true;
                    Value = v;
                }}

                public static {0} None => none;

                public bool HasValue {{ get; }}

                public {1} Value {{ get; }}
            }}
            ",
            name,
            payload_type_name
        );
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
            public enum {name} {{
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
    body: String,
    results: Vec<String>,
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
    payloads: Vec<String>,
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
            payloads: Vec::new(),
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
                    WasmType::Pointer => "0",
                    WasmType::PointerOrI64 => "0L",
                    WasmType::Length => "0",
                }
                .to_owned()
            })),
            Instruction::I32Load { offset }
            | Instruction::PointerLoad { offset }
            | Instruction::LengthLoad { offset } => match self.gen.direction {
                Direction::Import => results.push(format!("ReturnArea.GetS32(ptr + {offset})")),
                Direction::Export => results.push(format!("returnArea.GetS32({offset})")),
            },
            Instruction::I32Load8U { offset } => match self.gen.direction {
                Direction::Import => results.push(format!("ReturnArea.GetU8(ptr + {offset})")),
                Direction::Export => results.push(format!("returnArea.GetU8({offset})")),
            },
            Instruction::I32Load8S { offset } => {
                results.push(format!("returnArea.GetS8({offset})"))
            }
            Instruction::I32Load16U { offset } => match self.gen.direction {
                Direction::Import => results.push(format!("ReturnArea.GetU16(ptr + {offset})")),
                Direction::Export => results.push(format!("returnArea.GetU16({offset})")),
            },
            Instruction::I32Load16S { offset } => {
                results.push(format!("returnArea.GetS16({offset})"))
            }
            Instruction::I64Load { offset } => results.push(format!("ReturnArea.GetS64({offset})")),
            Instruction::F32Load { offset } => {
                results.push(format!("ReturnArea.GetF32(ptr, {offset})"))
            }
            Instruction::F64Load { offset } => results.push(format!("ReturnArea.GetF64({offset})")),

            Instruction::I32Store { offset }
            | Instruction::PointerStore { offset }
            | Instruction::LengthStore { offset } => {
                uwriteln!(self.src, "returnArea.SetS32({}, {});", offset, operands[0])
            }
            Instruction::I32Store8 { offset } => {
                uwriteln!(self.src, "returnArea.SetS8({}, {});", offset, operands[0])
            }
            Instruction::I32Store16 { offset } => {
                uwriteln!(
                    self.src,
                    "returnArea.SetS16({}, unchecked((short){}));",
                    offset,
                    operands[0]
                )
            }
            Instruction::I64Store { .. } => todo!("I64Store"),
            Instruction::F32Store { offset } => {
                uwriteln!(self.src, "returnArea.SetF32({}, {});", offset, operands[0])
            }
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
            Instruction::Float32FromF32 => {
                results.push(format!("unchecked((float){})", operands[0]))
            }
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
            | Instruction::Float64FromF64 => results.push(operands[0].clone()),

            Instruction::Bitcasts { .. } => todo!("Bitcasts"),

            Instruction::I32FromBool => {
                results.push(format!("({} ? 1 : 0)", operands[0]));
            }
            Instruction::BoolFromI32 => results.push(format!("({} != 0)", operands[0])),

            Instruction::FlagsLower {
                flags,
                name: _,
                ty: _,
            } => {
                if flags.flags.len() > 32 {
                    results.push(format!(
                        "unchecked((int)(((long){}) & uint.MaxValue))",
                        operands[0].to_string()
                    ));
                    results.push(format!(
                        "unchecked(((int)((long){} >> 32)))",
                        operands[0].to_string()
                    ));
                } else {
                    results.push(format!("(int){}", operands[0].to_string()));
                }
            }

            Instruction::FlagsLift { flags, name, ty } => {
                let qualified_type_name = format!(
                    "{}{}",
                    self.gen.qualifier(true, ty),
                    name.to_string().to_upper_camel_case()
                );
                if flags.flags.len() > 32 {
                    results.push(format!(
                        "({})(unchecked((uint)({})) | (ulong)(unchecked((uint)({}))) << 32)",
                        qualified_type_name,
                        operands[0].to_string(),
                        operands[1].to_string()
                    ));
                } else {
                    results.push(format!("({})({})", qualified_type_name, operands[0]))
                }
            }

            Instruction::RecordLower { record, .. } => {
                let op = &operands[0];
                for f in record.fields.iter() {
                    results.push(format!("({}).{}", op, f.name.to_csharp_ident()));
                }
            }
            Instruction::RecordLift { ty, name, .. } => {
                let qualified_type_name = format!(
                    "{}{}",
                    self.gen.qualifier(true, ty),
                    name.to_string().to_upper_camel_case()
                );
                let mut result = format!("new {} (\n", qualified_type_name);

                result.push_str(&operands.join(","));
                result.push_str(")");

                results.push(result);
            }
            Instruction::TupleLift { .. } => {
                let mut result = String::from("(");

                uwriteln!(result, "{}", operands.join(","));

                result.push_str(")");
                results.push(result);
            }

            Instruction::TupleLower { tuple, ty: _ } => {
                let op = &operands[0];
                match tuple.types.len() {
                    1 => results.push(format!("({})", op)),
                    _ => {
                        for i in 0..tuple.types.len() {
                            results.push(format!("({}).Item{}", op, i + 1));
                        }
                    }
                }
            }

            Instruction::VariantPayloadName => {
                let payload = self.locals.tmp("payload");
                results.push(payload.clone());
                self.payloads.push(payload);
            }

            Instruction::VariantLower { .. } => todo!("VariantLift"),

            Instruction::VariantLift { .. } => todo!("VariantLift"),

            Instruction::OptionLower {
                results: lowered_types,
                payload,
                ..
            } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                let some_payload = self.payloads.pop().unwrap();
                let none_payload = self.payloads.pop().unwrap();

                let lowered = lowered_types
                    .iter()
                    .map(|_| self.locals.tmp("lowered"))
                    .collect::<Vec<_>>();

                results.extend(lowered.iter().cloned());

                let declarations = lowered
                    .iter()
                    .zip(lowered_types.iter())
                    .map(|(lowered, ty)| format!("{} {lowered};", wasm_type(*ty)))
                    .collect::<Vec<_>>()
                    .join("\n");

                let op = &operands[0];

                let block = |ty: Option<&Type>, Block { body, results, .. }, payload| {
                    let payload = if let Some(_ty) = self.gen.non_empty_type(ty) {
                        format!("var {payload} = ({op}).Value;")
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
                        "{payload}
                         {body}
                         {assignments}"
                    )
                };

                let none = block(None, none, none_payload);
                let some = block(Some(payload), some, some_payload);

                uwrite!(
                    self.src,
                    r#"
                    {declarations}

                    if (({op}).HasValue) {{
                        {some}
                    }} else {{
                        {none}
                    }}
                    "#
                );
            }

            Instruction::OptionLift { payload, ty } => {
                let some = self.blocks.pop().unwrap();
                let _none = self.blocks.pop().unwrap();

                let ty = self.gen.type_name_with_qualifier(&Type::Id(*ty), true);
                let lifted = self.locals.tmp("lifted");
                let op = &operands[0];

                let payload = if self.gen.non_empty_type(Some(*payload)).is_some() {
                    some.results.into_iter().next().unwrap()
                } else {
                    "null".into()
                };

                let some = some.body;

                uwrite!(
                    self.src,
                    r#"
                    {ty} {lifted};

                    switch ({op}) {{
                        case 0: {{
                            {lifted} = {ty}.None;
                            break;
                        }}

                        case 1: {{
                            {some}
                            {lifted} = new ({payload});
                            break;
                        }}

                        default: throw new Exception("invalid discriminant: " + ({op}));
                    }}
                    "#
                );

                results.push(lifted);
            }

            Instruction::ResultLower { .. } => todo!("ResultLower"),

            Instruction::ResultLift { .. } => todo!("ResultLift"),

            Instruction::EnumLower { .. } => results.push(format!("(int){}", operands[0])),

            Instruction::EnumLift { ty, .. } => {
                let t = self.gen.type_name_with_qualifier(&Type::Id(*ty), true);
                let op = &operands[0];
                results.push(format!("({}){}", t, op));

                uwriteln!(
                    self.src,
                    "Debug.Assert(Enum.IsDefined(typeof({}), {}));",
                    t,
                    op
                );
            }

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

                if realloc.is_none() {
                    results.push(format!("{interop_string}.ToInt32()"));
                } else {
                    results.push(format!("{interop_string}.ToInt32()"));
                }
                results.push(format!("length{result_var}"));

                self.gen.gen.needs_interop_string = true;
            }

            Instruction::StringLift { .. } => match self.gen.direction {
                Direction::Import => results.push(format!("ReturnArea.GetUTF8String(ptr)")),
                Direction::Export => {
                    let address = &operands[0];
                    let length = &operands[1];

                    results.push(format!("returnArea.GetUTF8String({address}, {length})"));
                }
            },

            Instruction::ListLower { .. } => todo!("ListLower"),

            Instruction::ListLift { .. } => todo!("ListLift"),

            Instruction::IterElem { .. } => todo!("IterElem"),

            Instruction::IterBasePointer => todo!("IterBasePointer"),

            Instruction::CallWasm { sig, name } => {
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
                let interface_name = CSharp::get_class_name_from_qualified_name(module).1;

                let class_name_root = (match self.gen.function_level {
                    FunctionLevel::Interface => interface_name
                        .strip_prefix("I")
                        .unwrap()
                        .to_upper_camel_case(),
                    FunctionLevel::FreeStanding => interface_name,
                })
                .to_upper_camel_case();

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
                        .push_str(&format!("{class_name_root}Impl.{func_name}({oper});")),
                    1 => {
                        let ret = self.locals.tmp("ret");
                        uwriteln!(
                            self.src,
                            "var {ret} = {class_name_root}Impl.{func_name}({oper});"
                        );
                        results.push(ret);
                    }
                    _ => {
                        let ret = self.locals.tmp("ret");
                        uwriteln!(
                            self.src,
                            "var {ret} = {class_name_root}Impl.{func_name}({oper});"
                        );
                        let mut i = 1;
                        for _ in func.results.iter_types() {
                            results.push(format!("{}.Item{}", ret, i));
                            i += 1;
                        }
                    }
                }
            }

            Instruction::Return { amt: _, func } => match func.results.len() {
                0 => (),
                1 => uwriteln!(self.src, "return {};", operands[0]),
                _ => {
                    let results = operands.join(", ");
                    uwriteln!(self.src, "return ({results});")
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
        match self.gen.direction {
            Direction::Import => {
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
            }
            Direction::Export => {
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
            body: mem::replace(&mut self.src, body),
            results: mem::take(operands),
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
        WasmType::Pointer => "int",
        WasmType::PointerOrI64 => "long",
        WasmType::Length => "int",
    }
}

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

fn interface_name(
    csharp: &mut CSharp,
    resolve: &Resolve,
    name: &WorldKey,
    direction: Direction,
) -> String {
    let pkg = match name {
        WorldKey::Name(_) => None,
        WorldKey::Interface(id) => {
            let pkg = resolve.interfaces[*id].package.unwrap();
            Some(resolve.packages[pkg].name.clone())
        }
    };

    let name = match name {
        WorldKey::Name(name) => name.to_upper_camel_case(),
        WorldKey::Interface(id) => resolve.interfaces[*id]
            .name
            .as_ref()
            .unwrap()
            .to_upper_camel_case(),
    };

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

    let world_namespae = &csharp.qualifier();

    format!(
        "{}wit.{}.{}I{name}",
        world_namespae,
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
