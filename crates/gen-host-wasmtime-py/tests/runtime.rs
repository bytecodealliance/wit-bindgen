use std::path::Path;
use std::process::Command;

test_helpers::runtime_component_tests!("py");

fn execute(name: &str, lang: &str, wasm: &Path, py: &Path) {
    let dir = test_helpers::test_directory("runtime", "wasmtime-py", &format!("{lang}/{name}"));
    let wasm = std::fs::read(wasm).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    let mut files = Default::default();
    wit_bindgen_core::component::generate(
        &mut *wit_bindgen_gen_host_wasmtime_py::Opts::default().build(),
        name,
        &wasm,
        &mut files,
    )
    .unwrap();
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    let cwd = std::env::current_dir().unwrap();
    println!("Running mypy...");
    let pathdir = std::env::join_paths([
        dir.parent().unwrap().to_str().unwrap(),
        cwd.join("tests").to_str().unwrap(),
    ])
    .unwrap();
    test_helpers::run_command(
        Command::new("mypy")
            .env("MYPYPATH", &pathdir)
            .arg(py)
            .arg("--cache-dir")
            .arg(dir.parent().unwrap().join("mypycache").join(name)),
    );

    test_helpers::run_command(Command::new("python3").env("PYTHONPATH", &pathdir).arg(py));
}
