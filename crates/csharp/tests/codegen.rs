// TODO: Implement tests similar to the other generators.
// This requires that we have any dependencies either included here or published to NuGet or similar.
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use wit_component::StringEncoding;

macro_rules! codegen_test {
    ($id:ident $name:tt $test:tt) => {
        #[test]
        fn $id() {
            test_helpers::run_world_codegen_test(
                "guest-csharp",
                $test.as_ref(),
                |resolve, world, files| {
                    if [
                        "conventions",
                        "guest-name",
                        "import-and-export-resource",
                        "import-and-export-resource-alias",
                        "import-func",
                        "interface-has-golang-keyword",
                        "issue544",
                        "issue551",
                        "issue569",
                        "issue573",
                        "issue607",
                        "issue668",
                        "enum-has-golang-keyword",
                        "just-export",
                        "lift-lower-foreign",
                        "lists",
                        "many-arguments",
                        "option-result",
                        "record-has-keyword-used-in-func",
                        "rename-interface",
                        "resource-alias",
                        "resource-borrow-in-record",
                        "resource-borrow-in-record-export",
                        "resource-local-alias",
                        "resource-local-alias-borrow",
                        "resource-local-alias-borrow-import",
                        "resource-own-in-other-interface",
                        "resources",
                        "resources-in-aggregates",
                        "resources-with-lists",
                        "result-empty",
                        "return-resource-from-export",
                        "same-names5",
                        "simple-http",
                        "small-anonymous",
                        "unused-import",
                        "use-across-interfaces",
                        "worlds-with-types",
                        "variants-unioning-types",
                        "go_params",
                        "wasi-cli",
                        "wasi-clocks",
                        "wasi-filesystem",
                        "wasi-http",
                        "wasi-io",
                        "issue929",
                        "issue929-no-import",
                        "issue929-no-export",
                        "issue929-only-methods",
                    ]
                    .contains(&$name)
                    {
                        return;
                    }
                    #[cfg(any(all(target_os = "windows", feature = "aot"), feature = "mono"))]
                    wit_bindgen_csharp::Opts {
                        generate_stub: true,
                        string_encoding: StringEncoding::UTF8,
                        #[cfg(all(target_os = "windows", feature = "aot"))]
                        runtime: Default::default(),
                        #[cfg(feature = "mono")]
                        runtime: wit_bindgen_csharp::CSharpRuntime::Mono,
                    }
                    .build()
                    .generate(resolve, world, files)
                    .unwrap()
                },
                verify,
            )
        }
    };
}
test_helpers::codegen_tests!();

fn verify(dir: &Path, name: &str) {
    #[cfg(all(target_os = "windows", feature = "aot"))]
    aot_verify(dir, name);

    #[cfg(feature = "mono")]
    mono_verify(dir, name);
}

#[cfg(feature = "aot")]
fn aot_verify(dir: &Path, name: &str) {
    let mut project = wit_bindgen_csharp::CSProject::new(dir.to_path_buf(), &name, "the_world");
    project.aot();
    project.clean();
    project.generate().unwrap();

    let dotnet_root_env = "DOTNET_ROOT";
    let dotnet_cmd: PathBuf;
    match env::var(dotnet_root_env) {
        Ok(val) => dotnet_cmd = Path::new(&val).join("dotnet"),
        Err(_e) => dotnet_cmd = "dotnet".into(),
    }

    let mut cmd = Command::new(dotnet_cmd.clone());

    cmd.current_dir(&dir);

    let mut wasm_filename = dir.join(name);
    wasm_filename.set_extension("wasm");
    //  add .arg("/bl") to diagnose dotnet build problems
    cmd.arg("build")
        .arg(dir.join(format!("TheWorldWorld.csproj")))
        .arg("-r")
        .arg("wasi-wasm")
        .arg("-c")
        .arg("Debug")
        .arg("/p:PlatformTarget=AnyCPU")
        .arg("/p:MSBuildEnableWorkloadResolver=false")
        .arg("--self-contained")
        .arg("/p:UseAppHost=false")
        .arg("-o")
        .arg(&wasm_filename);
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

    let mut cmd = Command::new(dotnet_cmd);
    match cmd
        .stdout(Stdio::null())
        .current_dir(&dir)
        .arg("clean")
        .spawn()
    {
        Err(e) => println!(
            "failed to clean project which may cause disk pressure in CI. {}",
            e
        ),
        _ => {}
    }
}

#[cfg(feature = "mono")]
fn mono_verify(dir: &Path, name: &str) {
    let mut project =
        wit_bindgen_csharp::CSProject::new_mono(dir.to_path_buf(), &name, "the_world");
    //project.aot();
    project.clean();
    project.generate().unwrap();

    let dotnet_root_env = "DOTNET_ROOT";
    let dotnet_cmd: PathBuf;
    match env::var(dotnet_root_env) {
        Ok(val) => dotnet_cmd = Path::new(&val).join("dotnet"),
        Err(_e) => dotnet_cmd = "dotnet".into(),
    }

    let mut cmd = Command::new(dotnet_cmd.clone());

    cmd.current_dir(&dir);

    let wasm_filename = dir.join(name);

    cmd.arg("build")
        .arg(dir.join(format!("TheWorld.csproj")))
        .arg("-c")
        .arg("Debug")
        .arg("-o")
        .arg(&wasm_filename);

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

    let mut cmd = Command::new(dotnet_cmd);
    match cmd
        .stdout(Stdio::null())
        .current_dir(&dir)
        .arg("clean")
        .spawn()
    {
        Err(e) => println!(
            "failed to clean project which may cause disk pressure in CI. {}",
            e
        ),
        _ => {}
    }
}
