use std::fs;
use std::path::{Path, PathBuf};
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

    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-teavm-java",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_teavm_java::Opts {
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
    // Derived from `test_helpers::test_directory`
    const DEPTH_FROM_TARGET_DIR: u32 = 3;

    let base_dir = {
        let mut dir = dir.to_owned();
        for _ in 0..DEPTH_FROM_TARGET_DIR {
            dir.pop();
        }
        dir
    };

    let teavm_interop_jar = base_dir.join("teavm-interop-0.2.8.jar");

    if !teavm_interop_jar.is_file() {
        panic!("please run ci/download-teavm.sh prior to running the Java tests")
    }

    let mut files = Vec::new();
    move_java_files(&dir.join("wit"), &dir.join("src/main/java/wit"), &mut files);
    fs::remove_dir_all(&dir.join("wit")).unwrap();

    let mut cmd = Command::new("javac");
    cmd.arg("-cp")
        .arg(&teavm_interop_jar)
        .arg("-d")
        .arg("target/classes");

    for file in files {
        cmd.arg(file);
    }

    test_helpers::run_command(&mut cmd);
}

fn move_java_files(src: &Path, dst: &Path, files: &mut Vec<PathBuf>) {
    if src.is_dir() {
        for entry in fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            move_java_files(&path, &dst.join(path.strip_prefix(src).unwrap()), files);
        }
    } else if let Some("java") = src.extension().map(|ext| ext.to_str().unwrap()) {
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        fs::rename(src, dst).unwrap();
        files.push(dst.to_owned());
    }
}
