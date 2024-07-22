use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    // todo: implement resource support and then remove the following lines:
    (resources $name:tt $test:tt) => {};
    (resource_alias $name:tt $test:tt) => {};
    (return_resource_from_export $name:tt $test:tt) => {};
    (import_and_export_resource $name:tt $test:tt) => {};
    (import_and_export_resource_alias $name:tt $test:tt) => {};
    (resources_with_lists $name:tt $test:tt) => {};
    (resource_local_alias $name:tt $test:tt) => {};
    (resource_local_alias_borrow $name:tt $test:tt) => {};
    (resource_local_alias_borrow_import $name:tt $test:tt) => {};
    (resource_borrow_in_record $name:tt $test:tt) => {};
    (resource_borrow_in_record_export $name:tt $test:tt) => {};
    (resource_own_in_other_interface $name:tt $test:tt) => {};
    (same_names5 $name:tt $test:tt) => {};
    (resources_in_aggregates $name:tt $test:tt) => {};
    (issue668 $name:tt $test:tt) => {};
    (multiversion $name:tt $test:tt) => {};
    (wasi_cli $name:tt $test:tt) => {};
    (wasi_clocks $name:tt $test:tt) => {};
    (wasi_filesystem $name:tt $test:tt) => {};
    (wasi_http $name:tt $test:tt) => {};
    (wasi_io $name:tt $test:tt) => {};
    (issue929 $name:tt $test:tt) => {};
    (issue929_no_import $name:tt $test:tt) => {};
    (issue929_no_export $name:tt $test:tt) => {};
    (issue929_only_methods $name:tt $test:tt) => {};

    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-moonbit",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_moonbit::Opts {
                        generate_stub: true,
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
