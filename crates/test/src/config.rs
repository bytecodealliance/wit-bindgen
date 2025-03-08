//! Configuration support for tests.
//!
//! This module contains the various structures and type definitions which are
//! used to configure both runtime tests and codegen tests.
//!
//! Test configuration happens by parsing TOML-in-comments at the start of
//! source files. Configuration is delimited by being at the top of a source
//! file and prefixed with a language's line-comment syntax followed by `@`. For
//! example in Rust that would look like:
//!
//! ```text
//! //@ some-key = 'some-value'
//!
//! include!(...);
//!
//! // ... rest of the test here
//! ```
//!
//! Here `some-key = 'some-value'` is the TOML to parse into configuration.
//! There are two kinds of configuration here defined in this file:
//!
//! * `RuntimeTestConfig` - this is for runtime tests or `test.rs` and
//!   `runner.rs` for example. This configures per-language and per-compilation
//!   options.
//!
//! * `WitConfig` - this is per-`*.wit` file either as a codegen test or a
//!   `test.wit` input for runtime tests.

use anyhow::Context;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;

/// Configuration that can be placed at the top of runtime tests in source
/// language files. This is currently language-agnostic.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RuntimeTestConfig {
    /// Extra command line arguments to pass to the language-specific bindings
    /// generator.
    ///
    /// This is either a string which is whitespace delimited or it's an array
    /// of strings. By default no extra arguments are passed.
    #[serde(default)]
    pub args: StringList,
    //
    // Maybe add something like this eventually if necessary? For example plumb
    // arbitrary configuration from tests to the "compile" backend. This would
    // then thread through as `Compile` and could be used to pass compiler flags
    // for example.
    //
    // lang: HashMap<String, String>,

    // ...
    //
    // or alternatively could also have something dedicated like:
    // compile_flags: StringList,
    //
    // unclear! This should be expanded on over time as necessary.
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum StringList {
    String(String),
    List(Vec<String>),
}

impl From<StringList> for Vec<String> {
    fn from(list: StringList) -> Vec<String> {
        match list {
            StringList::String(s) => s.split_whitespace().map(|s| s.to_string()).collect(),
            StringList::List(s) => s,
        }
    }
}

impl Default for StringList {
    fn default() -> StringList {
        StringList::List(Vec::new())
    }
}

/// Configuration found in `*.wit` file either in codegen tests or in `test.wit`
/// files for runtime tests.
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WitConfig {
    /// Indicates that this WIT test uses the component model async features
    /// and/or proposal.
    ///
    /// This can be used to help expect failure in languages that do not yet
    /// support this proposal.
    #[serde(default, rename = "async")]
    pub async_: bool,

    /// When set to `true` disables the passing of per-language default bindgen
    /// arguments. For example with Rust it avoids passing `--generate-all` by
    /// default to bindings generation.
    pub default_bindgen_args: Option<bool>,
}

/// Parses the configuration `T` from `contents` in comments at the start of the
/// file where comments are lines prefixed by `comment`.
pub fn parse_test_config<T>(contents: &str, comment: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let config_lines: Vec<_> = contents
        .lines()
        .take_while(|l| l.starts_with(comment))
        .map(|l| &l[comment.len()..])
        .collect();
    let config_text = config_lines.join("\n");

    toml::from_str(&config_text).context("failed to parse the test configuration")
}
