use std::{
    io,
    process::{Command, Stdio},
};

fn main() {
    println!("cargo:rerun-if-changed=modules/crates");
    println!("cargo:rerun-if-changed=modules/Cargo.toml");
    println!("cargo:rerun-if-changed=modules/Cargo.lock");

    build_modules().unwrap();
}

fn build_modules() -> io::Result<()> {
    let mut cmd = Command::new("cargo");

    cmd.args(&["build", "--target=wasm32-wasi"]);

    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }

    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir("modules");

    let output = cmd.output()?;

    let status = output.status;
    if !status.success() {
        panic!(
            "Building tests failed: exit code: {}",
            status.code().unwrap()
        );
    }

    Ok(())
}
