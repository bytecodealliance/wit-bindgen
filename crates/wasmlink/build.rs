use std::process::{Command, Stdio};

fn main() {
    println!("cargo:rerun-if-changed=runtime/src");
    println!("cargo:rerun-if-changed=runtime/Cargo.toml");
    println!("cargo:rerun-if-changed=runtime/Cargo.lock");

    let mut cmd = Command::new("cargo");

    cmd.args(&["build", "--target=wasm32-unknown-unknown"]);

    if !cfg!(debug_assertions) {
        cmd.arg("--release");
        cmd.env("CARGO_PROFILE_RELEASE_LTO", "true");

        println!("cargo:rustc-env=RUNTIME_WASM_PATH=../runtime/target/wasm32-unknown-unknown/release/wasmlink_runtime.wasm");
    } else {
        println!("cargo:rustc-env=RUNTIME_WASM_PATH=../runtime/target/wasm32-unknown-unknown/debug/wasmlink_runtime.wasm");
    };

    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir("runtime");

    let output = cmd.output().expect("command should execute");

    let status = output.status;
    if !status.success() {
        panic!(
            "Building wasmlink runtime failed: exit code: {}",
            status.code().unwrap()
        );
    }
}
