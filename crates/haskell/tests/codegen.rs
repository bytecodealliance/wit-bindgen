use std::path::Path;
use std::process::Command;

use heck::*;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-haskell",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_haskell::Opts::default()
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

fn verify(dir: &Path, name: &str) {
    let name = name.to_upper_camel_case();
    let mut cmd = Command::new("wasm32-wasi-ghc");
    cmd.arg(format!("{name}/Exports.hs"));
    cmd.arg("-o");
    cmd.arg(format!("{name}.wasm"));
    cmd.arg("-no-hs-main");
    cmd.arg("-optl-mexec-model=reactor");
    cmd.arg("-optl-Wl");
    cmd.arg("-rdynamic");
    cmd.current_dir(dir);
    test_helpers::run_command(&mut cmd);
}
