use anyhow::{bail, Context, Result};
use std::{env, ffi::OsStr, fs};
use wasmlink::{Module, ModuleAdapter};
use wasmprinter::print_bytes;
use wat::parse_file;

#[test]
fn wasmlink_file_tests() -> Result<()> {
    for entry in fs::read_dir("tests")? {
        let entry = entry?;

        let path = entry.path();

        match (
            path.file_stem().and_then(OsStr::to_str),
            path.extension().and_then(OsStr::to_str),
        ) {
            (Some(stem), Some("wat")) => {
                let bytes = parse_file(&path)?;

                let mut module = Module::new(stem, &bytes)?;

                let mut witx_path = path.clone();
                assert!(witx_path.set_extension("witx"));
                if witx_path.is_file() {
                    assert!(module.read_interface(&witx_path)?);
                }

                let adapter = ModuleAdapter::new(&module);

                let output = match adapter.adapt() {
                    Ok(adapted) => print_bytes(&adapted.finish())?,
                    Err(e) => e.to_string(),
                };

                let baseline_path = path.with_extension("baseline");
                if env::var_os("BLESS").is_some() {
                    fs::write(&baseline_path, output)?;
                } else {
                    let expected = fs::read_to_string(&baseline_path).context(format!(
                        "failed to read test baseline file {}\nthis can be fixed with BLESS=1",
                        baseline_path.display()
                    ))?;

                    // Normalize line endings
                    let expected = expected.replace("\r\n", "\n");

                    if expected != output {
                        bail!(
                            "file test `{}` failed: expected `{:?}` but found `{:?}`",
                            stem,
                            expected,
                            output
                        );
                    }
                }
            }
            _ => continue,
        }
    }

    Ok(())
}
