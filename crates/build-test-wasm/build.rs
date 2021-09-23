use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use witx_bindgen_gen_core::{witx2, Generator};

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let mut wasms = Vec::new();

    if cfg!(feature = "rust") {
        let checked = build_rust(&out_dir, false);
        fs::copy(&checked, out_dir.join("rust-checked.wasm")).unwrap();

        let unchecked = build_rust(&out_dir, true);
        fs::copy(&unchecked, out_dir.join("rust-unchecked.wasm")).unwrap();

        wasms.push(("rust-checked", out_dir.join("rust-checked.wasm")));
        wasms.push(("rust-unchecked", out_dir.join("rust-unchecked.wasm")));
        println!("cargo:rerun-if-changed=../test-rust-wasm");
        println!("cargo:rerun-if-changed=../gen-rust-wasm");
        println!("cargo:rerun-if-changed=../gen-rust");
        println!("cargo:rerun-if-changed=../rust-wasm");
        println!("cargo:rerun-if-changed=../rust-wasm-impl");
        println!("cargo:rerun-if-changed=../../tests/host.witx");
        println!("cargo:rerun-if-changed=../../tests/wasm.witx");
    }

    if cfg!(feature = "c") {
        let host = witx2::Interface::parse_file("../../tests/host.witx").unwrap();
        let mut host_files = Default::default();
        witx_bindgen_gen_c::Opts::default()
            .build()
            .generate_all(&[host], &[], &mut host_files);
        let wasm = witx2::Interface::parse_file("../../tests/wasm.witx").unwrap();
        let mut wasm_files = Default::default();
        witx_bindgen_gen_c::Opts::default()
            .build()
            .generate_all(&[], &[wasm], &mut wasm_files);
        println!("cargo:rerun-if-changed=../../tests/host.witx");
        println!("cargo:rerun-if-changed=../../tests/wasm.witx");
        println!("cargo:rerun-if-changed=imports.c");
        println!("cargo:rerun-if-changed=exports.c");
        println!("cargo:rerun-if-changed=invalid.c");

        for (file, contents) in host_files.iter() {
            let dst = out_dir.join(file);
            fs::write(dst, contents).unwrap();
        }
        for (file, contents) in wasm_files.iter() {
            let dst = out_dir.join(file);
            fs::write(dst, contents).unwrap();
        }

        let path =
            PathBuf::from(env::var_os("WASI_SDK_PATH").expect(
                "point the `WASI_SDK_PATH` environment variable to the path of your wasi-sdk",
            ));
        let mut cmd = Command::new(path.join("bin/clang"));
        cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
        cmd.arg("exports.c")
            .arg("imports.c")
            .arg("invalid.c")
            .arg(out_dir.join("wasm.c"))
            .arg(out_dir.join("host.c"))
            .arg("-I")
            .arg(&out_dir)
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            .arg("-Wno-unused-parameter")
            .arg("-mexec-model=reactor")
            .arg("-g")
            .arg("-o")
            .arg(out_dir.join("c.wasm"));
        println!("{:?}", cmd);
        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => panic!("failed to spawn compiler: {}", e),
        };

        if !output.status.success() {
            println!("status: {}", output.status);
            println!("stdout: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stderr));
            panic!("failed to compile");
        }

        wasms.push(("c", out_dir.join("c.wasm")));
    }

    let src = format!("pub const WASMS: &[(&str, &str)] = &{:?};", wasms);
    std::fs::write(out_dir.join("wasms.rs"), src).unwrap();
}

fn build_rust(out_dir: &Path, unchecked: bool) -> PathBuf {
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
