use anyhow::{bail, Result};
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap, HashSet},
};
use wasmparser::{
    Chunk, Encoding, ExternalKind, FuncType, Parser, Payload, Type, TypeDef, TypeRef, Validator,
};
use wit_parser::{
    abi::{AbiVariant, WasmSignature, WasmType},
    Interface,
};

fn is_wasi(name: &str) -> bool {
    name == "wasi_unstable" || name == "wasi_snapshot_preview1"
}

fn is_canonical_function(name: &str) -> bool {
    name.starts_with("canonical_abi_")
}

pub fn expected_export_name<'a>(interface: Option<&str>, func: &'a str) -> Cow<'a, str> {
    // TODO: wit-bindgen currently doesn't mangle its export names, so this
    // only works with the default (i.e. `None`) interface.
    match interface {
        Some(interface) => format!("{}#{}", interface, func).into(),
        None => func.into(),
    }
}

fn wasm_sig_to_func_type(signature: WasmSignature) -> FuncType {
    fn from_wasm_type(ty: &WasmType) -> Type {
        match ty {
            WasmType::I32 => Type::I32,
            WasmType::I64 => Type::I64,
            WasmType::F32 => Type::F32,
            WasmType::F64 => Type::F64,
        }
    }

    FuncType {
        params: signature
            .params
            .iter()
            .map(from_wasm_type)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        returns: signature
            .results
            .iter()
            .map(from_wasm_type)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

/// This function validates the following:
/// * The bytes represent a core WebAssembly module.
/// * The module's imports are all satisfied by the given import interfaces.
/// * The given default and exported interfaces are satisfied by the module's exports.
///
/// Returns the set of imported interfaces required by the module.
pub fn validate_module<'a>(
    mut bytes: &'a [u8],
    interface: &Option<&Interface>,
    imports: &[Interface],
    exports: &[Interface],
) -> Result<HashSet<&'a str>> {
    let imports: HashMap<&str, &Interface> = imports.iter().map(|i| (i.name.as_str(), i)).collect();
    let exports: HashMap<&str, &Interface> = exports.iter().map(|i| (i.name.as_str(), i)).collect();

    let mut parser = Parser::new(0);
    let mut validator = Validator::new();
    let mut types = Vec::new();
    let mut functions = Vec::new();
    let mut import_funcs = HashMap::new();
    let mut export_funcs = HashMap::new();

    loop {
        match parser.parse(bytes, true)? {
            Chunk::Parsed { payload, consumed } => {
                bytes = &bytes[consumed..];
                match payload {
                    Payload::Version {
                        num,
                        encoding,
                        range,
                    } => {
                        validator.version(num, encoding, &range)?;
                        if encoding != Encoding::Module {
                            bail!("data is not a WebAssembly module");
                        }
                    }
                    Payload::TypeSection(s) => {
                        validator.type_section(&s)?;
                        types.reserve(s.get_count() as usize);
                        for ty in s {
                            match ty? {
                                TypeDef::Func(ty) => {
                                    types.push(ty);
                                }
                            }
                        }
                    }
                    Payload::ImportSection(s) => {
                        validator.import_section(&s)?;
                        for import in s {
                            let import = import?;
                            if is_wasi(import.module) {
                                continue;
                            }
                            match import.ty {
                                TypeRef::Func(ty) => {
                                    let map = match import_funcs.entry(import.module) {
                                        Entry::Occupied(e) => e.into_mut(),
                                        Entry::Vacant(e) => e.insert(HashMap::new()),
                                    };

                                    if map.insert(import.name, ty).is_some() {
                                        bail!(
                                            "duplicate import `{}::{}`",
                                            import.module,
                                            import.name
                                        );
                                    }

                                    functions.push(ty);
                                }
                                _ => bail!("module is only allowed to import functions"),
                            }
                        }
                    }
                    Payload::FunctionSection(s) => {
                        validator.function_section(&s)?;
                        functions.reserve(s.get_count() as usize);
                        for ty in s {
                            functions.push(ty?);
                        }
                    }
                    Payload::TableSection(s) => {
                        validator.table_section(&s)?;
                    }
                    Payload::MemorySection(s) => {
                        validator.memory_section(&s)?;
                    }
                    Payload::TagSection(s) => {
                        validator.tag_section(&s)?;
                    }
                    Payload::GlobalSection(s) => {
                        validator.global_section(&s)?;
                    }
                    Payload::ExportSection(s) => {
                        validator.export_section(&s)?;

                        for export in s {
                            let export = export?;

                            match export.kind {
                                ExternalKind::Func => {
                                    if is_canonical_function(export.name) {
                                        continue;
                                    }

                                    if export_funcs.insert(export.name, export.index).is_some() {
                                        bail!("duplicate exported function `{}`", export.name);
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                    Payload::StartSection { func, range } => {
                        validator.start_section(func, &range)?;
                    }
                    Payload::ElementSection(s) => {
                        validator.element_section(&s)?;
                    }
                    Payload::DataCountSection { count, range } => {
                        validator.data_count_section(count, &range)?;
                    }
                    Payload::DataSection(s) => {
                        validator.data_section(&s)?;
                    }
                    Payload::CodeSectionStart { count, range, .. } => {
                        validator.code_section_start(count, &range)?;
                    }
                    Payload::CodeSectionEntry(body) => {
                        let mut v = validator.code_section_entry(&body)?;
                        v.validate(&body)?;
                    }

                    // Component sections shouldn't be present in a module
                    Payload::ComponentTypeSection(_)
                    | Payload::ComponentImportSection(_)
                    | Payload::ComponentFunctionSection(_)
                    | Payload::ModuleSection { .. }
                    | Payload::ComponentSection { .. }
                    | Payload::InstanceSection(_)
                    | Payload::ComponentExportSection(_)
                    | Payload::ComponentStartSection(_)
                    | Payload::AliasSection(_) => unreachable!(),

                    Payload::CustomSection { .. } => {
                        // Ignore custom sections
                    }
                    Payload::UnknownSection { id, range, .. } => {
                        validator.unknown_section(id, &range)?;
                    }
                    Payload::End(_) => break,
                }
            }
            Chunk::NeedMoreData(_) => unreachable!(),
        }
    }

    for (name, funcs) in &import_funcs {
        if name.is_empty() {
            bail!("module imports from an empty module name");
        }

        match imports.get(name) {
            Some(interface) => {
                validate_imported_interface(interface, name, funcs, &types)?;
            }
            None => bail!("module requires an import interface named `{}`", name),
        }
    }

    if let Some(interface) = interface {
        validate_exported_interface(interface, None, &export_funcs, &types, &functions)?;
    }

    for (name, interface) in exports {
        if name.is_empty() {
            bail!("cannot export an interface with an empty name");
        }

        validate_exported_interface(interface, Some(name), &export_funcs, &types, &functions)?;
    }

    Ok(import_funcs.keys().copied().collect())
}

fn validate_imported_interface(
    interface: &Interface,
    name: &str,
    imports: &HashMap<&str, u32>,
    types: &[FuncType],
) -> Result<()> {
    for (func_name, ty) in imports {
        match interface.functions.iter().find(|f| f.name == *func_name) {
            Some(f) => {
                let expected =
                    wasm_sig_to_func_type(interface.wasm_signature(AbiVariant::GuestImport, f));
                let ty = &types[*ty as usize];
                if ty != &expected {
                    bail!(
                        "type mismatch for function `{}` on imported interface `{}`: expected `{:?} -> {:?}` but found `{:?} -> {:?}`",
                        func_name,
                        name,
                        expected.params,
                        expected.returns,
                        ty.params,
                        ty.returns
                    );
                }
            }
            None => bail!(
                "import interface `{}` is missing function `{}` that is required by the module",
                name,
                func_name,
            ),
        }
    }

    Ok(())
}

fn validate_exported_interface(
    interface: &Interface,
    name: Option<&str>,
    exports: &HashMap<&str, u32>,
    types: &[FuncType],
    funcs: &[u32],
) -> Result<()> {
    for f in &interface.functions {
        let expected_export = expected_export_name(name, &f.name);
        match exports.get(expected_export.as_ref()) {
            Some(ty) => {
                let expected_ty =
                    wasm_sig_to_func_type(interface.wasm_signature(AbiVariant::GuestExport, f));
                let ty = &types[funcs[*ty as usize] as usize];
                if ty != &expected_ty {
                    match name {
                        Some(name) => bail!(
                            "type mismatch for function `{}` from exported interface `{}`: expected `{:?} -> {:?}` but found `{:?} -> {:?}`",
                            f.name,
                            name,
                            expected_ty.params,
                            expected_ty.returns,
                            ty.params,
                            ty.returns
                        ),
                        None => bail!(
                            "type mismatch for default interface function `{}`: expected `{:?} -> {:?}` but found `{:?} -> {:?}`",
                            f.name,
                            expected_ty.params,
                            expected_ty.returns,
                            ty.params,
                            ty.returns
                        )
                    }
                }
            }
            None => bail!(
                "module does not export required function `{}`",
                expected_export
            ),
        }
    }

    Ok(())
}
