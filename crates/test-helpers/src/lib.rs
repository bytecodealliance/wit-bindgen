pub use test_helpers_macros::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_bindgen_core::Generator;

pub enum Direction {
    Import,
    Export,
}

/// Executes a "codegen" test by using `gen` to generate bindings for the
/// `*.wit` file specified by `wit_contents`. This will then use the `verify`
/// function to verify that the generated language is correct (e.g. compiles,
/// lints, etc).
///
/// For an example of this see the JS host's `codegen.rs` test.
pub fn run_codegen_test(
    gen_name: &str,
    wit_name: &str,
    wit_contents: &str,
    dir: Direction,
    mut gen: impl Generator,
    verify: fn(&Path, &str),
) {
    let mut files = Default::default();
    let iface = wit_parser::Interface::parse(wit_name, wit_contents).unwrap();
    let (imports, exports) = match dir {
        Direction::Import => (vec![iface], vec![]),
        Direction::Export => (vec![], vec![iface]),
    };
    gen.generate_all(&imports, &exports, &mut files);

    let gen_name = format!(
        "{gen_name}-{}",
        match dir {
            Direction::Import => "import",
            Direction::Export => "export",
        }
    );
    let dir = test_directory("codegen", &gen_name, wit_name);
    for (file, contents) in files.iter() {
        std::fs::write(dir.join(file), contents).unwrap();
    }
    verify(&dir, wit_name);
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
    me.push(wit_name);

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
