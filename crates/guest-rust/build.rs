use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(wasi_has_initialize)");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    // Rust 1.100 supplies `_initialize` for WASI cdylibs:
    // https://github.com/rust-lang/rust/pull/161421
    // Remove the version check once the MSRV reaches 1.100.
    if target_os == "wasi" && rustc_minor_version().unwrap_or(0) >= 100 {
        println!("cargo:rustc-cfg=wasi_has_initialize");
    }
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or(String::new());
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or(String::new());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or(String::new());

    if target_family != "wasm" {
        return;
    }

    if target_arch != "wasm32" {
        panic!("only wasm32 supports cabi-realloc right now");
    }

    if target_env == "p1" || target_env == "" {
        link_lib("cabi_realloc");
    }
    link_lib("cabi_wasip3");

    // The no-std async runtime's task context uses WASIp3 TLS. Keep this
    // separate from the MVP task helper needed by every wasm32 target.
    if target_env == "p3" && !cfg!(feature = "std") {
        link_lib("cabi_wasip3_tls");
    }
}

fn rustc_minor_version() -> Option<u32> {
    let rustc = env::var_os("RUSTC")?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    let version = std::str::from_utf8(&output.stdout).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    pieces.next()?.parse().ok()
}

fn link_lib(name: &str) {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut src = env::current_dir().unwrap();
    src.push("src");
    src.push("rt");
    src.push(&format!("libwit_bindgen_{name}.a"));

    let dst_name = format!(
        "wit_bindgen_{name}_{}",
        env!("CARGO_PKG_VERSION").replace(".", "_")
    );
    let dst = out_dir.join(format!("lib{dst_name}.a"));

    std::fs::copy(&src, &dst).unwrap();

    println!("cargo:rustc-link-lib=static={dst_name}");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
}
