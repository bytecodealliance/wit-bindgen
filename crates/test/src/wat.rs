use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::Result;

pub struct Wat;

impl LanguageMethods for Wat {
    fn display(&self) -> &str {
        "wat"
    }

    fn bindgen_name(&self) -> Option<&str> {
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

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some(";;@")
    }

    fn prepare(&self, _runner: &mut Runner<'_>) -> Result<()> {
        Ok(())
    }

    fn compile(&self, runner: &Runner<'_>, compile: &Compile<'_>) -> Result<()> {
        let wasm = wat::parse_file(&compile.component.path)?;
        if wasmparser::Parser::is_component(&wasm) {
            super::write_if_different(&compile.output, wasm)?;
            return Ok(());
        }

        let p1 = compile.output.with_extension("core.wasm");
        super::write_if_different(&p1, wasm)?;
        runner.convert_p1_to_component(&p1, compile)?;
        Ok(())
    }

    fn verify(&self, _runner: &Runner<'_>, _verify: &Verify<'_>) -> Result<()> {
        // doesn't participate in codegen tests
        Ok(())
    }
}
