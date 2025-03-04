use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::{Context, Result};
use clap::Parser;
use heck::ToSnakeCase;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::Command;

#[derive(Default, Debug, Clone, Parser)]
pub struct GoOpts {
    // no Go-specific options just yet
}

pub struct Go;

impl LanguageMethods for Go {
    fn display(&self) -> &str {
        "tiny-go"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        _name: &str,
        config: &crate::config::CodegenTestConfig,
        _args: &[String],
    ) -> bool {
        config.async_
    }

    fn prepare(&self, runner: &mut Runner<'_>) -> Result<()> {
        let _ = &runner.opts.go;
        runner.run_command(Command::new("tinygo").arg("version"))?;

        Ok(())
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile<'_>) -> Result<()> {
        let filename = compile.component.path.file_name().unwrap();
        fs::copy(
            &compile.component.path,
            compile.artifacts_dir.join(filename),
        )?;
        let go_mod = format!("module wit_{}_go\n\ngo 1.20", compile.component.kind);
        super::write_if_different(&compile.artifacts_dir.join("go.mod"), go_mod)?;

        let p1wasm = compile.output.with_extension("p1.wasm");
        let mut cmd = Command::new("tinygo");
        cmd.arg("build");
        cmd.arg("-target=wasi");
        cmd.arg("-o");
        cmd.arg(&p1wasm);
        cmd.arg(filename);
        cmd.current_dir(&compile.artifacts_dir);
        runner.run_command(&mut cmd)?;

        runner
            .convert_p1_to_component(&p1wasm, compile)
            .with_context(|| format!("failed to convert {p1wasm:?} to a component"))?;
        Ok(())
    }

    fn verify(&self, runner: &Runner<'_>, verify: &Verify<'_>) -> Result<()> {
        let name = verify.world.to_snake_case();
        let dir = verify.bindings_dir;
        let main = dir.join(format!("{name}.go"));

        // The generated go package is named after the world's name.
        // But tinygo currently does not support non-main package and requires
        // a `main()` function in the module to compile.
        // The following code replaces the package name to `package main` and
        // adds a `func main() {}` function at the bottom of the file.

        // TODO: However, there is still an issue. Since the go module does not
        // invoke the imported functions, they will be skipped by the compiler.
        // This will weaken the test's ability to verify imported functions
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&main)?;
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        reader.read_until(b'\n', &mut buf)?;
        // Skip over `package $WORLD` line
        reader.read_until(b'\n', &mut Vec::new())?;
        buf.append(&mut "package main\n".as_bytes().to_vec());

        // check if {name}_types.go exists
        let types_file = dir.join(format!("{name}_types.go"));
        if std::fs::metadata(types_file).is_ok() {
            // create a directory called option and move the type file to option
            std::fs::create_dir_all(dir.join("option"))?;
            std::fs::rename(
                dir.join(format!("{name}_types.go")),
                dir.join("option").join(format!("{name}_types.go")),
            )?;
            buf.append(&mut format!("import . \"{name}/option\"\n").as_bytes().to_vec());
        }

        reader.read_to_end(&mut buf)?;
        buf.append(&mut "func main() {}".as_bytes().to_vec());
        std::fs::write(&main, buf)?;

        // create go.mod file
        let mod_file = dir.join("go.mod");
        let mut file = std::fs::File::create(mod_file)?;
        file.write_all(format!("module {name}\n\ngo 1.20").as_bytes())?;

        // run tinygo on Dir directory

        let mut cmd = Command::new("tinygo");
        cmd.arg("build");
        cmd.arg("-target=wasi");
        cmd.arg("-o");
        cmd.arg("go.wasm");
        cmd.arg(format!("{name}.go"));
        cmd.current_dir(dir);
        runner.run_command(&mut cmd)
    }
}
