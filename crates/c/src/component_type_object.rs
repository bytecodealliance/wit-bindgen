use anyhow::Result;
use heck::ToSnakeCase;
use wasm_encoder::{
    CodeSection, CustomSection, Function, FunctionSection, LinkingSection, Module, SymbolTable,
    TypeSection,
};
use wit_bindgen_core::wit_parser::{Resolve, WorldId};
use wit_component::StringEncoding;

pub fn linking_symbol(name: &str) -> String {
    let snake = name.to_snake_case();
    format!("__component_type_object_force_link_{snake}")
}

pub fn object(
    resolve: &Resolve,
    world: WorldId,
    world_name: &str,
    encoding: StringEncoding,
    suffix: Option<&str>,
) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // Build a module with one function that's a "dummy function"
    let mut types = TypeSection::new();
    types.ty().function([], []);
    module.section(&types);
    let mut funcs = FunctionSection::new();
    funcs.function(0);
    module.section(&funcs);
    let mut code = CodeSection::new();
    let mut func = Function::new([]);
    func.instruction(&wasm_encoder::Instruction::End);
    code.function(&func);
    module.section(&code);

    let mut producers = wasm_metadata::Producers::empty();
    producers.add(
        "processed-by",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );
    let data = wit_component::metadata::encode(resolve, world, encoding, Some(&producers)).unwrap();

    // The custom section name here must start with "component-type" but
    // otherwise is attempted to be unique here to ensure that this doesn't get
    // concatenated to other custom sections by LLD by accident since LLD will
    // concatenate custom sections of the same name.
    let section_name = format!("component-type:{world_name}{}", suffix.unwrap_or(""));

    // Add our custom section
    module.section(&CustomSection {
        name: std::borrow::Cow::Borrowed(&section_name),
        data: std::borrow::Cow::Borrowed(data.as_slice()),
    });

    // Append the linking section, so that lld knows the custom section's symbol name
    let mut linking = LinkingSection::new();
    let mut symbols = SymbolTable::new();
    symbols.function(0, 0, Some(&linking_symbol(world_name)));
    linking.symbol_table(&symbols);
    module.section(&linking);

    Ok(module.finish())
}
