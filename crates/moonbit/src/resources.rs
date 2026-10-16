use std::fmt::Write;

use wit_bindgen_core::{
    uwrite, uwriteln,
    wit_parser::{
        Function, FunctionKind, LiftLowerAbi, ManglingAndAbi, ResourceIntrinsic, Type, TypeDefKind,
        TypeId, WasmExport, WasmImport,
    },
};

use crate::{
    InterfaceGenerator,
    async_support::background_group_name,
    pkg::{MoonbitSignature, ToMoonBitTypeIdent},
};

impl InterfaceGenerator<'_> {
    // The implementation receives the object; callers of an owned handle get
    // the same WIT methods after a checked lookup. Constructors register the
    // returned object exactly once before the ABI transfers ownership.
    pub(super) fn export_resource_forwarder(
        &mut self,
        func: &Function,
        sig: &MoonbitSignature,
        async_: bool,
    ) {
        let Some(id) = func.kind.resource() else {
            return;
        };
        if !self
            .world_gen
            .pkg_resolver
            .resource_handle_names
            .contains_key(&id)
        {
            return;
        }
        let resolver = &mut self.world_gen.pkg_resolver;
        let handle = resolver.type_constructor(self.name, &Type::Id(id));
        let payload = resolver.resource_helper_name(id, "payload");
        let new = resolver.resource_helper_name(id, "new");
        let (_, method) = sig.name.rsplit_once("::").unwrap();
        let is_method = matches!(
            func.kind,
            FunctionKind::Method(_) | FunctionKind::AsyncMethod(_)
        );
        let mut params = Vec::new();
        let mut args = Vec::new();
        for (i, (name, ty)) in sig.params.iter().enumerate() {
            let (ty, arg) = if i == 0 && is_method {
                (handle.clone(), format!("{name}.{payload}()"))
            } else {
                (resolver.type_name(self.name, ty), name.clone())
            };
            params.push(format!("{name} : {ty}"));
            args.push(arg);
        }
        let result = sig
            .result_type
            .as_ref()
            .map(|ty| resolver.type_name(self.name, ty))
            .unwrap_or_else(|| "Unit".into());
        self.add_async_export_stub_parameter(func, async_, &mut params);
        if async_ {
            args.push(background_group_name(func));
        }
        let call = format!("{}({})", sig.name, args.join(", "));
        let call = if matches!(func.kind, FunctionKind::Constructor(_)) {
            let Some(Type::Id(result)) = sig.result_type else {
                unreachable!()
            };
            if matches!(self.resolve.types[result].kind, TypeDefKind::Result(_)) {
                format!(
                    "match {call} {{ Ok(value) => Ok({handle}::{new}(value)); Err(error) => Err(error) }}"
                )
            } else {
                format!("{handle}::{new}({call})")
            }
        } else {
            call
        };
        let async_keyword = if async_ { "async " } else { "" };
        let params = params.join(", ");
        uwrite!(
            self.src,
            r#"
            ///|
            pub {async_keyword}fn {handle}::{method}({params}) -> {result} {{
                {call}
            }}
        "#
        );
    }

    pub(super) fn export_resource(&mut self, id: TypeId, name: &str) {
        let data = self.resolve.types[id]
            .name
            .as_ref()
            .unwrap()
            .to_moonbit_type_ident();
        let resolver = &self.world_gen.pkg_resolver;
        let new = resolver.resource_helper_name(id, "new");
        let payload = resolver.resource_helper_name(id, "payload");
        let drop = resolver.resource_helper_name(id, "drop");
        let dtor = resolver.resource_helper_name(id, "dtor");
        let from = resolver.resource_helper_name(id, "wasm_from_handle");
        let take = resolver.resource_helper_name(id, "wasm_take_handle");
        let borrow = resolver.resource_helper_name(id, "wasm_borrow");
        let is_error = self.derive_opts.derive_error && name.contains("Error");
        let get_state = if is_error {
            format!("let {name}(state) = self")
        } else {
            "let state = self.handle".into()
        };
        let wrap_handle = if is_error {
            format!("{name}(Ref(Some(handle)))")
        } else {
            "{ handle: Ref(Some(handle)) }".into()
        };
        if self.derive_opts.derive_eq {
            let other_state = if is_error {
                format!("let {name}(other_state) = other")
            } else {
                "let other_state = other.handle".into()
            };
            uwriteln!(
                self.src,
                r#"
                ///|
                pub impl Eq for {name} with equal(self, other) {{
                    {get_state}
                    {other_state}
                    physical_equal(state, other_state)
                }}
            "#
            );
            self.emit_derived_methods(name, &["Eq"]);
        }
        let (drop_module, drop_name) = self.resolve.wasm_import_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmImport::ResourceIntrinsic {
                resource: id,
                interface: self.interface,
                intrinsic: ResourceIntrinsic::ExportedDrop,
            },
        );
        let (new_module, new_name) = self.resolve.wasm_import_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmImport::ResourceIntrinsic {
                resource: id,
                interface: self.interface,
                intrinsic: ResourceIntrinsic::ExportedNew,
            },
        );
        let (rep_module, rep_name) = self.resolve.wasm_import_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmImport::ResourceIntrinsic {
                resource: id,
                interface: self.interface,
                intrinsic: ResourceIntrinsic::ExportedRep,
            },
        );

        uwrite!(
            self.src,
            r#"
            ///|
            /// Application-defined payload for `{name}`. Implement this type in
            /// a separate file in this package. Exported borrows receive this
            /// payload directly; owned values use `{name}`.
            declare type {data}

            ///|
            /// Releases application resources when the component resource is destroyed.
            /// The generated bindings remove the payload from their storage first.
            declare fn {data}::{dtor}(_self : {data}) -> Unit

            ///|
            let wasm{name}Payloads : Array[{data}?] = []

            ///|
            let wasm{name}FreeReps : Array[Int] = []

            ///|
            /// Creates an owned resource and retains its payload until destruction.
            pub fn {name}::{new}(payload : {data}) -> {name} {{
                let rep = match wasm{name}FreeReps.pop() {{
                    Some(rep) => {{
                        wasm{name}Payloads[rep] = Some(payload)
                        rep
                    }}
                    None => {{
                        let rep = wasm{name}Payloads.length()
                        wasm{name}Payloads.push(Some(payload))
                        rep
                    }}
                }}
                {name}::{from}(wasmExportResourceNew{name}(rep))
            }}

            ///|
            /// Accesses the payload of a locally owned resource.
            /// Traps if ownership has already been transferred or dropped.
            pub fn {name}::{payload}(self : {name}) -> {data} {{
                {get_state}
                guard state.val is Some(handle) else {{ abort("resource is no longer owned") }}
                {name}::{borrow}(wasmExportResourceRep{name}(handle))
            }}

            ///|
            /// Releases ownership and invokes the resource destructor once.
            pub fn {name}::{drop}(self : {name}) -> Unit {{
                {get_state}
                if state.val is Some(handle) {{
                    // Invalidate aliases before calling potentially reentrant cleanup.
                    state.val = None
                    wasmExportResourceDrop{name}(handle)
                }}
            }}

            ///|
            #doc(hidden)
            pub fn {name}::{from}(handle : Int) -> {name} {{
                {wrap_handle}
            }}

            ///|
            #doc(hidden)
            pub fn {name}::{take}(self : {name}) -> Int {{
                {get_state}
                guard state.val is Some(handle) else {{ abort("resource is no longer owned") }}
                state.val = None
                handle
            }}

            ///|
            #doc(hidden)
            pub fn {name}::{borrow}(rep : Int) -> {data} {{
                wasm{name}Payloads[rep].unwrap()
            }}

            ///|
            fn wasmExportResourceNew{name}(rep : Int) -> Int = "{new_module}" "{new_name}"

            ///|
            fn wasmExportResourceDrop{name}(handle : Int) = "{drop_module}" "{drop_name}"

            ///|
            fn wasmExportResourceRep{name}(handle : Int) -> Int = "{rep_module}" "{rep_name}"
            "#,
        );

        let func_name = self
            .world_gen
            .export_ns
            .tmp(&format!("wasmExport{name}Dtor"));
        uwrite!(
            self.ffi,
            r#"
            ///|
            #doc(hidden)
            pub fn {func_name}(rep : Int) -> Unit {{
                let payload = wasm{name}Payloads[rep].unwrap()
                // Free the slot before user cleanup, which may create another resource.
                wasm{name}Payloads[rep] = None
                wasm{name}FreeReps.push(rep)
                {data}::{dtor}(payload)
            }}
            "#,
        );
        let export_name = self.resolve.wasm_export_name(
            ManglingAndAbi::Legacy(LiftLowerAbi::Sync),
            WasmExport::ResourceDtor {
                interface: self.interface.unwrap(),
                resource: id,
            },
        );
        let qualifier = self
            .world_gen
            .pkg_resolver
            .qualify_package(self.world_gen.opts.gen_dir.as_str(), self.name);
        let export = format!(
            r#"
            ///|
            #doc(hidden)
            pub fn {func_name}(rep : Int) -> Unit {{
                {qualifier}{func_name}(rep)
            }}
            "#,
        );
        self.world_gen
            .export
            .insert(export_name, (func_name, export));
    }
}
