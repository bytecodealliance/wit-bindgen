use anyhow::Result;
use heck::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_parser::{Resolve, UnresolvedPackage};

macro_rules! codegen_test {
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
            test_helpers::run_world_codegen_test(
                "guest-c-autodrop-borrows",
                $test.as_ref(),
                |resolve, world, files| {
                    let mut opts = wit_bindgen_c::Opts::default();
                    opts.autodrop_borrows = wit_bindgen_c::Enabled::Yes;
                    opts.build().generate(resolve, world, files).unwrap()
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
    let c_src = dir.join(format!("{name}.c"));

    let shared_args = vec![
        "--sysroot",
        sysroot.to_str().unwrap(),
        "-I",
        dir.to_str().unwrap(),
        "-Wall",
        "-Wextra",
        "-Wc++-compat",
        "-Werror",
        "-Wno-unused-parameter",
        "-c",
        "-o",
    ];

    let mut cmd = Command::new(sdk_path.join("bin/clang"));
    cmd.args(&shared_args);
    cmd.arg(dir.join("obj.o"));
    cmd.arg(&c_src);
    test_helpers::run_command(&mut cmd);

    let cpp_src = c_src.with_extension("cpp");
    std::fs::write(&cpp_src, format!("#include \"{name}.h\"\n")).unwrap();
    let mut cmd = Command::new(sdk_path.join("bin/clang++"));
    cmd.args(&shared_args);
    cmd.arg(dir.join("obj.o"));
    cmd.arg(&cpp_src);
    test_helpers::run_command(&mut cmd);
}

#[test]
fn rename_option() -> Result<()> {
    let dir = test_helpers::test_directory("codegen", "guest-c", "rename-option");

    let mut opts = wit_bindgen_c::Opts::default();
    opts.rename.push(("a".to_string(), "rename1".to_string()));
    opts.rename
        .push(("foo:bar/b".to_string(), "rename2".to_string()));
    opts.rename.push(("c".to_string(), "rename3".to_string()));

    let mut resolve = Resolve::default();
    let pkg = resolve.push(UnresolvedPackage::parse(
        "input.wit".as_ref(),
        r#"
            package foo:bar;

            interface b {
                f: func();
            }

            world rename-option {
                import a: interface {
                    f: func();
                }
                import b;

                export run: func();

                export c: interface {
                    f: func();
                }
                export b;
            }
        "#,
    )?)?;
    let world = resolve.select_world(pkg, None)?;
    let mut files = Default::default();
    opts.build().generate(&resolve, world, &mut files)?;
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    std::fs::write(
        dir.join("rename_option.c"),
        r#"
#include "rename_option.h"

void rename_option_run(void) {
    rename1_f();
    rename2_f();
}

void rename3_f() {}

void exports_rename2_f() {}
        "#,
    )?;

    verify(&dir, "rename-option");
    Ok(())
}
