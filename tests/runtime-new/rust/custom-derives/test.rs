//@ args = '-d Hash -d Clone -d std::cmp::PartialEq -d core::cmp::Eq'

include!(env!("BINDINGS"));

use crate::exports::my::inline::blah::{Foo, Guest};
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
}

export!(Component);
