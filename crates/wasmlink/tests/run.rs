use anyhow::{Context, Result};
use pretty_assertions::assert_eq;
use std::{env, ffi::OsStr, fs, path::Path};
use wasmlink::{Module, ModuleAdapter};
use wasmprinter::print_bytes;
use wat::parse_file;

fn adapt(name: &str, bytes: &[u8], wai_path: &Path) -> Result<wasm_encoder::Module> {
    let module = Module::new(
        name,
        bytes,
        if wai_path.is_file() {
            vec![wai_parser::Interface::parse_file(wai_path)?]
        } else {
            Vec::new()
        },
    )?;

    let mut next_resource_id = 0;
    let adapter = ModuleAdapter::new(&module, &mut next_resource_id);

    adapter.adapt()
}

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

                let mut wai_path = path.clone();
                assert!(wai_path.set_extension("wai"));

                let output = match adapt(stem, &bytes, &wai_path) {
                    Ok(adapted) => print_bytes(&adapted.finish())?,
                    Err(e) => e.to_string(),
                };

                let baseline_path = path.with_extension("baseline");
                if env::var_os("BLESS").is_some() {
                    fs::write(&baseline_path, output)?;
                } else {
                    let expected = fs::read_to_string(&baseline_path)
                        .context(format!(
                            "failed to read test baseline file {}\nthis can be fixed with BLESS=1",
                            baseline_path.display()
                        ))?
                        .replace("\r\n", "\n");

                    let expected: Vec<_> = expected.split("\n").collect();
                    let output: Vec<_> = output.split("\n").collect();

                    let mut line = 0;

                    for (expected, output) in expected.iter().zip(output.iter()) {
                        line += 1;
                        assert_eq!(
                            expected, output,
                            "file test `{}` failed on line {}",
                            stem, line
                        );
                    }

                    if line < expected.len() {
                        // Output was too short
                        assert_eq!(
                            expected[line],
                            "<EOF>",
                            "file test `{}` failed on line {} (not in output)",
                            stem,
                            line + 1
                        );
                        unreachable!()
                    }

                    if line < output.len() {
                        // Output was too long
                        assert_eq!(
                            "<EOF>",
                            output[line],
                            "file test `{}` failed on line {} of the output (not in baseline)",
                            stem,
                            line + 1
                        );
                        unreachable!()
                    }
                }
            }
            _ => continue,
        }
    }

    Ok(())
}
