pub use codegen_macro::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wasm_encoder::{Encode, Section};
use wit_bindgen_core::Files;
use wit_component::StringEncoding;
use wit_parser::{ManglingAndAbi, Resolve, WorldId};

/// Returns a suitable directory to place output for tests within.
///
/// This tries to pick a location in the `target` directory that can be
/// relatively easily debugged if a test goes wrong.
pub fn test_directory(suite_name: &str, gen_name: &str, wit_name: &str) -> PathBuf {
    let mut me = std::env::current_exe().unwrap();
    me.pop(); // chop off exe name
    me.pop(); // chop off 'deps'
    me.pop(); // chop off 'debug' / 'release'
    me.push(format!("{suite_name}-tests"));
    me.push(gen_name);

    // replace `-` with `_` for Python where the directory needs to be a valid
    // Python package name.
    me.push(wit_name.replace("-", "_"));

    drop(fs::remove_dir_all(&me));
    fs::create_dir_all(&me).unwrap();
    return me;
}

/// Helper function to execute a process during tests and print informative
/// information if it fails.
pub fn run_command(cmd: &mut Command) {
    let command = format!("{cmd:?}");
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run executable: {e}; command was `{command}`"));

    if output.status.success() {
        return;
    }
    panic!(
        "
command: {cmd:?}
status: {status}

stdout ---
{stdout}

stderr ---
{stderr}",
        status = output.status,
        stdout = String::from_utf8_lossy(&output.stdout).replace("\n", "\n\t"),
        stderr = String::from_utf8_lossy(&output.stderr).replace("\n", "\n\t"),
    );
}

pub fn run_world_codegen_test(
    gen_name: &str,
    wit_path: &Path,
    generate: fn(&Resolve, WorldId, &mut Files),
    verify: fn(&Path, &str),
) {
    let (resolve, world) = parse_wit(wit_path);
    let world_name = &resolve.worlds[world].name;

    let wit_name = if wit_path.is_dir() {
        wit_path
            .parent()
            .unwrap()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap()
    } else {
        wit_path.file_stem().and_then(|s| s.to_str()).unwrap()
    };
    let gen_name = format!("{gen_name}-{wit_name}");
    let dir = test_directory("codegen", &gen_name, &world_name);

    let mut files = Default::default();
    generate(&resolve, world, &mut files);
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    verify(&dir, &world_name);
}

pub fn run_component_codegen_test(
    gen_name: &str,
    wit_path: &Path,
    generate: fn(&str, &[u8], &mut Files),
    verify: fn(&Path, &str),
) {
    let (resolve, world) = parse_wit(wit_path);
    let world_name = &resolve.worlds[world].name;
    let mut wasm = wit_component::dummy_module(&resolve, world, ManglingAndAbi::Standard32);
    let encoded =
        wit_component::metadata::encode(&resolve, world, StringEncoding::UTF8, None).unwrap();
    let section = wasm_encoder::CustomSection {
        name: std::borrow::Cow::Borrowed("component-type"),
        data: std::borrow::Cow::Borrowed(&encoded),
    };
    wasm.push(section.id());
    section.encode(&mut wasm);

    let component = wit_component::ComponentEncoder::default()
        .module(&wasm)
        .unwrap()
        .validate(true)
        .encode()
        .unwrap();

    let wit_name = if wit_path.is_dir() {
        wit_path
            .parent()
            .unwrap()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap()
    } else {
        wit_path.file_stem().and_then(|s| s.to_str()).unwrap()
    };

    let gen_name = format!("{gen_name}-{wit_name}",);
    let dir = test_directory("codegen", &gen_name, &world_name);
    std::fs::write(dir.join("component.wasm"), &component).unwrap();

    let mut files = Default::default();
    generate(&world_name, &component, &mut files);
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    verify(&dir, &world_name);
}

fn parse_wit(path: &Path) -> (Resolve, WorldId) {
    let mut resolve = Resolve::default();
    let (pkg, _files) = resolve.push_path(path).unwrap();
    let world = resolve.select_world(pkg, None).unwrap_or_else(|_| {
        // note: if there are multiples worlds in the wit package, we assume the "imports" world
        resolve.select_world(pkg, Some("imports")).unwrap()
    });
    (resolve, world)
}
