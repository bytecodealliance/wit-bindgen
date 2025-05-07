use crate::config::StringList;
use crate::{Compile, Kind, LanguageMethods, Runner, Verify};
use anyhow::{Context, Result};
use clap::Parser;
use heck::ToSnakeCase;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct COpts {
    /// Path to the installation of wasi-sdk
    #[clap(long, env = "WASI_SDK_PATH", value_name = "PATH")]
    wasi_sdk_path: Option<PathBuf>,

    /// Name of the C target to compile for.
    #[clap(long, default_value = "wasm32-wasip2", value_name = "TARGET")]
    c_target: String,
}

pub struct C;

pub struct Cpp;

/// C/C++-specific configuration of component files
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LangConfig {
    /// Space-separated list or array of compiler flags to pass.
    #[serde(default)]
    cflags: StringList,
}

fn clang(runner: &Runner<'_>) -> PathBuf {
    let target = &runner.opts.c.c_target;
    match &runner.opts.c.wasi_sdk_path {
        Some(path) => path.join(format!("bin/{target}-clang")),
        None => format!("{target}-clang").into(),
    }
}

fn clangpp(runner: &Runner<'_>) -> PathBuf {
    let target = &runner.opts.c.c_target;
    match &runner.opts.c.wasi_sdk_path {
        Some(path) => path.join(format!("bin/{target}-clang++")),
        None => format!("{target}-clang++").into(),
    }
}

impl LanguageMethods for C {
    fn display(&self) -> &str {
        "c"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        _name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        config.error_context
    }

    fn codegen_test_variants(&self) -> &[(&str, &[&str])] {
        &[
            ("no-sig-flattening", &["--no-sig-flattening"]),
            ("autodrop", &["--autodrop-borrows=yes"]),
            ("async", &["--async=all"]),
        ]
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        prepare(runner, clang(runner))
    }

    fn compile(&self, runner: &Runner<'_>, c: &Compile<'_>) -> Result<()> {
        compile(runner, c, clang(runner))
    }

    fn verify(&self, runner: &Runner<'_>, v: &Verify<'_>) -> Result<()> {
        verify(runner, v, clang(runner))
    }
}

impl LanguageMethods for Cpp {
    fn display(&self) -> &str {
        "cpp"
    }

    fn bindgen_name(&self) -> Option<&str> {
        Some("c")
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        name: &str,
        config: &crate::config::WitConfig,
        args: &[String],
    ) -> bool {
        C.should_fail_verify(name, config, args)
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        prepare(runner, clangpp(runner))
    }

    fn compile(&self, runner: &Runner<'_>, c: &Compile<'_>) -> Result<()> {
        compile(runner, c, clangpp(runner))
    }

    fn verify(&self, runner: &Runner<'_>, v: &Verify<'_>) -> Result<()> {
        verify(runner, v, clangpp(runner))
    }
}

fn prepare(runner: &mut Runner<'_>, compiler: PathBuf) -> Result<()> {
    let cwd = env::current_dir()?;
    let dir = cwd.join(&runner.opts.artifacts).join("c");

    super::write_if_different(&dir.join("test.c"), "int main() { return 0; }")?;

    println!("Testing if `{}` works...", compiler.display());
    runner
        .run_command(Command::new(&compiler).current_dir(&dir).arg("test.c"))
        .inspect_err(|_| {
            eprintln!(
                "Error: failed to find `{}`. Hint: pass `--wasi-sdk-path` or set `WASI_SDK_PATH`",
                compiler.display()
            );
        })?;

    Ok(())
}

fn compile(runner: &Runner<'_>, compile: &Compile<'_>, compiler: PathBuf) -> Result<()> {
    let config = compile.component.deserialize_lang_config::<LangConfig>()?;

    // Compile the C-based bindings to an object file.
    let bindings_object = compile.output.with_extension("bindings.o");
    let mut cmd = Command::new(clang(runner));
    cmd.arg(
        compile
            .bindings_dir
            .join(format!("{}.c", compile.component.bindgen.world)),
    )
    .arg("-I")
    .arg(&compile.bindings_dir)
    .arg("-Wall")
    .arg("-Wextra")
    .arg("-Werror")
    .arg("-Wno-unused-parameter")
    .arg("-c")
    .arg("-o")
    .arg(&bindings_object);
    for flag in Vec::from(config.cflags.clone()) {
        cmd.arg(flag);
    }
    runner.run_command(&mut cmd)?;

    // Now compile the runner's source code to with the above object and the
    // component-type object into a final component.
    let output = if produces_component(runner) {
        compile.output.to_path_buf()
    } else {
        compile.output.with_extension("core.wasm")
    };
    let mut cmd = Command::new(compiler);
    cmd.arg(&compile.component.path)
        .arg(&bindings_object)
        .arg(compile.bindings_dir.join(format!(
            "{}_component_type.o",
            compile.component.bindgen.world
        )))
        .arg("-I")
        .arg(&compile.bindings_dir)
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wc++-compat")
        .arg("-Wno-unused-parameter")
        .arg("-g")
        .arg("-o")
        .arg(&output);
    for flag in Vec::from(config.cflags) {
        cmd.arg(flag);
    }
    match compile.component.kind {
        Kind::Runner => {}
        Kind::Test => {
            cmd.arg("-mexec-model=reactor");
        }
    }
    runner.run_command(&mut cmd)?;

    if !produces_component(runner) {
        runner
            .convert_p1_to_component(&output, compile)
            .with_context(|| format!("failed to convert {output:?}"))?;
    }
    Ok(())
}

fn produces_component(runner: &Runner<'_>) -> bool {
    match runner.opts.c.c_target.as_str() {
        "wasm32-wasip1" => false,
        _ => true,
    }
}

fn verify(runner: &Runner<'_>, verify: &Verify<'_>, compiler: PathBuf) -> Result<()> {
    let mut cmd = Command::new(compiler);
    cmd.arg(
        verify
            .bindings_dir
            .join(format!("{}.c", verify.world.to_snake_case())),
    )
    .arg("-I")
    .arg(&verify.bindings_dir)
    .arg("-Wall")
    .arg("-Wextra")
    .arg("-Werror")
    .arg("-Wc++-compat")
    .arg("-Wno-unused-parameter")
    .arg("-c")
    .arg("-o")
    .arg(verify.artifacts_dir.join("tmp.o"));
    runner.run_command(&mut cmd)
}
