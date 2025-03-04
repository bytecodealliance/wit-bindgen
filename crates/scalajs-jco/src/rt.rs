use crate::context::ScalaJsFile;
use crate::Opts;
use std::fmt::Write;
use wit_bindgen_core::uwriteln;

pub fn render_runtime_module(opts: &Opts) -> ScalaJsFile {
    let wit_scala = include_str!("../scala/wit.scala");

    let mut package = opts.base_package_segments();
    package.push("wit".to_string());

    let mut source = String::new();
    uwriteln!(source, "package {}", opts.base_package_segments().join("."));
    uwriteln!(source, "");
    uwriteln!(source, "{wit_scala}");

    ScalaJsFile {
        package,
        name: "package".to_string(),
        source,
    }
}
