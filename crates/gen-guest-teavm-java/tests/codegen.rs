use heck::{ToSnakeCase, ToUpperCamelCase};
use std::{fs, path::Path, process::Command};

mod imports {
    test_helpers::codegen_teavm_java_import!(
        "*.wit"

        // TODO: implement async, resource, and multi-return support
        "!async-functions.wit"
        "!resource.wit"
        "!multi-return.wit"
    );
}

mod exports {
    test_helpers::codegen_teavm_java_export!(
        "*.wit"

        // TODO: implement async, resource, and multi-return support
        "!async-functions.wit"
        "!resource.wit"
        "!multi-return.wit"
    );
}

fn verify(dir: &str, name: &str) {
    let dir = Path::new(dir);
    let java_dir = &dir.join("src").join("main").join("java");
    let package_dir = &java_dir.join(format!("wit_{}", &name.to_snake_case()));

    fs::create_dir_all(package_dir).unwrap();

    for file_name in [
        format!("{}.java", name.to_upper_camel_case()),
        format!("{}Impl.java", name.to_upper_camel_case()),
    ] {
        let src = &dir.join(&file_name);
        let dst = &package_dir.join(&file_name);
        if src.exists() {
            fs::rename(src, dst).unwrap();
        }
    }

    fs::write(dir.join("pom.xml"), include_bytes!("pom.xml")).unwrap();
    fs::write(java_dir.join("Main.java"), include_bytes!("Main.java")).unwrap();

    let mut cmd = Command::new("mvn");
    cmd.arg("prepare-package").current_dir(dir);

    println!("{cmd:?}");
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => panic!("failed to run Maven: {e}"),
    };

    if output.status.success() {
        return;
    }
    println!("status: {}", output.status);
    println!("stdout: ------------------------------------------");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: ------------------------------------------");
    println!("{}", String::from_utf8_lossy(&output.stderr));
    panic!("failed to build");
}
