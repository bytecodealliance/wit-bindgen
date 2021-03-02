use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let status = Command::new("cargo")
        .arg("build")
        .current_dir("../test-rust-wasm")
        .arg("--target=wasm32-wasi")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .status()
        .unwrap();
    assert!(status.success());
    let wasm = Path::new(&out_dir).join("wasm32-wasi/debug/test_rust_wasm.wasm");
    println!("cargo:rustc-env=WASM={}", wasm.display());
}
