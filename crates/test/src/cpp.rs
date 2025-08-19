use crate::config::StringList;
use crate::{Kind, LanguageMethods, Runner};
use anyhow::Context;
use heck::ToSnakeCase;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

// option wasi_sdk_path is inherited from C

pub struct Cpp;

/// C/C++-specific configuration of component files
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LangConfig {
    /// Space-separated list or array of compiler flags to pass.
    #[serde(default)]
    cflags: StringList,
}

fn clangpp(runner: &Runner<'_>) -> PathBuf {
    match &runner.opts.c.wasi_sdk_path {
        Some(path) => path.join("bin/wasm32-wasip2-clang++"),
        None => "wasm32-wasip2-clang++".into(),
    }
}

impl LanguageMethods for Cpp {
    fn display(&self) -> &str {
        "cpp"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn bindgen_name(&self) -> Option<&str> {
        Some("cpp")
    }

    fn should_fail_verify(
        &self,
        name: &str,
        _config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        match name {
            "async-trait-function.wit-base"
            | "error-context.wit-base"
            | "futures.wit-base"
            | "import_export_func.wit-base"
            | "import-func.wit-base"
            | "issue573.wit-base"
            | "issue929-no-export.wit-base"
            | "keywords.wit-base"
            | "lift-lower-foreign.wit-base"
            | "lists.wit-base"
            | "multiversion-base"
            | "resource-alias.wit-base"
            | "resource-borrow-in-record.wit-base"
            | "resources.wit-base"
            | "resources-in-aggregates.wit-base"
            | "resources-with-futures.wit-base"
            | "resources-with-streams.wit-base"
            | "ret-areas.wit-base"
            | "return-resource-from-export.wit-base"
            | "same-names1.wit-base"
            | "same-names5.wit-base"
            | "simple-http.wit-base"
            | "variants.wit-base"
            | "variants-unioning-types.wit-base"
            | "wasi-cli-base"
            | "wasi-filesystem-base"
            | "wasi-http-base"
            | "wasi-io-base"
            | "worlds-with-types.wit-base"
            | "streams.wit-base" => true,
            _ => false,
        }
    }

    fn prepare(&self, runner: &mut crate::Runner<'_>) -> anyhow::Result<()> {
        let compiler = clangpp(runner);
        let cwd = std::env::current_dir()?;
        let dir = cwd.join(&runner.opts.artifacts).join("cpp");

        super::write_if_different(&dir.join("test.cpp"), "int main() { return 0; }")?;

        println!("Testing if `{}` works...", compiler.display());
        runner
            .run_command(Command::new(&compiler).current_dir(&dir).arg("test.cpp"))
            .inspect_err(|_| {
                eprintln!(
                    "Error: failed to find `{}`. Hint: pass `--wasi-sdk-path` or set `WASI_SDK_PATH`",
                    compiler.display()
                );
            })?;

        Ok(())
    }

    fn generate_bindings_prepare(
        &self,
        _runner: &Runner<'_>,
        bindgen: &crate::Bindgen,
        dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let mut export_header_dir = bindgen.wit_path.clone();
        export_header_dir.pop();
        export_header_dir.push("cpp");

        // copy resource implementation in header files to target dir
        if export_header_dir.is_dir() {
            if !dir.exists() {
                std::fs::create_dir_all(dir).context("failed to create bindings dir")?;
            }
            for entry in export_header_dir
                .read_dir()
                .context("failed to read test header directory")?
            {
                let entry = entry.context("failed to read test header directory entry")?;
                let path = entry.path();
                let mut dest = PathBuf::from(dir);
                dest.push(path.file_name().unwrap());
                std::fs::copy(path, dest).context("failed to copy header file")?;
            }
        }
        Ok(())
    }

    fn compile(&self, runner: &crate::Runner<'_>, compile: &crate::Compile) -> anyhow::Result<()> {
        let compiler = clangpp(runner);
        let config = compile.component.deserialize_lang_config::<LangConfig>()?;

        let cwd = std::env::current_dir()?;
        let mut helper_dir = cwd.clone();
        helper_dir.push("crates");
        helper_dir.push("cpp");
        helper_dir.push("helper-types");
        // for expected
        let mut helper_dir2 = cwd;
        helper_dir2.push("crates");
        helper_dir2.push("cpp");
        helper_dir2.push("test_headers");

        // Compile the C-based bindings to an object file.
        let bindings_object = compile.output.with_extension("bindings.o");
        let mut cmd = Command::new(clangpp(runner));
        cmd.arg(
            compile
                .bindings_dir
                .join(format!("{}.cpp", compile.component.bindgen.world)),
        )
        .arg("-I")
        .arg(&compile.bindings_dir)
        .arg("-I")
        .arg(helper_dir.to_str().unwrap().to_string())
        .arg("-I")
        .arg(helper_dir2.to_str().unwrap().to_string())
        .arg("-fno-exceptions")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wno-unused-parameter")
        .arg("-std=c++20")
        .arg("-c")
        .arg("-g")
        .arg("-o")
        .arg(&bindings_object);
        runner.run_command(&mut cmd)?;

        // Now compile the runner's source code to with the above object and the
        // component-type object into a final component.
        let mut cmd = Command::new(compiler);
        cmd.arg(&compile.component.path)
            .arg(&bindings_object)
            .arg(compile.bindings_dir.join(format!(
                "{}_component_type.o",
                compile.component.bindgen.world
            )))
            .arg("-I")
            .arg(&compile.bindings_dir)
            .arg("-I")
            .arg(helper_dir.to_str().unwrap().to_string())
            .arg("-I")
            .arg(helper_dir2.to_str().unwrap().to_string())
            .arg("-fno-exceptions")
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            .arg("-Wc++-compat")
            .arg("-Wno-unused-parameter")
            .arg("-std=c++20")
            .arg("-g")
            .arg("-o")
            .arg(&compile.output);
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
        Ok(())
    }

    fn verify(&self, runner: &crate::Runner<'_>, verify: &crate::Verify) -> anyhow::Result<()> {
        // for expected
        let cwd = std::env::current_dir()?;
        let mut helper_dir2 = cwd;
        helper_dir2.push("crates");
        helper_dir2.push("cpp");
        helper_dir2.push("test_headers");

        let compiler = clangpp(runner);
        let mut cmd = Command::new(compiler);
        cmd.arg(
            verify
                .bindings_dir
                .join(format!("{}.cpp", verify.world.to_snake_case())),
        )
        .arg("-I")
        .arg(&verify.bindings_dir)
        .arg("-I")
        .arg(helper_dir2.to_str().unwrap().to_string())
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-Wc++-compat")
        .arg("-Wno-unused-parameter")
        .arg("-std=c++20")
        .arg("-c")
        .arg("-o")
        .arg(verify.artifacts_dir.join("tmp.o"));
        runner.run_command(&mut cmd)
    }
}
