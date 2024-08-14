use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-moonbit",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_moonbit::Opts {
                        derive_show: true,
                        derive_eq: true,
                        ignore_stub: false,
                    }
                    .build()
                    .generate(resolve, world, files)
                    .unwrap()
                },
                verify,
            )
        }
    };
}
test_helpers::codegen_tests!();

fn verify(dir: &Path, _name: &str) {
    let mut cmd = Command::new("moon");
    cmd.arg("check")
        .arg("--target")
        .arg("wasm")
        .arg("--source-dir")
        .arg(dir);

    test_helpers::run_command(&mut cmd);
}
