use crate::{
    adapter::call::CallAdapter,
    linker::{to_val_type, CANONICAL_ABI_MODULE_NAME},
    resources::Resources,
    Module,
};
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use wasmparser::{FuncType, Type};

mod call;

pub const RUNTIME_MODULE_NAME: &str = "$runtime";
pub const PARENT_MODULE_NAME: &str = "$parent";
pub const MEMORY_EXPORT_NAME: &str = "memory";
pub const REALLOC_EXPORT_NAME: &str = "canonical_abi_realloc";
pub const FREE_EXPORT_NAME: &str = "canonical_abi_free";
pub const FUNCTION_TABLE_NAME: &str = "$funcs";

const ORIGINAL_MODULE_INDEX: u32 = 0;
const RESOURCES_SHIM_MODULE_INDEX: u32 = 1;

lazy_static::lazy_static! {
    pub static ref REALLOC_FUNC_TYPE: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32, Type::I32, Type::I32]),
            returns: Box::new([Type::I32])
        }
    };
    pub static ref FREE_FUNC_TYPE: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32, Type::I32]),
            returns: Box::new([])
        }
    };
}

/// Responsible for adapting a WebAssembly module.
pub struct ModuleAdapter<'a> {
    pub(crate) module: &'a Module<'a>,
    resources: Resources<'a>,
}

impl<'a> ModuleAdapter<'a> {
    /// Constructs a new adapter for the given module.
    pub fn new(module: &'a Module, next_resource_id: &mut u32) -> Self {
        Self {
            module,
            resources: Resources::new(module, next_resource_id),
        }
    }

    /// Adapts the module and returns the resulting encoded module.
    pub fn adapt(&self) -> Result<wasm_encoder::Module> {
        if !self.module.must_adapt {
            return Ok(self.module.encode());
        }

        let mut module = wasm_encoder::Module::new();
        let mut types = HashMap::new();
        let mut parent_realloc_index = None;
        let mut num_imported_funcs = 0;
        let mut num_aliased_funcs = 0;
        let mut num_adapted_func_aliases = 0;
        let mut num_defined_funcs = 0;
        let mut implicit_instances = BTreeMap::new();
        let mut resource_functions = HashMap::new();

        self.write_type_section(&mut module, &mut types);
        self.write_import_section(
            &mut module,
            &types,
            &mut num_imported_funcs,
            &mut parent_realloc_index,
            &mut implicit_instances,
        );
        self.write_module_section(&mut module)?;
        self.write_instance_section(&mut module, &implicit_instances);
        self.write_alias_section(
            &mut module,
            &implicit_instances,
            num_imported_funcs,
            &mut num_aliased_funcs,
            &mut num_adapted_func_aliases,
            &mut resource_functions,
        );
        self.write_function_section(&mut module, &types, &mut num_defined_funcs);
        self.write_export_section(
            &mut module,
            num_imported_funcs,
            num_aliased_funcs,
            num_adapted_func_aliases,
        );
        self.write_element_section(&mut module, num_imported_funcs, num_adapted_func_aliases);
        self.write_code_section(
            &mut module,
            parent_realloc_index,
            num_imported_funcs,
            &resource_functions,
        );

        // TODO: write a names section for the adapted module?

        Ok(module)
    }

    pub(crate) fn encode_shim(&self) -> Option<wasm_encoder::Module> {
        if !self.module.must_adapt {
            return None;
        }

        let mut type_map = HashMap::new();
        let mut types = wasm_encoder::TypeSection::new();
        let mut functions = wasm_encoder::FunctionSection::new();
        let mut tables = wasm_encoder::TableSection::new();
        let mut exports = wasm_encoder::ExportSection::new();
        let mut code = wasm_encoder::CodeSection::new();
        let mut func_count = 0;
        let mut index = 0u32;

        for interface in &self.module.interfaces {
            func_count += interface.inner().functions.len() as u32;

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

                let mut func = wasm_encoder::Function::new(std::iter::empty());

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
        }

        self.resources.write_shim_sections(
            &mut type_map,
            func_count,
            &mut types,
            &mut functions,
            &mut exports,
            &mut code,
        );

        let table_len = func_count + self.resources.exported_count();

        tables.table(wasm_encoder::TableType {
            element_type: wasm_encoder::ValType::FuncRef,
            limits: wasm_encoder::Limits {
                min: table_len,
                max: Some(table_len),
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

    pub(crate) fn aliases(&self) -> impl Iterator<Item = &str> {
        self.module
            .interfaces
            .iter()
            .flat_map(|i| i.iter())
            .map(|(f, _)| f.name.as_str())
            .chain(self.resources.aliases())
    }

    fn write_type_section(
        &self,
        module: &mut wasm_encoder::Module,
        types: &mut HashMap<&'a FuncType, u32>,
    ) {
        let mut section = wasm_encoder::TypeSection::new();

        for ty in self
            .module
            .imports
            .iter()
            .filter_map(|i| {
                // The adapter will implement canonical ABI imports
                if i.module == CANONICAL_ABI_MODULE_NAME {
                    None
                } else {
                    Some(
                        self.module
                            .import_func_type(i)
                            .expect("expected import to be a function"),
                    )
                }
            })
            .chain(
                self.module
                    .interfaces
                    .iter()
                    .flat_map(|i| i.iter())
                    .filter_map(|(_, info)| {
                        if info.must_adapt {
                            Some(&info.import_type)
                        } else {
                            None
                        }
                    }),
            )
        {
            let index = types.len() as u32;
            types.entry(ty).or_insert_with(|| {
                section.function(
                    ty.params.iter().map(to_val_type),
                    ty.returns.iter().map(to_val_type),
                );
                index
            });
        }

        if self.module.needs_memory_funcs {
            let index = types.len() as u32;
            types.entry(&REALLOC_FUNC_TYPE).or_insert_with(|| {
                section.function(
                    REALLOC_FUNC_TYPE.params.iter().map(to_val_type),
                    REALLOC_FUNC_TYPE.returns.iter().map(to_val_type),
                );
                index
            });
        }

        self.resources
            .write_adapter_type_section(types, &mut section);

        module.section(&section);
    }

    fn write_import_section(
        &self,
        module: &mut wasm_encoder::Module,
        types: &HashMap<&'a FuncType, u32>,
        num_imported_funcs: &mut u32,
        parent_realloc_index: &mut Option<u32>,
        implicit_instances: &mut BTreeMap<&'a str, u32>,
    ) {
        let mut section = wasm_encoder::ImportSection::new();

        for (import_module, import_field, entity) in self.module.imports.iter().filter_map(|i| {
            // The adapter will implement canonical ABI imports
            if i.module == CANONICAL_ABI_MODULE_NAME {
                None
            } else {
                let ty = self
                    .module
                    .import_func_type(i)
                    .expect("import should be a function");
                Some((
                    i.module,
                    i.field,
                    wasm_encoder::EntityType::Function(types[ty]),
                ))
            }
        }) {
            *num_imported_funcs += 1;

            section.import(import_module, import_field, entity);

            let index = implicit_instances.len() as u32;
            implicit_instances.entry(import_module).or_insert(index);
        }

        if self.module.needs_memory {
            section.import(
                PARENT_MODULE_NAME,
                Some(MEMORY_EXPORT_NAME),
                wasm_encoder::EntityType::Memory(wasm_encoder::MemoryType {
                    limits: wasm_encoder::Limits { min: 0, max: None },
                }),
            );

            let index = implicit_instances.len() as u32;
            implicit_instances
                .entry(PARENT_MODULE_NAME)
                .or_insert(index);
        }

        if self.module.needs_memory_funcs {
            *parent_realloc_index = Some(*num_imported_funcs);
            *num_imported_funcs += 1;

            section.import(
                PARENT_MODULE_NAME,
                Some(REALLOC_EXPORT_NAME),
                wasm_encoder::EntityType::Function(types[&REALLOC_FUNC_TYPE as &FuncType]),
            );

            let index = implicit_instances.len() as u32;
            implicit_instances
                .entry(PARENT_MODULE_NAME)
                .or_insert(index);
        }

        self.resources.write_adapter_import_section(
            types,
            num_imported_funcs,
            implicit_instances,
            &mut section,
        );

        module.section(&section);
    }

    fn write_module_section(&self, module: &mut wasm_encoder::Module) -> Result<()> {
        let mut section = wasm_encoder::ModuleSection::new();

        // Order here matters: write the original module before the resource module
        section.module(&self.module.encode());

        if let Some(resources) = self.resources.encode()? {
            section.module(&resources);
        }

        module.section(&section);

        Ok(())
    }

    fn write_instance_section(
        &self,
        module: &mut wasm_encoder::Module,
        implicit_instances: &BTreeMap<&'a str, u32>,
    ) {
        let mut section = wasm_encoder::InstanceSection::new();

        self.resources.write_adapter_instance_section(
            RESOURCES_SHIM_MODULE_INDEX,
            implicit_instances,
            &mut section,
        );

        let mut args: Vec<_> = implicit_instances
            .iter()
            .filter_map(|(name, index)| match *name {
                PARENT_MODULE_NAME | RUNTIME_MODULE_NAME => None,
                _ => Some((*name, wasm_encoder::Export::Instance(*index))),
            })
            .collect();

        if self.module.has_resources {
            args.push((
                CANONICAL_ABI_MODULE_NAME,
                wasm_encoder::Export::Instance(implicit_instances.len() as u32),
            ));
        }

        section.instantiate(ORIGINAL_MODULE_INDEX, args);

        module.section(&section);
    }

    fn write_alias_section(
        &self,
        module: &mut wasm_encoder::Module,
        implicit_instances: &BTreeMap<&'a str, u32>,
        num_imported_funcs: u32,
        num_aliased_funcs: &mut u32,
        num_adapted_func_aliases: &mut u32,
        resource_functions: &mut HashMap<&'a str, (u32, u32)>,
    ) {
        let mut section = wasm_encoder::AliasSection::new();

        let (original_instance, resources_instance) = if self.module.has_resources {
            (
                implicit_instances.len() as u32 + 1,
                Some(implicit_instances.len() as u32),
            )
        } else {
            (implicit_instances.len() as u32, None)
        };

        if self.module.needs_memory {
            section.instance_export(
                original_instance,
                wasm_encoder::ItemKind::Memory,
                MEMORY_EXPORT_NAME,
            );
        }

        // Order here matters: realloc, then free, then adapted functions
        if self.module.needs_memory_funcs {
            section.instance_export(
                original_instance,
                wasm_encoder::ItemKind::Function,
                REALLOC_EXPORT_NAME,
            );

            section.instance_export(
                original_instance,
                wasm_encoder::ItemKind::Function,
                FREE_EXPORT_NAME,
            );
        }

        // Add the adapted function aliases
        for (f, _) in self.module.interfaces.iter().flat_map(|i| i.iter()) {
            *num_aliased_funcs += 1;
            *num_adapted_func_aliases += 1;

            section.instance_export(
                original_instance,
                wasm_encoder::ItemKind::Function,
                f.name.as_str(),
            );
        }

        // Add resource-related aliases
        if let Some(resources_instance) = resources_instance {
            self.resources.write_adapter_alias_section(
                original_instance,
                resources_instance,
                num_aliased_funcs,
                num_imported_funcs
                    + *num_adapted_func_aliases
                    + if self.module.needs_memory_funcs { 2 } else { 0 },
                resource_functions,
                &mut section,
            );
        }

        module.section(&section);
    }

    fn write_function_section(
        &self,
        module: &mut wasm_encoder::Module,
        types: &HashMap<&'a FuncType, u32>,
        num_defined_funcs: &mut u32,
    ) {
        let mut section = wasm_encoder::FunctionSection::new();

        // Populate the adapted functions
        for (_, info) in self.module.interfaces.iter().flat_map(|i| i.iter()) {
            if !info.must_adapt {
                continue;
            }

            *num_defined_funcs += 1;
            section.function(types[&info.import_type]);
        }

        module.section(&section);
    }

    fn write_export_section(
        &self,
        module: &mut wasm_encoder::Module,
        num_imported_funcs: u32,
        num_aliased_funcs: u32,
        num_adapted_func_aliases: u32,
    ) {
        let mut section = wasm_encoder::ExportSection::new();

        if self.module.needs_memory {
            section.export(
                MEMORY_EXPORT_NAME,
                wasm_encoder::Export::Memory(call::ADAPTED_MEMORY_INDEX),
            );
        }

        let alias_start_index = if self.module.needs_memory_funcs {
            section.export(
                REALLOC_EXPORT_NAME,
                wasm_encoder::Export::Function(num_imported_funcs),
            );
            section.export(
                FREE_EXPORT_NAME,
                wasm_encoder::Export::Function(num_imported_funcs + 1),
            );
            num_imported_funcs + 2
        } else {
            num_imported_funcs
        };

        let defined_start_index = alias_start_index + num_aliased_funcs;
        let mut adapted_count = 0;

        for (index, (f, info)) in self
            .module
            .interfaces
            .iter()
            .flat_map(|i| i.iter())
            .enumerate()
        {
            section.export(
                f.name.as_str(),
                wasm_encoder::Export::Function(if info.must_adapt {
                    defined_start_index + adapted_count
                } else {
                    alias_start_index + index as u32
                }),
            );

            if info.must_adapt {
                adapted_count += 1;
            }
        }

        self.resources.write_adapter_export_section(
            alias_start_index + num_adapted_func_aliases,
            &mut section,
        );

        module.section(&section);
    }

    fn write_element_section(
        &self,
        module: &mut wasm_encoder::Module,
        num_imported_funcs: u32,
        num_adapted_func_aliases: u32,
    ) {
        if !self.module.has_resources {
            return;
        }

        let mut section = wasm_encoder::ElementSection::new();

        let alias_start_index =
            num_imported_funcs + if self.module.needs_memory_funcs { 2 } else { 0 };

        self.resources.write_adapter_element_section(
            alias_start_index + num_adapted_func_aliases,
            &mut section,
        );

        module.section(&section);
    }

    fn write_code_section(
        &self,
        module: &mut wasm_encoder::Module,
        parent_realloc_index: Option<u32>,
        num_imported_funcs: u32,
        resource_functions: &HashMap<&'a str, (u32, u32)>,
    ) {
        let mut section = wasm_encoder::CodeSection::new();

        // Realloc and free are the first functions aliased after imports
        let (realloc_index, free_index, alias_start_index) = if self.module.needs_memory_funcs {
            (
                Some(num_imported_funcs),
                Some(num_imported_funcs + 1),
                num_imported_funcs + 2,
            )
        } else {
            (None, None, num_imported_funcs)
        };

        for interface in &self.module.interfaces {
            if !interface.must_adapt {
                continue;
            }

            for (index, (func, info)) in interface.iter().enumerate() {
                if !info.must_adapt {
                    continue;
                }

                let adapter = CallAdapter::new(
                    interface,
                    &info.import_signature,
                    func,
                    alias_start_index + index as u32,
                    realloc_index,
                    free_index,
                    parent_realloc_index,
                    resource_functions,
                );

                section.function(&adapter.adapt());
            }
        }

        module.section(&section);
    }
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
