use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::{Context, Result};
use clap::Parser;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct DOpts {}

pub struct D;

fn ldc2(_runner: &Runner) -> PathBuf {
    format!("ldc2").into()
}

impl LanguageMethods for D {
    fn display(&self) -> &str {
        "d"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        config.async_ || config.error_context || name == "map.wit"
    }

    fn default_bindgen_args_for_codegen(&self) -> &[&str] {
        &["--emit-export-stubs"]
    }

    fn prepare(&self, runner: &mut Runner) -> Result<()> {
        prepare(runner, ldc2(runner))
    }

    fn compile(&self, runner: &Runner, c: &Compile<'_>) -> Result<()> {
        compile(runner, c, ldc2(runner))
    }

    fn verify(&self, runner: &Runner, v: &Verify<'_>) -> Result<()> {
        verify(runner, v, ldc2(runner))
    }
}

fn prepare(runner: &mut Runner, compiler: PathBuf) -> Result<()> {
    let cwd = env::current_dir()?;
    let dir = cwd.join(&runner.opts.artifacts).join("d");

    super::write_if_different(&dir.join("test.d"), "extern(C) void _start() {}")?;

    println!("Testing if `{}` works...", compiler.display());
    runner
        .run_command(
            Command::new(&compiler)
                .current_dir(&dir)
                .arg("-mtriple=wasm32-unknown-unknown")
                .arg("-betterC")
                .arg("test.d"),
        )
        .inspect_err(|_| {
            eprintln!("Error: failed to find `{}`.", compiler.display());
        })?;

    Ok(())
}

fn search_for_world_package(bindings_root: &Path) -> Option<PathBuf> {
    // Look for a package.d generated from a world nested at wit/*/*/*/package.d

    // TODO: If we had access to the full package+version of the world being
    // generated, we wouldn't need to search.

    // ./wit/*
    fs::read_dir(bindings_root.join("wit"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        // ./wit/*/*
        .filter_map(|p| fs::read_dir(p).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        // ./wit/*/*/*
        .filter_map(|p| fs::read_dir(p).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        // ./wit/*/*/*/package.d
        .filter_map(|p| fs::read_dir(p).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file() && p.file_name().unwrap() == "package.d")
}

fn compile(runner: &Runner, compile: &Compile<'_>, compiler: PathBuf) -> Result<()> {
    let mut cmd = Command::new(compiler);

    let output = compile.output.with_extension("core.wasm");
    cmd.arg(&compile.component.path)
        .arg("-betterC")
        .arg("-mtriple=wasm32-unknown-unknown")
        .arg("-I")
        .arg(&compile.bindings_dir)
        .arg("-i") // compile included dependencies
        .arg("--de") // deperecations are errors
        .arg("-w") // warnings are errors
        .arg("-L--no-entry")
        .arg("-L--no-export-dynamic")
        .arg("--d-version=WitBindings_DummyLibc") // to provide bump allocator and `abort`
        .arg("--checkaction=halt") // to trap instead of using libc __assert
        .arg("-of")
        .arg(&output);

    runner.run_command(&mut cmd)?;

    runner
        .convert_p1_to_component(&output, compile)
        .with_context(|| format!("failed to convert {output:?}"))?;

    Ok(())
}

fn verify(runner: &Runner, verify: &Verify<'_>, compiler: PathBuf) -> Result<()> {
    let mut cmd = Command::new(compiler);

    let world_path = search_for_world_package(verify.bindings_dir).unwrap();

    cmd.arg(world_path)
        .arg("-betterC")
        .arg("-mtriple=wasm32-unknown-unknown")
        .arg("-I")
        .arg(&verify.bindings_dir)
        .arg("-i") // compile included dependencies
        .arg("-c") // compile only
        .arg("--de") // deperecations are errors
        .arg("-w") // warnigns are errors
        .arg("-of")
        .arg(verify.artifacts_dir.join("tmp.o"));
    runner.run_command(&mut cmd)
}
