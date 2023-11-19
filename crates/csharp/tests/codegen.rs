// TODO: Implement tests similar to the other generators.
// This requires that we have any dependencies either included here or published to NuGet or similar.
use std::{
    env, fs,
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
                        "flags",
                        "guest-name",
                        "import-and-export-resource",
                        "import-and-export-resource-alias",
                        "import-func",
                        "integers",
                        "issue544",
                        "issue551",
                        "issue569",
                        "issue573",
                        "issue607",
                        "issue668",
                        "just-export",
                        "keywords",
                        "lift-lower-foreign",
                        "lists",
                        "many-arguments",
                        "multi-return",
                        "multiversion",
                        "option-result",
                        "records",
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
                        "ret-areas",
                        "return-resource-from-export",
                        "same-names5",
                        "simple-functions",
                        "simple-http",
                        "simple-lists",
                        "small-anonymous",
                        "strings",
                        "unused-import",
                        "use-across-interfaces",
                        "variants",
                        "worlds-with-types",
                        "zero-size-tuple",
                    ]
                    .contains(&$name)
                    {
                        return;
                    }
                    wit_bindgen_csharp::Opts {
                        generate_stub: true,
                        string_encoding: StringEncoding::UTF8,
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
}

fn aot_verify(dir: &Path, name: &str) {
    let mut wasm_filename = dir.join(name);
    wasm_filename.set_extension("wasm");

    fs::write(
        dir.join("nuget.config"),
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
    ).unwrap();

    fs::write(
        dir.join("rd.xml"),
        format!(
            r#"<Directives xmlns="http://schemas.microsoft.com/netfx/2013/01/metadata">
        <Application>
            <Assembly Name="{name}">
            </Assembly>
        </Application>
    </Directives>"#
        ),
    )
    .unwrap();

    let mut csproj = format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">

<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <LangVersion>preview</LangVersion>
  <RootNamespace>{name}</RootNamespace>
  <ImplicitUsings>enable</ImplicitUsings>
  <Nullable>enable</Nullable>
  <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
</PropertyGroup>

<PropertyGroup>
    <PublishTrimmed>true</PublishTrimmed>
    <AssemblyName>{name}</AssemblyName>
</PropertyGroup>
"
    );

    csproj.push_str(
        r#"
<ItemGroup>
    <RdXmlFile Include="rd.xml" />
</ItemGroup>

"#,
    );

    csproj.push_str("\t<ItemGroup>\n");
    csproj.push_str(&format!(
        "\t\t<NativeLibrary Include=\"the_world_component_type.o\" />\n"
    ));
    csproj.push_str("\t</ItemGroup>\n\n");

    csproj.push_str(
        r#"
            <ItemGroup>
                <CustomLinkerArg Include="-Wl,--export,_initialize" />
                <CustomLinkerArg Include="-Wl,--no-entry" />
                <CustomLinkerArg Include="-mexec-model=reactor" />
            </ItemGroup>
            "#,
    );

    // In CI we run out of disk space if we don't clean up the files, we don't need to keep any of it around.
    csproj.push_str(&format!(
        "<Target Name=\"CleanAndDelete\"  AfterTargets=\"Clean\">
            <!-- Remove obj folder -->
            <RemoveDir Directories=\"$(BaseIntermediateOutputPath)\" />
            <!-- Remove bin folder -->
            <RemoveDir Directories=\"$(BaseOutputPath)\" />
            <RemoveDir Directories=\"{}\" />
            <RemoveDir Directories=\".packages\" />
        </Target>

",
        wasm_filename.display()
    ));

    csproj.push_str(
            r#"
    <ItemGroup>
        <PackageReference Include="Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
        <PackageReference Include="runtime.win-x64.Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
    </ItemGroup>
</Project>
            "#,
        );

    fs::write(dir.join(format!("{name}.csproj")), csproj).unwrap();

    let dotnet_root_env = "DOTNET_ROOT";
    let dotnet_cmd: PathBuf;
    match env::var(dotnet_root_env) {
        Ok(val) => dotnet_cmd = Path::new(&val).join("dotnet"),
        Err(_e) => dotnet_cmd = "dotnet".into(),
    }

    let mut cmd = Command::new(dotnet_cmd.clone());

    cmd.current_dir(&dir);

    //  add .arg("/bl") to diagnose dotnet build problems
    cmd.arg("build")
        .arg(dir.join(format!("{name}.csproj")))
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
