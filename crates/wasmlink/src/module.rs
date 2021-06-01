use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use wasmparser::{
    Chunk, Export, ExternalKind, FuncType, Import, ImportSectionEntryType, Parser, Payload, Range,
    SectionReader, TypeDef, Validator,
};

const INTERFACE_SECTION_NAME: &str = ".interface";

fn import_kind(ty: ImportSectionEntryType) -> &'static str {
    match ty {
        ImportSectionEntryType::Function(_) => "function",
        ImportSectionEntryType::Table(_) => "table",
        ImportSectionEntryType::Memory(_) => "memory",
        ImportSectionEntryType::Event(_) => {
            unimplemented!("event imports are not implemented")
        }
        ImportSectionEntryType::Global(_) => "global",
        ImportSectionEntryType::Module(_) => "module",
        ImportSectionEntryType::Instance(_) => "instance",
    }
}

fn export_kind(kind: ExternalKind) -> &'static str {
    match kind {
        ExternalKind::Function => "function",
        ExternalKind::Table => "table",
        ExternalKind::Memory => "memory",
        ExternalKind::Event => unimplemented!("event exports are not implemented"),
        ExternalKind::Global => "global",
        ExternalKind::Type => unimplemented!("type exports are not implemented"),
        ExternalKind::Module => "module",
        ExternalKind::Instance => "instance",
    }
}

/// Represents a parsed WebAssembly module.
#[derive(Debug)]
pub struct Module<'a> {
    /// The name of the parsed module.
    pub name: &'a str,
    /// The bytes of the parsed module.
    pub bytes: &'a [u8],
    pub(crate) types: Vec<TypeDef<'a>>,
    pub(crate) imports: Vec<Import<'a>>,
    pub(crate) exports: Vec<Export<'a>>,
    functions: Vec<u32>,
    sections: Vec<wasm_encoder::RawSection<'a>>,
    pub(crate) interface: Option<witx::Module>,
}

impl<'a> Module<'a> {
    /// Constructs a new WebAssembly module from a name and the module's bytes.
    pub fn new(name: &'a str, bytes: &'a [u8]) -> Result<Self> {
        let mut module = Self {
            name,
            bytes,
            types: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            functions: Vec::new(),
            sections: Vec::new(),
            interface: None,
        };
        module.parse()?;
        Ok(module)
    }

    fn add_section(&mut self, id: wasm_encoder::SectionId, range: Range) {
        self.sections.push(wasm_encoder::RawSection {
            id: id as u8,
            data: &self.bytes[range.start..range.end],
        });
    }

    fn parse(&mut self) -> Result<()> {
        let mut parser = Parser::new(0);
        let mut validator = Validator::new();

        let mut data = self.bytes;
        loop {
            let payload = match parser.parse(data, true)? {
                Chunk::NeedMoreData(_) => unreachable!(),
                Chunk::Parsed { payload, consumed } => {
                    data = &data[consumed..];
                    payload
                }
            };

            match payload {
                Payload::Version { num, range } => validator.version(num, &range)?,
                Payload::TypeSection(types) => {
                    validator.type_section(&types)?;
                    self.add_section(wasm_encoder::SectionId::Type, types.range());

                    for ty in types {
                        let ty = ty?;
                        self.types.push(ty);
                    }
                }
                Payload::ImportSection(imports) => {
                    validator.import_section(&imports)?;
                    self.add_section(wasm_encoder::SectionId::Import, imports.range());

                    self.imports.reserve(imports.get_count() as usize);

                    for import in imports {
                        self.imports.push(import?);
                    }
                }
                Payload::AliasSection(_) => {
                    bail!("module is already linked as it contains an alias section")
                }
                Payload::InstanceSection(_) => {
                    bail!("module is already linked as it contains an instance section")
                }
                Payload::FunctionSection(functions) => {
                    validator.function_section(&functions)?;
                    self.add_section(wasm_encoder::SectionId::Function, functions.range());

                    self.functions.reserve(functions.get_count() as usize);
                    for f in functions {
                        self.functions.push(f?);
                    }
                }
                Payload::TableSection(tables) => {
                    validator.table_section(&tables)?;
                    self.add_section(wasm_encoder::SectionId::Table, tables.range())
                }
                Payload::MemorySection(memories) => {
                    validator.memory_section(&memories)?;
                    self.add_section(wasm_encoder::SectionId::Memory, memories.range())
                }
                Payload::EventSection(_) => bail!("module contains unsupported event section"),
                Payload::GlobalSection(globals) => {
                    validator.global_section(&globals)?;
                    self.add_section(wasm_encoder::SectionId::Global, globals.range())
                }
                Payload::ExportSection(exports) => {
                    validator.export_section(&exports)?;
                    self.add_section(wasm_encoder::SectionId::Export, exports.range());

                    self.exports.reserve(exports.get_count() as usize);
                    for export in exports {
                        self.exports.push(export?);
                    }
                }
                Payload::StartSection { func, range } => {
                    validator.start_section(func, &range)?;
                    self.add_section(wasm_encoder::SectionId::Start, range);
                }
                Payload::ElementSection(elements) => {
                    validator.element_section(&elements)?;
                    self.add_section(wasm_encoder::SectionId::Element, elements.range());
                }
                Payload::DataCountSection { count, range } => {
                    validator.data_count_section(count, &range)?;
                    self.add_section(wasm_encoder::SectionId::DataCount, range)
                }
                Payload::DataSection(data) => {
                    validator.data_section(&data)?;
                    self.add_section(wasm_encoder::SectionId::Data, data.range())
                }
                Payload::CodeSectionStart {
                    count,
                    range,
                    size: _,
                } => {
                    validator.code_section_start(count, &range)?;
                    self.add_section(wasm_encoder::SectionId::Code, range)
                }
                Payload::CodeSectionEntry(body) => {
                    let mut validator = validator.code_section_entry()?;
                    validator.validate(&body)?;
                }
                Payload::ModuleSectionStart { .. } => {
                    bail!("module is already linked as it contains a module section")
                }
                Payload::ModuleSectionEntry { .. } => unreachable!(),
                Payload::CustomSection {
                    name, range, data, ..
                } => {
                    if name == INTERFACE_SECTION_NAME {
                        if self.interface.is_some() {
                            bail!("module contains multiple interface sections");
                        }
                        self.interface = Some(
                            witx::parse(
                                std::str::from_utf8(data)
                                    .map_err(|e| anyhow!("invalid interface section: {}", e))?,
                            )
                            .map_err(|e| anyhow!("failed to parse interface section: {}", e))?,
                        );
                    }
                    self.add_section(wasm_encoder::SectionId::Custom, range)
                }
                Payload::UnknownSection { id, .. } => {
                    bail!("unknown section with id `{}`", id)
                }
                Payload::End => break,
            }
        }

        Ok(())
    }

    /// Reads the module's interface from the given file path.
    ///
    /// If the module has an embedded interface definition, the external file is ignored.
    pub fn read_interface(&mut self, path: impl AsRef<Path>) -> Result<bool> {
        if self.interface.is_some() {
            return Ok(false);
        }

        let source = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "failed to read interface file `{}`",
                path.as_ref().display()
            )
        })?;

        self.interface = Some(
            witx::parse(&source)
                .map_err(|e| anyhow!("failed to parse interface section: {}", e))?,
        );

        Ok(true)
    }

    pub(crate) fn func_type(&self, index: u32) -> Option<&FuncType> {
        let ty = match self.imports.get(index as usize) {
            Some(import) => match &import.ty {
                ImportSectionEntryType::Function(ty) => *ty,
                _ => return None,
            },
            None => *self.functions.get(index as usize - self.imports.len())?,
        };

        self.types.get(ty as usize).and_then(|t| match t {
            TypeDef::Func(ft) => Some(ft),
            _ => None,
        })
    }

    pub(crate) fn import_func_type(&self, import: &Import) -> Option<&FuncType> {
        match import.ty {
            ImportSectionEntryType::Function(idx) => match self.types.get(idx as usize) {
                Some(TypeDef::Func(ty)) => Some(ty),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn resolve_import(&self, import: &Import, module: &Self) -> Result<()> {
        let export = self
            .exports
            .iter()
            .find(|e| Some(e.field) == import.field)
            .ok_or_else(|| {
                anyhow!(
                    "module `{}` does not export a {} named `{}`",
                    self.name,
                    import_kind(import.ty),
                    import.field.unwrap_or("")
                )
            })?;

        match (import.ty, export.kind) {
            (ImportSectionEntryType::Function(_), ExternalKind::Function) => {
                let compatible = match (
                    module.import_func_type(import),
                    self.func_type(export.index),
                ) {
                    (Some(i), Some(e)) => i == e,
                    _ => false,
                };

                if !compatible {
                    bail!(
                        "module `{}` imports function `{}` from module `{}` but the types are incompatible", module.name, import.field.unwrap_or(""), self.name
                    );
                }
            }
            (ImportSectionEntryType::Table(_), ExternalKind::Table) => {
                bail!("importing tables is not currently supported")
            }
            (ImportSectionEntryType::Memory(_), ExternalKind::Memory) => {
                bail!("importing memories is not currently supported")
            }
            (ImportSectionEntryType::Global(_), ExternalKind::Global) => {
                bail!("importing globals is not currently supported")
            }
            (ImportSectionEntryType::Module(_), ExternalKind::Module) => {
                bail!("importing modules is not currently supported")
            }
            (ImportSectionEntryType::Instance(_), ExternalKind::Instance) => {
                bail!("importing instances is not currently supported")
            }
            (ImportSectionEntryType::Event(_), _) => unreachable!(),
            (_, _) => bail!(
                "expected a {} for export `{}` from module `{}` but found a {}",
                import_kind(import.ty),
                import.field.unwrap_or(""),
                self.name,
                export_kind(export.kind)
            ),
        }

        Ok(())
    }

    pub(crate) fn encode(&self) -> wasm_encoder::Module {
        let mut module = wasm_encoder::Module::new();

        for section in &self.sections {
            module.section(section);
        }

        module
    }
}
