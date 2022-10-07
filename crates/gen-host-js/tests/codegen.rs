use std::path::Path;
use std::process::Command;

macro_rules! gen_test {
    ($name:ident $test:tt $dir:ident) => {
        #[test]
        fn $name() {
            test_helpers::run_codegen_test(
                "js",
                std::path::Path::new($test)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                include_str!($test),
                test_helpers::Direction::$dir,
                wit_bindgen_gen_host_js::Opts::default().build(),
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
    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    test_helpers::run_command(
        Command::new(cmd)
            .args(args)
            .arg("eslint")
            .arg("-c")
            .arg(".eslintrc.js")
            .arg(dir.join(&format!("{}.js", name))),
    );
}
