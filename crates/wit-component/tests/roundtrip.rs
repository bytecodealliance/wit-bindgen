//! Testing the round tripping of interfaces to component encodings and back.

use anyhow::{Context, Result};
use pretty_assertions::assert_eq;
use std::{ffi::OsStr, fs};
use wit_component::{decode_interface_component, ComponentEncoder, InterfacePrinter};
use wit_parser::Interface;

#[test]
fn roundtrip_interfaces() -> Result<()> {
    for entry in fs::read_dir("tests/wit")? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) != Some("wit") {
            continue;
        }

        let interface = Interface::parse_file(&path)?;

        let encoder = ComponentEncoder::default()
            .interface(&interface)
            .types_only(true);

        let bytes = encoder.encode().with_context(|| {
            format!(
                "failed to encode a component from interface `{}`",
                path.display()
            )
        })?;

        let interface = decode_interface_component(&bytes)?;

        let mut printer = InterfacePrinter::default();
        let output = printer.print(&interface)?;

        if std::env::var_os("BLESS").is_some() {
            fs::write(&path, output)?;
        } else {
            assert_eq!(
                output,
                fs::read_to_string(&path)?.replace("\r\n", "\n"),
                "encoding of wit file `{}` did not match the the decoded interface",
                path.display(),
            );
        }
    }

    Ok(())
}
