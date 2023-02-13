use std::path::Path;
use std::process::Command;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-teavm-java",
                $test.as_ref(),
                |resolve, world, files| {
                    wit_bindgen_gen_guest_teavm_java::Opts {
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
test_helpers::codegen_tests!("*.wit");

// TODO: As of this writing, Maven has started failing to resolve dependencies on Windows in the GitHub action,
// apparently unrelated to any code changes we've made.  Until we've either resolved the problem or developed a
// workaround, we're disabling the tests.
//
// See https://github.com/bytecodealliance/wit-bindgen/issues/495 for more information.
#[cfg(windows)]
fn verify(_dir: &Path, _name: &str) {
    _ = mvn;
    _ = pom_xml;
}

#[cfg(unix)]
fn verify(dir: &Path, name: &str) {
    use heck::{ToSnakeCase, ToUpperCamelCase};
    use std::fs;

    let java_dir = &dir.join("src/main/java");
    let snake = name.to_snake_case();
    let package_dir = &java_dir.join(format!("wit_{snake}"));

    fs::create_dir_all(package_dir).unwrap();

    let upper = name.to_upper_camel_case();

    let src_files = fs::read_dir(&dir).unwrap().filter_map(|entry| {
        let path = entry.unwrap().path();
        if let Some("java") = path.extension().map(|ext| ext.to_str().unwrap()) {
            Some(path)
        } else {
            None
        }
    });

    for src in src_files {
        let dst = &package_dir.join(src.file_name().unwrap());
        fs::rename(src, dst).unwrap();
    }

    fs::write(
        dir.join("pom.xml"),
        pom_xml(&[&format!("wit_{snake}.{upper}")]),
    )
    .unwrap();
    fs::write(java_dir.join("Main.java"), include_bytes!("Main.java")).unwrap();

    let mut cmd = mvn();
    cmd.arg("prepare-package").current_dir(dir);

    test_helpers::run_command(&mut cmd);
}

#[cfg(unix)]
fn mvn() -> Command {
    Command::new("mvn")
}

#[cfg(windows)]
fn mvn() -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(&["/c", "mvn"]);
    cmd
}

fn pom_xml(classes_to_preserve: &[&str]) -> Vec<u8> {
    let xml = include_str!("pom.xml");
    let position = xml.find("<mainClass>").unwrap();
    let (before, after) = xml.split_at(position);
    let classes_to_preserve = classes_to_preserve
        .iter()
        .map(|&class| format!("<param>{class}</param>"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{before}
         <classesToPreserve>
            {classes_to_preserve}
         </classesToPreserve>
         {after}"
    )
    .into_bytes()
}
