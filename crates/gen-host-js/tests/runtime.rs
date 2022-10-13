use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

test_helpers::runtime_component_tests!("ts");

fn execute(name: &str, lang: &str, wasm: &Path, ts: &Path) {
    let dir = test_helpers::test_directory("runtime", "js", &format!("{name}-{lang}"));
    let wasm = std::fs::read(wasm).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    let mut files = Default::default();
    wit_bindgen_gen_host_js::Opts::default()
        .generate(name, &wasm, &mut files)
        .unwrap();
    for (file, contents) in files.iter() {
        let dst = dir.join(file);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(&dst, contents).unwrap();
    }

    let (cmd, args) = if cfg!(windows) {
        ("cmd.exe", &["/c", "npx.cmd"] as &[&str])
    } else {
        ("npx", &[] as &[&str])
    };

    fs::copy(ts, dir.join("host.ts")).unwrap();
    fs::copy("tests/helpers.ts", dir.join("helpers.ts")).unwrap();
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

    test_helpers::run_command(
        Command::new(cmd)
            .args(args)
            .arg("tsc")
            .arg("--project")
            .arg(&config),
    );

    fs::write(dir.join("package.json"), "{\"type\":\"module\"}").unwrap();
    let mut path = Vec::new();
    path.push(env::current_dir().unwrap());
    path.push(dir.clone());
    test_helpers::run_command(
        Command::new("node")
            .arg("--stack-trace-limit=1000")
            .arg(dir.join("host.js"))
            .env("NODE_PATH", std::env::join_paths(&path).unwrap())
            .arg(dir),
    );
}
