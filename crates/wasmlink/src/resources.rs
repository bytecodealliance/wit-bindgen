use crate::{
    adapter::{FUNCTION_TABLE_NAME, RUNTIME_MODULE_NAME},
    linker::to_val_type,
    module::export_kind,
    Module,
};
use anyhow::{anyhow, bail, Result};
use heck::SnakeCase;
use std::collections::{BTreeMap, HashMap};
use wasm_encoder::EntityType;
use wasmparser::{ExternalKind, FuncType, Type};

const RESOURCE_INSERT_FUNC_NAME: &str = "resource_insert";
const RESOURCE_GET_FUNC_NAME: &str = "resource_get";
const RESOURCE_CLONE_FUNC_NAME: &str = "resource_clone";
const RESOURCE_REMOVE_FUNC_NAME: &str = "resource_remove";

lazy_static::lazy_static! {
    static ref FT_PI32_RI32: FuncType = {
        FuncType {
            params: Box::new([Type::I32]),
            returns: Box::new([Type::I32])
        }
    };
    static ref FT_PI32: FuncType = {
        FuncType {
            params: Box::new([Type::I32]),
            returns: Box::new([])
        }
    };
    static ref FT_PI32_PI32_RI32: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32]),
            returns: Box::new([Type::I32])
        }
    };
    static ref FT_PI32_PI32_RI64: FuncType = {
        FuncType {
            params: Box::new([Type::I32, Type::I32]),
            returns: Box::new([Type::I64])
        }
    };
}

struct ResourceFunction {
    name: String,
    ty: &'static FuncType,
    type_index: u32,
    exported: bool,
}

struct Resource<'a> {
    inner: &'a witx2::Resource,
    id: u32,
    new: ResourceFunction,
    get: ResourceFunction,
    clone: ResourceFunction,
    drop: ResourceFunction,
    drop_callback: String,
}

impl Resource<'_> {
    fn funcs(&self) -> impl ExactSizeIterator<Item = &ResourceFunction> {
        // This is used to define a stable order of the functions
        let funcs = [&self.new, &self.get, &self.clone, &self.drop];
        std::array::IntoIter::new(funcs)
    }
}

/// Represents the canonical ABI implementation for resources in an adapted module.
pub struct Resources<'a> {
    module: &'a Module<'a>,
    types: Vec<&'a FuncType>,
    imports: Vec<(&'a str, Option<&'a str>, wasm_encoder::EntityType)>,
    resources: Vec<Resource<'a>>,
    insert_index: Option<u32>,
    get_index: Option<u32>,
    clone_index: Option<u32>,
    remove_index: Option<u32>,
    drop_callback_type_index: Option<u32>,
}

impl<'a> Resources<'a> {
    pub fn new(module: &'a Module, next_resource_id: &mut u32) -> Self {
        let mut types = Vec::new();
        let mut imports = Vec::new();
        let mut resources = Vec::new();
        let mut insert_index = None;
        let mut get_index = None;
        let mut clone_index = None;
        let mut remove_index = None;
        let mut drop_callback_type_index = None;

        if module.has_resources {
            let mut type_map = HashMap::new();

            // Populate the types
            for ty in [
                &FT_PI32_RI32 as &FuncType,
                &FT_PI32,
                &FT_PI32_PI32_RI32,
                &FT_PI32_PI32_RI64,
            ] {
                type_map.entry(ty).or_insert_with(|| {
                    let index = types.len();
                    types.push(ty);
                    index as u32
                });
            }

            // Populate the imports
            insert_index = Some(imports.len() as u32);
            imports.push((
                RUNTIME_MODULE_NAME,
                Some(RESOURCE_INSERT_FUNC_NAME),
                EntityType::Function(type_map[&FT_PI32_PI32_RI32 as &FuncType]),
            ));
            get_index = Some(imports.len() as u32);
            imports.push((
                RUNTIME_MODULE_NAME,
                Some(RESOURCE_GET_FUNC_NAME),
                EntityType::Function(type_map[&FT_PI32_PI32_RI32 as &FuncType]),
            ));
            clone_index = Some(imports.len() as u32);
            imports.push((
                RUNTIME_MODULE_NAME,
                Some(RESOURCE_CLONE_FUNC_NAME),
                EntityType::Function(type_map[&FT_PI32_PI32_RI32 as &FuncType]),
            ));
            remove_index = Some(imports.len() as u32);
            imports.push((
                RUNTIME_MODULE_NAME,
                Some(RESOURCE_REMOVE_FUNC_NAME),
                EntityType::Function(type_map[&FT_PI32_PI32_RI64 as &FuncType]),
            ));

            let ft_pi32_ri32_index = type_map[&FT_PI32_RI32 as &FuncType];
            let ft_pi32_index = type_map[&FT_PI32 as &FuncType];

            drop_callback_type_index = Some(ft_pi32_index);

            for (_, resource) in module
                .interfaces
                .iter()
                .flat_map(|i| i.inner().resources.iter())
            {
                if resource.foreign_module.is_some() {
                    continue;
                }

                let name = resource.name.to_snake_case();

                resources.push(Resource {
                    inner: resource,
                    id: *next_resource_id,
                    new: ResourceFunction {
                        name: format!("resource_new_{}", name),
                        ty: &FT_PI32_RI32,
                        type_index: ft_pi32_ri32_index,
                        exported: false,
                    },
                    get: ResourceFunction {
                        name: format!("resource_get_{}", name),
                        ty: &FT_PI32_RI32,
                        type_index: ft_pi32_ri32_index,
                        exported: false,
                    },
                    clone: ResourceFunction {
                        name: format!("resource_clone_{}", name),
                        ty: &FT_PI32_RI32,
                        type_index: ft_pi32_ri32_index,
                        exported: true,
                    },
                    drop: ResourceFunction {
                        name: format!("resource_drop_{}", name),
                        ty: &FT_PI32,
                        type_index: ft_pi32_index,
                        exported: true,
                    },
                    drop_callback: format!("canonical_abi_drop_{}", name),
                });

                *next_resource_id += 1;
            }
        }

        Self {
            module,
            types,
            imports,
            resources,
            insert_index,
            get_index,
            clone_index,
            remove_index,
            drop_callback_type_index,
        }
    }

    pub fn exported_count(&self) -> u32 {
        self.resources.iter().fold(0, |v, r| {
            v + r.funcs().filter(|f| f.exported).count() as u32
        })
    }

    pub fn write_adapter_type_section(
        &self,
        types: &mut HashMap<&FuncType, u32>,
        section: &mut wasm_encoder::TypeSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        for ty in [&FT_PI32_PI32_RI32 as &FuncType, &FT_PI32_PI32_RI64] {
            let index = types.len() as u32;
            types.entry(ty).or_insert_with(|| {
                section.function(
                    ty.params.iter().map(to_val_type),
                    ty.returns.iter().map(to_val_type),
                );
                index
            });
        }
    }

    pub fn write_adapter_import_section(
        &self,
        types: &HashMap<&'a FuncType, u32>,
        num_imported_funcs: &mut u32,
        implicit_instances: &mut BTreeMap<&'a str, u32>,
        section: &mut wasm_encoder::ImportSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        for (name, ty) in [
            (RESOURCE_INSERT_FUNC_NAME, &FT_PI32_PI32_RI32 as &FuncType),
            (RESOURCE_GET_FUNC_NAME, &FT_PI32_PI32_RI32),
            (RESOURCE_CLONE_FUNC_NAME, &FT_PI32_PI32_RI32),
            (RESOURCE_REMOVE_FUNC_NAME, &FT_PI32_PI32_RI64),
        ] {
            *num_imported_funcs += 1;
            section.import(
                RUNTIME_MODULE_NAME,
                Some(name),
                wasm_encoder::EntityType::Function(types[ty]),
            );
        }

        let index = implicit_instances.len() as u32;
        implicit_instances
            .entry(RUNTIME_MODULE_NAME)
            .or_insert(index);
    }

    pub fn write_adapter_instance_section(
        &self,
        module_index: u32,
        implicit_instances: &BTreeMap<&'a str, u32>,
        section: &mut wasm_encoder::InstanceSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        section.instantiate(
            module_index,
            vec![(
                RUNTIME_MODULE_NAME,
                wasm_encoder::Export::Instance(
                    *implicit_instances.get(RUNTIME_MODULE_NAME).unwrap(),
                ),
            )],
        );
    }

    pub fn write_adapter_alias_section(
        &self,
        original_instance: u32,
        resources_instance: u32,
        num_aliased_funcs: &mut u32,
        mut start_index: u32,
        resource_functions: &mut HashMap<&'a str, (u32, u32)>,
        section: &mut wasm_encoder::AliasSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        section.instance_export(
            resources_instance,
            wasm_encoder::ItemKind::Table,
            FUNCTION_TABLE_NAME,
        );

        for r in &self.resources {
            let mut clone_index = None;
            let mut get_index = None;

            for f in r.funcs() {
                section.instance_export(
                    resources_instance,
                    wasm_encoder::ItemKind::Function,
                    &f.name,
                );

                if std::ptr::eq(f, &r.clone) {
                    clone_index = Some(start_index);
                } else if std::ptr::eq(f, &r.get) {
                    get_index = Some(start_index);
                }

                *num_aliased_funcs += 1;
                start_index += 1;
            }

            resource_functions.insert(&r.inner.name, (clone_index.unwrap(), get_index.unwrap()));

            section.instance_export(
                original_instance,
                wasm_encoder::ItemKind::Function,
                &r.drop_callback,
            );

            *num_aliased_funcs += 1;
            start_index += 1;
        }
    }

    pub fn write_adapter_export_section(
        &self,
        mut start_index: u32,
        section: &mut wasm_encoder::ExportSection,
    ) {
        for r in &self.resources {
            for f in r.funcs() {
                if f.exported {
                    section.export(&f.name, wasm_encoder::Export::Function(start_index));
                }
                start_index += 1;
            }

            // Account for the drop callback
            start_index += 1;
        }
    }

    pub fn write_adapter_element_section(
        &self,
        mut start_index: u32,
        section: &mut wasm_encoder::ElementSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        let mut elements = Vec::new();

        // Add every drop callback
        for r in &self.resources {
            start_index += r.funcs().count() as u32;
            elements.push(wasm_encoder::Element::Func(start_index));
            start_index += 1;
        }

        section.active(
            Some(0),
            wasm_encoder::Instruction::I32Const(0),
            wasm_encoder::ValType::FuncRef,
            wasm_encoder::Elements::Expressions(&elements),
        );
    }

    pub fn write_shim_sections(
        &self,
        type_map: &mut HashMap<&FuncType, u32>,
        mut start_index: u32,
        types: &mut wasm_encoder::TypeSection,
        functions: &mut wasm_encoder::FunctionSection,
        exports: &mut wasm_encoder::ExportSection,
        code: &mut wasm_encoder::CodeSection,
    ) {
        if self.resources.is_empty() {
            return;
        }

        for ty in [&FT_PI32_RI32 as &FuncType, &FT_PI32] {
            let index = type_map.len() as u32;
            type_map.entry(ty).or_insert_with(|| {
                types.function(
                    ty.params.iter().map(to_val_type),
                    ty.returns.iter().map(to_val_type),
                );
                index
            });
        }

        for r in &self.resources {
            for f in r.funcs() {
                if !f.exported {
                    continue;
                }
                functions.function(type_map[f.ty]);
                exports.export(f.name.as_str(), wasm_encoder::Export::Function(start_index));

                let mut func = wasm_encoder::Function::new(std::iter::empty());

                for i in 0..f.ty.params.len() as u32 {
                    func.instruction(wasm_encoder::Instruction::LocalGet(i));
                }

                func.instruction(wasm_encoder::Instruction::I32Const(start_index as i32));
                func.instruction(wasm_encoder::Instruction::CallIndirect {
                    ty: type_map[f.ty],
                    table: 0,
                });

                func.instruction(wasm_encoder::Instruction::End);

                code.function(&func);

                start_index += 1;
            }
        }
    }

    pub fn aliases(&self) -> impl Iterator<Item = &str> {
        self.resources
            .iter()
            .flat_map(|r| r.funcs().filter(|f| f.exported).map(|f| f.name.as_str()))
    }

    pub fn encode(&self) -> Result<Option<wasm_encoder::Module>> {
        if self.resources.is_empty() {
            return Ok(None);
        }

        self.validate()?;

        let mut module = wasm_encoder::Module::new();

        self.write_type_section(&mut module);
        self.write_import_section(&mut module);
        self.write_function_section(&mut module);
        self.write_table_section(&mut module);
        self.write_export_section(&mut module);
        self.write_code_section(&mut module);

        // TODO: write a names section for the module?

        Ok(Some(module))
    }

    fn validate(&self) -> Result<()> {
        // Ensure all required resource drop callbacks are exported by the module
        for r in &self.resources {
            let export = self
                .module
                .exports
                .iter()
                .find(|e| e.field == r.drop_callback)
                .ok_or_else(|| {
                    anyhow!(
                        "module `{}` declares a resource named `{}` but does not export a function named `{}`",
                        self.module.name,
                        r.inner.name,
                        r.drop_callback
                    )
                })?;

            match export.kind {
                ExternalKind::Function => {
                    let ty = self
                        .module
                        .func_type(export.index)
                        .expect("function index must be in range");
                    if ty != &FT_PI32 as &FuncType {
                        bail!("unexpected function type for export `{}` from module `{}`: expected [i32] -> []", r.drop_callback, self.module.name);
                    }
                }
                _ => {
                    bail!(
                        "expected a function for export `{}` from module `{}` but found a {}",
                        r.drop_callback,
                        self.module.name,
                        export_kind(export.kind)
                    )
                }
            }
        }

        Ok(())
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

        for (module, name, ty) in &self.imports {
            section.import(module, *name, *ty);
        }

        module.section(&section);
    }

    fn write_function_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::FunctionSection::new();

        for r in &self.resources {
            for f in r.funcs() {
                section.function(f.type_index);
            }
        }

        module.section(&section);
    }

    fn write_table_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::TableSection::new();

        let count = self.resources.len() as u32;

        section.table(wasm_encoder::TableType {
            element_type: wasm_encoder::ValType::FuncRef,
            limits: wasm_encoder::Limits {
                min: count,
                max: Some(count),
            },
        });

        module.section(&section);
    }

    fn write_export_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::ExportSection::new();

        let mut index = self.imports.len() as u32;

        for r in &self.resources {
            for f in r.funcs() {
                section.export(&f.name, wasm_encoder::Export::Function(index));

                index += 1;
            }
        }

        section.export(FUNCTION_TABLE_NAME, wasm_encoder::Export::Table(0));

        module.section(&section);
    }

    fn write_code_section(&self, module: &mut wasm_encoder::Module) {
        let mut section = wasm_encoder::CodeSection::new();

        for (index, r) in self.resources.iter().enumerate() {
            for f in r.funcs() {
                let func = if std::ptr::eq(f, &r.new) {
                    self.emit_resource_new(r.id)
                } else if std::ptr::eq(f, &r.get) {
                    self.emit_resource_get(r.id)
                } else if std::ptr::eq(f, &r.clone) {
                    self.emit_resource_clone(r.id)
                } else if std::ptr::eq(f, &r.drop) {
                    self.emit_resource_drop(r.id, index as u32)
                } else {
                    unimplemented!()
                };

                section.function(&func);
            }
        }

        module.section(&section);
    }

    fn emit_resource_new(&self, id: u32) -> wasm_encoder::Function {
        use wasm_encoder::Instruction;

        let mut func = wasm_encoder::Function::new(std::iter::empty());

        func.instruction(Instruction::I32Const(id as i32));
        func.instruction(Instruction::LocalGet(0));
        func.instruction(Instruction::Call(self.insert_index.unwrap()));
        func.instruction(Instruction::End);
        func
    }

    fn emit_resource_get(&self, id: u32) -> wasm_encoder::Function {
        use wasm_encoder::Instruction;

        let mut func = wasm_encoder::Function::new(std::iter::empty());

        func.instruction(Instruction::I32Const(id as i32));
        func.instruction(Instruction::LocalGet(0));
        func.instruction(Instruction::Call(self.get_index.unwrap()));
        func.instruction(Instruction::End);
        func
    }

    fn emit_resource_clone(&self, id: u32) -> wasm_encoder::Function {
        use wasm_encoder::Instruction;

        let mut func = wasm_encoder::Function::new(std::iter::empty());

        func.instruction(Instruction::I32Const(id as i32));
        func.instruction(Instruction::LocalGet(0));
        func.instruction(Instruction::Call(self.clone_index.unwrap()));
        func.instruction(Instruction::End);
        func
    }

    fn emit_resource_drop(&self, id: u32, drop_callback_index: u32) -> wasm_encoder::Function {
        use wasm_encoder::Instruction;

        let mut func =
            wasm_encoder::Function::new(std::iter::once((1, wasm_encoder::ValType::I64)));

        func.instruction(Instruction::I32Const(id as i32));
        func.instruction(Instruction::LocalGet(0));
        func.instruction(Instruction::Call(self.remove_index.unwrap()));

        // The function returns a 64-bit value where:
        // * The high-order 32-bits is non-zero if the resource is still alive or 0 if it should drop.
        // * The low-order 32-bits is zero if the resource is still alive or
        //   the original resource value to pass to the drop callback if it is being dropped.

        func.instruction(Instruction::LocalTee(1));

        // Check the higher 32-bits to see if the resource is still alive
        func.instruction(Instruction::I64Const(32));
        func.instruction(Instruction::I64ShrU);
        func.instruction(Instruction::I32WrapI64);
        func.instruction(Instruction::BrIf(0));

        // At this point the resource is being dropped, mask the lower 32-bits and pass to
        // the drop callback referenced via the callback table
        func.instruction(Instruction::LocalGet(1));
        func.instruction(Instruction::I32WrapI64);
        func.instruction(wasm_encoder::Instruction::I32Const(
            drop_callback_index as i32,
        ));
        func.instruction(wasm_encoder::Instruction::CallIndirect {
            ty: self.drop_callback_type_index.unwrap(),
            table: 0,
        });
        func.instruction(Instruction::End);
        func
    }
}
