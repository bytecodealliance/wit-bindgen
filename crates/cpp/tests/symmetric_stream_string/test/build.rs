use std::env;

fn main() {
    let manifest = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    println!(
        r"cargo:rustc-env=BINDINGS={}/src/test.rs",
        manifest.into_string().unwrap()
    );
}
