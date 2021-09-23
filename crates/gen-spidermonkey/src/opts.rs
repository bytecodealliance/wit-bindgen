#![allow(missing_docs)]

use crate::SpiderMonkeyWasm;
use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, structopt::StructOpt)]
pub struct Opts {
    /// The path to the JavaScript module.
    pub js: PathBuf,
}

impl Opts {
    pub fn build<'a>(self, js_source: impl Into<Cow<'a, str>>) -> SpiderMonkeyWasm<'a> {
        SpiderMonkeyWasm::new(self.js, js_source)
    }
}
