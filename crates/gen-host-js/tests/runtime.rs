use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_bindgen_core::Generator;

test_helpers::runtime_tests!("ts");

fn execute(name: &str, wasm: &Path, ts: &Path, imports: &Path, exports: &Path) {
    let mut dir = PathBuf::from(env!("OUT_DIR"));
    dir.push(name);
    drop(fs::remove_dir_all(&dir));
    fs::create_dir_all(&dir).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    // We call `generate_all` with exports from the imports.wit file, and
    // imports from the exports.wit wit file. It's reversed because we're
    // implementing the host side of these APIs.
    let imports = wit_bindgen_core::wit_parser::Interface::parse_file(imports).unwrap();
    let exports = wit_bindgen_core::wit_parser::Interface::parse_file(exports).unwrap();
    let mut files = Default::default();
    wit_bindgen_gen_host_js::Opts::default()
        .build()
        .generate_all(&[exports], &[imports], &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join(file), contents).unwrap();
    }

    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    fs::copy(ts, dir.join("host.ts")).unwrap();
    fs::copy("tests/helpers.d.ts", dir.join("helpers.d.ts")).unwrap();
    fs::copy("tests/helpers.js", dir.join("helpers.js")).unwrap();
    let config = dir.join("tsconfig.json");
    fs::write(
        &config,
        format!(
            r#"
                {{
                    "files": ["host.ts"],
                    "compilerOptions": {{
                        "module": "esnext",
                        "target": "es2020",
                        "strict": true,
                        "strictNullChecks": true,
                        "baseUrl": {0:?},
                        "outDir": {0:?}
                    }}
                }}
            "#,
            dir,
        ),
    )
    .unwrap();

    run(Command::new(cmd)
        .args(args)
        .arg("tsc")
        .arg("--project")
        .arg(&config));

    // Currently there's mysterious uvwasi errors creating a `WASI` on Windows.
    // Unsure what's happening so let's ignore these tests for now since there's
    // not much Windows-specific here anyway.
    if cfg!(windows) {
        return;
    }

    fs::write(dir.join("package.json"), "{\"type\":\"module\"}").unwrap();
    let mut path = Vec::new();
    path.push(env::current_dir().unwrap());
    path.push(dir.clone());
    println!("{:?}", std::env::join_paths(&path));
    run(Command::new("node")
        .arg("--experimental-wasi-unstable-preview1")
        .arg(dir.join("host.js"))
        .env("NODE_PATH", std::env::join_paths(&path).unwrap())
        .arg(wasm));
}

fn run(cmd: &mut Command) {
    println!("running {:?}", cmd);
    let output = cmd.output().expect("failed to executed");
    println!("status: {}", output.status);
    println!(
        "stdout:\n  {}",
        String::from_utf8_lossy(&output.stdout).replace("\n", "\n  ")
    );
    println!(
        "stderr:\n  {}",
        String::from_utf8_lossy(&output.stderr).replace("\n", "\n  ")
    );
    assert!(output.status.success());
}
