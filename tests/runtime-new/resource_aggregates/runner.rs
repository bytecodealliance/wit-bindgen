include!(env!("BINDINGS"));

use crate::test::resource_aggregates::to_test::*;

fn main() {
    assert_eq!(
        foo(
            R1 {
                thing: Thing::new(0)
            },
            &R2 {
                thing: &Thing::new(1)
            },
            R3 {
                thing1: &Thing::new(2),
                thing2: Thing::new(3),
            },
            (
                Thing::new(4),
                R1 {
                    thing: Thing::new(5)
                }
            ),
            &(&Thing::new(6),),
            V1::Thing(Thing::new(7)),
            &V2::Thing(&Thing::new(8)),
            vec![Thing::new(9), Thing::new(10)],
            &[&Thing::new(11), &Thing::new(12)],
            Some(Thing::new(13)),
            Some(&Thing::new(14)),
            Ok(Thing::new(15)),
            Ok(&Thing::new(16))
        ),
        (0..17).map(|i| i + 1).sum::<u32>() + 3,
    );
}
