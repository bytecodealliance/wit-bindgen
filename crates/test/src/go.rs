use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::{Context as _, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Go;

impl LanguageMethods for Go {
    fn display(&self) -> &str {
        "go"
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
        // TODO: We _do_ support async, but only with a build of Go that has
        // [this
        // patch](https://github.com/dicej/go/commit/a1c83220fc9576cdb810e9624366cb998e69b22b).
        // Once we either publish builds containing that patch or upstream
        // something equivalent, we can remove the ` || config.async_` here.
        config.error_context || config.async_
    }

    fn default_bindgen_args_for_codegen(&self) -> &[&str] {
        &["--generate-stubs"]
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        let cwd = env::current_dir()?;
        let dir = cwd.join(&runner.opts.artifacts).join("go");

        super::write_if_different(&dir.join("test.go"), "package main\n\nfunc main() {}")?;
        super::write_if_different(&dir.join("go.mod"), "module test\n\ngo 1.25")?;

        println!("Testing if `go build` works...");
        runner.run_command(
            Command::new("go")
                .current_dir(&dir)
                .env("GOOS", "wasip1")
                .env("GOARCH", "wasm")
                .arg("build")
                .arg("-buildmode=c-shared")
                .arg("-ldflags=-checklinkname=0"),
        )
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile<'_>) -> Result<()> {
        let output = compile.output.with_extension("core.wasm");

        // Tests which involve importing and/or exporting more than one
        // interface may require more than one file since we can't define more
        // than one package in a single file in Go (AFAICT).  Here we search for
        // files related to `compile.component.path` based on a made-up naming
        // convention.  For example, if the filename is `test.go`, then we'll
        // also include `${prefix}+test.go` for any value of `${prefix}`.
        for path in all_paths(&compile.component.path)? {
            let test = fs::read_to_string(&path)
                .with_context(|| format!("unable to read `{}`", path.display()))?;
            let package_name = package_name(&test);
            let package_dir = compile.bindings_dir.join(package_name);
            fs::create_dir_all(&package_dir)
                .with_context(|| format!("unable to create `{}`", package_dir.display()))?;
            let output = &package_dir.join(path.file_name().unwrap());
            fs::write(output, test.as_bytes())
                .with_context(|| format!("unable to write `{}`", output.display()))?;
        }

        runner.run_command(
            Command::new("go")
                .current_dir(&compile.bindings_dir)
                .env("GOOS", "wasip1")
                .env("GOARCH", "wasm")
                .arg("build")
                .arg("-o")
                .arg(&output)
                .arg("-buildmode=c-shared")
                .arg("-ldflags=-checklinkname=0"),
        )?;

        runner.convert_p1_to_component(&output, compile)?;

        Ok(())
    }

    fn verify(&self, runner: &Runner<'_>, verify: &Verify<'_>) -> Result<()> {
        runner.run_command(
            Command::new("go")
                .current_dir(&verify.bindings_dir)
                .env("GOOS", "wasip1")
                .env("GOARCH", "wasm")
                .arg("build")
                .arg("-o")
                .arg(verify.artifacts_dir.join("tmp.wasm"))
                .arg("-buildmode=c-shared")
                .arg("-ldflags=-checklinkname=0"),
        )
    }
}

fn package_name(package: &str) -> &str {
    package
        .split_once('\n')
        .unwrap()
        .0
        .strip_prefix("package ")
        .unwrap()
        .trim()
}

fn all_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = vec![path.into()];
    let suffix = ".go";
    if let Some(name) = path
        .file_name()
        .unwrap()
        .to_str()
        .and_then(|name| name.strip_suffix(suffix))
    {
        let suffix = &format!("+{name}{suffix}");
        let parent = path.parent().unwrap();
        for entry in parent
            .read_dir()
            .with_context(|| format!("unable to read dir `{}`", parent.display()))?
        {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(suffix))
                .is_some()
            {
                paths.push(entry.path());
            }
        }
    }
    Ok(paths)
}
