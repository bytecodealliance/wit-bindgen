//! Testing the encoding of components.

use anyhow::{Context, Result};
use std::{fs, path::Path};
use wit_component::ComponentEncoder;
use wit_parser::Interface;

fn read_interface(path: &Path) -> Result<Interface> {
    wit_parser::Interface::parse_file(&path)
        .with_context(|| format!("failed to parse interface file `{}`", path.display()))
}

fn read_interfaces(dir: &Path, pattern: &str) -> Result<Vec<Interface>> {
    glob::glob(dir.join(pattern).to_str().unwrap())?
        .map(|p| read_interface(&p?))
        .collect::<Result<_>>()
}

#[test]
fn component_encoding() -> Result<()> {
    for entry in fs::read_dir("tests/components")? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }

        let module_path = path.join("module.wat");
        let interface_path = path.join("default.wit");
        let component_path = path.join("component.wat");
        let module = wat::parse_file(&module_path)
            .with_context(|| format!("expected file `{}`", module_path.display()))?;
        let interface = interface_path
            .is_file()
            .then(|| read_interface(&interface_path))
            .transpose()?;
        let imports = read_interfaces(&path, "imports-*.wit")?;
        let exports = read_interfaces(&path, "exports-*.wit")?;

        let mut encoder = ComponentEncoder::default()
            .module(&module)
            .imports(&imports)
            .exports(&exports)
            .validate(true);

        if let Some(interface) = &interface {
            encoder = encoder.interface(interface);
        }

        let bytes = encoder.encode()?;
        let output = wasmprinter::print_bytes(&bytes)?;

        if std::env::var_os("BLESS").is_some() {
            fs::write(&component_path, output)?;
        } else {
            assert_eq!(
                output,
                fs::read_to_string(&component_path)?.replace("\r\n", "\n")
            );
        }
    }

    Ok(())
}
