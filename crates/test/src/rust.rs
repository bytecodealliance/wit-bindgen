use crate::{Compile, Kind, LanguageMethods, Runner, Verify};
use anyhow::Result;
use clap::Parser;
use heck::ToSnakeCase;
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct RustOpts {
    /// A custom `path` dependency to use for `wit-bindgen`.
    #[clap(long, conflicts_with = "rust_wit_bindgen_version", value_name = "PATH")]
    rust_wit_bindgen_path: Option<PathBuf>,

    /// A custom version to use for the `wit-bindgen` dependency.
    #[clap(long, conflicts_with = "rust_wit_bindgen_path", value_name = "X.Y.Z")]
    rust_wit_bindgen_version: Option<String>,

    /// A custom version to use for the `wit-bindgen` dependency.
    #[clap(long, default_value = "wasm32-wasip2", value_name = "TARGET")]
    rust_target: String,
}

pub struct Rust;

#[derive(Default)]
pub struct State {
    wit_bindgen_rlib: PathBuf,
    wit_bindgen_deps: Vec<PathBuf>,
}

impl LanguageMethods for Rust {
    fn display(&self) -> &str {
        "rust"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        name: &str,
        config: &crate::config::CodegenTestConfig,
        args: &[String],
    ) -> bool {
        // no_std doesn't currently work with async
        if config.async_ && args.iter().any(|s| s == "--std-feature") {
            return true;
        }

        // Currently there's a bug with this borrowing mode which means that
        // this variant does not pass.
        if name == "wasi-http-borrowed-duplicate" {
            return true;
        }

        false
    }

    fn codegen_test_variants(&self) -> &[(&str, &[&str])] {
        &[
            ("borrowed", &["--ownership=borrowing"]),
            (
                "borrowed-duplicate",
                &["--ownership=borrowing-duplicate-if-necessary"],
            ),
            ("async", &["--async=all"]),
            ("no-std", &["--std-feature"]),
        ]
    }

    fn default_bindgen_args(&self) -> &[&str] {
        &["--generate-all"]
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        let cwd = env::current_dir()?;
        let opts = &runner.opts.rust;
        let dir = cwd.join(&runner.opts.artifacts).join("rust");
        let wit_bindgen = dir.join("wit-bindgen");

        let wit_bindgen_dep = match &opts.rust_wit_bindgen_path {
            Some(path) => format!("path = {:?}", cwd.join(path)),
            None => {
                let version = opts
                    .rust_wit_bindgen_version
                    .as_deref()
                    .unwrap_or(env!("CARGO_PKG_VERSION"));
                format!("version = \"{version}\"")
            }
        };

        super::write_if_different(
            &wit_bindgen.join("Cargo.toml"),
            &format!(
                r#"
[package]
name = "tmp"

[workspace]

[dependencies]
wit-bindgen = {{ {wit_bindgen_dep} }}

[lib]
path = 'lib.rs'
            "#,
            ),
        )?;
        super::write_if_different(&wit_bindgen.join("lib.rs"), "")?;

        println!("Building `wit-bindgen` from crates.io...");
        runner.run_command(
            Command::new("cargo")
                .current_dir(&wit_bindgen)
                .arg("build")
                .arg("-p")
                .arg("wit-bindgen")
                .arg("--target")
                .arg(&opts.rust_target),
        )?;

        let target_out_dir = wit_bindgen.join("target/wasm32-wasip2/debug");
        let host_out_dir = wit_bindgen.join("target/debug");
        let rlib = target_out_dir.join("libwit_bindgen.rlib");
        assert!(rlib.exists());

        runner.rust_state = Some(State {
            wit_bindgen_rlib: rlib,
            wit_bindgen_deps: vec![target_out_dir.join("deps"), host_out_dir.join("deps")],
        });
        Ok(())
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile) -> Result<()> {
        let mut cmd = runner.rustc();

        cmd.current_dir(compile.component.path.parent().unwrap())
            .env("CARGO_MANIFEST_DIR", ".")
            .env(
                "BINDINGS",
                compile
                    .bindings_dir
                    .join(format!("{}.rs", compile.component.kind)),
            )
            .arg(compile.component.path.file_name().unwrap())
            .arg("-Dwarnings")
            .arg("-o")
            .arg(&compile.output);
        match compile.component.kind {
            Kind::Runner => {}
            Kind::Test => {
                cmd.arg("--crate-type=cdylib");
            }
        }
        runner.run_command(&mut cmd)?;
        Ok(())
    }

    fn verify(&self, runner: &Runner<'_>, verify: &Verify<'_>) -> Result<()> {
        let mut cmd = runner.rustc();
        let bindings = verify
            .bindings_dir
            .join(format!("{}.rs", verify.world.to_snake_case()));
        cmd.arg(&bindings)
            .arg("--crate-type=rlib")
            .arg("-o")
            .arg(verify.artifacts_dir.join("tmp"));
        runner.run_command(&mut cmd)?;

        // If bindings are generated in `#![no_std]` mode then verify that it
        // compiles as such.
        if verify.args.iter().any(|s| s == "--std-feature") {
            let no_std_root = verify.artifacts_dir.join("no_std.rs");
            super::write_if_different(
                &no_std_root,
                r#"
#![no_std]
include!(env!("BINDINGS"));

// This empty module named 'core' is here to catch module path
// conflicts with 'core' modules used in code generated by the
// wit_bindgen::generate macro.
// Ref: https://github.com/bytecodealliance/wit-bindgen/pull/568
mod core {}
                "#,
            )?;
            let mut cmd = runner.rustc();
            cmd.arg(&no_std_root)
                .env("BINDINGS", &bindings)
                .arg("--crate-type=rlib")
                .arg("-o")
                .arg(verify.artifacts_dir.join("tmp"));
            runner.run_command(&mut cmd)?;
        }
        Ok(())
    }
}

impl Runner<'_> {
    fn rustc(&self) -> Command {
        let state = self.rust_state.as_ref().unwrap();
        let opts = &self.opts.rust;
        let mut cmd = Command::new("rustc");
        cmd.arg("--edition=2021")
            .arg(&format!(
                "--extern=wit_bindgen={}",
                state.wit_bindgen_rlib.display()
            ))
            .arg("--target")
            .arg(&opts.rust_target);
        for dep in state.wit_bindgen_deps.iter() {
            cmd.arg(&format!("-Ldependency={}", dep.display()));
        }
        cmd
    }
}
