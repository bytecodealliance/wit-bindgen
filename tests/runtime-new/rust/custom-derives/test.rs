//@ args = [
//@   '-dHash',
//@   '-dClone',
//@   '-dstd::cmp::PartialEq',
//@   '-dcore::cmp::Eq',
//@   '--additional-derive-ignore=ignoreme',
//@ ]

include!(env!("BINDINGS"));

use crate::exports::my::inline::blag;
use crate::exports::my::inline::blah::{Foo, Guest, Ignoreme};
use std::collections::{hash_map::RandomState, HashSet};

struct Component;

impl Guest for Component {
    fn bar(cool: Foo) {
        let _blah: HashSet<Foo, RandomState> = HashSet::from_iter([
            Foo {
                field1: "hello".to_string(),
                field2: vec![1, 2, 3],
            },
            cool,
        ]);
    }

    fn barry(_: Ignoreme) {}
}

struct MyInputStream;

impl blag::Guest for Component {
    type InputStream = MyInputStream;
}

impl blag::GuestInputStream for MyInputStream {
    fn read(&self, _len: u64) -> Vec<u8> {
        todo!()
    }
}

export!(Component);
