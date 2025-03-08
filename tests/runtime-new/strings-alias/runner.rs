include!(env!("BINDINGS"));

fn main() {
    // Test the argument is `&str`
    cat::foo("hello");

    // Test the return type is `String`
    let t: String = cat::bar();
    assert_eq!(t, "world");
}
