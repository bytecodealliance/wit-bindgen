use crate::{Bindgen, Compile, LanguageMethods, Runner, Verify};
use anyhow::{bail, Context, Result};
use clap::Parser;
use std::env;
use std::path::Path;
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct CustomOpts {
    /// Specifies how to compile programs not natively known to this executable.
    ///
    /// For example `--custom foo=my-foo-script.sh` will register that files
    /// with the extension `foo` (e.g. `test.foo`) will be compiled with
    /// `my-foo-script.sh` by this program.
    ///
    /// The script specified will be invoked with its first argument as one of
    /// three values:
    ///
    /// * `prepare` - this is used to perform any one-time setup for an entire
    ///   test run, such as downloading artifacts. This has the `PREP_DIR`
    ///   environment variable set.
    ///
    /// * `bindgen` - this is used to perform bindings generation for the
    ///   program at-hand. This has the `WIT`, and `BINDINGS_DIR` env
    ///   vars set.
    ///
    /// * `compile` - this is used to perform an actual compilation which
    ///   creates a component. This has the `SOURCE`, `KIND`, `PREP_DIR`,
    ///   `BINDINGS_DIR`, `ARTIFACTS_DIR`, and `OUTPUT` env vars set.
    ///
    /// * `verify` - this is used to verify that generated bindings are valid,
    ///   but does not create an actual component necessarily.  This has the
    ///   `WIT`, `BINDINGS_DIR`, and `ARTIFACTS_DIR` env vars set.
    ///
    /// Environment variables are used to communicate various bits and pieces of
    /// data to scripts. Environment variables used are:
    ///
    /// * `PREP_DIR` - the output of the `prepare` step and also available
    ///   during the `compile` step. Used for once-per-test-run storage.
    ///
    /// * `WIT` - path to a `*.wit` file during the `bindgen` step.
    ///
    /// * `BINDINGS_DIR` - the output directory of `bindgen` and also inputs
    ///   to `compile`.
    ///
    /// * `SOURCE` - the source file being compiled as part of `compile`.
    ///
    /// * `KIND` - either `runner` or `test` as part of the `compile` step.
    ///
    /// * `ARTIFACTS_DIR` - temporary directory which contains `BINDINGS_DIR`
    ///   where temporary artifacts can be stored. Part of the `compile` step.
    ///
    /// * `OUTPUT` - where to place the final output component.
    #[arg(long , value_name = "EXT=PATH", value_parser = parse_custom)]
    pub custom: Vec<(String, String)>,
}

fn parse_custom(s: &str) -> Result<(String, String)> {
    let mut parts = s.splitn(2, '=');
    Ok((
        parts.next().unwrap().to_string(),
        parts
            .next()
            .context("must be of the form `a=b`")?
            .to_string(),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Language {
    extension: String,
    script: String,
}

impl Language {
    pub fn lookup(runner: &Runner<'_>, language: &str) -> Result<Language> {
        for (ext, script) in runner.opts.custom.custom.iter() {
            if ext == language {
                return Ok(Language {
                    extension: ext.to_string(),
                    script: script.to_string(),
                });
            }
        }

        bail!(
            "file extension `{language}` is unknown, but you can pass \
             a script with `--custom {language}=my-script.sh` to get it working"
        )
    }
}

impl LanguageMethods for Language {
    fn display(&self) -> &str {
        &self.extension
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        None
    }

    fn should_fail_verify(
        &self,
        _name: &str,
        _config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        false
    }

    fn generate_bindings(&self, runner: &Runner<'_>, bindgen: &Bindgen, dir: &Path) -> Result<()> {
        runner.run_command(
            Command::new(&self.script)
                .arg("bindgen")
                .env("WIT", &bindgen.wit_path)
                .env("BINDINGS_DIR", dir),
        )
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        let dir = env::current_dir()?
            .join(&runner.opts.artifacts)
            .join(&self.extension);
        runner.run_command(
            Command::new(&self.script)
                .arg("prepare")
                .env("PREP_DIR", &dir),
        )
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile<'_>) -> Result<()> {
        let dir = env::current_dir()?
            .join(&runner.opts.artifacts)
            .join(&self.extension);
        runner.run_command(
            Command::new(&self.script)
                .arg("compile")
                .env("SOURCE", &compile.component.path)
                .env("KIND", compile.component.kind.to_string())
                .env("PREP_DIR", &dir)
                .env("BINDINGS_DIR", &compile.bindings_dir)
                .env("ARTIFACTS_DIR", &compile.artifacts_dir)
                .env("OUTPUT", &compile.output),
        )
    }

    fn verify(&self, runner: &Runner<'_>, verify: &Verify<'_>) -> Result<()> {
        runner.run_command(
            Command::new(&self.script)
                .arg("verify")
                .env("WIT", verify.wit_test)
                .env("BINDINGS_DIR", &verify.bindings_dir)
                .env("ARTIFACTS_DIR", &verify.artifacts_dir),
        )
    }
}
