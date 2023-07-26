use heck::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! codegen_test {
    // TODO: support importing and exporting the same interface containing one
    // or more resources, then remove the following lines:
    (import_and_export_resource $name:tt $test:tt) => {};
    (import_and_export_resource_alias $name:tt $test:tt) => {};
    (resource_alias $name:tt $test:tt) => {};
    (resource_local_alias $name:tt $test:tt) => {};
    (resources_with_lists $name:tt $test:tt) => {};

    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-c",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_c::Opts::default()
                        .build()
                        .generate(resolve, world, files)
                        .unwrap()
                },
                verify,
            );
            test_helpers::run_world_codegen_test(
                "guest-c-no-sig-flattening",
                $test.as_ref(),
                |resolve, world, files| {
                    let mut opts = wit_bindgen_c::Opts::default();
                    opts.no_sig_flattening = true;
                    opts.build().generate(resolve, world, files).unwrap()
                },
                verify,
            );
        }
    };
}

test_helpers::codegen_tests!();

fn verify(dir: &Path, name: &str) {
    let path = PathBuf::from(
        env::var_os("WASI_SDK_PATH").expect("environment variable WASI_SDK_PATH should be set"),
    );
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
