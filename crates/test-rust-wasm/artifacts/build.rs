use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    std::env::remove_var("CARGO_ENCODED_RUSTFLAGS");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../../wasi_snapshot_preview1")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", &out_dir);
    let status = cmd.status().unwrap();
    assert!(status.success());
    println!("cargo:rerun-if-changed=../../wasi_snapshot_preview1");
    let wasi_adapter = out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm");

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .current_dir("../../test-rust-wasm")
        .arg("--target=wasm32-wasi")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let mut wasms = Vec::new();
    for file in out_dir.join("wasm32-wasi/debug").read_dir().unwrap() {
        let file = file.unwrap().path();
        if file.extension().and_then(|s| s.to_str()) != Some("wasm") {
            continue;
        }

        let dep_file = file.with_extension("d");
        let deps = fs::read_to_string(&dep_file).expect("failed to read dep file");
        for dep in deps
            .splitn(2, ":")
            .skip(1)
            .next()
            .unwrap()
            .split_whitespace()
        {
            println!("cargo:rerun-if-changed={}", dep);
        }

        wasms.push(file);
    }
    println!("cargo:rerun-if-changed=../../test-rust-wasm/Cargo.toml");

    let src = format!(
        "
            pub const ADAPTER: &str = {wasi_adapter:?};
            pub const WASMS: &[&str] = &{wasms:?};
        ",
    );
    std::fs::write(out_dir.join("wasms.rs"), src).unwrap();
}
