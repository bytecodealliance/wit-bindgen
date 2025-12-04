//@ args = '--std-feature --raw-strings'

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // Test the argument is `&str`
        cat::foo(b"hello");

        // Test the return type is `String`
        let t: Vec<u8> = cat::bar();
        assert_eq!(t, b"world");
    }
}
