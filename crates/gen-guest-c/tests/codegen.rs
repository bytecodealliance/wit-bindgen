use heck::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        #[test]
        fn $name() {
            test_helpers::run_world_codegen_test(
                "guest-c",
                $test.as_ref(),
                |name, interfaces, files| {
                    wit_bindgen_gen_guest_c::Opts::default()
                        .build()
                        .generate(name, interfaces, files)
                },
                verify,
            )
        }
    };
}

test_helpers::codegen_tests!("*.wit");

fn verify(dir: &Path, name: &str) {
    let path = PathBuf::from(env::var_os("WASI_SDK_PATH").unwrap());
    let mut cmd = Command::new(path.join("bin/clang"));
    cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
    cmd.arg(dir.join(format!("{}.c", name.to_snake_case())));
    cmd.arg("-I").arg(dir);
    cmd.arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wno-unused-parameter");
    cmd.arg("-c");
    cmd.arg("-o").arg(dir.join("obj.o"));

    test_helpers::run_command(&mut cmd);
}
