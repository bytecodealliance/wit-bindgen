use anyhow::Result;
use heck::ToUpperCamelCase;

use std::borrow::Cow;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
use wasm_encoder::{Encode, Section};
use wasmtime::component::{Component, Instance, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiCtxBuilder, WasiView};
use wit_component::{ComponentEncoder, StringEncoding};
use wit_parser::{Resolve, WorldItem};

mod flavorful;
mod lists;
mod many_arguments;
mod numbers;
mod ownership;
mod records;
mod resource_aggregates;
mod resource_alias;
mod resource_alias_redux;
mod resource_borrow_export;
mod resource_borrow_import;
mod resource_borrow_in_record;
mod resource_borrow_simple;
mod resource_floats;
mod resource_import_and_export;
mod resource_into_inner;
mod resource_with_lists;
mod resources;
mod smoke;
mod strings;
mod variants;

struct MyCtx {}

struct Wasi<T: Send>(T, MyCtx, Table, WasiCtx);

// wasi trait
impl<T: Send> WasiView for Wasi<T> {
    fn table(&self) -> &Table {
        &self.2
    }
    fn table_mut(&mut self) -> &mut Table {
        &mut self.2
    }
    fn ctx(&self) -> &WasiCtx {
        &self.3
    }
    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.3
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
    T: Send,
{
    run_test_from_dir(name, name, add_to_linker, instantiate, test)
}

fn run_test_from_dir<T, U>(
    dir_name: &str,
    name: &str,
    add_to_linker: fn(&mut Linker<Wasi<T>>) -> Result<()>,
    instantiate: fn(&mut Store<Wasi<T>>, &Component, &Linker<Wasi<T>>) -> Result<(U, Instance)>,
    test: fn(U, &mut Store<Wasi<T>>) -> Result<()>,
) -> Result<()>
where
    T: Default,
    T: Send,
{
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    for wasm in tests(name, dir_name)? {
        let component = Component::from_file(&engine, &wasm)?;
        let mut linker = Linker::new(&engine);

        add_to_linker(&mut linker)?;
        let state = MyCtx {};

        let table = Table::new();
        let wasi: WasiCtx = WasiCtxBuilder::new().inherit_stdout().args(&[""]).build();

        let data = Wasi(T::default(), state, table, wasi);

        let mut store = Store::new(&engine, data);

        wasmtime_wasi::preview2::command::sync::add_to_linker(&mut linker)?;

        let (exports, _) = instantiate(&mut store, &component, &linker)?;

        println!("testing {wasm:?}");
        test(exports, &mut store)?;
    }

    Ok(())
}

fn tests(name: &str, dir_name: &str) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    let mut dir = PathBuf::from("./tests/runtime");
    dir.push(dir_name);

    let mut resolve = Resolve::new();
    let (pkg, _files) = resolve.push_dir(&dir).unwrap();
    let world = resolve.select_world(pkg, None).unwrap();

    let mut rust = Vec::new();
    let mut c = Vec::new();
    let mut java = Vec::new();
    let mut go = Vec::new();
    let mut c_sharp: Vec<PathBuf> = Vec::new();
    for file in dir.read_dir()? {
        let path = file?.path();
        match path.extension().and_then(|s| s.to_str()) {
            Some("c") => c.push(path),
            Some("java") => java.push(path),
            Some("rs") => rust.push(path),
            Some("go") => go.push(path),
            Some("cs") => {
                if cfg!(windows) {
                    c_sharp.push(path);
                }
            }
            _ => {}
        }
    }

    let mut out_dir = std::env::current_exe()?;
    out_dir.pop();
    out_dir.pop();
    out_dir.pop();
    out_dir.push("runtime-tests");
    out_dir.push(name);

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
            .realloc_via_memory_grow(true)
            .encode()?;

        let dst = out_dir.join("rust.wasm");
        println!("rust component {dst:?}");
        std::fs::write(&dst, &wasm)?;
        result.push(dst);
    }

    #[cfg(feature = "c")]
    for path in c.iter() {
        let world_name = &resolve.worlds[world].name;
        let out_dir = out_dir.join(format!("c-{}", world_name));
        drop(fs::remove_dir_all(&out_dir));
        fs::create_dir_all(&out_dir).unwrap();

        let snake = world_name.replace("-", "_");
        let mut files = Default::default();
        let mut opts = wit_bindgen_c::Opts::default();
        if let Some(path) = path.file_name().and_then(|s| s.to_str()) {
            if path.contains("utf16") {
                opts.string_encoding = wit_component::StringEncoding::UTF16;
            }
        }
        opts.build().generate(&resolve, world, &mut files).unwrap();

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

    // FIXME: need to fix flaky Go test
    #[cfg(feature = "go")]
    if !go.is_empty() {
        let world_name = &resolve.worlds[world].name;
        let out_dir = out_dir.join(format!("go-{}", world_name));
        let snake = world_name.replace("-", "_");
        drop(fs::remove_dir_all(&out_dir));

        let mut files = Default::default();
        wit_bindgen_go::Opts::default()
            .build()
            .generate(&resolve, world, &mut files)
            .unwrap();
        let gen_dir = out_dir.join("gen");
        fs::create_dir_all(&gen_dir).unwrap();
        for (file, contents) in files.iter() {
            let dst = gen_dir.join(file);
            fs::write(dst, contents).unwrap();
        }
        for go_impl in &go {
            fs::copy(&go_impl, out_dir.join(format!("{snake}.go"))).unwrap();
        }

        let go_mod = format!("module wit_{snake}_go\n\ngo 1.20");
        fs::write(out_dir.join("go.mod"), go_mod).unwrap();

        let out_wasm = out_dir.join("go.wasm");

        let mut cmd = Command::new("tinygo");
        cmd.arg("build");
        cmd.arg("-target=wasi");
        cmd.arg("-o");
        cmd.arg(&out_wasm);
        cmd.arg(format!("{snake}.go"));
        cmd.current_dir(&out_dir);

        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => panic!("failed to spawn compiler: {}", e),
        };

        if !output.status.success() {
            println!("dir: {}", out_dir.display());
            println!("status: {}", output.status);
            println!("stdout: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stderr));
            panic!("failed to compile");
        }

        // Translate the canonical ABI module into a component.

        let mut module = fs::read(&out_wasm).expect("failed to read wasm file");
        let encoded =
            wit_component::metadata::encode(&resolve, world, StringEncoding::UTF8, None, None)?;

        let section = wasm_encoder::CustomSection {
            name: Cow::Borrowed("component-type"),
            data: Cow::Borrowed(&encoded),
        };
        module.push(section.id());
        section.encode(&mut module);

        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .expect("pull custom sections from module")
            .validate(true)
            .adapter("wasi_snapshot_preview1", &wasi_adapter)
            .expect("adapter failed to get loaded")
            .encode()
            .expect(&format!(
                "module {:?} can't be translated to a component",
                out_wasm
            ));
        let component_path = out_wasm.with_extension("component.wasm");
        fs::write(&component_path, component).expect("write component to disk");

        result.push(component_path);
    }

    #[cfg(feature = "teavm-java")]
    if !java.is_empty() {
        const DEPTH_FROM_TARGET_DIR: u32 = 2;

        let base_dir = {
            let mut dir = out_dir.to_owned();
            for _ in 0..DEPTH_FROM_TARGET_DIR {
                dir.pop();
            }
            dir
        };

        let teavm_interop_jar = base_dir.join("teavm-interop-0.2.8.jar");
        let teavm_cli_jar = base_dir.join("teavm-cli-0.2.8.jar");
        if !(teavm_interop_jar.is_file() && teavm_cli_jar.is_file()) {
            panic!("please run ci/download-teavm.sh prior to running the Java tests")
        }

        let world_name = &resolve.worlds[world].name;
        let out_dir = out_dir.join(format!("java-{}", world_name));
        drop(fs::remove_dir_all(&out_dir));
        let java_dir = out_dir.join("src/main/java");
        let mut files = Default::default();

        wit_bindgen_teavm_java::Opts::default()
            .build()
            .generate(&resolve, world, &mut files)
            .unwrap();

        let mut dst_files = Vec::new();

        fs::create_dir_all(&java_dir).unwrap();
        for (file, contents) in files.iter() {
            let dst = java_dir.join(file);
            fs::create_dir_all(dst.parent().unwrap()).unwrap();
            fs::write(&dst, contents).unwrap();
            dst_files.push(dst);
        }

        for java_impl in java {
            let dst = java_dir.join(
                java_impl
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('_', "/"),
            );
            fs::copy(&java_impl, &dst).unwrap();
            dst_files.push(dst);
        }

        let main = java_dir.join("Main.java");

        fs::write(
            &main,
            include_bytes!("../../crates/teavm-java/tests/Main.java"),
        )
        .unwrap();

        dst_files.push(main);

        let mut cmd = Command::new("javac");
        cmd.arg("-cp")
            .arg(&teavm_interop_jar)
            .arg("-d")
            .arg(out_dir.join("target/classes"));

        for file in &dst_files {
            cmd.arg(file);
        }

        println!("{cmd:?}");
        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => panic!("failed to run javac: {}", e),
        };

        if !output.status.success() {
            println!("status: {}", output.status);
            println!("stdout: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: ------------------------------------------");
            println!("{}", String::from_utf8_lossy(&output.stderr));
            panic!("failed to build");
        }

        let mut cmd = Command::new("java");
        cmd.arg("-jar")
            .arg(&teavm_cli_jar)
            .arg("-p")
            .arg(out_dir.join("target/classes"))
            .arg("-d")
            .arg(out_dir.join("target/generated/wasm/teavm-wasm"))
            .arg("-t")
            .arg("wasm")
            .arg("-g")
            .arg("-O")
            .arg("1");

        for file in dst_files {
            cmd.arg("--preserve-class").arg(
                file.strip_prefix(&java_dir)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .strip_suffix(".java")
                    .unwrap()
                    .replace('/', "."),
            );
        }

        cmd.arg("Main");

        println!("{cmd:?}");
        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => panic!("failed to run teavm: {}", e),
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
    }

    #[cfg(feature = "csharp-mono")]
    for path in c_sharp.iter() {
        todo!()
    }

    #[cfg(feature = "csharp-naot")]
    for path in c_sharp.iter() {
        let world_name = &resolve.worlds[world].name;
        let out_dir = out_dir.join(format!("csharp-{}", world_name));
        drop(fs::remove_dir_all(&out_dir));
        fs::create_dir_all(&out_dir).unwrap();

        for csharp_impl in &c_sharp {
            fs::copy(
                &csharp_impl,
                &out_dir.join(csharp_impl.file_name().unwrap()),
            )
            .unwrap();
        }

        let snake = world_name.replace("-", "_");

        let assembly_name = format!(
            "csharp-{}",
            path.file_stem().and_then(|s| s.to_str()).unwrap()
        );

        let out_wasm = out_dir.join(&assembly_name);

        let mut files = Default::default();
        let mut opts = wit_bindgen_csharp::Opts::default();
        if let Some(path) = path.file_name().and_then(|s| s.to_str()) {
            if path.contains("utf16") {
                opts.string_encoding = wit_component::StringEncoding::UTF16;
            }
        }
        opts.build().generate(&resolve, world, &mut files).unwrap();

        fs::write(
            out_dir.join("nuget.config"),
            r#"<?xml version="1.0" encoding="utf-8"?>
        <configuration>
            <config>
                <add key="globalPackagesFolder" value=".packages" />
            </config>
            <packageSources>
            <!--To inherit the global NuGet package sources remove the <clear/> line below -->
            <clear />
            <add key="nuget" value="https://api.nuget.org/v3/index.json" />
            <add key="dotnet-experimental" value="https://pkgs.dev.azure.com/dnceng/public/_packaging/dotnet-experimental/nuget/v3/index.json" />
            <!--<add key="dotnet-experimental" value="C:\github\runtimelab\artifacts\packages\Debug\Shipping" />-->
          </packageSources>
        </configuration>"#,
        )?;

        fs::write(
            out_dir.join("rd.xml"),
            format!(
                r#"<Directives xmlns="http://schemas.microsoft.com/netfx/2013/01/metadata">
            <Application>
                <Assembly Name="{assembly_name}">
                </Assembly>
            </Application>
        </Directives>"#
            ),
        )?;

        let mut csproj = format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">

    <PropertyGroup>
      <TargetFramework>net8.0</TargetFramework>
      <LangVersion>preview</LangVersion>
      <RootNamespace>{assembly_name}</RootNamespace>
      <ImplicitUsings>enable</ImplicitUsings>
      <Nullable>enable</Nullable>
      <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    </PropertyGroup>

    <PropertyGroup>
        <PublishTrimmed>true</PublishTrimmed>
        <AssemblyName>{assembly_name}</AssemblyName>
    </PropertyGroup>
    "
        );

        for (file, contents) in files.iter() {
            let dst = out_dir.join(file);
            fs::write(dst, contents).unwrap();
        }

        csproj.push_str(
            r#"
    <ItemGroup>
        <RdXmlFile Include="rd.xml" />
    </ItemGroup>

"#,
        );

        // Write the WasmImports for NativeAOT-LLVM.
        // See https://github.com/dotnet/runtimelab/issues/2383

        for world in &resolve.worlds {
            for import in &world.1.imports {
                let module_name = resolve.name_world_key(import.0);
                csproj.push_str("\t<ItemGroup>\n");

                match import.1 {
                    WorldItem::Function(f) => {
                        let wasm_import = format!(
                            "\t\t<WasmImport Include=\"{}!{}\" />\n",
                            module_name, f.name
                        );
                        csproj.push_str(&wasm_import);
                    }
                    WorldItem::Interface(id) => {
                        for (_, f) in resolve.interfaces[*id].functions.iter() {
                            let wasm_import = format!(
                                "\t\t<WasmImport Include=\"{}!{}\" />\n",
                                module_name, f.name
                            );
                            csproj.push_str(&wasm_import);
                        }
                    }
                    WorldItem::Type(_) => {}
                }

                csproj.push_str("\t</ItemGroup>\n\n");
            }
        }

        csproj.push_str("\t<ItemGroup>\n");
        csproj.push_str(&format!(
            "\t\t<NativeLibrary Include=\"{snake}_component_type.o\" />\n"
        ));
        csproj.push_str("\t</ItemGroup>\n\n");

        //TODO: Is this handled by the source generator? (Temporary just to test with numbers and strings)
        csproj.push_str(
            r#"
            <ItemGroup>
                <CustomLinkerArg Include="-Wl,--export,_initialize" />
                <CustomLinkerArg Include="-Wl,--no-entry" />
                <CustomLinkerArg Include="-mexec-model=reactor" />
            </ItemGroup>
            "#,
        );

        csproj.push_str(
            r#"
    <ItemGroup>
        <PackageReference Include="Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
        <PackageReference Include="runtime.win-x64.Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
    </ItemGroup>
</Project>
            "#,
        );

        let camel = snake.to_upper_camel_case();

        fs::write(out_dir.join(format!("{camel}.csproj")), csproj)?;

        // Copy test file to target location to be included in compilation
        let file_name = path.file_name().unwrap();
        fs::copy(path, out_dir.join(file_name.to_str().unwrap()))?;

        let dotnet_root_env = "DOTNET_ROOT";
        let dotnet_cmd: PathBuf;
        match env::var(dotnet_root_env) {
            Ok(val) => dotnet_cmd = Path::new(&val).join("dotnet"),
            Err(_e) => dotnet_cmd = "dotnet".into(),
        }

        let mut cmd = Command::new(dotnet_cmd);
        let mut wasm_filename = out_wasm.join(assembly_name);
        wasm_filename.set_extension("wasm");

        cmd.current_dir(&out_dir);

        //  add .arg("/bl") to diagnose dotnet build problems
        cmd.arg("publish")
            .arg(out_dir.join(format!("{camel}.csproj")))
            .arg("-r")
            .arg("wasi-wasm")
            .arg("-c")
            .arg("Debug")
            .arg("/p:PlatformTarget=AnyCPU")
            .arg("/p:MSBuildEnableWorkloadResolver=false")
            .arg("--self-contained")
            .arg("/p:UseAppHost=false")
            .arg("-o")
            .arg(&out_wasm);
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

        let module = fs::read(&wasm_filename).expect("failed to read wasm file");
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .expect("pull custom sections from module")
            .validate(true)
            .adapter("wasi_snapshot_preview1", &wasi_adapter)
            .expect("adapter failed to get loaded")
            .realloc_via_memory_grow(true)
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                out_wasm
            ));
        let component_path = out_wasm.with_extension("component.wasm");
        fs::write(&component_path, component).expect("write component to disk");

        result.push(component_path);
    }

    Ok(result)
}
