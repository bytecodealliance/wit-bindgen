use self::generator::CodeGenerator;
use crate::{linker::to_val_type, Module};
use anyhow::{anyhow, bail, Result};
use std::{collections::HashMap, fmt, rc::Rc};
use wasmparser::{ExternalKind, FuncType, Type};
use witx::{CallMode, Function, WasmType};

mod generator;

pub const PARENT_MODULE_NAME: &str = "$parent";
const MEMORY_EXPORT_NAME: &str = "memory";
pub const MALLOC_EXPORT_NAME: &str = "witx_malloc";
const FREE_EXPORT_NAME: &str = "witx_free";
pub const FUNCTION_TABLE_NAME: &str = "$funcs";

lazy_static::lazy_static! {
    pub static ref MALLOC_FUNC_TYPE: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32]),
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

fn from_witx_type(ty: &WasmType) -> Type {
    match ty {
        WasmType::I32 => Type::I32,
        WasmType::I64 => Type::I64,
        WasmType::F32 => Type::F32,
        WasmType::F64 => Type::F64,
    }
}

#[derive(Debug)]
struct AdaptedFunction {
    ty: FuncType,
    function: Rc<Function>,
}

struct AdaptionState<'a> {
    module: wasm_encoder::Module,
    type_map: HashMap<FuncType, u32>,
    instance_map: HashMap<Option<&'a str>, u32>,
    parent_malloc_index: Option<u32>,
    malloc_index: Option<u32>,
    free_index: Option<u32>,
}

#[derive(Debug)]
pub struct ModuleAdapter<'a> {
    pub(crate) module: &'a Module<'a>,
    adapted_funcs: Vec<AdaptedFunction>,
}

impl PartialEq for ModuleAdapter<'_> {
    fn eq(&self, rhs: &Self) -> bool {
        std::ptr::eq(self.module, rhs.module)
    }
}

impl Eq for ModuleAdapter<'_> {}

impl std::hash::Hash for ModuleAdapter<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.module, state);
    }
}

impl<'a> ModuleAdapter<'a> {
    pub fn new(module: &'a Module) -> Self {
        let adapted_funcs = module
            .interface
            .as_ref()
            .and_then(|m| {
                Some(
                    m.funcs()
                        .map(|f| {
                            let signature = f.wasm_signature(CallMode::DeclaredExport);

                            let params = signature
                                .params
                                .iter()
                                .map(from_witx_type)
                                .collect::<Vec<_>>()
                                .into_boxed_slice();
                            let returns = signature
                                .results
                                .iter()
                                .map(from_witx_type)
                                .collect::<Vec<_>>()
                                .into_boxed_slice();

                            AdaptedFunction {
                                ty: FuncType { params, returns },
                                function: f,
                            }
                        })
                        .collect(),
                )
            })
            .unwrap_or_default();

        Self {
            module,
            adapted_funcs,
        }
    }

    pub fn adapted_funcs(&self) -> impl Iterator<Item = &str> {
        self.adapted_funcs.iter().map(|f| f.function.name.as_str())
    }

    pub fn adapt(&self) -> Result<wasm_encoder::Module> {
        if self.adapted_funcs.is_empty() {
            return Ok(self.module.encode());
        }

        self.validate()?;

        let mut state = AdaptionState {
            module: wasm_encoder::Module::new(),
            type_map: HashMap::new(),
            instance_map: HashMap::new(),
            parent_malloc_index: None,
            malloc_index: None,
            free_index: None,
        };

        self.write_type_section(&mut state);
        self.write_import_section(&mut state);
        self.write_module_section(&mut state);
        self.write_instance_section(&mut state);
        self.write_alias_section(&mut state);
        self.write_function_section(&mut state);
        self.write_export_section(&mut state);
        self.write_code_section(&mut state);

        // TODO: write a names section for the adapted module?

        Ok(state.module)
    }

    pub fn shim(&self) -> Option<wasm_encoder::Module> {
        if self.adapted_funcs.is_empty() {
            return None;
        }

        let mut type_map = HashMap::new();
        let mut types = wasm_encoder::TypeSection::new();
        let mut functions = wasm_encoder::FunctionSection::new();
        let mut tables = wasm_encoder::TableSection::new();
        let mut exports = wasm_encoder::ExportSection::new();
        let mut code = wasm_encoder::CodeSection::new();

        let mut index = 0u32;
        for (func_index, f) in self.adapted_funcs.iter().enumerate() {
            let type_index = type_map.entry(&f.ty).or_insert_with(|| {
                types.function(
                    f.ty.params.iter().map(to_val_type),
                    f.ty.returns.iter().map(to_val_type),
                );
                let i = index;
                index += 1;
                i
            });

            functions.function(*type_index);

            exports.export(
                f.function.name.as_str(),
                wasm_encoder::Export::Function(func_index as u32),
            );

            let mut func = wasm_encoder::Function::new(
                f.ty.params
                    .iter()
                    .enumerate()
                    .map(|(index, ty)| (index as u32, to_val_type(ty))),
            );

            for i in 0..f.ty.params.len() as u32 {
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

        tables.table(wasm_encoder::TableType {
            element_type: wasm_encoder::ValType::FuncRef,
            limits: wasm_encoder::Limits {
                min: self.adapted_funcs.len() as u32,
                max: Some(self.adapted_funcs.len() as u32),
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
                MALLOC_EXPORT_NAME,
                ExpectedExportType::Function(&MALLOC_FUNC_TYPE),
                false,
            ),
            (
                FREE_EXPORT_NAME,
                ExpectedExportType::Function(&FREE_FUNC_TYPE),
                false,
            ),
        ];

        expected.extend(self.adapted_funcs.iter().map(|f| {
            (
                f.function.name.as_str(),
                ExpectedExportType::Function(&f.ty),
                false,
            )
        }));

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

    fn write_type_section(&self, state: &mut AdaptionState<'a>) {
        let mut section = wasm_encoder::TypeSection::new();

        let mut type_index = 0;
        for ty in self
            .module
            .imports
            .iter()
            .map(|i| {
                self.module
                    .import_func_type(i)
                    .expect("expected import to be a function")
            })
            .chain(self.adapted_funcs.iter().map(|f| &f.ty))
            .chain(std::iter::once(&MALLOC_FUNC_TYPE as &FuncType))
        {
            state.type_map.entry(ty.clone()).or_insert_with(|| {
                section.function(
                    ty.params.iter().map(to_val_type),
                    ty.returns.iter().map(to_val_type),
                );

                let index = type_index;
                type_index += 1;
                index
            });
        }

        state.module.section(&section);
    }

    fn write_import_section(&self, state: &mut AdaptionState<'a>) {
        let mut section = wasm_encoder::ImportSection::new();

        for import in &self.module.imports {
            let ty = self
                .module
                .import_func_type(import)
                .expect("import should be a function");

            let len = state.instance_map.len();
            state
                .instance_map
                .entry(Some(import.module))
                .or_insert(len as u32);

            section.import(
                import.module,
                import.field,
                wasm_encoder::EntityType::Function(state.type_map[ty]),
            );
        }

        state
            .instance_map
            .insert(Some(PARENT_MODULE_NAME), state.instance_map.len() as u32);

        // Add an import for the parent's memory
        section.import(
            PARENT_MODULE_NAME,
            Some("memory"),
            wasm_encoder::EntityType::Memory(wasm_encoder::MemoryType {
                limits: wasm_encoder::Limits { min: 0, max: None },
            }),
        );

        // Add an import for the parent's malloc (index 0)
        section.import(
            PARENT_MODULE_NAME,
            Some(MALLOC_EXPORT_NAME),
            wasm_encoder::EntityType::Function(state.type_map[&MALLOC_FUNC_TYPE as &FuncType]),
        );

        state.parent_malloc_index = Some(self.module.imports.len() as u32);

        state.module.section(&section);
    }

    fn write_module_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::ModuleSection::new();
        section.module(&self.module.encode());
        state.module.section(&section);
    }

    fn write_instance_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::InstanceSection::new();

        let args: Vec<_> = state
            .instance_map
            .iter()
            .filter_map(|(name, index)| {
                name.and_then(|name| match name {
                    PARENT_MODULE_NAME => None,
                    _ => Some((name, wasm_encoder::Export::Instance(*index))),
                })
            })
            .collect();

        section.instantiate(0, args);
        state
            .instance_map
            .insert(None, state.instance_map.len() as u32);

        state.module.section(&section);
    }

    fn write_alias_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::AliasSection::new();

        let instance = state.instance_map[&None];

        section.instance_export(instance, wasm_encoder::ItemKind::Memory, MEMORY_EXPORT_NAME);

        section.instance_export(
            instance,
            wasm_encoder::ItemKind::Function,
            MALLOC_EXPORT_NAME,
        );
        state.malloc_index = Some(state.parent_malloc_index.unwrap() + 1);

        section.instance_export(instance, wasm_encoder::ItemKind::Function, FREE_EXPORT_NAME);
        state.free_index = Some(state.malloc_index.unwrap() + 1);

        // Add the adapted function aliases
        for f in &self.adapted_funcs {
            section.instance_export(
                instance,
                wasm_encoder::ItemKind::Function,
                f.function.name.as_str(),
            );
        }

        state.module.section(&section);
    }

    fn write_function_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::FunctionSection::new();

        // Add the adapted functions
        for f in &self.adapted_funcs {
            section.function(state.type_map[&f.ty]);
        }

        state.module.section(&section);
    }

    fn write_export_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::ExportSection::new();

        let start_index = state.free_index.unwrap() + self.adapted_funcs.len() as u32 + 1;

        for (index, f) in self.adapted_funcs.iter().enumerate() {
            section.export(
                f.function.name.as_str(),
                wasm_encoder::Export::Function(start_index + index as u32),
            );
        }

        state.module.section(&section);
    }

    fn write_code_section(&self, state: &mut AdaptionState) {
        let mut section = wasm_encoder::CodeSection::new();

        let parent_malloc_index = state.parent_malloc_index.unwrap();
        let malloc_index = state.malloc_index.unwrap();
        let free_index = state.free_index.unwrap();

        for (index, f) in self.adapted_funcs.iter().enumerate() {
            let mut generator = CodeGenerator::new(
                &f.function,
                index as u32 + free_index + 1,
                parent_malloc_index,
                malloc_index,
                free_index,
            );

            f.function.call(
                self.module.interface.as_ref().unwrap().name(),
                CallMode::DeclaredExport,
                &mut generator,
            );

            section.function(&generator.into_function());
        }

        state.module.section(&section);
    }
}
