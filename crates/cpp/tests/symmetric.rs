use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

fn tests(
    dir_name: &str,
    out_dir: &PathBuf,
    toplevel: &PathBuf,
    source_files: &PathBuf,
    tester_source_dir: &PathBuf,
) -> io::Result<()> {
    // modelled after wit-bindgen/tests/runtime/main.rs
    let mut dir = source_files.clone();
    dir.push(dir_name);

    // let mut rust = Vec::new();
    let mut cpp = Vec::new();
    for file in dir.read_dir()? {
        let path = file?.path();
        match path.extension().and_then(|s| s.to_str()) {
            // Some("rs") => rust.push(path),
            Some("cpp") => cpp.push(path),
            _ => {}
        }
    }

    let mut out_dir = out_dir.clone();
    out_dir.push(dir_name);
    // println!("{cpp:?} {out_dir:?}");

    drop(std::fs::remove_dir_all(&out_dir));
    std::fs::create_dir_all(&out_dir)?;
    let mut testee_dir = out_dir.clone();
    testee_dir.push("rust");
    std::fs::create_dir(&testee_dir)?;
    let testee_cargo = format!(
        "[package]\n\
            name = \"{dir_name}\"\n\
            publish = false\n\
            edition = \"2021\"\n\
            \n\
            [dependencies]\n\
            wit-bindgen = {{ path = \"{toplevel}/crates/guest-rust\" }}\n\
            \n\
            [lib]\n\
            crate-type = [\"cdylib\"]\n\
            ",
        toplevel = toplevel.display()
    );
    let mut filename = testee_dir.clone();
    filename.push("Cargo.toml");
    File::create(&filename)?.write_all(testee_cargo.as_bytes())?;
    filename.pop();
    filename.push("src");
    std::fs::create_dir(&filename)?;
    filename.push(format!("lib.rs"));
    let mut original = dir.clone();
    original.push("wasm.rs");
    std::os::unix::fs::symlink(original, filename);
    drop(testee_cargo);

    let tester_cargo = format!(
        "[workspace]\n\
            members = [ \"rust\" ]\n\
            resolver = \"2\"\n\
            \n\
            [package]\n\
            name = \"tester\"\n\
            publish = false\n\
            edition = \"2021\"\n\
            \n\
            [dependencies]\n\
            wit-bindgen = {{ path = \"{toplevel}/crates/guest-rust\" }}\n\
            {dir_name} = {{ path = \"rust\" }}\n\
            ",
        toplevel = toplevel.display()
    );
    let mut filename = out_dir.clone();
    filename.push("Cargo.toml");
    File::create(&filename)?.write_all(tester_cargo.as_bytes())?;
    filename.pop();
    filename.push("src");
    std::fs::create_dir(&filename)?;
    filename.push(format!("main.rs"));
    let mut original = tester_source_dir.clone();
    original.push(&format!("{dir_name}.rs"));
    std::os::unix::fs::symlink(original, &filename);

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .current_dir(testee_dir)
        .env("SYMMETRIC_ABI", "1")
        .env("WIT_BINDGEN_DEBUG", "1");
    let status = cmd.status().unwrap();
    assert!(status.success());

    Ok(())
}

#[test]
fn symmetric_integration() -> io::Result<()> {
    let mut out_dir = std::env::current_exe()?;
    out_dir.pop();
    out_dir.pop();
    out_dir.pop();
    out_dir.push("symmetric-tests");

    let mut manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut toplevel = manifest_dir.clone();
    toplevel.pop();
    toplevel.pop();

    let mut test_link = out_dir.clone();
    test_link.push("tests");
    if !fs::exists(&test_link)? {
        let mut original = toplevel.clone();
        original.push("tests");
        std::os::unix::fs::symlink(original, &test_link);
    }

    let mut source_files = toplevel.clone();
    source_files.push("tests");
    source_files.push("runtime");

    let mut tester_source_dir = manifest_dir.clone();
    tester_source_dir.push("tests");
    tester_source_dir.push("symmetric_tests");

    tests(
        "smoke",
        &out_dir,
        &toplevel,
        &source_files,
        &tester_source_dir,
    )?;
    Ok(())
}
