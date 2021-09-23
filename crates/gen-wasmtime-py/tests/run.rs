use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use witx_bindgen_gen_core::Generator;

#[test]
fn run() {
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
    witx_bindgen_gen_wasmtime_py::Opts::default()
        .build()
        .generate_all(&[iface], &[], &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("imports").join(file), contents).unwrap();
    }
    fs::write(dir.join("imports").join("__init__.py"), "").unwrap();

    let iface =
        witx_bindgen_gen_core::witx2::Interface::parse_file("../../tests/wasm.witx").unwrap();
    let mut files = Default::default();
    witx_bindgen_gen_wasmtime_py::Opts::default()
        .build()
        .generate_all(&[], &[iface], &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("exports").join(file), contents).unwrap();
    }
    fs::write(dir.join("exports").join("__init__.py"), "").unwrap();

    println!("Running mypy...");
    exec(
        Command::new("mypy")
            .env("MYPYPATH", &dir)
            .arg("tests/run.py"),
    );

    for (_name, wasm) in build_test_wasm::WASMS {
        println!("Running {}...", wasm);
        exec(
            Command::new("python3")
                .env("PYTHONPATH", &dir)
                .arg("tests/run.py")
                .arg(wasm),
        );
    }
}

fn exec(cmd: &mut Command) {
    println!("{:?}", cmd);
    let output = cmd.output().unwrap();
    if output.status.success() {
        return;
    }
    println!("status: {}", output.status);
    println!(
        "stdout ---\n  {}",
        String::from_utf8_lossy(&output.stdout).replace("\n", "\n  ")
    );
    println!(
        "stderr ---\n  {}",
        String::from_utf8_lossy(&output.stderr).replace("\n", "\n  ")
    );
    panic!("no success");
}
