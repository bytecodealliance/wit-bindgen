use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    // TODO: should fix handling of new `WorldKey` in teavm-java generator
    ($id:ident $name:tt $test:tt) => {};

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
                },
                verify,
            )
        }
    };
}
test_helpers::codegen_tests!();

fn verify(dir: &Path, name: &str) {
    use heck::ToSnakeCase;
    use std::fs;

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

    let java_dir = &dir.join("src/main/java");
    let snake = name.to_snake_case();
    let package_dir = &java_dir.join(format!("wit_{snake}"));

    fs::create_dir_all(package_dir).unwrap();

    let src_files = fs::read_dir(&dir).unwrap().filter_map(|entry| {
        let path = entry.unwrap().path();
        if let Some("java") = path.extension().map(|ext| ext.to_str().unwrap()) {
            Some(path)
        } else {
            None
        }
    });

    let dst_files = src_files.map(|src| {
        let dst = package_dir.join(src.file_name().unwrap());
        fs::rename(src, &dst).unwrap();
        dst
    });

    let mut cmd = Command::new("javac");
    cmd.arg("-cp")
        .arg(&teavm_interop_jar)
        .arg("-d")
        .arg("target/classes");

    for file in dst_files {
        cmd.arg(file);
    }

    test_helpers::run_command(&mut cmd);
}
