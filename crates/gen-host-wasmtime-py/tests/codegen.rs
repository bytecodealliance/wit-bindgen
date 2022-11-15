use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        #[test]
        fn $name() {
            drop(include_str!($test));
            test_helpers::run_component_codegen_test(
                "wasmtime-py",
                $test.as_ref(),
                |name, component, files| {
                    wit_bindgen_core::component::generate(
                        &mut *wit_bindgen_gen_host_wasmtime_py::Opts::default().build(),
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
    test_helpers::run_command(
        Command::new("mypy")
            .arg(dir)
            .arg("--config-file")
            .arg("mypy.ini")
            .arg("--cache-dir")
            .arg(dir.parent().unwrap().join("mypycache").join(name)),
    );
}
