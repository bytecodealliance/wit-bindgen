use heck::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{prelude::*, BufReader};

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        #[test]
        fn $name() {
            test_helpers::run_world_codegen_test(
                "guest-go",
                $test.as_ref(),
                |world, files| {
                    wit_bindgen_gen_guest_go::Opts::default()
                        .build()
                        .generate(world, files);
                    wit_bindgen_gen_guest_c::Opts::default()
                        .build()
                        .generate(world, files)
                },
                verify,
            )
        }
    };
}

test_helpers::codegen_tests!("*.wit");

fn verify(dir: &Path, name: &str) {
    let name = name.to_kebab_case();
    let dir = dir.join(format!("{name}.go"));

    // The generated go package is named after the world's name.
    // But tinygo currently does not support non-main package and requires
    // a `main()` function in the module to compile.
    // The following code replaces the package name to `package main` and 
    // adds a `func main() {}` function at the bottom of the file.

    // TODO: However, there is still an issue. Since the go module does not
    // invoke the imported functions, they will be skipped by the compiler. 
    // This will weaken the test's ability to verify imported functions
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&dir)
        .expect("failed to open file");
    let mut reader = BufReader::new(file);
    reader.read_until(b'\n', &mut Vec::new()).unwrap();
    let mut buf = Vec::new();
    buf.append(&mut "package main".as_bytes().to_vec());
    reader.read_to_end(&mut buf);
    buf.append(&mut "func main() {}".as_bytes().to_vec());
    
    std::fs::write(&dir, buf).expect("Failed to write to file");
    
    let mut cmd = Command::new("tinygo");
    cmd.arg("build");
    cmd.arg("-wasm-abi=generic");
    cmd.arg("-target=wasi");
    cmd.arg("-gc=leaking");
    cmd.arg("-no-debug");
    cmd.arg("-o");
    cmd.arg("go.wasm");
    cmd.arg(&dir);
    test_helpers::run_command(&mut cmd);
}
