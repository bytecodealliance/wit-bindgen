use heck::*;
use std::env;
use std::io::{prelude::*, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let main = dir.join(format!("{name}.go"));

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
        .open(&main)
        .expect("failed to open file");
    let mut reader = BufReader::new(file);
    reader.read_until(b'\n', &mut Vec::new()).unwrap();
    let mut buf = Vec::new();
    buf.append(&mut "package main\n".as_bytes().to_vec());

    // check if {name}_types.go exists
    let types_file = dir.join(format!("{name}_types.go"));
    if let Ok(_) = std::fs::metadata(types_file) {
        // create a directory called option and move the type file to option
        std::fs::create_dir(dir.join("option")).expect("Failed to create directory");
        std::fs::rename(
            dir.join(format!("{name}_types.go")),
            dir.join("option").join(format!("{name}_types.go")),
        )
        .expect("Failed to move file");
        buf.append(&mut format!("import . \"{name}/option\"\n").as_bytes().to_vec());
    }

    reader.read_to_end(&mut buf);
    buf.append(&mut "func main() {}".as_bytes().to_vec());
    std::fs::write(&main, buf).expect("Failed to write to file");

    // create go.mod file
    let mod_file = dir.join("go.mod");
    let mut file = std::fs::File::create(mod_file).expect("Failed to create file go.mod");
    file.write_all(format!("module {name}\n\ngo 1.19").as_bytes()).expect("Failed to write to file");

    let mut cmd = Command::new("tinygo");
    cmd.arg("build");
    cmd.arg("-wasm-abi=generic");
    cmd.arg("-target=wasi");
    cmd.arg("-gc=leaking");
    cmd.arg("-no-debug");
    cmd.arg("-o");
    cmd.arg("go.wasm");
    cmd.arg(&main);
    test_helpers::run_command(&mut cmd);
}
