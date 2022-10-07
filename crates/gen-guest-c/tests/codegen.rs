use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! gen_test {
    ($name:ident $test:tt $dir:ident) => {
        #[test]
        fn $name() {
            test_helpers::run_codegen_test(
                "guest-c",
                std::path::Path::new($test)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                include_str!($test),
                test_helpers::Direction::$dir,
                wit_bindgen_gen_guest_c::Opts::default().build(),
                super::verify,
            )
        }
    };
}

mod exports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => (gen_test!($name $test Export);)
    }
    test_helpers::codegen_tests!("*.wit");
}

mod imports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => (gen_test!($name $test Import);)
    }
    test_helpers::codegen_tests!("*.wit");
}

fn verify(dir: &Path, name: &str) {
    let path = PathBuf::from(env::var_os("WASI_SDK_PATH").unwrap());
    let mut cmd = Command::new(path.join("bin/clang"));
    cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
    cmd.arg(dir.join(format!("{}.c", name)));
    cmd.arg("-I").arg(dir);
    cmd.arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wno-unused-parameter");
    cmd.arg("-c");
    cmd.arg("-o").arg(dir.join("obj.o"));

    test_helpers::run_command(&mut cmd);
}
