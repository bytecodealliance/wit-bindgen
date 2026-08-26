use crate::config::StringList;
use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::{Context, Result};
use clap::Parser;
use heck::ToSnakeCase;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct RustOpts {
    /// A custom `path` dependency to use for `wit-bindgen`.
    #[clap(long, conflicts_with = "rust_wit_bindgen_version", value_name = "PATH")]
    rust_wit_bindgen_path: Option<PathBuf>,

    /// A custom version to use for the `wit-bindgen` dependency.
    #[clap(long, conflicts_with = "rust_wit_bindgen_path", value_name = "X.Y.Z")]
    rust_wit_bindgen_version: Option<String>,

    /// Name of the Rust target to compile for.
    #[clap(long, default_value = "wasm32-wasip2", value_name = "TARGET")]
    rust_target: String,
}

pub struct Rust;

#[derive(Default)]
pub struct State {
    wit_bindgen_files: Vec<String>,
    futures_files: Vec<String>,
    wit_bindgen_deps: Vec<PathBuf>,
}

/// Rust-specific configuration of component files
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RustConfig {
    /// Space-separated list or array of compiler flags to pass.
    #[serde(default)]
    rustflags: StringList,

    /// List of path to rust files to build as external crates and link to the
    /// main crate.
    #[serde(default)]
    externs: Vec<String>,

    /// Whether or not to link the main crate as a shared library.
    ///
    /// This is implied if `extern_dylibs` is specified.
    #[serde(default)]
    link_shared: bool,

    /// Paths to Rust files to build as dynamic libraries. If non-empty the
    /// main file is also built as a dynamic library.
    #[serde(default)]
    extern_dylibs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum CargoMessage {
    CompilerArtifact {
        target: CargoTarget,
        filenames: Vec<String>,
    },
    BuildScriptExecuted {
        linked_paths: Vec<String>,
    },
}

#[derive(Deserialize, Debug)]
struct CargoTarget {
    name: String,
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
        _runner: &Runner,
        name: &str,
        _config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        // Currently there's a bug with this borrowing mode which means that
        // this variant does not pass.
        if name == "wasi-http-borrowed-duplicate" || name == "more-variants.wit-borrowed-duplicate"
        {
            return true;
        }

        // Named fixed-length lists don't work with async yet.
        if name == "named-fixed-length-list.wit-async" {
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
            ("merge-equal", &["--merge-structurally-equal-types"]),
            ("hashmap", &["--map-type=std::collections::HashMap"]),
        ]
    }

    fn default_bindgen_args(&self) -> &[&str] {
        &["--generate-all", "--format"]
    }

    fn default_bindgen_args_for_codegen(&self) -> &[&str] {
        &["--stubs"]
    }

    fn prepare(&self, runner: &mut Runner) -> Result<()> {
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
wit-bindgen = {{ {wit_bindgen_dep}, features = ['async-spawn', 'inter-task-wakeup', 'futures-stream'] }}
futures = "0.3.31"

[lib]
path = 'lib.rs'
            "#,
            ),
        )?;
        super::write_if_different(&wit_bindgen.join("lib.rs"), "")?;

        println!("Building `wit-bindgen` from crates.io...");
        let json = runner.run_command(
            Command::new("cargo")
                .current_dir(&wit_bindgen)
                .arg("build")
                .arg("-pwit-bindgen")
                .arg("-pfutures")
                .arg("--target")
                .arg(&opts.rust_target)
                .arg("--message-format=json"),
        )?;
        let mut wit_bindgen_files = None;
        let mut futures_files = None;
        let mut wit_bindgen_deps = Vec::new();
        let mut deps_seen = HashSet::new();

        for line in json.lines() {
            let Ok(msg) = serde_json::from_str(line) else {
                continue;
            };
            match msg {
                CargoMessage::CompilerArtifact { target, filenames } => {
                    if target.name == "futures" {
                        futures_files = Some(filenames.clone());
                    } else if target.name == "wit_bindgen" {
                        wit_bindgen_files = Some(filenames.clone());
                    }
                    for file in &filenames {
                        let dir = Path::new(file).parent().unwrap();
                        if deps_seen.insert(dir.to_path_buf()) {
                            wit_bindgen_deps.push(dir.to_path_buf());
                        }
                    }
                }
                CargoMessage::BuildScriptExecuted { linked_paths } => {
                    for path in linked_paths {
                        let path = PathBuf::from(path);
                        if deps_seen.insert(path.clone()) {
                            wit_bindgen_deps.push(path);
                        }
                    }
                }
            }
        }

        runner.rust_state = Some(State {
            wit_bindgen_files: wit_bindgen_files.unwrap().into(),
            futures_files: futures_files.unwrap().into(),
            wit_bindgen_deps,
        });
        Ok(())
    }

    fn compile(&self, runner: &Runner, compile: &Compile) -> Result<()> {
        let config = compile.component.deserialize_lang_config::<RustConfig>()?;
        let link_shared = config.link_shared || !config.extern_dylibs.is_empty();

        // If this rust target doesn't natively produce a component then place
        // the compiler output in a temporary location which is componentized
        // later on.
        let output = compile.output.with_extension("core.wasm");

        // Compile all extern crates, if any
        let mut externs = Vec::new();
        let mut dylibs = Vec::new();
        let manifest_dir = compile.component.path.parent().unwrap();

        let wasi_sdk_path = if link_shared {
            let path = runner.opts.c.wasi_sdk_path.as_ref();
            Some(path.ok_or_else(|| anyhow::anyhow!("need a wasi-sdk-path"))?)
        } else {
            None
        };

        let rustc = |path: &Path, output: &Path| {
            // Compile the main crate, passing `--extern` for all upstream crates.
            let mut cmd = runner.rustc(Edition::E2021);
            cmd.env("CARGO_MANIFEST_DIR", manifest_dir)
                .arg(path)
                .arg("-o")
                .arg(&output);
            for flag in Vec::from(config.rustflags.clone()) {
                cmd.arg(flag);
            }
            if link_shared {
                cmd.arg("-Clink-arg=-shared");
                cmd.arg("-Clink-self-contained=n");
                cmd.arg(&format!(
                    "-Clinker={}/bin/clang",
                    wasi_sdk_path.unwrap().display()
                ));
                cmd.arg("-L").arg(&compile.artifacts_dir);
            }
            cmd
        };

        let compile_cdylib = |cmd: &mut Command| {
            cmd.arg("--crate-type=cdylib");
            if runner.produces_component() {
                if link_shared {
                    cmd.arg("-Clink-arg=-Wl,--skip-wit-component");
                } else {
                    cmd.arg("-Clink-arg=--skip-wit-component");
                }
            }
        };

        for file in config.externs.iter() {
            let file = manifest_dir.join(file);
            let stem = file.file_stem().unwrap().to_str().unwrap();
            let output = compile.artifacts_dir.join(format!("lib{stem}.rlib"));
            runner.run_command(rustc(&file, &output).arg("--crate-type=rlib"))?;
            externs.push((stem.to_string(), output));
        }

        for file in config.extern_dylibs.iter() {
            let file = manifest_dir.join(file);
            let stem = file.file_stem().unwrap().to_str().unwrap();
            let output = compile.artifacts_dir.join(format!("lib{stem}.so"));
            let mut cmd = rustc(&file, &output);
            compile_cdylib(&mut cmd);
            runner.run_command(&mut cmd)?;
            dylibs.push(output);
        }

        // Compile the main crate, passing `--extern` for all upstream crates.
        let mut cmd = rustc(&compile.component.path, &output);
        cmd.env(
            "BINDINGS",
            compile.bindings_dir.join(format!(
                "{}.rs",
                compile.component.bindgen.world.replace('-', "_")
            )),
        );
        for (name, path) in externs {
            let arg = format!("--extern={name}={}", path.display());
            cmd.arg(arg);
        }
        compile_cdylib(&mut cmd);
        runner.run_command(&mut cmd)?;

        if link_shared {
            let libc_so = wasi_sdk_path.unwrap().join(&format!(
                "share/wasi-sysroot/lib/{}/libc.so",
                runner.opts.rust.rust_target,
            ));
            if !libc_so.is_file() {
                anyhow::bail!("libc.so not found at {libc_so:?}");
            }
            dylibs.insert(0, libc_so);
            dylibs.push(output.clone());
            runner
                .link_dylibs_to_component(&dylibs, compile)
                .with_context(|| format!("failed to link {output:?}"))?;
        } else {
            runner
                .convert_p1_to_component(&output, compile)
                .with_context(|| format!("failed to convert {output:?}"))?;
        }

        Ok(())
    }

    fn verify(&self, runner: &Runner, verify: &Verify<'_>) -> Result<()> {
        let bindings = verify
            .bindings_dir
            .join(format!("{}.rs", verify.world.to_snake_case()));
        let test_edition = |edition: Edition| -> Result<()> {
            let mut cmd = runner.rustc(edition);
            cmd.arg(&bindings)
                .arg("--crate-type=rlib")
                .arg("-o")
                .arg(verify.artifacts_dir.join("tmp"));
            runner.run_command(&mut cmd)?;
            Ok(())
        };

        test_edition(Edition::E2021)?;
        test_edition(Edition::E2024)?;

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
            let mut cmd = runner.rustc(Edition::E2021);
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

enum Edition {
    E2021,
    E2024,
}

impl Runner {
    fn rustc(&self, edition: Edition) -> Command {
        let state = self.rust_state.as_ref().unwrap();
        let opts = &self.opts.rust;
        let mut cmd = Command::new("rustc");
        cmd.arg(match edition {
            Edition::E2021 => "--edition=2021",
            Edition::E2024 => "--edition=2024",
        })
        .arg("--target")
        .arg(&opts.rust_target)
        .arg("-Dwarnings")
        .arg("-Cdebuginfo=1");
        for dep in state.wit_bindgen_deps.iter() {
            let dep = dep.display();
            if dep.to_string().contains('=') {
                cmd.arg(&format!("-L{dep}"));
            } else {
                cmd.arg(&format!("-Ldependency={dep}"));
            }
        }
        for (name, files) in [
            ("wit_bindgen", &state.wit_bindgen_files),
            ("futures", &state.futures_files),
        ] {
            for file in files {
                cmd.arg(&format!("--extern={name}={file}",));
            }
        }
        cmd
    }

    fn produces_component(&self) -> bool {
        match self.opts.rust.rust_target.as_str() {
            "wasm32-unknown-unknown" | "wasm32-wasi" | "wasm32-wasip1" => false,
            _ => true,
        }
    }
}
