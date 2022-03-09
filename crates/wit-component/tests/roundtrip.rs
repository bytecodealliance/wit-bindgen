//! Testing the round tripping of interfaces to component encodings and back.

use anyhow::{Context, Result};
use std::fs;
use wit_component::{decode_interface_component, ComponentEncoder, InterfacePrinter};
use wit_parser::Interface;

#[test]
fn roundtrip_interfaces() -> Result<()> {
    for file in fs::read_dir("tests/wit")? {
        let file = file?;
        let interface = Interface::parse_file(file.path())?;

        let encoder = ComponentEncoder::default()
            .interface(&interface)
            .types_only(true);

        let bytes = encoder.encode().with_context(|| {
            format!(
                "failed to encode a component from interface `{}`",
                file.path().display()
            )
        })?;

        let interface = decode_interface_component(&bytes)?;

        let mut printer = InterfacePrinter::default();
        let output = printer.print(&interface)?;

        if std::env::var_os("BLESS").is_some() {
            fs::write(file.path(), output)?;
        } else {
            assert_eq!(
                output,
                fs::read_to_string(file.path())?.replace("\r\n", "\n")
            );
        }
    }

    Ok(())
}
