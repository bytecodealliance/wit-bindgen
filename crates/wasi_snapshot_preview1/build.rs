fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=--import-memory");
    println!("cargo:rustc-link-arg=-zstack-size=0");
}
