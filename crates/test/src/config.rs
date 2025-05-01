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
use std::collections::HashMap;

/// Configuration that can be placed at the top of runtime tests in source
/// language files.
///
/// This is a union of language-agnostic and language-specific configuration.
/// Language-agnostic configuration can be bindings generator arguments:
///
/// ```toml
/// args = '--foo --bar'
/// #  or ...
/// args = ['--foo', '--bar']
/// ```
///
/// but languages may each have their own configuration:
///
/// ```toml
/// [lang]
/// rustflags = '-O'
/// ```
///
/// The `Component::deserialize_lang_config` helper is used to deserialize the
/// `lang` field here.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RuntimeTestConfig<T = HashMap<String, toml::Value>> {
    /// Extra command line arguments to pass to the language-specific bindings
    /// generator.
    ///
    /// This is either a string which is whitespace delimited or it's an array
    /// of strings. By default no extra arguments are passed.
    #[serde(default)]
    pub args: StringList,

    /// Language-specific configuration
    //
    // Note that this is an `Option<T>` where `T` defaults to a catch-all hash
    // map with a bunch of toml values in it. The idea here is that tests are
    // first parsed with the `HashMap` configuration. If that's not present
    // then the language uses its default configuration but if it is present
    // then the fields are re-parsed where `T` is specific-per-language. The
    // `Component::deserialize_lang_config` helper is intended for this.
    pub lang: Option<T>,
}

#[derive(Deserialize, Clone, Debug)]
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

    /// Whether or not this test uses `error-context`
    #[serde(default)]
    pub error_context: bool,

    /// When set to `true` disables the passing of per-language default bindgen
    /// arguments. For example with Rust it avoids passing `--generate-all` by
    /// default to bindings generation.
    pub default_bindgen_args: Option<bool>,

    /// Name of the world for the "runner" component, and note that this affects
    /// filenames as well.
    pub runner: Option<String>,

    /// List of worlds for "test" components. This affects filenames and these
    /// are all available to import to the "runner".
    pub dependencies: Option<StringList>,

    /// Path to a `*.wac` file to specify how composition is done.
    pub wac: Option<String>,
}

impl WitConfig {
    /// Returns the name of the "runner" world
    pub fn runner_world(&self) -> &str {
        self.runner.as_deref().unwrap_or("runner")
    }

    /// Returns the list of dependency worlds that this configuration uses.
    pub fn dependency_worlds(&self) -> Vec<String> {
        match self.dependencies.clone() {
            Some(list) => list.into(),
            None => vec!["test".to_string()],
        }
    }
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
