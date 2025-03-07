//! Configuration support for tests.
//!
//! This module contains the various structures and type definitions which are
//! used to configure both runtime tests and codegen tests.

use anyhow::Context;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTestConfig {
    #[serde(default)]
    pub args: StringList,
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

#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WitConfig {
    #[serde(default, rename = "async")]
    pub async_: bool,
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
