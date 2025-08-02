use crate::LanguageMethods;
use anyhow::bail;
use serde::Deserialize;
use std::process::Command;

/// MoonBit configuration of project files
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LangConfig {
    #[serde(default)]
    path: String,
}

pub struct MoonBit;

impl LanguageMethods for MoonBit {
    fn display(&self) -> &str {
        "moonbit"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn default_bindgen_args(&self) -> &[&str] {
        &["--derive-show", "--derive-eq", "--derive-error"]
    }

    fn prepare(&self, runner: &mut crate::Runner<'_>) -> anyhow::Result<()> {
        println!("Testing if MoonBit toolchain exists...");
        if runner
            .run_command(Command::new("moon").arg("version"))
            .is_err()
        {
            bail!("MoonBit toolchain not found. Check out <https://www.moonbitlang.com/download>");
        }
        Ok(())
    }

    fn compile(&self, runner: &crate::Runner<'_>, compile: &crate::Compile) -> anyhow::Result<()> {
        let config = compile.component.deserialize_lang_config::<LangConfig>()?;
        // Copy the file to the bindings directory
        if !config.path.is_empty() {
            let src_path = &compile.component.path;
            let dest_path = compile.bindings_dir.join(config.path);
            std::fs::copy(src_path, dest_path)?;
        }
        // Compile the MoonBit bindings to a wasm file
        let mut cmd = Command::new("moon");
        cmd.arg("build")
            .arg("--no-strip") // for debugging
            .arg("--target")
            .arg("wasm")
            .arg("-C")
            .arg(compile.bindings_dir);
        runner.run_command(&mut cmd)?;
        // Build the component
        let artifact = compile
            .bindings_dir
            .join("target/wasm/release/build/gen/gen.wasm");
        // Embed WIT files
        let manifest_dir = compile.component.path.parent().unwrap();
        let mut cmd = Command::new("wasm-tools");
        cmd.arg("component")
            .arg("embed")
            .args(["--encoding", "utf16"])
            .args(["-o", artifact.to_str().unwrap()])
            .args(["-w", &compile.component.kind.to_string()])
            .arg(manifest_dir)
            .arg(&artifact);
        runner.run_command(&mut cmd)?;
        // Componentize the Wasm
        let mut cmd = Command::new("wasm-tools");
        cmd.arg("component")
            .arg("new")
            .args(["-o", compile.output.to_str().unwrap()])
            .arg(&artifact);
        runner.run_command(&mut cmd)?;
        Ok(())
    }

    fn should_fail_verify(
        &self,
        _name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        config.async_
    }

    fn verify(&self, runner: &crate::Runner<'_>, verify: &crate::Verify) -> anyhow::Result<()> {
        let mut cmd = Command::new("moon");
        cmd.arg("check")
            .arg("--target")
            .arg("wasm")
            .arg("--warn-list")
            .arg("-28")
            .arg("--deny-warn")
            .arg("--source-dir")
            .arg(verify.bindings_dir);

        runner.run_command(&mut cmd)?;
        let mut cmd = Command::new("moon");
        cmd.arg("build")
            .arg("--target")
            .arg("wasm")
            .arg("--source-dir")
            .arg(verify.bindings_dir);

        runner.run_command(&mut cmd)?;
        Ok(())
    }
}
