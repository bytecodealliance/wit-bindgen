//@ args = '--generate-all'

include!(env!("BINDINGS"));

struct Test;

export!(Test);

use crate::exports::foo::baz::a::Guest;

impl Guest for Test {
    fn x() {}
}
