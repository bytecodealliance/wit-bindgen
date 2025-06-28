include!(env!("BINDINGS"));

use crate::test::common::to_test::{F1, R1, V1};

use exports::test::common::to_test;

pub struct Test {}

export!(Test);

impl to_test::Guest for Test {
    fn wrap(flag: F1) -> R1 {
        crate::test::common::to_test::wrap(flag)
    }

    fn var_f() -> V1 {
        crate::test::common::to_test::var_f()
    }
}
