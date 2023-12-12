use heck::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "cpp",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_cpp::Opts::default()
                        .build()
                        .generate(resolve, world, files)
                        .unwrap()
                },
                verify,
            );
        }
    };
}

test_helpers::codegen_tests!();

fn verify(dir: &Path, name: &str) {
    let name = name.to_snake_case();
    let sdk_path = PathBuf::from(
        env::var_os("WASI_SDK_PATH").expect("environment variable WASI_SDK_PATH should be set"),
    );
    let sysroot = sdk_path.join("share/wasi-sysroot");
    let c_src = dir.join(format!("{name}.cpp"));

    let shared_args = vec![
        "--sysroot",
        sysroot.to_str().unwrap(),
        "-I",
        dir.to_str().unwrap(),
        "-Wall",
        "-Wextra",
        "-Werror",
        "-Wno-unused-parameter",
        "-c",
        "-o",
    ];

    let mut cmd = Command::new(sdk_path.join("bin/clang++"));
    cmd.args(&shared_args);
    cmd.arg(dir.join("obj.o"));
    cmd.arg(&c_src);
    test_helpers::run_command(&mut cmd);
}
