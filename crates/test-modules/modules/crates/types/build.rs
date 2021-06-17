fn main() {
    println!("cargo:rerun-if-changed=crates/types/types.witx");
}
