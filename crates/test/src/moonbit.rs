use crate::{LanguageMethods, Runner};
use anyhow::{Context, bail};
use serde::Deserialize;
use std::process::Command;

/// MoonBit configuration of project files
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LangConfig {
    #[serde(default)]
    path: String,
    #[serde(default)]
    pkg_config: Option<String>,
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
        &[
            "--derive-debug",
            "--derive-show",
            "--derive-eq",
            "--derive-error",
        ]
    }

    fn prepare(&self, runner: &mut Runner) -> anyhow::Result<()> {
        println!("Testing if MoonBit toolchain exists...");
        if runner
            .run_command(Command::new("moon").arg("version"))
            .is_err()
        {
            bail!("MoonBit toolchain not found. Check out <https://www.moonbitlang.com/download>");
        }
        Ok(())
    }

    fn compile(&self, runner: &Runner, compile: &crate::Compile) -> anyhow::Result<()> {
        let config = compile.component.deserialize_lang_config::<LangConfig>()?;
        // Copy the file to the bindings directory
        if !config.path.is_empty() {
            let src_path = &compile.component.path;
            let dest_path = compile.bindings_dir.join(&config.path);
            std::fs::copy(src_path, dest_path)?;

            // Write the moon.pkg.json if provided
            if let Some(pkg_config) = config.pkg_config {
                let dest_path = compile
                    .bindings_dir
                    .join(&config.path)
                    .parent()
                    .unwrap()
                    .join("moon.pkg.json");
                std::fs::write(dest_path, pkg_config)?;
            }
        }

        // Compile the MoonBit bindings to a wasm file
        let mut cmd = Command::new("moon");
        cmd.arg("build")
            .arg("--target")
            .arg("wasm")
            .arg("--release")
            .arg("--no-strip") // for debugging
            .current_dir(&compile.bindings_dir);
        runner.run_command(&mut cmd)?;
        // Build the component. MoonBit toolchains may use either `_build` or
        // `target` output roots depending on version/configuration.
        let artifact_candidates = [
            compile
                .bindings_dir
                .join("_build/wasm/release/build/gen/gen.wasm"),
            compile
                .bindings_dir
                .join("target/wasm/release/build/gen/gen.wasm"),
            compile
                .bindings_dir
                .join("_build/wasm/debug/build/gen/gen.wasm"),
            compile
                .bindings_dir
                .join("target/wasm/debug/build/gen/gen.wasm"),
        ];
        let artifact = artifact_candidates
            .iter()
            .find(|path| path.exists())
            .cloned()
            .with_context(|| {
                format!("failed to locate MoonBit output wasm, looked in: {artifact_candidates:?}",)
            })?;
        // Embed WIT files
        let manifest_dir = compile.component.path.parent().unwrap();
        let embedded = artifact.with_extension("embedded.wasm");
        let mut cmd = Command::new("wasm-tools");
        cmd.arg("component")
            .arg("embed")
            .args(["--encoding", "utf16"])
            .args(["-o", embedded.to_str().unwrap()])
            .args(["-w", &compile.component.bindgen.world])
            .arg(manifest_dir)
            .arg(&artifact);
        runner.run_command(&mut cmd)?;
        // Componentize the Wasm
        let mut cmd = Command::new("wasm-tools");
        cmd.arg("component")
            .arg("new")
            .args(["-o", compile.output.to_str().unwrap()])
            .arg(&embedded);
        runner.run_command(&mut cmd)?;
        Ok(())
    }

    fn should_fail_verify(
        &self,
        name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        // async-resource-func actually works, but most other async tests
        // fail during codegen or verification
        config.async_ && name != "async-resource-func.wit"
    }

    fn verify(&self, runner: &Runner, verify: &crate::Verify) -> anyhow::Result<()> {
        let mut cmd = Command::new("moon");
        cmd.arg("check")
            .arg("--warn-list")
            .arg("-28") // avoid warning noise in generated bindings
            .current_dir(&verify.bindings_dir);

        runner.run_command(&mut cmd)?;
        let mut cmd = Command::new("moon");
        cmd.arg("build").current_dir(&verify.bindings_dir);

        runner.run_command(&mut cmd)?;
        Ok(())
    }
}
