use crate::csharp_ident::ToCSharpIdent;
use crate::function::ResourceInfo;
use crate::interface::{InterfaceFragment, InterfaceGenerator, InterfaceTypeAndFragments};
use crate::{CSharpRuntime, Opts};
use heck::ToUpperCamelCase;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::ops::Deref;
use std::{iter, mem};
use wit_bindgen_core::{uwrite, Direction, Files, InterfaceGenerator as _, WorldGenerator};
use wit_component::WitPrinter;
use wit_parser::abi::WasmType;
use wit_parser::{
    Function, FunctionKind, InterfaceId, Resolve, SizeAlign, Type, TypeId, TypeOwner, WorldId,
    WorldKey,
};

#[derive(Default)]
pub struct CSharp {
    pub(crate) opts: Opts,
    pub(crate) name: String,
    pub(crate) usings: HashSet<String>,
    #[allow(unused)]
    pub(crate) interop_usings: HashSet<String>,
    pub(crate) return_area_size: usize,
    pub(crate) return_area_align: usize,
    pub(crate) tuple_counts: HashSet<usize>,
    pub(crate) needs_result: bool,
    pub(crate) needs_option: bool,
    pub(crate) needs_interop_string: bool,
    pub(crate) needs_export_return_area: bool,
    pub(crate) needs_rep_table: bool,
    pub(crate) needs_wit_exception: bool,
    pub(crate) interface_fragments: HashMap<String, InterfaceTypeAndFragments>,
    pub(crate) world_fragments: Vec<InterfaceFragment>,
    pub(crate) sizes: SizeAlign,
    pub(crate) interface_names: HashMap<InterfaceId, String>,
    pub(crate) anonymous_type_owners: HashMap<TypeId, TypeOwner>,
    pub(crate) all_resources: HashMap<TypeId, ResourceInfo>,
    pub(crate) world_resources: HashMap<TypeId, ResourceInfo>,
    pub(crate) import_funcs_called: bool,
}

impl CSharp {
    pub(crate) fn access_modifier(&self) -> &'static str {
        if self.opts.internal {
            "internal"
        } else {
            "public"
        }
    }

    pub(crate) fn qualifier(&self) -> String {
        let world = self.name.to_upper_camel_case();
        format!("{world}World.")
    }

    fn interface<'a>(
        &'a mut self,
        resolve: &'a Resolve,
        name: &'a str,
        direction: Direction,
    ) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: String::new(),
            csharp_interop_src: String::new(),
            stub: String::new(),
            csharp_gen: self,
            resolve,
            name,
            direction,
            usings: HashSet::<String>::new(),
            interop_usings: HashSet::<String>::new(),
        }
    }

    // returns the qualifier and last part
    pub(crate) fn get_class_name_from_qualified_name(qualified_type: &str) -> (String, String) {
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

    pub(crate) fn require_using(&mut self, using_ns: &str) {
        if !self.usings.contains(using_ns) {
            let using_ns_string = using_ns.to_string();
            self.usings.insert(using_ns_string);
        }
    }

    #[allow(unused)]
    fn require_interop_using(&mut self, using_ns: &str) {
        if !self.interop_usings.contains(using_ns) {
            let using_ns_string = using_ns.to_string();
            self.interop_usings.insert(using_ns_string);
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
    ) -> anyhow::Result<()> {
        let name = interface_name(self, resolve, key, Direction::Import);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, Direction::Import);

        let mut old_resources = mem::take(&mut gen.csharp_gen.all_resources);
        gen.types(id);
        let new_resources = mem::take(&mut gen.csharp_gen.all_resources);
        old_resources.extend(new_resources.clone());
        gen.csharp_gen.all_resources = old_resources;

        for (resource, funcs) in by_resource(
            resolve.interfaces[id]
                .functions
                .iter()
                .map(|(k, v)| (k.as_str(), v)),
            new_resources.keys().copied(),
        ) {
            if let Some(resource) = resource {
                gen.start_resource(resource, Some(key));
            }

            let import_module_name = &resolve.name_world_key(key);
            for func in funcs {
                gen.import(import_module_name, func);
            }

            if resource.is_some() {
                gen.end_resource();
            }
        }

        // for anonymous types
        gen.define_interface_types(id);

        gen.add_interface_fragment(false);

        Ok(())
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        self.import_funcs_called = true;

        let name = &format!("{}-world", resolve.worlds[world].name).to_upper_camel_case();
        let name = &format!("{name}.I{name}");
        let mut gen = self.interface(resolve, name, Direction::Import);

        for (resource, funcs) in by_resource(
            funcs.iter().copied(),
            gen.csharp_gen.world_resources.keys().copied(),
        ) {
            if let Some(resource) = resource {
                gen.start_resource(resource, None);
            }

            for func in funcs {
                gen.import("$root", func);
            }

            if resource.is_some() {
                gen.end_resource();
            }
        }

        gen.add_world_fragment();
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let name = interface_name(self, resolve, key, Direction::Export);
        self.interface_names.insert(id, name.clone());
        let mut gen = self.interface(resolve, &name, Direction::Export);

        let mut old_resources = mem::take(&mut gen.csharp_gen.all_resources);
        gen.types(id);
        let new_resources = mem::take(&mut gen.csharp_gen.all_resources);
        old_resources.extend(new_resources.clone());
        gen.csharp_gen.all_resources = old_resources;

        for (resource, funcs) in by_resource(
            resolve.interfaces[id]
                .functions
                .iter()
                .map(|(k, v)| (k.as_str(), v)),
            new_resources.keys().copied(),
        ) {
            if let Some(resource) = resource {
                gen.start_resource(resource, Some(key));
            }

            for func in funcs {
                gen.export(func, Some(key));
            }

            if resource.is_some() {
                gen.end_resource();
            }
        }

        // for anonymous types
        gen.define_interface_types(id);

        gen.add_interface_fragment(true);
        Ok(())
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) -> anyhow::Result<()> {
        let name = &format!("{}-world", resolve.worlds[world].name).to_upper_camel_case();
        let name = &format!("{name}.I{name}");
        let mut gen = self.interface(resolve, name, Direction::Export);

        for (resource, funcs) in by_resource(funcs.iter().copied(), iter::empty()) {
            if let Some(resource) = resource {
                gen.start_resource(resource, None);
            }

            for func in funcs {
                gen.export(func, None);
            }

            if resource.is_some() {
                gen.end_resource();
            }
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
        let name = &format!("{}-world", resolve.worlds[world].name).to_upper_camel_case();
        let name = &format!("{name}.I{name}");
        let mut gen = self.interface(resolve, name, Direction::Import);

        let mut old_resources = mem::take(&mut gen.csharp_gen.all_resources);
        for (ty_name, ty) in types {
            gen.define_type(ty_name, *ty);
        }
        let new_resources = mem::take(&mut gen.csharp_gen.all_resources);
        old_resources.extend(new_resources.clone());
        gen.csharp_gen.all_resources = old_resources;
        gen.csharp_gen.world_resources = new_resources;

        gen.add_world_fragment();
    }

    fn finish(&mut self, resolve: &Resolve, id: WorldId, files: &mut Files) -> anyhow::Result<()> {
        if !self.import_funcs_called {
            // Ensure that we emit type declarations for any top-level imported resource types:
            self.import_funcs(resolve, id, &[], files);
        }

        let world = &resolve.worlds[id];
        let world_namespace = self.qualifier();
        let world_namespace = world_namespace.strip_suffix(".").unwrap();
        let namespace = format!("{world_namespace}");
        let name = world.name.to_upper_camel_case();

        let version = env!("CARGO_PKG_VERSION");
        let header = format!(
            "\
            // Generated by `wit-bindgen` {version}. DO NOT EDIT!
            // <auto-generated />
            #nullable enable
            "
        );
        let mut src = String::new();
        src.push_str(&header);

        let access = self.access_modifier();

        let using_pos = src.len();

        uwrite!(
            src,
            "
             namespace {world_namespace} {{

             {access} interface I{name}World {{
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

        let usings: Vec<_> = self
            .world_fragments
            .iter()
            .flat_map(|f| &f.usings)
            .cloned()
            .collect();
        usings.iter().for_each(|u| {
            self.require_using(u);
        });

        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        src.push_str("}\n");

        if self.needs_result {
            self.require_using("System.Runtime.InteropServices");
            uwrite!(
                src,
                r#"

                {access} readonly struct None {{}}

                [StructLayout(LayoutKind.Sequential)]
                {access} readonly struct Result<TOk, TErr>
                {{
                    {access} readonly byte Tag;
                    private readonly object value;

                    private Result(byte tag, object value)
                    {{
                        Tag = tag;
                        this.value = value;
                    }}

                    {access} static Result<TOk, TErr> Ok(TOk ok)
                    {{
                        return new Result<TOk, TErr>(Tags.Ok, ok!);
                    }}

                    {access} static Result<TOk, TErr> Err(TErr err)
                    {{
                        return new Result<TOk, TErr>(Tags.Err, err!);
                    }}

                    {access} bool IsOk => Tag == Tags.Ok;
                    {access} bool IsErr => Tag == Tags.Err;

                    {access} TOk AsOk
                    {{
                        get
                        {{
                            if (Tag == Tags.Ok)
                            {{
                                return (TOk)value;
                            }}

                            throw new ArgumentException("expected k, got " + Tag);
                        }}
                    }}

                    {access} TErr AsErr
                    {{
                        get
                        {{
                            if (Tag == Tags.Err)
                            {{
                                return (TErr)value;
                            }}

                            throw new ArgumentException("expected Err, got " + Tag);
                        }}
                    }}

                    {access} class Tags
                    {{
                        {access} const byte Ok = 0;
                        {access} const byte Err = 1;
                    }}
                }}
                "#,
            )
        }

        if self.needs_option {
            self.require_using("System.Diagnostics.CodeAnalysis");
            uwrite!(
                src,
                r#"

                {access} class Option<T> {{
                    private static Option<T> none = new ();

                    private Option()
                    {{
                        HasValue = false;
                    }}

                    {access} Option(T v)
                    {{
                        HasValue = true;
                        Value = v;
                    }}

                    {access} static Option<T> None => none;

                    [MemberNotNullWhen(true, nameof(Value))]
                    {access} bool HasValue {{ get; }}

                    {access} T? Value {{ get; }}
                }}
                "#,
            )
        }

        if self.needs_interop_string {
            self.require_using("System.Text");
            self.require_using("System.Runtime.InteropServices");
            uwrite!(
                src,
                r#"
                {access} static class InteropString
                {{
                    internal static IntPtr FromString(string input, out int length)
                    {{
                        var utf8Bytes = Encoding.UTF8.GetBytes(input);
                        length = utf8Bytes.Length;
                        var gcHandle = GCHandle.Alloc(utf8Bytes, GCHandleType.Pinned);
                        return gcHandle.AddrOfPinnedObject();
                    }}
                }}
                "#,
            )
        }

        if self.needs_wit_exception {
            uwrite!(
                src,
                r#"
                {access} class WitException: Exception {{
                    {access} object Value {{ get; }}
                    {access} uint NestingLevel {{ get; }}

                    {access} WitException(object v, uint level)
                    {{
                        Value = v;
                        NestingLevel = level;
                    }}
                }}

                {access} class WitException<T>: WitException {{
                    {access} T TypedValue {{ get {{ return (T)this.Value;}} }}

                    {access} WitException(T v, uint level) : base(v!, level)
                    {{
                    }}
                }}
                "#,
            )
        }

        // Declare a statically-allocated return area, if needed. We only do
        // this for export bindings, because import bindings allocate their
        // return-area on the stack.
        if self.needs_export_return_area {
            let mut ret_area_str = String::new();

            let (array_size, element_type) =
                dotnet_aligned_array(self.return_area_size, self.return_area_align);

            self.require_using("System.Runtime.CompilerServices");
            uwrite!(
                ret_area_str,
                "
                {access} static class InteropReturnArea
                {{
                    [InlineArray({0})]
                    [StructLayout(LayoutKind.Sequential, Pack = {1})]
                    internal struct ReturnArea
                    {{
                        private {2} buffer;

                        internal unsafe nint AddressOfReturnArea()
                        {{
                            return (nint)Unsafe.AsPointer(ref buffer);
                        }}
                    }}

                    [ThreadStatic]
                    [FixedAddressValueType]
                    internal static ReturnArea returnArea = default;
                }}
                ",
                array_size,
                self.return_area_align,
                element_type
            );

            src.push_str(&ret_area_str);
        }

        if self.needs_rep_table {
            src.push_str("\n");
            src.push_str(include_str!("RepTable.cs"));
        }

        if !&self.world_fragments.is_empty() {
            src.push_str("\n");

            src.push_str("namespace exports {\n");

            src.push_str(
                &self
                    .world_fragments
                    .iter()
                    .flat_map(|f| &f.interop_usings)
                    .into_iter()
                    .collect::<HashSet<&String>>() // de-dup across world_fragments
                    .iter()
                    .map(|s| "using ".to_owned() + s + ";")
                    .collect::<Vec<String>>()
                    .join("\n"),
            );
            src.push_str("\n");

            src.push_str(&format!("{access} static class {name}World\n"));
            src.push_str("{");

            for fragment in &self.world_fragments {
                src.push_str("\n");

                src.push_str(&fragment.csharp_interop_src);
            }
            src.push_str("}\n");
            src.push_str("}\n");
        }

        src.push_str("\n");

        src.push_str("}\n");

        src.insert_str(
            using_pos,
            &self
                .usings
                .iter()
                .map(|s| "using ".to_owned() + s + ";")
                .collect::<Vec<String>>()
                .join("\n"),
        );

        files.push(&format!("{name}.cs"), indent(&src).as_bytes());

        let generate_stub = |name: String, files: &mut Files, stubs: Stubs| {
            let (stub_namespace, interface_or_class_name) =
                CSharp::get_class_name_from_qualified_name(&name);

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

            if fragments.iter().all(|f| f.stub.is_empty()) {
                return;
            }

            let body = fragments
                .iter()
                .map(|f| f.stub.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let body = format!(
                "{header}
                 namespace {fully_qualified_namespace};

                 {access} partial class {stub_class_name} : {interface_or_class_name} {{
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

        if !self.opts.skip_support_files {
            //TODO: This is currently needed for mono even if it's built as a library.
            if self.opts.runtime == CSharpRuntime::Mono {
                files.push(
                    &format!("MonoEntrypoint.cs",),
                    indent(&format!(
                        r#"
                        {access} class MonoEntrypoint() {{
                            {access} static void Main() {{
                            }}
                        }}
                        "#
                    ))
                    .as_bytes(),
                );
            }

            // For the time being, we generate both a .wit file and a .o file to
            // represent the component type.  Newer releases of the .NET runtime
            // will be able to use the former, but older ones will need the
            // latter.
            //
            // TODO: stop generating the .o file once a new-enough release is
            // available for us to test using only the .wit file.

            {
                // When generating a WIT file, we first round-trip through the
                // binary encoding.  This has the effect of flattening any
                // `include`d worlds into the specified world and excluding
                // unrelated worlds, ensuring the output WIT contains no extra
                // information beyond what the binary representation contains.
                //
                // This is important because including more than one world in
                // the output would make it ambigious, and since this file is
                // intended to be used non-interactively at link time, the
                // linker will have no additional information to resolve such
                // ambiguity.
                let (resolve, world) =
                    wit_parser::decoding::decode_world(&wit_component::metadata::encode(
                        &resolve,
                        id,
                        self.opts.string_encoding,
                        None,
                    )?)?;
                let pkg = resolve.worlds[world].package.unwrap();

                files.push(
                    &format!("{world_namespace}_component_type.wit"),
                    WitPrinter::default()
                        .emit_docs(false)
                        .print(
                            &resolve,
                            pkg,
                            &resolve
                                .packages
                                .iter()
                                .filter_map(|(id, _)| if id == pkg { None } else { Some(id) })
                                .collect::<Vec<_>>(),
                        )?
                        .as_bytes(),
                );
            }

            // TODO: remove when we switch to dotnet 9
            let mut wasm_import_linakge_src = String::new();

            uwrite!(
                wasm_import_linakge_src,
                r#"{header}
                #if !NET9_0_OR_GREATER
                // temporarily add this attribute until it is available in dotnet 9
                namespace System.Runtime.InteropServices
                {{
                    internal partial class WasmImportLinkageAttribute : Attribute {{}}
                }}
                #endif
                "#,
            );
            files.push(
                &format!("{world_namespace}_wasm_import_linkage_attribute.cs"),
                indent(&wasm_import_linakge_src).as_bytes(),
            );
        }

        for (full_name, interface_type_and_fragments) in &self.interface_fragments {
            let fragments = &interface_type_and_fragments.interface_fragments;

            let (namespace, interface_name) =
                &CSharp::get_class_name_from_qualified_name(full_name);

            // C#
            let body = fragments
                .iter()
                .map(|f| f.csharp_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            if body.len() > 0 {
                let body = format!(
                    "{header}
                    {0}

                    namespace {namespace};

                    {access} interface {interface_name} {{
                        {body}
                    }}
                    ",
                    fragments
                        .iter()
                        .flat_map(|f| &f.usings)
                        .map(|s| "using ".to_owned() + s + ";")
                        .collect::<Vec<String>>()
                        .join("\n"),
                );

                files.push(&format!("{full_name}.cs"), indent(&body).as_bytes());
            }

            // C# Interop
            let body = fragments
                .iter()
                .map(|f| f.csharp_interop_src.deref())
                .collect::<Vec<_>>()
                .join("\n");

            let class_name = interface_name.strip_prefix("I").unwrap();
            let body = format!(
                "{header}
                 {0}

                namespace {namespace}
                {{
                  {access} static class {class_name}Interop {{
                      {body}
                  }}
                }}
                ",
                fragments
                    .iter()
                    .flat_map(|f| &f.interop_usings)
                    .map(|s| "using ".to_owned() + s + ";\n")
                    .collect::<Vec<String>>()
                    .join(""),
            );

            files.push(
                &format!("{namespace}.{class_name}Interop.cs"),
                indent(&body).as_bytes(),
            );

            if interface_type_and_fragments.is_export && self.opts.generate_stub {
                generate_stub(full_name.to_string(), files, Stubs::Interface(fragments));
            }
        }

        Ok(())
    }
}

enum Stubs<'a> {
    World(&'a Vec<InterfaceFragment>),
    Interface(&'a Vec<InterfaceFragment>),
}

// We cant use "StructLayout.Pack" as dotnet will use the minimum of the type and the "Pack" field,
// so for byte it would always use 1 regardless of the "Pack".
pub fn dotnet_aligned_array(array_size: usize, required_alignment: usize) -> (usize, String) {
    match required_alignment {
        1 => {
            return (array_size, "byte".to_owned());
        }
        2 => {
            return ((array_size + 1) / 2, "ushort".to_owned());
        }
        4 => {
            return ((array_size + 3) / 4, "uint".to_owned());
        }
        8 => {
            return ((array_size + 7) / 8, "ulong".to_owned());
        }
        _ => todo!("unsupported return_area_align {}", required_alignment),
    }
}

pub fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "int",
        WasmType::I64 => "long",
        WasmType::F32 => "float",
        WasmType::F64 => "double",
        WasmType::Pointer => "nint",
        WasmType::PointerOrI64 => "long",
        WasmType::Length => "int",
    }
}

fn indent(code: &str) -> String {
    let mut indented = String::with_capacity(code.len());
    let mut indent = 0;
    let mut was_empty = false;
    for line in code.trim().lines() {
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
        if !trimmed.is_empty() {
            indented.extend(iter::repeat(' ').take(indent * 4));
            indented.push_str(trimmed);
        }
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

    let world_namespace = &csharp.qualifier();

    format!(
        "{}wit.{}.{}I{name}",
        world_namespace,
        match direction {
            Direction::Import => "imports",
            Direction::Export => "exports",
        },
        namespace
    )
}

pub fn is_primitive(ty: &Type) -> bool {
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
            | Type::F32
            | Type::F64
    )
}

/// Group the specified functions by resource (or `None` for freestanding functions).
///
/// The returned map is constructed by iterating over `funcs`, then iterating over `all_resources`, thereby
/// ensuring that even resources with no associated functions will be represented in the result.
fn by_resource<'a>(
    funcs: impl Iterator<Item = (&'a str, &'a Function)>,
    all_resources: impl Iterator<Item = TypeId>,
) -> IndexMap<Option<TypeId>, Vec<&'a Function>> {
    let mut by_resource = IndexMap::<_, Vec<_>>::new();
    for (_, func) in funcs {
        by_resource
            .entry(match &func.kind {
                FunctionKind::Freestanding => None,
                FunctionKind::Method(resource)
                | FunctionKind::Static(resource)
                | FunctionKind::Constructor(resource) => Some(*resource),
            })
            .or_default()
            .push(func);
    }
    for id in all_resources {
        by_resource.entry(Some(id)).or_default();
    }
    by_resource
}
