include!(env!("BINDINGS"));

use crate::exports::test::resource_aggregates::to_test::*;
use core::ops::{Deref, DerefMut};

pub struct Test {}

export!(Test);

impl Deref for Thing {
    type Target = MyThing;
    fn deref(&self) -> &MyThing {
        self.get()
    }
}

impl DerefMut for Thing {
    fn deref_mut(&mut self) -> &mut MyThing {
        self.get_mut()
    }
}

impl Deref for ThingBorrow<'_> {
    type Target = MyThing;
    fn deref(&self) -> &MyThing {
        self.get()
    }
}

#[derive(Debug)]
pub struct MyThing {
    value: u32,
}

impl Guest for Test {
    type Thing = MyThing;

    fn foo(
        r1: R1,
        r2: R2,
        r3: R3,
        t1: T1,
        t2: T2,
        v1: V1,
        v2: V2,
        l1: L1,
        l2: L2,
        o1: Option<Thing>,
        o2: Option<ThingBorrow<'_>>,
        result1: Result<Thing, ()>,
        result2: Result<ThingBorrow<'_>, ()>,
    ) -> u32 {
        r1.thing.value
            + r2.thing.value
            + r3.thing1.value
            + r3.thing2.value
            + t1.0.value
            + t1.1.thing.value
            + t2.0.value
            + match v1 {
                V1::Thing(v) => v.value,
            }
            + match v2 {
                V2::Thing(v) => v.value,
            }
            + l1.into_iter().fold(0, |a, f| a + f.value)
            + l2.into_iter().fold(0, |a, f| a + f.value)
            + o1.map(|o| o.value).unwrap_or_default()
            + o2.map(|o| o.value).unwrap_or_default()
            + result1.map(|o| o.value).unwrap_or_default()
            + result2.map(|o| o.value).unwrap_or_default()
            + 3
    }
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self { value: v + 1 }
    }
}
