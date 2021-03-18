use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let checked = build(&out_dir, false);
    fs::copy(&checked, out_dir.join("checked.wasm")).unwrap();

    let unchecked = build(&out_dir, true);
    fs::copy(&unchecked, out_dir.join("unchecked.wasm")).unwrap();

    println!("cargo:rustc-env=CHECKED={}", checked.display());
    println!("cargo:rustc-env=UNCHECKED={}", unchecked.display());
    println!("cargo:rerun-if-changed=../test-rust-wasm");
    println!("cargo:rerun-if-changed=../gen-rust-wasm");
    println!("cargo:rerun-if-changed=../gen-rust");
    println!("cargo:rerun-if-changed=../rust-wasm");
    println!("cargo:rerun-if-changed=../rust-wasm-impl");
    println!("cargo:rerun-if-changed=../../tests/host.witx");
    println!("cargo:rerun-if-changed=../../tests/wasm.witx");
}

fn build(out_dir: &Path, unchecked: bool) -> PathBuf {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .current_dir("../test-rust-wasm")
        .arg("--target=wasm32-wasi")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1");
    if unchecked {
        cmd.arg("--features=unchecked");
    }
    let status = cmd.status().unwrap();
    assert!(status.success());
    out_dir.join("wasm32-wasi/debug/test_rust_wasm.wasm")
}
