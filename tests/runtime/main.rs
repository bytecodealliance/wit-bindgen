use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use wasmtime::component::{Component, Instance, Linker};
use wasmtime::{Config, Engine, Store};
use wit_component::ComponentEncoder;
use wit_parser::Resolve;

mod flavorful;
mod lists;
mod many_arguments;
mod numbers;
mod records;
mod smoke;
mod strings;
mod unions;
mod variants;

wasmtime::component::bindgen!("testwasi" in "crates/wasi_snapshot_preview1/wit");

#[derive(Default)]
struct Wasi<T>(T);

impl<T> testwasi::Testwasi for Wasi<T> {
    fn log(&mut self, bytes: Vec<u8>) -> Result<()> {
        std::io::stdout().write_all(&bytes)?;
        Ok(())
    }

    fn log_err(&mut self, bytes: Vec<u8>) -> Result<()> {
        std::io::stderr().write_all(&bytes)?;
        Ok(())
    }
}

fn run_test<T, U>(
    name: &str,
    add_to_linker: fn(&mut Linker<Wasi<T>>) -> Result<()>,
    instantiate: fn(&mut Store<Wasi<T>>, &Component, &Linker<Wasi<T>>) -> Result<(U, Instance)>,
    test: fn(U, &mut Store<Wasi<T>>) -> Result<()>,
) -> Result<()>
where
    T: Default,
{
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    for wasm in tests(name)? {
        let component = Component::from_file(&engine, &wasm)?;
        let mut linker = Linker::new(&engine);

        add_to_linker(&mut linker)?;
        crate::testwasi::add_to_linker(&mut linker, |x| x)?;
        let mut store = Store::new(&engine, Wasi::default());
        let (exports, _) = instantiate(&mut store, &component, &linker)?;

        println!("testing {wasm:?}");
        test(exports, &mut store)?;
    }

    Ok(())
}

fn tests(name: &str) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    let mut dir = PathBuf::from("./tests/runtime");
    dir.push(name);

    let mut resolve = Resolve::new();
    let (pkg, _files) = resolve.push_dir(&dir).unwrap();
    let world = resolve.packages[pkg]
        .documents
        .iter()
        .filter_map(|(_, doc)| resolve.documents[*doc].default_world)
        .next()
        .expect("no default world found");

    let mut rust = Vec::new();
    let mut c = Vec::new();
    let mut java = Vec::new();
    for file in dir.read_dir()? {
        let path = file?.path();
        match path.extension().and_then(|s| s.to_str()) {
            Some("c") => c.push(path),
            Some("java") => java.push(path),
            Some("rs") => rust.push(path),
            _ => {}
        }
    }

    let mut out_dir = std::env::current_exe()?;
    out_dir.pop();
    out_dir.pop();
    out_dir.pop();
    out_dir.push("runtime-tests");
    out_dir.push(name);

    println!("wasi adapter = {:?}", test_artifacts::ADAPTER);
    let wasi_adapter = std::fs::read(&test_artifacts::ADAPTER)?;

    drop(std::fs::remove_dir_all(&out_dir));
    std::fs::create_dir_all(&out_dir)?;

    if cfg!(feature = "rust") && !rust.is_empty() {
        let core = test_artifacts::WASMS
            .iter()
            .map(PathBuf::from)
            .find(|p| match p.file_stem().and_then(|s| s.to_str()) {
                Some(n) => n == name,
                None => false,
            })
            .unwrap();
        println!("rust core module = {core:?}");
        let module = std::fs::read(&core)?;
        let wasm = ComponentEncoder::default()
            .module(&module)?
            .validate(true)
            .adapter("wasi_snapshot_preview1", &wasi_adapter)?
            .encode()?;

        let dst = out_dir.join("rust.wasm");
        println!("rust component {dst:?}");
        std::fs::write(&dst, &wasm)?;
        result.push(dst);
    }

    #[cfg(feature = "c")]
    for path in c.iter() {
        let snake = resolve.worlds[world].name.replace("-", "_");
        let mut files = Default::default();
        let mut opts = wit_bindgen_gen_guest_c::Opts::default();
        if let Some(path) = path.file_name().and_then(|s| s.to_str()) {
            if path.contains("utf16") {
                opts.string_encoding = wit_component::StringEncoding::UTF16;
            }
        }
        opts.build().generate(&resolve, world, &mut files);

        for (file, contents) in files.iter() {
            let dst = out_dir.join(file);
            fs::write(dst, contents).unwrap();
        }

        let sdk =
            PathBuf::from(std::env::var_os("WASI_SDK_PATH").expect(
                "point the `WASI_SDK_PATH` environment variable to the path of your wasi-sdk",
            ));
        let mut cmd = Command::new(sdk.join("bin/clang"));
        let out_wasm = out_dir.join(format!(
            "c-{}.wasm",
            path.file_stem().and_then(|s| s.to_str()).unwrap()
        ));
        cmd.arg("--sysroot").arg(sdk.join("share/wasi-sysroot"));
        cmd.arg(path)
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
        let component_path = out_wasm.with_extension("component.wasm");
        fs::write(&component_path, component).expect("write component to disk");

        result.push(component_path);
    }

    // TODO: As of this writing, Maven has started failing to resolve dependencies on Windows in the GitHub action,
    // apparently unrelated to any code changes we've made.  Until we've either resolved the problem or developed a
    // workaround, we're disabling the tests.
    //
    // See https://github.com/bytecodealliance/wit-bindgen/issues/495 for more information.
    #[cfg(unix)]
    #[cfg(feature = "teavm-java")]
    if !java.is_empty() {
        use heck::*;

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
        for java_impl in &java {
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
            include_bytes!("../../crates/gen-guest-teavm-java/tests/Main.java"),
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

        result.push(component_path);

        fn mvn() -> Command {
            if cfg!(windows) {
                let mut cmd = Command::new("cmd");
                cmd.args(&["/c", "mvn"]);
                cmd
            } else {
                Command::new("mvn")
            }
        }

        fn pom_xml(classes_to_preserve: &[&str]) -> Vec<u8> {
            let xml = include_str!("../../crates/gen-guest-teavm-java/tests/pom.xml");
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
    }

    Ok(result)
}
