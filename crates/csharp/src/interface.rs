use crate::csharp_ident::ToCSharpIdent;
use crate::function::FunctionBindgen;
use crate::function::ResourceInfo;
use crate::world_generator::CSharp;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Deref;
use wit_bindgen_core::abi::LiftLower;
use wit_bindgen_core::{
    abi, uwrite, uwriteln, Direction, InterfaceGenerator as CoreInterfaceGenerator,
};
use wit_parser::abi::AbiVariant;
use wit_parser::{
    Docs, Enum, Flags, FlagsRepr, Function, FunctionKind, Handle, Int, InterfaceId, LiveTypes,
    Record, Resolve, Result_, Tuple, Type, TypeDefKind, TypeId, TypeOwner, Variant, WorldKey,
};

pub(crate) struct InterfaceFragment {
    pub(crate) csharp_src: String,
    pub(crate) csharp_interop_src: String,
    pub(crate) stub: String,
}

pub(crate) struct InterfaceTypeAndFragments {
    pub(crate) is_export: bool,
    pub(crate) interface_fragments: Vec<InterfaceFragment>,
}

impl InterfaceTypeAndFragments {
    pub(crate) fn new(is_export: bool) -> Self {
        InterfaceTypeAndFragments {
            is_export,
            interface_fragments: Vec::<InterfaceFragment>::new(),
        }
    }
}

/// InterfaceGenerator generates the C# code for wit interfaces.
/// It produces types by interface in wit and then generates the interop code
/// by calling out to FunctionGenerator
pub(crate) struct InterfaceGenerator<'a> {
    pub(crate) src: String,
    pub(crate) csharp_interop_src: String,
    pub(crate) stub: String,
    pub(crate) csharp_gen: &'a mut CSharp,
    pub(crate) resolve: &'a Resolve,
    pub(crate) name: &'a str,
    pub(crate) direction: Direction,
    pub(crate) futures: Vec<String>,
}

impl InterfaceGenerator<'_> {
    pub fn is_async(kind: &FunctionKind) -> bool {
        matches!(
            kind,
            FunctionKind::AsyncFreestanding
                | FunctionKind::AsyncStatic(_)
                | FunctionKind::AsyncMethod(_)
        )
    }

    pub(crate) fn define_interface_types(&mut self, id: InterfaceId) {
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
            self.csharp_gen
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
            TypeDefKind::List(t) => self.type_list(type_id, typedef_name, t, &type_def.docs),
            TypeDefKind::Variant(t) => self.type_variant(type_id, typedef_name, t, &type_def.docs),
            TypeDefKind::Result(t) => self.type_result(type_id, typedef_name, t, &type_def.docs),
            TypeDefKind::Handle(_) => {
                // Handles don't require a separate definition beyond what we already define for the corresponding
                // resource types.
            }
            TypeDefKind::Future(t) => self.type_future(type_id, typedef_name, t, &type_def.docs),
            _ => unreachable!(),
        }
    }

    pub(crate) fn qualifier(&self, when: bool, ty: &TypeId) -> String {
        // anonymous types dont get an owner from wit-parser, so assume they are part of an interface here.
        let owner = if let Some(owner_type) = self.csharp_gen.anonymous_type_owners.get(ty) {
            *owner_type
        } else {
            let type_def = &self.resolve.types[*ty];
            type_def.owner
        };

        let global_prefix = self.global_if_user_type(&Type::Id(*ty));

        if let TypeOwner::Interface(id) = owner {
            if let Some(name) = self.csharp_gen.interface_names.get(&id) {
                if name != self.name {
                    return format!("{global_prefix}{name}.");
                }
            }
        }

        if when {
            let name = self.name;
            format!("{global_prefix}{name}.")
        } else {
            String::new()
        }
    }

    pub(crate) fn add_interface_fragment(self, is_export: bool) {
        self.csharp_gen
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

    pub(crate) fn add_futures(&mut self, import_module_name: &str) {
        if self.futures.is_empty() {
            return;
        }

        let (_namespace, interface_name) =
            &CSharp::get_class_name_from_qualified_name(self.name);
        let interop_name = format!("{}Interop", interface_name.strip_prefix("I").unwrap());

        for future in &self.futures {
            let camel_name = future.to_upper_camel_case();

            //TODO: we need these per future type.
            // Create a hash map..
            uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-new-0][async]{future}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern ulong {camel_name}VoidNew();
                "#
            );

            // TODO: Move this and other type dependent functions out to another function.
            uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-cancel-read-0][async]{future}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern uint {camel_name}CancelRead(int readable);
                "#
            );

            uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-cancel-write-0][async]{future}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern uint {camel_name}CancelWrite(int writeable);
                "#
            );

            uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-drop-writeable-0][async]{future}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern void {camel_name}DropWriteable(int writeable);
                "#
            );

            self.csharp_gen
                .interface_fragments
                .entry(self.name.to_string())
                .or_insert_with(|| InterfaceTypeAndFragments::new(false))
                .interface_fragments
                .push(InterfaceFragment {
                    csharp_src: format!(r#"
                    public static (FutureReader, FutureWriter) {camel_name}VoidNew() 
                    {{
                         var packed = {interop_name}.{camel_name}VoidNew();
                         var readerHandle = (int)(packed & 0xFFFFFFFF);
                         var writerHandle = (int)(packed >> 32);

                         return (new FutureReaderVoid(readerHandle), new FutureWriterVoid(writerHandle));
                    }}
                    "#).to_string(),
                    csharp_interop_src: "".to_string(),
                    stub: "".to_string(),
                });
            
            self.csharp_gen.needs_future_reader_support = true;
            self.csharp_gen.needs_future_writer_support = true;
        }

        uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-new]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern int WaitableSetNew();
                "#
        );

        uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-join]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern void WaitableJoin(int waitable, int set);
                "#
        );

        uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-wait]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static unsafe extern int WaitableSetWait(int waitable, int* waitableHandlePtr);
                "#
        );

        uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-drop]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static unsafe extern void WaitableSetDrop(int waitable);
                "#
        );


        // TODO: for each type:
        let future_type_name = "Void";

        self.csharp_gen
            .interface_fragments
            .entry(self.name.to_string())
            .or_insert_with(|| InterfaceTypeAndFragments::new(false))
            .interface_fragments
            .push(InterfaceFragment {
                csharp_src: format!(r#"
                    public class FutureReader{future_type_name} : FutureReader
                    {{
                        public FutureReader{future_type_name}(int handle) : base(handle) {{ }}

                        protected override int ReadInternal()
                        {{
                            return {interop_name}.FutureRead{future_type_name}(Handle, IntPtr.Zero);
                        }}

                        protected override void Drop()
                        {{
                            {interop_name}.FutureDropReader{future_type_name}(Handle);
                        }}
                    }}

                    public class FutureWriter{future_type_name} : FutureWriter
                    {{
                        public FutureWriter{future_type_name}(int handle) : base(handle) {{ }}

                        protected override int Write(int handle, IntPtr buffer)
                        {{
                            return {interop_name}.FutureWrite{future_type_name}(handle, buffer);
                        }}

                        protected override void Drop()
                        {{
                            {interop_name}.FutureDropWriter{future_type_name}(Handle);
                        }}
                    }}  
                    "#).to_string(),
                csharp_interop_src: format!(r#"
                    [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[async-lower][future-read-0][async]read-future"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                    internal static unsafe extern int FutureRead{future_type_name}(int readable, IntPtr ptr);

                    [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[async-lower][future-write-0][async]read-future"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                    internal static unsafe extern int FutureWrite{future_type_name}(int writeable, IntPtr buffer);
            
                    [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-drop-readable-0][async]read-future"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                    internal static extern void FutureDropReader{future_type_name}(int readable);

                    [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "[future-drop-writable-0][async]read-future"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                    internal static extern void FutureDropWriter{future_type_name}(int readable);
                "#).to_string(),
                stub: "".to_string(),
            });

        self.csharp_gen
            .interface_fragments
            .entry(self.name.to_string())
            .or_insert_with(|| InterfaceTypeAndFragments::new(false))
            .interface_fragments
            .push(InterfaceFragment {
                csharp_src: format!(r#"
                public static WaitableSet WaitableSetNew() 
                {{
                        var waitable = {interop_name}.WaitableSetNew();
                        return new WaitableSet(waitable);
                }}

                public static void Join(FutureWriter writer, WaitableSet set) 
                {{
                        {interop_name}.WaitableJoin(writer.Handle, set.Handle);
                }}

                public unsafe static EventWaitable WaitableSetWait(WaitableSet set) 
                {{
                    int* buffer = stackalloc int[2];
                    var eventCode = (EventCode){interop_name}.WaitableSetWait(set.Handle, buffer);
                    var res = new EventWaitable(eventCode, buffer[1]);
                    res.Waitable = buffer[0];
                    return res;
                }}

                "#).to_string(),
                csharp_interop_src: "".to_string(),
                stub: "".to_string(),
        });
    }

    pub(crate) fn add_world_fragment(self) {
        self.csharp_gen.world_fragments.push(InterfaceFragment {
            csharp_src: self.src,
            csharp_interop_src: self.csharp_interop_src,
            stub: self.stub,
        });
    }

    pub(crate) fn import(&mut self, import_module_name: &str, func: &Function) {
        let camel_name = match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Static(_)
            | FunctionKind::AsyncFreestanding
            | FunctionKind::AsyncStatic(_) => func.item_name().to_upper_camel_case(),
            FunctionKind::Method(_) | FunctionKind::AsyncMethod(_) => {
                func.item_name().to_upper_camel_case()
            }
            FunctionKind::Constructor(id) => {
                self.csharp_gen.all_resources[id].name.to_upper_camel_case()
            }
        };

        let access = self.csharp_gen.access_modifier();

        let modifiers = modifiers(func, &camel_name, Direction::Import);

        let interop_camel_name = func.item_name().to_upper_camel_case();

        let sig = self.resolve.wasm_signature(AbiVariant::GuestImport, func);

        let is_async = matches!(
            func.kind,
            FunctionKind::AsyncFreestanding
                | FunctionKind::AsyncStatic(_)
                | FunctionKind::AsyncMethod(_)
        );

        let mut wasm_result_type = match &sig.results[..] {
            [] => {
                if is_async {
                    "uint"
                } else {
                    "void"
                }
            }
            [result] => crate::world_generator::wasm_type(*result),
            _ => unreachable!(),
        };

        let (result_type, results) = self.func_payload_and_return_type(func);

        let is_async = InterfaceGenerator::is_async(&func.kind);

        let requires_async_return_buffer_param = is_async && !sig.results.is_empty();
        let sig_unsafe = if requires_async_return_buffer_param {
            "unsafe "
        } else {
            ""
        };

        let wasm_params: String = {
            let param_list = sig
                .params
                .iter()
                .enumerate()
                .map(|(i, param)| {
                    let ty = crate::world_generator::wasm_type(*param);
                    format!("{ty} p{i}")
                })
                .collect::<Vec<_>>()
                .join(", ");

            if requires_async_return_buffer_param {
                if param_list.is_empty() {
                    "void *taskResultBuffer".to_string()
                } else {
                    format!("{}, void *taskResultBuffer", param_list)
                }
            } else {
                param_list
            }
        };

        let mut funcs: Vec<(String, String)> = Vec::new();
        funcs.push(self.gen_import_src(func, &results, ParameterType::ABI));

        let include_additional_functions = func
            .params
            .iter()
            .skip(if let FunctionKind::Method(_) = &func.kind {
                1
            } else {
                0
            })
            .any(|param| self.is_primative_list(&param.1));

        if include_additional_functions {
            funcs.push(self.gen_import_src(func, &results, ParameterType::Span));
            funcs.push(self.gen_import_src(func, &results, ParameterType::Memory));
        }

        let import_name = if is_async {
            wasm_result_type = "uint";
            format!("[async-lower]{}", func.name)
        } else {
            func.name.to_string()
        };

        uwrite!(
            self.csharp_interop_src,
            r#"
            internal static class {interop_camel_name}WasmInterop
            {{
                [global::System.Runtime.InteropServices.DllImportAttribute("{import_module_name}", EntryPoint = "{import_name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                internal static extern {sig_unsafe}{wasm_result_type} wasmImport{interop_camel_name}({wasm_params});
            }}
            "#
        );

        for (src, params) in funcs {
            uwrite!(
                self.src,
                r#"
                    {access} {modifiers} unsafe {result_type} {camel_name}({params})
                    {{
                        {src}
                    }}
                "#
            );
        }
    }

    fn func_payload_and_return_type(&mut self, func: &Function) -> (String, Vec<TypeId>) {
        let return_type = self.func_return_type(func);
        let results = if let FunctionKind::Constructor(_) = &func.kind {
            Vec::new()
        } else {
            let payload = match func.result {
                None => Vec::new(),
                Some(ty) => {
                    let (_payload, results) = payload_and_results(
                        self.resolve,
                        ty,
                        self.csharp_gen.opts.with_wit_results,
                    );
                    results
                }
            };

            payload
        };

        (return_type, results)
    }

    fn func_return_type(&mut self, func: &Function) -> String {
        let result_type = if let FunctionKind::Constructor(_) = &func.kind {
            String::new()
        } else {
            let base_type = match func.result {
                None => "void".to_string(),
                Some(_ty) => {
                    let result_type = match func.result {
                        None => "void".to_string(),
                        Some(ty) => {
                            let (payload, _results) = payload_and_results(
                                self.resolve,
                                ty,
                                self.csharp_gen.opts.with_wit_results,
                            );
                            if let Some(ty) = payload {
                                self.csharp_gen.needs_result = true;
                                self.type_name_with_qualifier(&ty, true)
                            } else {
                                "void".to_string()
                            }
                        }
                    };

                    result_type
                }
            };

            let asyncified_type = match &func.kind {
                FunctionKind::AsyncFreestanding
                | FunctionKind::AsyncStatic(_)
                | FunctionKind::AsyncMethod(_) => match func.result {
                    None => "Task".to_string(),
                    Some(_ty) => format!("Task<{}>", base_type),
                },
                _ => base_type,
            };

            asyncified_type
        };

        result_type
    }

    fn gen_import_src(
        &mut self,
        func: &Function,
        results: &Vec<TypeId>,
        parameter_type: ParameterType,
    ) -> (String, String) {
        let mut bindgen = FunctionBindgen::new(
            self,
            &func.item_name(),
            &func.kind,
            func.params
                .iter()
                .enumerate()
                .map(|(i, (name, _))| {
                    if i == 0 && matches!(&func.kind, FunctionKind::Method(_)) {
                        "this".to_owned()
                    } else {
                        name.to_csharp_ident()
                    }
                })
                .collect(),
            results.clone(),
            parameter_type,
            func.result,
        );

        abi::call(
            bindgen.interface_gen.resolve,
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut bindgen,
            false,
        );

        let src = bindgen.src;

        let params = func
            .params
            .iter()
            .skip(if let FunctionKind::Method(_) = &func.kind {
                1
            } else {
                0
            })
            .map(|param| {
                let ty = self.name_with_qualifier(&param.1, true, parameter_type);
                let param_name = &param.0;
                let param_name = param_name.to_csharp_ident();
                format!("{ty} {param_name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        (src, params)
    }

    pub(crate) fn export(&mut self, func: &Function, interface_name: Option<&WorldKey>) {
        let camel_name = match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::Static(_)
            | FunctionKind::AsyncFreestanding
            | FunctionKind::AsyncStatic(_) => func.item_name().to_upper_camel_case(),
            FunctionKind::Method(_) | FunctionKind::AsyncMethod(_) => {
                func.item_name().to_upper_camel_case()
            }
            FunctionKind::Constructor(id) => {
                self.csharp_gen.all_resources[id].name.to_upper_camel_case()
            }
        };

        let modifiers = modifiers(func, &camel_name, Direction::Export);

        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);

        let (result_type, results) = self.func_payload_and_return_type(func);

        let mut bindgen = FunctionBindgen::new(
            self,
            &func.item_name(),
            &func.kind,
            (0..sig.params.len()).map(|i| format!("p{i}")).collect(),
            results,
            ParameterType::ABI,
            func.result,
        );

        let async_ = matches!(
            func.kind,
            FunctionKind::AsyncFreestanding
                | FunctionKind::AsyncStatic(_)
                | FunctionKind::AsyncMethod(_)
        );

        abi::call(
            bindgen.interface_gen.resolve,
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut bindgen,
            async_,
        );

        let src = bindgen.src;

        let vars = bindgen
            .resource_drops
            .iter()
            .map(|(t, v)| format!("{t}? {v} = null;"))
            .collect::<Vec<_>>()
            .join(";\n");

        let wasm_result_type = if async_ {
            "uint"
        } else {
            match &sig.results[..] {
                [] => "void",
                [result] => crate::world_generator::wasm_type(*result),
                _ => unreachable!(),
            }
        };

        let wasm_params = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, param)| {
                let ty = crate::world_generator::wasm_type(*param);
                format!("{ty} p{i}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let params = func
            .params
            .iter()
            .skip(if let FunctionKind::Method(_) = &func.kind {
                1
            } else {
                0
            })
            .map(|(name, ty)| {
                let ty = self.type_name(ty);
                let name = name.to_csharp_ident();
                format!("{ty} {name}")
            })
            .collect::<Vec<String>>()
            .join(", ");

        let wasm_func_name = func.name.clone();
        let interop_name = format!("wasmExport{}", wasm_func_name.to_upper_camel_case());
        let core_module_name = interface_name.map(|s| self.resolve.name_world_key(s));
        let export_name = func.legacy_core_export_name(core_module_name.as_deref());

        let export_name = if async_ {
            format!("[async-lift]{export_name}")
        } else {
            export_name.to_string()
        };

        let access = self.csharp_gen.access_modifier();

        uwrite!(
            self.csharp_interop_src,
            r#"
            [global::System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute(EntryPoint = "{export_name}")]
            {access} static unsafe {wasm_result_type} {interop_name}({wasm_params}) {{
                {vars}
                {src}
            }}
            "#
        );

        if abi::guest_export_needs_post_return(self.resolve, func) {
            let params = sig
                .results
                .iter()
                .enumerate()
                .map(|(i, param)| {
                    let ty = crate::world_generator::wasm_type(*param);
                    format!("{ty} p{i}")
                })
                .collect::<Vec<_>>()
                .join(", ");

            let mut bindgen = FunctionBindgen::new(
                self,
                "INVALID",
                &func.kind,
                (0..sig.results.len()).map(|i| format!("p{i}")).collect(),
                Vec::new(),
                ParameterType::ABI,
                func.result,
            );

            abi::post_return(bindgen.interface_gen.resolve, func, &mut bindgen);

            let src = bindgen.src;

            uwrite!(
                self.csharp_interop_src,
                r#"
                [global::System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute(EntryPoint = "cabi_post_{export_name}")]
                {access} static unsafe void cabi_post_{interop_name}({params}) {{
                    {src}
                }}
                "#
            );
        }

        if async_ {
            let import_module_name = &self.resolve.name_world_key(interface_name.unwrap());
            let (_namespace, interface_name) =
                &CSharp::get_class_name_from_qualified_name(self.name);
            let impl_name = format!("{}Impl", interface_name.strip_prefix("I").unwrap());

            uwriteln!(
                self.csharp_interop_src,
                r#"
            [global::System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute(EntryPoint = "[callback][async-lift]{import_module_name}#{wasm_func_name}")]
            public static int {camel_name}Callback(uint eventRaw, uint waitable, uint code)
            {{
                // TODO: decode the parameters
                return {impl_name}.{camel_name}Callback();
            }}
            "#);

            uwriteln!(
                self.src,
                r#"
            public static abstract int {camel_name}Callback();
            "#);

            // TODO: The task return function can take up to 16 core parameters.
            let task_return_param = match &sig.results[..] {
                [] => "",
                [_result] => &format!("{} result", wasm_result_type),
                _ => unreachable!(),
            };

            uwriteln!(
                self.csharp_interop_src,
                r#"
            // TODO: The task return function can take up to 16 core parameters.
            [global::System.Runtime.InteropServices.DllImportAttribute("[export]{import_module_name}", EntryPoint = "[task-return]{wasm_func_name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
            internal static extern void {camel_name}TaskReturn({task_return_param});
            "#
            );
        }

        if !matches!(&func.kind, FunctionKind::Constructor(_)) {
            uwrite!(
                self.src,
                r#"{modifiers} {result_type} {camel_name}({params});

            "#
            );
        }

        if self.csharp_gen.opts.generate_stub {
            let sig = self.sig_string(func, true);

            uwrite!(
                self.stub,
                r#"
                {sig} {{
                    throw new global::System.NotImplementedException();
                }}
                "#
            );
        }
    }

    fn type_name(&mut self, ty: &Type) -> String {
        self.type_name_with_qualifier(ty, false)
    }

    // We use a global:: prefix to avoid conflicts with namespace clashes on partial namespace matches
    fn global_if_user_type(&self, ty: &Type) -> String {
        match ty {
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Option(_ty) => "".to_owned(),
                    TypeDefKind::Result(_result) => "".to_owned(),
                    TypeDefKind::List(_list) => "".to_owned(),
                    TypeDefKind::Tuple(_tuple) => "".to_owned(),
                    TypeDefKind::Type(inner_type) => self.global_if_user_type(inner_type),
                    _ => "global::".to_owned(),
                }
            }
            _ => "".to_owned(),
        }
    }

    pub(crate) fn type_name_with_qualifier(&mut self, ty: &Type, qualifier: bool) -> String {
        self.name_with_qualifier(ty, qualifier, ParameterType::ABI)
    }

    fn is_primative_list(&mut self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => self.is_primative_list(ty),
                    TypeDefKind::List(ty) if crate::world_generator::is_primitive(ty) => {
                        return true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub(crate) fn name_with_qualifier(
        &mut self,
        ty: &Type,
        qualifier: bool,
        parameter_type: ParameterType,
    ) -> String {
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
            Type::F32 => "float".to_owned(),
            Type::F64 => "double".to_owned(),
            Type::Char => "uint".to_owned(),
            Type::String => "string".to_owned(),
            Type::ErrorContext => todo!("error context name with qualifier"),
            Type::Id(id) => {
                let ty = &self.resolve.types[*id];
                match &ty.kind {
                    TypeDefKind::Type(ty) => {
                        self.name_with_qualifier(ty, qualifier, parameter_type)
                    }
                    TypeDefKind::List(ty) => {
                        if crate::world_generator::is_primitive(ty)
                            && self.direction == Direction::Import
                            && parameter_type == ParameterType::Span
                        {
                            format!("global::System.Span<{}>", self.type_name(ty))
                        } else if crate::world_generator::is_primitive(ty)
                            && self.direction == Direction::Import
                            && parameter_type == ParameterType::Memory
                        {
                            format!("global::System.Memory<{}>", self.type_name(ty))
                        } else if crate::world_generator::is_primitive(ty) {
                            format!("{}[]", self.type_name(ty))
                        } else {
                            format!(
                                "global::System.Collections.Generic.List<{}>",
                                self.type_name_with_qualifier(ty, qualifier)
                            )
                        }
                    }
                    TypeDefKind::Tuple(tuple) => {
                        let count = tuple.types.len();
                        self.csharp_gen.tuple_counts.insert(count);

                        let params = match count {
                            0 => String::new(),
                            1 => self
                                .type_name_with_qualifier(tuple.types.first().unwrap(), qualifier),
                            _ => format!(
                                "({})",
                                tuple
                                    .types
                                    .iter()
                                    .map(|ty| self.type_name_with_qualifier(ty, qualifier))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        };

                        params
                    }
                    TypeDefKind::Option(base_ty) => {
                        self.csharp_gen.needs_option = true;
                        let nesting = if let Type::Id(id) = base_ty {
                            matches!(&self.resolve.types[*id].kind, TypeDefKind::Option(_))
                        } else {
                            false
                        };
                        let base_ty = self.type_name_with_qualifier(base_ty, qualifier);
                        if nesting {
                            format!("Option<{base_ty}>")
                        } else {
                            format!("{base_ty}?")
                        }
                    }
                    TypeDefKind::Result(result) => {
                        self.csharp_gen.needs_result = true;
                        let mut name = |ty: &Option<Type>| {
                            ty.as_ref()
                                .map(|ty| self.type_name_with_qualifier(ty, qualifier))
                                .unwrap_or_else(|| "None".to_owned())
                        };
                        let ok = name(&result.ok);
                        let err = name(&result.err);

                        format!("Result<{ok}, {err}>")
                    }
                    TypeDefKind::Handle(handle) => {
                        let (Handle::Own(id) | Handle::Borrow(id)) = handle;
                        self.type_name_with_qualifier(&Type::Id(*id), qualifier)
                    }
                    TypeDefKind::Future(ty) => {
                        let name = ty
                            .as_ref()
                            .map(|ty| self.type_name_with_qualifier(ty, qualifier))
                            .unwrap_or_else(|| "".to_owned());
                        if name.is_empty() {
                            return "FutureReader".to_owned();
                        } else {
                            return format!("FutureReader<{name}>");
                        }
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            format!(
                                "{}{}",
                                self.qualifier(qualifier, id),
                                name.to_upper_camel_case()
                            )
                        } else {
                            unreachable!("todo: {ty:?}")
                        }
                    }
                }
            }
        }
    }

    fn print_docs(&mut self, docs: &Docs) {
        if let Some(docs) = &docs.contents {
            let lines = docs
                .trim()
                .replace("<", "&lt;")
                .replace(">", "&gt;")
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

    pub(crate) fn non_empty_type<'a>(&self, ty: Option<&'a Type>) -> Option<&'a Type> {
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

    pub(crate) fn start_resource(&mut self, id: TypeId, key: Option<&WorldKey>) {
        let access = self.csharp_gen.access_modifier();
        let qualified = self.type_name_with_qualifier(&Type::Id(id), true);
        let info = &self.csharp_gen.all_resources[&id];
        let name = info.name.clone();
        let upper_camel = name.to_upper_camel_case();
        let docs = info.docs.clone();
        self.print_docs(&docs);

        match self.direction {
            Direction::Import => {
                let module_name = key
                    .map(|key| self.resolve.name_world_key(key))
                    .unwrap_or_else(|| "$root".into());

                // As of this writing, we cannot safely drop a handle to an imported resource from a .NET finalizer
                // because it may still have one or more open child resources.  Once WIT has explicit syntax for
                // indicating parent/child relationships, we should be able to use that information to keep track
                // of child resources automatically in generated code, at which point we'll be able to drop them in
                // the correct order from finalizers.
                uwriteln!(
                    self.src,
                    r#"
                    {access} class {upper_camel}: global::System.IDisposable {{
                        internal int Handle {{ get; set; }}

                        {access} readonly record struct THandle(int Handle);

                        {access} {upper_camel}(THandle handle) {{
                            Handle = handle.Handle;
                        }}

                        public void Dispose() {{
                            Dispose(true);
                        }}

                        [global::System.Runtime.InteropServices.DllImportAttribute("{module_name}", EntryPoint = "[resource-drop]{name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                        private static extern void wasmImportResourceDrop(int p0);

                        protected virtual void Dispose(bool disposing) {{
                            if (disposing && Handle != 0) {{
                                wasmImportResourceDrop(Handle);
                                Handle = 0;
                            }}
                        }}
                    "#
                );
            }
            Direction::Export => {
                let prefix = key
                    .map(|s| format!("{}#", self.resolve.name_world_key(s)))
                    .unwrap_or_else(String::new);

                uwrite!(
                    self.csharp_interop_src,
                    r#"
                    [global::System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute(EntryPoint = "{prefix}[dtor]{name}")]
                    {access} static unsafe void wasmExportResourceDtor{upper_camel}(int rep) {{
                        var val = ({qualified}) {qualified}.repTable.Remove(rep);
                        val.Handle = 0;
                        // Note we call `Dispose` here even though the handle has already been disposed in case
                        // the implementation has overridden `Dispose(bool)`.
                        val.Dispose();
                    }}
                    "#
                );

                let module_name = key
                    .map(|key| format!("[export]{}", self.resolve.name_world_key(key)))
                    .unwrap_or_else(|| "[export]$root".into());

                // The ergonomics of exported resources are not ideal, currently. Implementing such a resource
                // requires both extending a class and implementing an interface. The reason for the class is to
                // allow implementers to inherit code which tracks and disposes of the resource handle; the reason
                // for the interface is to express the API contract which the implementation must fulfill,
                // including static functions.
                //
                // We could remove the need for the class (and its `IDisposable` implementation) entirely if we
                // were to dispose of the handle immediately when lifting an owned handle, in which case we would
                // be left with nothing to keep track of or dispose later. However, we keep the handle alive in
                // case we want to give ownership back to the host again, in which case we'll be able to reuse the
                // same handle instead of calling `[resource-new]` to allocate a new one. Whether this optimization
                // is worth the trouble is open to debate, but we currently consider it a worthwhile tradeoff.
                //
                // Note that applications which export resources are relatively rare compared to those which only
                // import them, so in practice most developers won't encounter any of this anyway.
                uwriteln!(
                    self.src,
                    r#"
                    {access} abstract class {upper_camel}: global::System.IDisposable {{
                        internal static RepTable<{upper_camel}> repTable = new ();
                        internal int Handle {{ get; set; }}

                        public void Dispose() {{
                            Dispose(true);
                            GC.SuppressFinalize(this);
                        }}

                        internal static class WasmInterop {{
                            [global::System.Runtime.InteropServices.DllImportAttribute("{module_name}", EntryPoint = "[resource-drop]{name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                            internal static extern void wasmImportResourceDrop(int p0);

                            [global::System.Runtime.InteropServices.DllImportAttribute("{module_name}", EntryPoint = "[resource-new]{name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                            internal static extern int wasmImportResourceNew(int p0);

                            [global::System.Runtime.InteropServices.DllImportAttribute("{module_name}", EntryPoint = "[resource-rep]{name}"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
                            internal static extern int wasmImportResourceRep(int p0);
                        }}

                        protected virtual void Dispose(bool disposing) {{
                            if (Handle != 0) {{
                                var handle = Handle;
                                Handle = 0;
                                WasmInterop.wasmImportResourceDrop(handle);
                            }}
                        }}

                        ~{upper_camel}() {{
                            Dispose(false);
                        }}
                    }}

                    {access} interface I{upper_camel} {{
                    "#
                );

                if self.csharp_gen.opts.generate_stub {
                    let super_ = self.type_name_with_qualifier(&Type::Id(id), true);
                    let interface = {
                        let split = super_.split('.').collect::<Vec<_>>();
                        split
                            .iter()
                            .map(|&v| v.to_owned())
                            .take(split.len() - 1)
                            .chain(split.last().map(|v| format!("I{v}")))
                            .collect::<Vec<_>>()
                            .join(".")
                    };

                    uwriteln!(
                        self.stub,
                        r#"
                        {access} class {upper_camel}: {super_}, {interface} {{
                        "#
                    );
                }
            }
        };
    }

    pub(crate) fn end_resource(&mut self) {
        if self.direction == Direction::Export && self.csharp_gen.opts.generate_stub {
            uwriteln!(
                self.stub,
                "
                }}
                "
            );
        }

        uwriteln!(
            self.src,
            "
            }}
            "
        );
    }

    fn sig_string(&mut self, func: &Function, qualifier: bool) -> String {
        let result_type = self.func_return_type(&func);

        let params = func
            .params
            .iter()
            .skip(if let FunctionKind::Method(_) = &func.kind {
                1
            } else {
                0
            })
            .map(|(name, ty)| {
                let ty = self.type_name_with_qualifier(ty, qualifier);
                let name = name.to_csharp_ident();
                format!("{ty} {name}")
            })
            .collect::<Vec<_>>()
            .join(", ");

        let (camel_name, modifiers) = match &func.kind {
            FunctionKind::Freestanding
            | FunctionKind::AsyncFreestanding
            | FunctionKind::Static(_)
            | FunctionKind::AsyncStatic(_) => (func.item_name().to_upper_camel_case(), "static"),
            FunctionKind::Method(_) | FunctionKind::AsyncMethod(_) => {
                (func.item_name().to_upper_camel_case(), "")
            }
            FunctionKind::Constructor(id) => (
                self.csharp_gen.all_resources[id].name.to_upper_camel_case(),
                "",
            ),
        };

        let access = self.csharp_gen.access_modifier();

        format!("{access} {modifiers} {result_type} {camel_name}({params})")
    }
    
    pub(crate) fn add_future(&mut self, func_name: &str) {
        self.futures.push(func_name.to_string());
    }
}

impl<'a> CoreInterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        let access = self.csharp_gen.access_modifier();

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
            format!("{access} const {name} INSTANCE = new {name}();")
        } else {
            record
                .fields
                .iter()
                .map(|field| {
                    format!(
                        "{access} readonly {} {};",
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
            {access} class {name} {{
                {fields}

                {access} {name}({parameters}) {{
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

        let access = self.csharp_gen.access_modifier();

        uwrite!(
            self.src,
            "
            {access} enum {name} {enum_type} {{
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
        let access = self.csharp_gen.access_modifier();

        let constructors = variant
            .cases
            .iter()
            .map(|case| {
                let case_name = case.name.to_csharp_ident();
                let tag = case.name.to_csharp_ident_upper();
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
                    "{access} static {name} {tag}({parameter}) {{
                         return new {name}(Tags.{tag}, {argument});
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
                    let tag = case.name.to_csharp_ident_upper();
                    let ty = self.type_name(ty);
                    format!(
                        r#"{access} {ty} As{case_name}
                        {{
                            get
                            {{
                                if (Tag == Tags.{tag})
                                    return ({ty})value!;
                                else
                                    throw new global::System.ArgumentException("expected {tag}, got " + Tag);
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
                let tag = case.name.to_csharp_ident_upper();
                format!("{access} const {tag_type} {tag} = {i};")
            })
            .collect::<Vec<_>>()
            .join("\n");

        uwrite!(
            self.src,
            "
            {access} class {name} {{
                {access} readonly {tag_type} Tag;
                private readonly object? value;

                private {name}({tag_type} tag, object? value) {{
                    this.Tag = tag;
                    this.value = value;
                }}

                {constructors}
                {accessors}

                {access} class Tags {{
                    {tags}
                }}
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

        let access = self.csharp_gen.access_modifier();

        uwrite!(
            self.src,
            "
            {access} enum {name} {{
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

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        // Here we just record information about the resource; we don't actually emit any code until we're ready to
        // visit any functions associated with the resource (e.g. in CSharp::import_interface, etc.).
        self.csharp_gen
            .all_resources
            .entry(id)
            .or_insert_with(|| ResourceInfo {
                module: self.name.to_owned(),
                name: name.to_owned(),
                docs: docs.clone(),
                direction: Direction::Import,
            })
            .direction = self.direction;
    }

    fn type_future(&mut self, id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }

    fn type_stream(&mut self, id: TypeId, _name: &str, _ty: &Option<Type>, _docs: &Docs) {
        self.type_name(&Type::Id(id));
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ParameterType {
    ABI,
    Span,
    Memory,
}

fn payload_and_results(
    resolve: &Resolve,
    ty: Type,
    with_wit_results: bool,
) -> (Option<Type>, Vec<TypeId>) {
    if with_wit_results {
        return (Some(ty), Vec::new());
    }

    fn recurse(resolve: &Resolve, ty: Type, results: &mut Vec<TypeId>) -> Option<Type> {
        if let Type::Id(id) = ty {
            if let TypeDefKind::Result(result) = &resolve.types[id].kind {
                results.push(id);
                if let Some(ty) = result.ok {
                    recurse(resolve, ty, results)
                } else {
                    None
                }
            } else {
                Some(ty)
            }
        } else {
            Some(ty)
        }
    }

    let mut results = Vec::new();
    let payload = recurse(resolve, ty, &mut results);
    (payload, results)
}

fn modifiers(func: &Function, name: &str, direction: Direction) -> String {
    let new_modifier = match &func.kind {
        // Avoid warnings about name clashes.
        //
        // TODO: add other `object` method names here
        FunctionKind::Method(_) if name == "GetType" => " new",
        _ => "",
    };

    let static_modifiers = match &func.kind {
        FunctionKind::Freestanding
        | FunctionKind::Static(_)
        | FunctionKind::AsyncFreestanding
        | FunctionKind::AsyncStatic(_) => "static",
        _ => "",
    };

    let abstract_modifier = if direction == Direction::Export {
        " abstract"
    } else {
        ""
    };

    let async_modifier = match &func.kind {
        FunctionKind::AsyncMethod(_)
        | FunctionKind::AsyncFreestanding
        | FunctionKind::AsyncStatic(_)
            if abstract_modifier == "" =>
        {
            " async"
        }
        _ => "",
    };

    format!("{static_modifiers}{abstract_modifier}{async_modifier}{new_modifier}")
}

fn int_type(int: Int) -> &'static str {
    match int {
        Int::U8 => "byte",
        Int::U16 => "ushort",
        Int::U32 => "uint",
        Int::U64 => "ulong",
    }
}
