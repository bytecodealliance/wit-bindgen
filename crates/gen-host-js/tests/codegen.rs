use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        #[test]
        fn $name() {
            drop(include_str!($test));
            test_helpers::run_component_codegen_test(
                "js",
                $test.as_ref(),
                |name, component, files| {
                    wit_bindgen_core::component::generate(
                        &mut *wit_bindgen_gen_host_js::Opts::default().build().unwrap(),
                        name,
                        component,
                        files,
                    )
                    .unwrap()
                },
                verify,
            )
        }
    };
}

test_helpers::codegen_tests!("*.wit");

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
