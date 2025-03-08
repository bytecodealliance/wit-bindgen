use anyhow::{Context, Result};
use std::ffi::OsString;
use std::process::Command;

/// Helper structure representing a test runner, which is a program argument
/// followed by a number of optional arguments.
pub struct TestRunner {
    path: OsString,
    args: Vec<String>,
}

impl TestRunner {
    pub fn new(runner: &OsString) -> Result<TestRunner> {
        // First see if `runner` itself is a valid executable, if so use it as-is.
        let original_err = match Command::new(runner).arg("--version").output() {
            Ok(_) => {
                return Ok(TestRunner {
                    path: runner.clone(),
                    args: Vec::new(),
                })
            }
            Err(e) => e,
        };

        // Failing that see if `runner` looks like `foo --bar --baz` where
        // space-delimited arguments are used.
        let runner_and_args = runner.to_str().context("--runner argument is not utf-8")?;
        let mut delimited = runner_and_args.split_whitespace();
        let command = delimited.next().unwrap();
        if Command::new(command).arg("--version").output().is_ok() {
            return Ok(TestRunner {
                path: command.into(),
                args: delimited.map(|s| s.to_string()).collect(),
            });
        }

        // Failing that return an error. It's left as a future extension to
        // support arguemnts-with-spaces or runtimes-with-spaces.
        Err(original_err).context(format!("runner `{runner_and_args}` failed to spawn"))
    }

    /// Returns a `Command` which can be used to execute this test runner.
    pub fn command(&self) -> Command {
        let mut ret = Command::new(&self.path);
        for arg in self.args.iter() {
            ret.arg(arg);
        }
        ret
    }
}
