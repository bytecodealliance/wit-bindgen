//! Testing the encoding of interfaces.

use anyhow::{Context, Result};
use pretty_assertions::assert_eq;
use std::{ffi::OsStr, fs};
use wit_component::ComponentEncoder;
use wit_parser::Interface;

#[test]
fn interface_encoding() -> Result<()> {
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

        let output = wasmprinter::print_bytes(&bytes)?;
        let output_path = path.with_extension("wat");

        if std::env::var_os("BLESS").is_some() {
            fs::write(&output_path, output)?;
        } else {
            assert_eq!(
                output,
                fs::read_to_string(&output_path)?.replace("\r\n", "\n"),
                "encoding of wit file `{}` did not match the expected wat file `{}`",
                path.display(),
                output_path.display(),
            );
        }
    }

    Ok(())
}
