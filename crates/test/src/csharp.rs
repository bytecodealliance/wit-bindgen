use crate::{Compile, Kind, LanguageMethods, Runner, Verify};
use anyhow::Result;
use heck::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct Csharp;

fn dotnet() -> Command {
    let dotnet_cmd = match env::var("DOTNET_ROOT") {
        Ok(val) => Path::new(&val).join("dotnet"),
        Err(_e) => "dotnet".into(),
    };

    Command::new(dotnet_cmd)
}

impl LanguageMethods for Csharp {
    fn display(&self) -> &str {
        "csharp"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn default_bindgen_args(&self) -> &[&str] {
        &["--runtime=native-aot"]
    }

    fn default_bindgen_args_for_codegen(&self) -> &[&str] {
        &["--generate-stub"]
    }

    fn should_fail_verify(
        &self,
        _name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        config.async_
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        runner.run_command(dotnet().arg("--version"))?;

        Ok(())
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile<'_>) -> Result<()> {
        let world_name = &compile.component.bindgen.world;
        let path = &compile.component.path;
        let test_dir = &compile.bindings_dir;

        let new_path = test_dir.join("testcase.cs");
        fs::copy(path, &new_path)?;

        let camel = format!("{}World", world_name.to_upper_camel_case());

        let assembly_name = "csharp-testcase";

        let out_wasm = test_dir.join(&assembly_name);

        let mut csproj =
            wit_bindgen_csharp::CSProject::new(test_dir.to_path_buf(), &assembly_name, world_name);
        csproj.aot();
        if let Kind::Runner = compile.component.kind {
            csproj.binary();
        }
        csproj.generate()?;

        let mut cmd = dotnet();
        let mut wasm_filename = out_wasm.join(assembly_name);
        wasm_filename.set_extension("wasm");

        cmd.current_dir(test_dir)
            .arg("publish")
            .arg(test_dir.join(format!("{camel}.csproj")))
            .arg("-r")
            .arg("wasi-wasm")
            .arg("-c")
            .arg("Debug")
            .arg("/p:PlatformTarget=AnyCPU")
            .arg("/p:MSBuildEnableWorkloadResolver=false")
            .arg("--self-contained")
            .arg("/p:UseAppHost=false")
            // .arg("/bl") // to diagnose dotnet build problems
            .arg("-o")
            .arg(&out_wasm);
        runner.run_command(&mut cmd)?;

        fs::copy(&wasm_filename, &compile.output)?;

        Ok(())
    }

    fn verify(&self, runner: &Runner<'_>, verify: &Verify<'_>) -> Result<()> {
        let dir = verify.bindings_dir;
        let name = verify.world;
        let mut project = wit_bindgen_csharp::CSProject::new(dir.to_path_buf(), &name, "the_world");
        project.aot();
        project.clean();
        project.generate().unwrap();

        let mut cmd = dotnet();

        cmd.current_dir(&dir);

        let mut wasm_filename = dir.join(name);
        wasm_filename.set_extension("wasm");
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
            // .arg("/bl") // to diagnose dotnet build problems
            .arg("-o")
            .arg(&wasm_filename);
        runner.run_command(&mut cmd)?;

        runner.run_command(dotnet().current_dir(&dir).arg("clean"))
    }
}
