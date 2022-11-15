#[cfg(feature = "macros")]
pub use test_helpers_macros::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_bindgen_core::Files;
use wit_component::ComponentInterfaces;
use wit_parser::abi::{AbiVariant, WasmType};
use wit_parser::{Function, Interface};

pub enum Direction {
    Import,
    Export,
}

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
    println!("running {cmd:?}");
    let output = cmd
        .output()
        .expect("failed to run executable; is it installed");

    if output.status.success() {
        return;
    }
    panic!(
        "
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
    dir: Direction,
    generate: fn(&str, &ComponentInterfaces, &mut Files),
    verify: fn(&Path, &str),
) {
    let iface = Interface::parse_file(wit_path).unwrap();
    let mut interfaces = ComponentInterfaces::default();

    match dir {
        Direction::Import => {
            interfaces.imports.insert(iface.name.clone(), iface);
        }
        Direction::Export => {
            interfaces.default = Some(iface);
        }
    }

    let name = wit_path.file_stem().and_then(|s| s.to_str()).unwrap();

    let gen_name = format!(
        "{gen_name}-{}",
        match dir {
            Direction::Import => "import",
            Direction::Export => "export",
        }
    );
    let dir = test_directory("codegen", &gen_name, name);

    let mut files = Default::default();
    generate(name, &interfaces, &mut files);
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    verify(&dir, name);
}

pub fn run_component_codegen_test(
    gen_name: &str,
    wit_path: &Path,
    dir: Direction,
    generate: fn(&str, &[u8], &mut Files),
    verify: fn(&Path, &str),
) {
    let iface = Interface::parse_file(wit_path).unwrap();
    let mut interfaces = ComponentInterfaces::default();

    match dir {
        Direction::Import => {
            interfaces.imports.insert(iface.name.clone(), iface);
        }
        Direction::Export => {
            interfaces.default = Some(iface);
        }
    }

    let wasm = dummy_module(&interfaces);
    let component = wit_component::ComponentEncoder::default()
        .module(&wasm)
        .unwrap()
        .validate(true)
        .interfaces(interfaces, wit_component::StringEncoding::UTF8)
        .unwrap()
        .encode()
        .unwrap();

    let name = wit_path.file_stem().and_then(|s| s.to_str()).unwrap();

    let gen_name = format!(
        "{gen_name}-{}",
        match dir {
            Direction::Import => "import",
            Direction::Export => "export",
        }
    );
    let dir = test_directory("codegen", &gen_name, name);
    std::fs::write(dir.join("component.wasm"), &component).unwrap();

    let mut files = Default::default();
    generate(name, &component, &mut files);
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    verify(&dir, name);
}

pub fn dummy_module(interfaces: &ComponentInterfaces) -> Vec<u8> {
    let mut wat = String::new();
    wat.push_str("(module\n");
    for (_, import) in interfaces.imports.iter() {
        for func in import.functions.iter() {
            let sig = import.wasm_signature(AbiVariant::GuestImport, func);

            wat.push_str(&format!(
                "(import \"{}\" \"{}\" (func",
                import.name, func.name
            ));
            push_tys(&mut wat, "param", &sig.params);
            push_tys(&mut wat, "result", &sig.results);
            wat.push_str("))\n");
        }
    }

    for (name, export) in interfaces.exports.iter() {
        for func in export.functions.iter() {
            let name = func.core_export_name(Some(name));
            push_func(&mut wat, &name, export, func);
        }
    }

    if let Some(default) = &interfaces.default {
        for func in default.functions.iter() {
            push_func(&mut wat, &func.name, default, func);
        }
    }

    wat.push_str("(memory (export \"memory\") 0)\n");
    wat.push_str(
        "(func (export \"cabi_realloc\") (param i32 i32 i32 i32) (result i32) unreachable)\n",
    );
    wat.push_str(")\n");

    return wat::parse_str(&wat).unwrap();

    fn push_func(wat: &mut String, name: &str, iface: &Interface, func: &Function) {
        let sig = iface.wasm_signature(AbiVariant::GuestExport, func);
        wat.push_str(&format!("(func (export \"{name}\")"));
        push_tys(wat, "param", &sig.params);
        push_tys(wat, "result", &sig.results);
        wat.push_str(" unreachable)\n");

        if iface.guest_export_needs_post_return(func) {
            wat.push_str(&format!("(func (export \"cabi_post_{name}\")"));
            push_tys(wat, "param", &sig.results);
            wat.push_str(")\n");
        }
    }

    fn push_tys(dst: &mut String, desc: &str, params: &[WasmType]) {
        if params.is_empty() {
            return;
        }
        dst.push_str(" (");
        dst.push_str(desc);
        for ty in params {
            dst.push_str(" ");
            match ty {
                WasmType::I32 => dst.push_str("i32"),
                WasmType::I64 => dst.push_str("i64"),
                WasmType::F32 => dst.push_str("f32"),
                WasmType::F64 => dst.push_str("f64"),
            }
        }
        dst.push_str(")");
    }
}
