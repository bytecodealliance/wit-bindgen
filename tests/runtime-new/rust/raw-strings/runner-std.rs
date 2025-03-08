//@ args = '--raw-strings'

include!(env!("BINDINGS"));

fn main() {
    // Test the argument is `&[u8]`
    cat::foo(b"hello");

    // Test the return type is `Vec<u8>`
    let t: Vec<u8> = cat::bar();
    assert_eq!(t, b"world");
}
