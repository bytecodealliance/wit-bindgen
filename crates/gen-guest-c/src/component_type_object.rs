use anyhow::{Context, Result};
use wasm_encoder::{
    CodeSection, CustomSection, Encode, Function, FunctionSection, Module, TypeSection,
};
use wit_bindgen_core::{wit_parser::Interface, Direction};
use wit_component::InterfaceEncoder;

pub fn linking_symbol(iface: &Interface, direction: Direction) -> String {
    format!(
        "__component_type_object_force_link_{}_{}",
        iface.name,
        match direction {
            Direction::Import => "import",
            Direction::Export => "export",
        }
    )
}

pub fn object(iface: &Interface, direction: Direction) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // Build a module with one function that's a "dummy function"
    let mut types = TypeSection::new();
    types.function([], []);
    module.section(&types);
    let mut funcs = FunctionSection::new();
    funcs.function(0);
    module.section(&funcs);
    let mut code = CodeSection::new();
    code.function(&Function::new([]));
    module.section(&code);

    let name = format!(
        "component-type:{}:{}",
        match direction {
            Direction::Import => "import",
            Direction::Export => "export",
        },
        iface.name
    );
    let data = InterfaceEncoder::new(iface)
        .encode()
        .with_context(|| format!("translating interface {} to component type", iface.name))?;
    // Add our custom section
    module.section(&CustomSection {
        name: &name,
        data: data.as_slice(),
    });

    // Append the `.linking` section
    let mut data = Vec::new();
    data.push(0x02); // version 2
    {
        let mut subsection = Vec::<u8>::new();
        subsection.push(0x01); // syminfo count
        subsection.push(0x00); // SYMTAB_FUNCTION
        0u32.encode(&mut subsection); // flags
        0u32.encode(&mut subsection); // index
        linking_symbol(iface, direction).encode(&mut subsection); // name

        data.push(0x08); // `WASM_SYMBOL_TABLE`
        subsection.encode(&mut data);
    }
    module.section(&CustomSection {
        name: "linking",
        data: &data,
    });

    Ok(module.finish())
}
