//! Testing the encoding of components.

use anyhow::{bail, Context, Result};
use pretty_assertions::assert_eq;
use std::{fs, path::Path};
use wit_component::ComponentEncoder;
use wit_parser::Interface;

fn read_interface(path: &Path) -> Result<Interface> {
    wit_parser::Interface::parse_file(&path)
        .with_context(|| format!("failed to parse interface file `{}`", path.display()))
}

fn read_interfaces(dir: &Path, pattern: &str) -> Result<Vec<Interface>> {
    glob::glob(dir.join(pattern).to_str().unwrap())?
        .map(|p| {
            let p = p?;
            let mut i = read_interface(&p)?;
            i.name = p
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_start_matches("import-")
                .trim_start_matches("export-")
                .to_string();
            Ok(i)
        })
        .collect::<Result<_>>()
}

#[test]
fn component_encoding() -> Result<()> {
    for entry in fs::read_dir("tests/components")? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }

        let test_case = path.file_stem().unwrap().to_str().unwrap();

        let module_path = path.join("module.wat");
        let interface_path = path.join("default.wit");
        let component_path = path.join("component.wat");
        let error_path = path.join("error.txt");

        let module = wat::parse_file(&module_path)
            .with_context(|| format!("expected file `{}`", module_path.display()))?;
        let interface = interface_path
            .is_file()
            .then(|| read_interface(&interface_path))
            .transpose()?;
        let imports = read_interfaces(&path, "import-*.wit")?;
        let exports = read_interfaces(&path, "export-*.wit")?;

        let mut encoder = ComponentEncoder::default()
            .module(&module)
            .imports(&imports)
            .exports(&exports)
            .validate(true);

        if let Some(interface) = &interface {
            encoder = encoder.interface(interface);
        }

        let r = encoder.encode();
        let (output, baseline_path) = if error_path.is_file() {
            match r {
                Ok(_) => bail!("encoding should fail for test case `{}`", test_case),
                Err(e) => (e.to_string(), &error_path),
            }
        } else {
            (
                wasmprinter::print_bytes(&r?).with_context(|| {
                    format!(
                        "failed to print component bytes for test case `{}`",
                        test_case
                    )
                })?,
                &component_path,
            )
        };

        if std::env::var_os("BLESS").is_some() {
            fs::write(&baseline_path, output)?;
        } else {
            assert_eq!(
                output,
                fs::read_to_string(&baseline_path)?.replace("\r\n", "\n"),
                "failed baseline comparison for test case `{}` ({})",
                test_case,
                baseline_path.display(),
            );
        }
    }

    Ok(())
}
