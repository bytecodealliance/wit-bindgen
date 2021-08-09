//! The WebAssembly module linker CLI.

#![deny(missing_docs)]

use anyhow::{bail, Context, Result};
use std::{collections::HashMap, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};
use wasmlink::{Linker, Module, Profile};

fn parse_module(s: &str) -> Result<(String, PathBuf)> {
    match s.split_once('=') {
        Some((name, path)) => Ok((name.into(), path.into())),
        None => bail!("expected a value with format `NAME=MODULE`"),
    }
}

fn parse_interface(s: &str) -> Result<(String, PathBuf)> {
    match s.split_once('=') {
        Some((name, path)) => Ok((name.into(), path.into())),
        None => bail!("expected a value with format `NAME=INTERFACE`"),
    }
}

/// WebAssembly module linker.
#[derive(Debug, StructOpt)]
#[structopt(name = "wasmlink", version = env!("CARGO_PKG_VERSION"), global_settings = &[
    AppSettings::VersionlessSubcommands,
    AppSettings::ColoredHelp,
    AppSettings::ArgRequiredElseHelp,
])]
pub struct App {
    /// A transitive imported module to the module being linked.
    #[structopt(long = "module", short = "m", value_name = "NAME=MODULE", parse(try_from_str = parse_module), required = true, min_values = 1)]
    pub modules: Vec<(String, PathBuf)>,

    /// The path to an interface definition file for an imported module.
    #[structopt(long = "interface", short = "i", value_name = "NAME=INTERFACE", parse(try_from_str = parse_interface))]
    pub interfaces: Vec<(String, PathBuf)>,

    /// The name of the target profile to use for the link.
    #[structopt(long, short = "p", value_name = "PROFILE")]
    pub profile: String,

    /// The path of the output linked module; defaults to replacing the main module.
    #[structopt(long, short = "o", value_name = "OUTPUT", parse(from_os_str))]
    pub output: Option<PathBuf>,

    /// The main module to link.
    #[structopt(index = 1, value_name = "MODULE", parse(from_os_str))]
    pub main: PathBuf,
}

impl App {
    /// Executes the application.
    pub fn execute(self) -> Result<()> {
        if self.modules.is_empty() {
            bail!("at least one import module must be specified");
        }

        let main_bytes = wat::parse_file(&self.main)
            .with_context(|| format!("failed to parse main module `{}`", self.main.display()))?;

        let main_module = Module::new(
            self.main.file_name().unwrap().to_str().unwrap(),
            &main_bytes,
            [],
        )
        .with_context(|| format!("failed to parse main module `{}`", self.main.display()))?;

        let import_bytes = self
            .modules
            .into_iter()
            .map(|(name, path)| {
                if !path.is_file() {
                    bail!(
                        "import module `{}` does not exist as a file",
                        path.display()
                    );
                }

                let bytes = wat::parse_file(&path).with_context(|| {
                    format!("failed to parse import module `{}`", path.display())
                })?;

                Ok((name, bytes))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let mut import_interfaces = self
            .interfaces
            .into_iter()
            .map(|(name, path)| {
                if !path.is_file() {
                    bail!("interface file `{}` does not exist", path.display());
                }

                Ok((
                    name,
                    witx2::Interface::parse_file(&path).with_context(|| {
                        format!("failed to parse interface file `{}`", path.display())
                    })?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let import_modules: HashMap<&str, Module> = import_bytes
            .iter()
            .map(|(name, bytes)| {
                let name = name.as_ref();
                Ok((
                    name,
                    Module::new(name, bytes, import_interfaces.remove(name))?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        // TODO: do something with the profile option

        let linker = Linker::new(Profile::new());

        let output = self.output.as_ref().unwrap_or(&self.main);
        std::fs::write(output, linker.link(&main_module, &import_modules)?)
            .with_context(|| format!("failed to write to output module `{}`", output.display()))?;

        Ok(())
    }
}
