use crate::adapted::call::CallAdapter;
use crate::{linker::to_val_type, Module};
use anyhow::{anyhow, bail, Result};
use std::{
    array::IntoIter,
    collections::{BTreeMap, HashMap},
    fmt,
};
use wasmparser::{ExternalKind, FuncType, Type};

mod call;

pub const PARENT_MODULE_NAME: &str = "$parent";
const MEMORY_EXPORT_NAME: &str = "memory";
pub const REALLOC_EXPORT_NAME: &str = "canonical_abi_realloc";
const FREE_EXPORT_NAME: &str = "canonical_abi_free";
pub const FUNCTION_TABLE_NAME: &str = "$funcs";

lazy_static::lazy_static! {
    pub static ref REALLOC_FUNC_TYPE: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32, Type::I32, Type::I32]),
            returns: Box::new([Type::I32])
        }
    };
    static ref FREE_FUNC_TYPE: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32, Type::I32]),
            returns: Box::new([])
        }
    };
}

pub fn must_adapt_type(interface: &witx2::Interface, ty: &witx2::Type) -> bool {
    match ty {
        witx2::Type::U8
        | witx2::Type::S8
        | witx2::Type::U16
        | witx2::Type::S16
        | witx2::Type::U32
        | witx2::Type::S32
        | witx2::Type::U64
        | witx2::Type::S64
        | witx2::Type::F32
        | witx2::Type::F64
        | witx2::Type::CChar
        | witx2::Type::Usize
        | witx2::Type::Char
        | witx2::Type::Handle(_) => false,

        witx2::Type::Id(id) => match &interface.types[*id].kind {
            witx2::TypeDefKind::List(_)
            | witx2::TypeDefKind::PushBuffer(_)
            | witx2::TypeDefKind::PullBuffer(_) => true,
            witx2::TypeDefKind::Variant(v) => v
                .cases
                .iter()
                .filter_map(|c| c.ty.as_ref())
                .any(|t| must_adapt_type(interface, t)),
            witx2::TypeDefKind::Type(t) => must_adapt_type(interface, t),
            witx2::TypeDefKind::Record(r) => {
                r.fields.iter().any(|f| must_adapt_type(interface, &f.ty))
            }
            witx2::TypeDefKind::Pointer(_) | witx2::TypeDefKind::ConstPointer(_) => false,
        },
    }
}

pub struct AdaptedModule<'a> {
    pub module: &'a Module<'a>,
    types: Vec<&'a FuncType>,
    imports: Vec<(&'a str, Option<&'a str>, wasm_encoder::EntityType)>,
    implicit_instances: BTreeMap<&'a str, u32>,
    functions: Vec<u32>,
    parent_realloc_index: Option<u32>,
    realloc_index: Option<u32>,
    free_index: Option<u32>,
}

impl<'a> AdaptedModule<'a> {
    pub fn new(module: &'a Module) -> Result<Self> {
        let mut adapted = Self {
            module,
            types: Vec::new(),
            imports: Vec::new(),
            implicit_instances: BTreeMap::new(),
            functions: Vec::new(),
            parent_realloc_index: None,
            realloc_index: None,
            free_index: None,
        };

        if let Some(interface) = module.interface.as_ref() {
            let mut type_map = HashMap::new();
            let mut num_imported_funcs = 0;

            // Populate the type map
            for ty in module
                .imports
                .iter()
                .map(|i| {
                    module
                        .import_func_type(i)
                        .expect("expected import to be a function")
                })
                .chain(interface.iter().filter_map(|(_, info)| {
                    if info.must_adapt {
                        Some(&info.import_type)
                    } else {
                        None
                    }
                }))
                .chain(std::iter::once(&REALLOC_FUNC_TYPE as &FuncType))
            {
                type_map.entry(ty).or_insert_with(|| {
                    let index = adapted.types.len();
                    adapted.types.push(ty);
                    index as u32
                });
            }

            // Populate imports
            for (import_module, import_field, entity) in module
                .imports
                .iter()
                .map(|i| {
                    let ty = module
                        .import_func_type(i)
                        .expect("import should be a function");
                    (
                        i.module,
                        i.field,
                        wasm_encoder::EntityType::Function(type_map[ty]),
                    )
                })
                .chain(IntoIter::new([
                    (
                        PARENT_MODULE_NAME,
                        Some(MEMORY_EXPORT_NAME),
                        wasm_encoder::EntityType::Memory(wasm_encoder::MemoryType {
                            limits: wasm_encoder::Limits { min: 0, max: None },
                        }),
                    ),
                    (
                        PARENT_MODULE_NAME,
                        Some(REALLOC_EXPORT_NAME),
                        wasm_encoder::EntityType::Function(
                            type_map[&REALLOC_FUNC_TYPE as &FuncType],
                        ),
                    ),
                ]))
            {
                if let wasm_encoder::EntityType::Function(_) = &entity {
                    if import_module == PARENT_MODULE_NAME
                        && import_field == Some(REALLOC_EXPORT_NAME)
                    {
                        adapted.parent_realloc_index = Some(num_imported_funcs);
                    }
                    num_imported_funcs += 1;
                }

                adapted.imports.push((import_module, import_field, entity));

                let len = adapted.implicit_instances.len();
                adapted
                    .implicit_instances
                    .entry(import_module)
                    .or_insert(len as u32);
            }

            // The realloc and free functions are aliases that come before any defined functions
            adapted.realloc_index = Some(num_imported_funcs);
            adapted.free_index = Some(num_imported_funcs + 1);

            // Populate the adapted functions
            for (_, info) in interface.iter() {
                if info.must_adapt {
                    adapted.functions.push(type_map[&info.import_type]);
                }
            }
        }

        Ok(adapted)
    }

    fn validate(&self) -> Result<()> {
        enum ExpectedExportType<'a> {
            Memory,
            Function(&'a FuncType),
        }

        impl fmt::Display for ExpectedExportType<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Memory => write!(f, "memory"),
                    Self::Function { .. } => write!(f, "function"),
                }
            }
        }

        let mut expected = vec![
            (MEMORY_EXPORT_NAME, ExpectedExportType::Memory, false),
            (
                REALLOC_EXPORT_NAME,
                ExpectedExportType::Function(&REALLOC_FUNC_TYPE),
                false,
            ),
            (
                FREE_EXPORT_NAME,
                ExpectedExportType::Function(&FREE_FUNC_TYPE),
                false,
            ),
        ];

        expected.extend(
            self.module
                .interface
                .as_ref()
                .unwrap()
                .iter()
                .map(|(f, info)| {
                    (
                        f.name.as_str(),
                        ExpectedExportType::Function(&info.export_type),
                        false,
                    )
                }),
        );

        for export in &self.module.exports {
            for (expected_name, expected_type, seen) in &mut expected {
                if export.field == *expected_name {
                    *seen = true;
                    match (export.kind, &expected_type) {
                        (ExternalKind::Function, ExpectedExportType::Function(expected_ty)) => {
                            let ty = self.module.func_type(export.index).ok_or_else(|| {
                                anyhow!(
                                    "required export `{}` from module `{}` is not a function",
                                    export.field,
                                    self.module.name
                                )
                            })?;

                            if ty != *expected_ty {
                                bail!("required export `{}` from module `{}` does not have the expected function signature of {:?} -> {:?}", export.field, self.module.name, expected_ty.params, expected_ty.returns);
                            }
                        }
                        (ExternalKind::Memory, ExpectedExportType::Memory) => {
                            // No further validation required for the memory's type
                        }
                        _ => {
                            bail!(
                                "required export `{}` from module `{}` is not a {}",
                                export.field,
                                self.module.name,
                                expected_type
                            )
                        }
                    }
                }
            }
        }

        for (name, _, seen) in &expected {
            if !*seen {
                bail!(
                    "required export `{}` is missing from module `{}`",
                    name,
                    self.module.name
                );
            }
        }

        Ok(())
    }

    pub fn encode(&self) -> Result<wasm_encoder::Module> {
        if !self.module.must_adapt() {
            return Ok(self.module.encode());
        }

        self.validate()?;

        let mut module = wasm_encoder::Module::new();

        self.write_type_section(&mut module);
        self.write_import_section(&mut module);
        self.write_module_section(&mut module);
        self.write_instance_section(&mut module);
        self.write_alias_section(&mut module);
        self.write_function_section(&mut module);
        self.write_export_section(&mut module);
        self.write_code_section(&mut module);

        // TODO: write a names section for the adapted module?

        Ok(module)
    }

    fn write_type_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::TypeSection::new();

        for ty in &self.types {
            section.function(
                ty.params.iter().map(to_val_type),
                ty.returns.iter().map(to_val_type),
            );
        }

        module.section(&section);
    }

    fn write_import_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::ImportSection::new();

        for (module, field, ty) in &self.imports {
            section.import(module, *field, *ty);
        }

        module.section(&section);
    }

    fn write_module_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::ModuleSection::new();
        section.module(&self.module.encode());
        module.section(&section);
    }

    fn write_instance_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::InstanceSection::new();

        let args: Vec<_> = self
            .implicit_instances
            .iter()
            .filter_map(|(name, index)| match *name {
                PARENT_MODULE_NAME => None,
                _ => Some((*name, wasm_encoder::Export::Instance(*index))),
            })
            .collect();

        section.instantiate(0, args);

        module.section(&section);
    }

    fn write_alias_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::AliasSection::new();

        // The inner module's instance is always *after* the implicit instances
        let instance = self.implicit_instances.len() as u32;

        section.instance_export(instance, wasm_encoder::ItemKind::Memory, MEMORY_EXPORT_NAME);

        // Order here matters: realloc, then free, then adapted functions
        section.instance_export(
            instance,
            wasm_encoder::ItemKind::Function,
            REALLOC_EXPORT_NAME,
        );

        section.instance_export(instance, wasm_encoder::ItemKind::Function, FREE_EXPORT_NAME);

        // Add the adapted function aliases
        for (f, _) in self.module.interface.as_ref().unwrap().iter() {
            section.instance_export(instance, wasm_encoder::ItemKind::Function, f.name.as_str());
        }

        module.section(&section);
    }

    fn write_function_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::FunctionSection::new();

        // Add the adapted functions
        for ty in &self.functions {
            section.function(*ty);
        }

        module.section(&section);
    }

    fn write_export_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::ExportSection::new();

        let interface = self.module.interface.as_ref().unwrap();

        let alias_start_index = self.free_index.unwrap() + 1;
        let func_start_index = alias_start_index + interface.inner().functions.len() as u32;
        let mut adapted_count = 0;

        for (index, (f, info)) in interface.iter().enumerate() {
            section.export(
                f.name.as_str(),
                wasm_encoder::Export::Function(if info.must_adapt {
                    func_start_index + adapted_count
                } else {
                    alias_start_index + index as u32
                }),
            );

            if info.must_adapt {
                adapted_count += 1;
            }
        }

        module.section(&section);
    }

    fn write_code_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::CodeSection::new();

        let parent_realloc_index = self.parent_realloc_index.unwrap();
        let realloc_index = self.realloc_index.unwrap();
        let free_index = self.free_index.unwrap();

        let interface = self.module.interface.as_ref().unwrap();

        for (index, (func, info)) in interface.iter().enumerate() {
            if !info.must_adapt {
                continue;
            }

            let adapter = CallAdapter::new(
                interface,
                &info.import_signature,
                func,
                index as u32 + free_index + 1,
                realloc_index,
                parent_realloc_index,
            );

            section.function(&adapter.adapt());
        }

        module.section(&section);
    }
}

impl PartialEq for AdaptedModule<'_> {
    fn eq(&self, rhs: &Self) -> bool {
        std::ptr::eq(self.module, rhs.module)
    }
}

impl Eq for AdaptedModule<'_> {}

impl std::hash::Hash for AdaptedModule<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.module, state);
    }
}

pub struct ModuleShim<'a> {
    module: &'a Module<'a>,
}

impl<'a> ModuleShim<'a> {
    pub fn new(module: &'a Module) -> Self {
        Self { module }
    }

    pub fn encode(&self) -> Option<wasm_encoder::Module> {
        if !self.module.must_adapt() {
            return None;
        }

        let interface = self.module.interface.as_ref()?;
        let mut type_map = HashMap::new();
        let mut types = wasm_encoder::TypeSection::new();
        let mut functions = wasm_encoder::FunctionSection::new();
        let mut tables = wasm_encoder::TableSection::new();
        let mut exports = wasm_encoder::ExportSection::new();
        let mut code = wasm_encoder::CodeSection::new();

        let mut index = 0u32;
        for (func_index, (f, info)) in interface.iter().enumerate() {
            let type_index = type_map.entry(&info.import_type).or_insert_with(|| {
                types.function(
                    info.import_type.params.iter().map(to_val_type),
                    info.import_type.returns.iter().map(to_val_type),
                );
                let i = index;
                index += 1;
                i
            });

            functions.function(*type_index);

            exports.export(
                f.name.as_str(),
                wasm_encoder::Export::Function(func_index as u32),
            );

            let mut func = wasm_encoder::Function::new(
                info.import_type
                    .params
                    .iter()
                    .enumerate()
                    .map(|(index, ty)| (index as u32, to_val_type(ty))),
            );

            for i in 0..info.import_type.params.len() as u32 {
                func.instruction(wasm_encoder::Instruction::LocalGet(i));
            }

            func.instruction(wasm_encoder::Instruction::I32Const(func_index as i32));
            func.instruction(wasm_encoder::Instruction::CallIndirect {
                ty: *type_index,
                table: 0,
            });

            func.instruction(wasm_encoder::Instruction::End);

            code.function(&func);
        }

        let funcs_len = interface.inner().functions.len() as u32;

        tables.table(wasm_encoder::TableType {
            element_type: wasm_encoder::ValType::FuncRef,
            limits: wasm_encoder::Limits {
                min: funcs_len,
                max: Some(funcs_len),
            },
        });

        exports.export(FUNCTION_TABLE_NAME, wasm_encoder::Export::Table(0));

        let mut module = wasm_encoder::Module::new();
        module.section(&types);
        module.section(&functions);
        module.section(&tables);
        module.section(&exports);
        module.section(&code);

        Some(module)
    }
}
