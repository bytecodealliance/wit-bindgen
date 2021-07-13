use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use witx_bindgen_gen_core::witx2::abi::Direction;
use witx_bindgen_gen_core::Generator;

fn main() {
    let mut dir = PathBuf::from(env!("OUT_DIR"));
    dir.push("run");
    drop(fs::remove_dir_all(&dir));
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&dir.join("imports")).unwrap();
    fs::create_dir_all(&dir.join("exports")).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    let iface =
        witx_bindgen_gen_core::witx2::Interface::parse_file("../../tests/host.witx").unwrap();
    let mut files = Default::default();
    witx_bindgen_gen_js::Opts::default()
        .build()
        .generate(&iface, Direction::Import, &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("imports").join(file), contents).unwrap();
    }

    let iface =
        witx_bindgen_gen_core::witx2::Interface::parse_file("../../tests/wasm.witx").unwrap();
    let mut files = Default::default();
    witx_bindgen_gen_js::Opts::default()
        .build()
        .generate(&iface, Direction::Export, &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("exports").join(file), contents).unwrap();
    }

    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    println!("Running tsc...");
    fs::copy("tests/run.ts", dir.join("run.ts")).unwrap();
    let status = Command::new(cmd)
        .args(args)
        .arg("tsc")
        .arg(dir.join("run.ts"))
        .arg("--strictNullChecks")
        .arg("--target")
        .arg("es2020")
        .arg("--module")
        .arg("esnext")
        .status()
        .unwrap();
    assert!(status.success());
    fs::write(dir.join("package.json"), "{\"type\":\"module\"}").unwrap();

    for (_name, wasm) in build_test_wasm::WASMS {
        println!("Running {}...", wasm);
        let status = Command::new("node")
            .arg("--experimental-wasi-unstable-preview1")
            .arg(dir.join("run.js"))
            .arg(wasm)
            .status()
            .unwrap();
        if status.success() {
            continue;
        }
        println!("status: {:?}", status);
        panic!("failed");
    }
}
