use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wit_bindgen_core::{wit_parser::Interface, Generator};

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let mut wasms = Vec::new();

    if cfg!(feature = "wasm-rust") {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .current_dir("../test-rust-wasm")
            .arg("--target=wasm32-wasi")
            .env("CARGO_TARGET_DIR", &out_dir)
            .env("CARGO_PROFILE_DEV_DEBUG", "1")
            .env("RUSTFLAGS", "-Clink-args=--export-table")
            .env_remove("CARGO_ENCODED_RUSTFLAGS");
        let status = cmd.status().unwrap();
        assert!(status.success());
        for file in out_dir.join("wasm32-wasi/debug").read_dir().unwrap() {
            let file = file.unwrap().path();
            if file.extension().and_then(|s| s.to_str()) != Some("wasm") {
                continue;
            }
            wasms.push((
                "rust",
                file.file_stem().unwrap().to_str().unwrap().to_string(),
                file.to_str().unwrap().to_string(),
            ));

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
        }
        println!("cargo:rerun-if-changed=../test-rust-wasm/Cargo.toml");
    }

    if cfg!(feature = "wasm-c") {
        for test_dir in fs::read_dir("../../tests/runtime").unwrap() {
            let test_dir = test_dir.unwrap().path();
            let c_impl = test_dir.join("wasm.c");
            if !c_impl.exists() {
                continue;
            }
            let imports = test_dir.join("imports.wit");
            let exports = test_dir.join("exports.wit");
            println!("cargo:rerun-if-changed={}", imports.display());
            println!("cargo:rerun-if-changed={}", exports.display());
            println!("cargo:rerun-if-changed={}", c_impl.display());

            let import = Interface::parse_file(&test_dir.join("imports.wit")).unwrap();
            let export = Interface::parse_file(&test_dir.join("exports.wit")).unwrap();
            let mut files = Default::default();
            // TODO: should combine this into one
            wit_bindgen_gen_guest_c::Opts::default()
                .build()
                .generate_all(&[import], &[], &mut files);
            wit_bindgen_gen_guest_c::Opts::default()
                .build()
                .generate_all(&[], &[export], &mut files);

            let out_dir = out_dir.join(format!(
                "c-{}",
                test_dir.file_name().unwrap().to_str().unwrap()
            ));
            drop(fs::remove_dir_all(&out_dir));
            fs::create_dir(&out_dir).unwrap();
            for (file, contents) in files.iter() {
                let dst = out_dir.join(file);
                fs::write(dst, contents).unwrap();
            }

            let path = PathBuf::from(env::var_os("WASI_SDK_PATH").expect(
                "point the `WASI_SDK_PATH` environment variable to the path of your wasi-sdk",
            ));
            let mut cmd = Command::new(path.join("bin/clang"));
            let out_wasm = out_dir.join("c.wasm");
            cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
            cmd.arg(c_impl)
                .arg(out_dir.join("imports.c"))
                .arg(out_dir.join("exports.c"))
                .arg("-I")
                .arg(&out_dir)
                .arg("-Wall")
                .arg("-Wextra")
                .arg("-Werror")
                .arg("-Wno-unused-parameter")
                .arg("-mexec-model=reactor")
                .arg("-g")
                .arg("-o")
                .arg(&out_wasm);
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

            wasms.push((
                "c",
                test_dir.file_stem().unwrap().to_str().unwrap().to_string(),
                out_wasm.to_str().unwrap().to_string(),
            ));
        }
    }

    if cfg!(feature = "wasm-spidermonkey") {
        for test_dir in fs::read_dir("../../tests/runtime").unwrap() {
            let test_dir = test_dir.unwrap().path();
            let js_impl = test_dir.join("wasm.js");
            if !js_impl.exists() {
                continue;
            }
            let imports = test_dir.join("imports.wit");
            let exports = test_dir.join("exports.wit");
            println!("cargo:rerun-if-changed={}", imports.display());
            println!("cargo:rerun-if-changed={}", exports.display());
            println!("cargo:rerun-if-changed={}", js_impl.display());

            let import = Interface::parse_file(&test_dir.join("imports.wit")).unwrap();
            let export = Interface::parse_file(&test_dir.join("exports.wit")).unwrap();
            let mut files = Default::default();
            let js = fs::read_to_string(&js_impl).unwrap();
            let mut gen =
                wit_bindgen_gen_guest_spidermonkey_js::SpiderMonkeyWasm::new("wasm.js", &js);
            gen.import_spidermonkey(true);
            gen.generate_all(&[import], &[export], &mut files);

            let out_dir = out_dir.join(format!(
                "js-{}",
                test_dir.file_name().unwrap().to_str().unwrap()
            ));
            drop(fs::remove_dir_all(&out_dir));
            fs::create_dir(&out_dir).unwrap();
            for (file, contents) in files.iter() {
                let dst = out_dir.join(file);
                fs::write(dst, contents).unwrap();
            }

            wasms.push((
                "spidermonkey",
                test_dir.file_stem().unwrap().to_str().unwrap().to_string(),
                out_dir.join("wasm.wasm").to_str().unwrap().to_string(),
            ));
        }
    }

    let src = format!("const WASMS: &[(&str, &str, &str)] = &{:?};", wasms);
    std::fs::write(out_dir.join("wasms.rs"), src).unwrap();
}
