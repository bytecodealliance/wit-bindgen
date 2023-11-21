use anyhow::Result;
use std::{fs, path::PathBuf};

use heck::ToUpperCamelCase;

pub struct CSProject {
    name: String,
    dir: PathBuf,
    aot: bool,
    clean_targets: bool,
    world_name: String,
    wasm_imports: Vec<(String, String)>,
}

impl CSProject {
    pub fn new(dir: PathBuf, name: &str, world_name: &str) -> CSProject {
        CSProject {
            name: name.to_string(),
            dir,
            aot: false,
            clean_targets: false,
            world_name: world_name.to_string(),
            wasm_imports: Vec::new(),
        }
    }

    pub fn generate(&self) -> Result<()> {
        let name = &self.name;
        let world = &self.world_name.replace("-", "_");
        let snake_world = world.to_upper_camel_case();

        fs::write(
            self.dir.join("rd.xml"),
            format!(
                r#"<Directives xmlns="http://schemas.microsoft.com/netfx/2013/01/metadata">
            <Application>
                <Assembly Name="{name}">
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
            <RootNamespace>{name}</RootNamespace>
            <ImplicitUsings>enable</ImplicitUsings>
            <Nullable>enable</Nullable>
            <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
        </PropertyGroup>
        
        <PropertyGroup>
            <PublishTrimmed>true</PublishTrimmed>
            <AssemblyName>{name}</AssemblyName>
        </PropertyGroup>
            <ItemGroup>
          <NativeLibrary Include=\"{world}_component_type.o\" />
   
        </ItemGroup>

        <ItemGroup>
            <RdXmlFile Include=\"rd.xml\" />
        </ItemGroup>
        "
        );

        if self.aot {
            //TODO: Is this handled by the source generator? (Temporary just to test with numbers and strings)
            csproj.push_str(
                r#"
                <ItemGroup>
                    <CustomLinkerArg Include="-Wl,--export,_initialize" />
                    <CustomLinkerArg Include="-Wl,--no-entry" />
                    <CustomLinkerArg Include="-mexec-model=reactor" />
                </ItemGroup>
   
                <ItemGroup>
                    <PackageReference Include="Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
                    <PackageReference Include="runtime.win-x64.Microsoft.DotNet.ILCompiler.LLVM" Version="8.0.0-*" />
                </ItemGroup>
                "#,
            );

            fs::write(
                self.dir.join("nuget.config"),
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
        }

        if !&self.wasm_imports.is_empty() {
            csproj.push_str("\t<ItemGroup>\n");
            for (module_name, func_name) in &self.wasm_imports {
                csproj.push_str(&format!(
                    r#"
                    <WasmImport Include="{}!{}" />
                    "#,
                    module_name, func_name,
                ));
            }
            csproj.push_str("\t</ItemGroup>\n\n");
        }

        if self.clean_targets {
            let mut wasm_filename = self.dir.join(name);
            wasm_filename.set_extension("wasm");
            // In CI we run out of disk space if we don't clean up the files, we don't need to keep any of it around.
            csproj.push_str(&format!(
                "<Target Name=\"CleanAndDelete\"  AfterTargets=\"Clean\">
                <!-- Remove obj folder -->
                <RemoveDir Directories=\"$(BaseIntermediateOutputPath)\" />
                <!-- Remove bin folder -->
                <RemoveDir Directories=\"$(BaseOutputPath)\" />
                <RemoveDir Directories=\"{}\" />
                <RemoveDir Directories=\".packages\" />
            </Target>",
                wasm_filename.display()
            ));
        }

        csproj.push_str(
            r#"</Project>
            "#,
        );

        let camel = snake_world.to_upper_camel_case();
        fs::write(self.dir.join(format!("{camel}.csproj")), csproj)?;

        Ok(())
    }

    pub fn aot(&mut self) {
        self.aot = true;
    }

    pub fn clean(&mut self) {
        self.clean_targets = true;
    }

    pub fn add_import(&mut self, module_name: &str, func_name: &str) {
        self.wasm_imports
            .push((module_name.to_string(), func_name.to_string()));
    }
}
