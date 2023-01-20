use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_bindgen_core::wit_parser::{Resolve, WorldId};
use wit_component::ComponentEncoder;

#[cfg(feature = "guest-c")]
fn guest_c(
    wasms: &mut Vec<(String, String, String, String)>,
    out_dir: &PathBuf,
    wasi_adapter: &[u8],
    utf_16: bool,
) {
    let utf16_suffix = if utf_16 { "_utf16" } else { "" };
    for test_dir in fs::read_dir("../../../tests/runtime").unwrap() {
        let test_dir = test_dir.unwrap().path();
        let c_impl = test_dir.join(format!("wasm{}.c", utf16_suffix));
        if !c_impl.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", c_impl.display());
        let (resolve, world) = read_world(&test_dir);
        let snake = resolve.worlds[world].name.replace("-", "_");
        let mut files = Default::default();
        let mut opts = wit_bindgen_gen_guest_c::Opts::default();
        if utf_16 {
            opts.string_encoding = wit_component::StringEncoding::UTF16;
        }
        opts.build().generate(&resolve, world, &mut files);

        let out_dir = out_dir.join(format!(
            "c{}-{}",
            utf16_suffix,
            test_dir.file_name().unwrap().to_str().unwrap()
        ));
        drop(fs::remove_dir_all(&out_dir));
        fs::create_dir(&out_dir).unwrap();
        for (file, contents) in files.iter() {
            let dst = out_dir.join(file);
            fs::write(dst, contents).unwrap();
        }

        let path =
            PathBuf::from(std::env::var_os("WASI_SDK_PATH").expect(
                "point the `WASI_SDK_PATH` environment variable to the path of your wasi-sdk",
            ));
        let mut cmd = Command::new(path.join("bin/clang"));
        let out_wasm = out_dir.join(format!("c{}.wasm", utf16_suffix));
        cmd.arg("--sysroot").arg(path.join("share/wasi-sysroot"));
        cmd.arg(c_impl)
            .arg(out_dir.join(format!("{snake}.c")))
            .arg(out_dir.join(format!("{snake}_component_type.o")))
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

        // Translate the canonical ABI module into a component.
        let module = fs::read(&out_wasm).expect("failed to read wasm file");
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .expect("pull custom sections from module")
            .validate(true)
            .adapter("wasi_snapshot_preview1", &wasi_adapter)
            .expect("adapter failed to get loaded")
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                out_wasm
            ));
        let component_path = out_dir.join(format!("c{}.component.wasm", utf16_suffix));
        fs::write(&component_path, component).expect("write component to disk");

        wasms.push((
            format!("c{}", utf16_suffix),
            resolve.worlds[world].name.to_string(),
            out_wasm.to_str().unwrap().to_string(),
            component_path.to_str().unwrap().to_string(),
        ));
    }
}

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let mut wasms: Vec<(String, String, String, String)> = Vec::new();

    // Build the `wasi_snapshot_preview1.wasm` adapter which is used to convert
    // core wasm modules below into components via `wit-component`.
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../../wasi_snapshot_preview1")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env(
            "RUSTFLAGS",
            "-Clink-args=--import-memory -Clink-args=-zstack-size=0",
        )
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());
    println!("cargo:rerun-if-changed=../../wasi_snapshot_preview1");
    let wasi_adapter = out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm");
    println!("wasi adapter: {:?}", &wasi_adapter);
    let wasi_adapter = std::fs::read(&wasi_adapter).unwrap();

    if cfg!(feature = "guest-rust") {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .current_dir("../../test-rust-wasm")
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
            let stem = file.file_stem().unwrap().to_str().unwrap().to_string();

            // Translate the canonical ABI module into a component.
            let module = fs::read(&file).expect("failed to read wasm file");
            let component = ComponentEncoder::default()
                .module(module.as_slice())
                .expect("pull custom sections from module")
                .validate(true)
                .adapter("wasi_snapshot_preview1", &wasi_adapter)
                .expect("adapter failed to get loaded")
                .encode()
                .expect(&format!(
                    "module {:?} can be translated to a component",
                    file
                ));
            let component_path = out_dir.join(format!("{}.component.wasm", stem));
            fs::write(&component_path, component).expect("write component to disk");

            wasms.push((
                "rust".into(),
                stem,
                file.to_str().unwrap().to_string(),
                component_path.to_str().unwrap().to_string(),
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
        println!("cargo:rerun-if-changed=../../test-rust-wasm/Cargo.toml");
    }

    #[cfg(feature = "guest-c")]
    {
        guest_c(&mut wasms, &out_dir, &wasi_adapter, false);
        guest_c(&mut wasms, &out_dir, &wasi_adapter, true);
    }

    #[cfg(feature = "guest-teavm-java")]
    {
        use heck::*;

        for test_dir in fs::read_dir("../../../tests/runtime").unwrap() {
            let test_dir = test_dir.unwrap().path();
            let java_impls = fs::read_dir(&test_dir)
                .unwrap()
                .filter_map(|entry| {
                    let path = entry.unwrap().path();
                    if let Some("java") = path.extension().map(|ext| ext.to_str().unwrap()) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if java_impls.is_empty() {
                continue;
            }
            for java_impl in &java_impls {
                println!("cargo:rerun-if-changed={}", java_impl.display());
            }

            let (resolve, world) = read_world(&test_dir);
            let world_name = &resolve.worlds[world].name;
            let out_dir = out_dir.join(format!("java-{}", world_name));
            drop(fs::remove_dir_all(&out_dir));
            let java_dir = out_dir.join("src/main/java");
            let mut files = Default::default();

            wit_bindgen_gen_guest_teavm_java::Opts::default()
                .build()
                .generate(&resolve, world, &mut files);

            let package_dir = java_dir.join(&format!("wit_{}", world_name));
            fs::create_dir_all(&package_dir).unwrap();
            for (file, contents) in files.iter() {
                let dst = package_dir.join(file);
                fs::write(dst, contents).unwrap();
            }

            let snake = world_name.to_snake_case();
            let upper = world_name.to_upper_camel_case();
            for java_impl in &java_impls {
                fs::copy(
                    &java_impl,
                    java_dir
                        .join(&format!("wit_{snake}"))
                        .join(java_impl.file_name().unwrap()),
                )
                .unwrap();
            }
            fs::write(
                out_dir.join("pom.xml"),
                pom_xml(&[
                    &format!("wit_{snake}.{upper}"),
                    &format!("wit_{snake}.{upper}World"),
                    &format!("wit_{snake}.Imports"),
                    &format!("wit_{snake}.Exports"),
                ]),
            )
            .unwrap();
            fs::write(
                java_dir.join("Main.java"),
                include_bytes!("../../gen-guest-teavm-java/tests/Main.java"),
            )
            .unwrap();

            let mut cmd = mvn();
            cmd.arg("prepare-package").current_dir(&out_dir);

            println!("{cmd:?}");
            let output = match cmd.output() {
                Ok(output) => output,
                Err(e) => panic!("failed to run Maven: {}", e),
            };

            if !output.status.success() {
                println!("status: {}", output.status);
                println!("stdout: ------------------------------------------");
                println!("{}", String::from_utf8_lossy(&output.stdout));
                println!("stderr: ------------------------------------------");
                println!("{}", String::from_utf8_lossy(&output.stderr));
                panic!("failed to build");
            }

            let out_wasm = out_dir.join("target/generated/wasm/teavm-wasm/classes.wasm");

            // Translate the canonical ABI module into a component.
            let module = fs::read(&out_wasm).expect("failed to read wasm file");
            let component = ComponentEncoder::default()
                .module(module.as_slice())
                .expect("pull custom sections from module")
                .validate(true)
                .adapter("wasi_snapshot_preview1", &wasi_adapter)
                .expect("adapter failed to get loaded")
                .encode()
                .expect(&format!(
                    "module {out_wasm:?} can be translated to a component",
                ));
            let component_path =
                out_dir.join("target/generated/wasm/teavm-wasm/classes.component.wasm");
            fs::write(&component_path, component).expect("write component to disk");

            wasms.push((
                "java".into(),
                test_dir.file_stem().unwrap().to_str().unwrap().to_string(),
                out_wasm.to_str().unwrap().to_string(),
                component_path.to_str().unwrap().to_string(),
            ));
        }
    }

    let src = format!("const WASMS: &[(&str, &str, &str, &str)] = &{:?};", wasms);
    std::fs::write(out_dir.join("wasms.rs"), src).unwrap();
}

fn read_world(dir: &Path) -> (Resolve, WorldId) {
    let mut resolve = Resolve::new();
    let (pkg, files) = resolve.push_dir(dir).unwrap();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    let world = resolve.packages[pkg]
        .documents
        .iter()
        .filter_map(|(_, doc)| resolve.documents[*doc].default_world)
        .next()
        .expect("no default world found");
    (resolve, world)
}

#[cfg(feature = "guest-teavm-java")]
fn mvn() -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(&["/c", "mvn"]);
        cmd
    } else {
        Command::new("mvn")
    }
}

#[cfg(feature = "guest-teavm-java")]
fn pom_xml(classes_to_preserve: &[&str]) -> Vec<u8> {
    let xml = include_str!("../../gen-guest-teavm-java/tests/pom.xml");
    let position = xml.find("<mainClass>").unwrap();
    let (before, after) = xml.split_at(position);
    let classes_to_preserve = classes_to_preserve
        .iter()
        .map(|&class| format!("<param>{class}</param>"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{before}
         <classesToPreserve>
            {classes_to_preserve}
         </classesToPreserve>
         {after}"
    )
    .into_bytes()
}
