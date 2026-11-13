use crate::{Compile, LanguageMethods, Runner, Verify};
use anyhow::{Context, Result, bail};
use std::fs;
use std::process::Command;

pub struct Zig;

impl LanguageMethods for Zig {
    fn display(&self) -> &str {
        "zig"
    }

    fn comment_prefix_for_test_config(&self) -> Option<&str> {
        Some("//@")
    }

    fn should_fail_verify(
        &self,
        _runner: &Runner,
        name: &str,
        config: &crate::config::WitConfig,
        _args: &[String],
    ) -> bool {
        config.async_
            || config.error_context
            || name.contains("future")
            || name.contains("stream")
            || name.contains("map")
            || name.contains("resource-borrow-in-record")
    }

    fn prepare(&self, runner: &mut Runner) -> Result<()> {
        println!("Testing if `zig` toolchain exists...");
        if runner
            .run_command(Command::new("zig").arg("version"))
            .is_err()
        {
            bail!("`zig` toolchain not found");
        }
        Ok(())
    }

    fn compile(&self, runner: &Runner, compile: &Compile<'_>) -> Result<()> {
        let output = compile.output.with_extension("core.wasm");
        let wit_root = compile.bindings_dir.join("wit.zig");

        let mut cmd = Command::new("zig");
        cmd.arg("build-exe")
            .arg("-target")
            .arg("wasm32-wasi")
            .arg("-O")
            .arg("ReleaseSmall")
            .arg("-fno-entry")
            .arg("-rdynamic")
            .arg("-mexec-model=reactor")
            .arg("--dep")
            .arg("wit")
            .arg(format!("-Mroot={}", compile.component.path.display()))
            .arg(format!("-Mwit={}", wit_root.display()))
            .arg(format!("-femit-bin={}", output.display()));

        runner.run_command(&mut cmd)?;

        runner
            .convert_core_module_to_component(&output, compile)
            .with_context(|| format!("failed to convert {output:?}"))?;

        Ok(())
    }

    fn verify(&self, runner: &Runner, verify: &Verify<'_>) -> Result<()> {
        let check_zig = verify.artifacts_dir.join("check.zig");
        let wit_root = verify.bindings_dir.join("wit.zig");
        let tmp_obj = verify.artifacts_dir.join("check.o");

        let check_src = r#"const std = @import("std");
const wit = @import("wit");

fn collect(comptime T: type) []const *const anyopaque {
    var out: []const *const anyopaque = &.{};
    for (std.meta.declarations(T)) |decl| {
        const Decl = @TypeOf(@field(T, decl.name));
        if (Decl == type) {
            const Inner = @field(T, decl.name);
            switch (@typeInfo(Inner)) {
                .@"struct", .@"enum", .@"union" => out = out ++ collect(Inner),
                else => {},
            }
        } else if (@typeInfo(Decl) == .@"fn" and !@typeInfo(Decl).@"fn".is_generic) {
            out = out ++ [1]*const anyopaque{@ptrCast(&@field(T, decl.name))};
        }
    }
    return out;
}

const all = blk: {
    @setEvalBranchQuota(10_000_000);
    const list = collect(wit);
    break :blk list[0..list.len].*;
};

export const table: [all.len]*const anyopaque = all;
"#;
        fs::write(&check_zig, check_src)?;

        let mut cmd = Command::new("zig");
        cmd.arg("build-obj")
            .arg("-target")
            .arg("wasm32-wasi")
            .arg("-O")
            .arg("ReleaseSmall")
            .arg("--dep")
            .arg("wit")
            .arg(format!("-Mroot={}", check_zig.display()))
            .arg(format!("-Mwit={}", wit_root.display()))
            .arg(format!("-femit-bin={}", tmp_obj.display()));

        runner.run_command(&mut cmd)?;
        Ok(())
    }
}
