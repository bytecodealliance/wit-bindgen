include!(env!("BINDINGS"));

fn main() {
    exports::foo();
    exports::bar();
}
