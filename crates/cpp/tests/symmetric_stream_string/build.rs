use std::env;

fn main() {
    let out = env::var_os("OUT_DIR").unwrap();
    println!(
        r"cargo:rustc-link-search={}/../../../deps",
        out.into_string().unwrap()
    );
    let manifest = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    println!(
        r"cargo:rustc-env=BINDINGS={}/src/runner.rs",
        manifest.into_string().unwrap()
    );
}
