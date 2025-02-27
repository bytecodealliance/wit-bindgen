use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_bindgen_scalajs_jco::ScalaDialect::Scala2;

macro_rules! codegen_test {
    // TODO: implement support for stream, future, and error-context, and then
    // remove these lines:
    (streams $name:tt $test:tt) => {};
    (futures $name:tt $test:tt) => {};
    (resources_with_streams $name:tt $test:tt) => {};
    (resources_with_futures $name:tt $test:tt) => {};
    (error_context $name:tt $test:tt) => {};

    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-scalajs",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_scalajs_jco::Opts {
                        base_package: Some("test".to_string()),
                        skeleton_base_package: Some("skeleton".to_string()),
                        scala_dialect: Scala2,
                        generate_skeleton: true,
                        skeleton_root: None,
                        binding_root: None,
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

fn verify(dir: &Path, name: &str) {
    println!("name: {name}, dir: {dir:?}");
    let mut files = Vec::new();
    move_scala_files(dir, &dir.join("src/main/scala"), &mut files);

    write_build_sbt(dir);
    write_plugins_sbt(dir);

    let mut cmd = Command::new("sbt");
    cmd.current_dir(dir);
    cmd.arg("fastLinkJS");

    test_helpers::run_command(&mut cmd);
}

fn move_scala_files(src: &Path, dst: &Path, files: &mut Vec<PathBuf>) {
    if src.is_dir() {
        for entry in fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            move_scala_files(&path, &dst.join(path.strip_prefix(src).unwrap()), files);
        }
    } else if let Some("scala") = src.extension().map(|ext| ext.to_str().unwrap()) {
        fs::create_dir_all(dst.parent().unwrap()).unwrap();
        fs::rename(src, dst).unwrap();
        files.push(dst.to_owned());
    }
}

fn write_build_sbt(dir: &Path) {
    let build_sbt = include_str!("../scala/build.sbt");
    fs::write(dir.join("build.sbt"), build_sbt).unwrap();
}

fn write_plugins_sbt(dir: &Path) {
    let plugins_sbt = include_str!("../scala/plugins.sbt");
    let build_properties = include_str!("../scala/build.properties");

    let project_dir = dir.join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(project_dir.join("plugins.sbt"), plugins_sbt).unwrap();
    fs::write(project_dir.join("build.properties"), build_properties).unwrap();
}
