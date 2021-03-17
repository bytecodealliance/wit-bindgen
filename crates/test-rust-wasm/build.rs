fn main() {
    println!("cargo:rerun-if-changed=../../tests/host.witx");
    println!("cargo:rerun-if-changed=../../tests/wasm.witx");
}
