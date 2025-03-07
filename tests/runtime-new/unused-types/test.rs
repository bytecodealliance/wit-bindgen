//@ args = '--generate-unused-types'

include!(env!("BINDINGS"));

use exports::foo::bar::component::Guest;
#[expect(unused_imports)]
use exports::foo::bar::component::UnusedEnum as _;
#[expect(unused_imports)]
use exports::foo::bar::component::UnusedRecord as _;
#[expect(unused_imports)]
use exports::foo::bar::component::UnusedVariant as _;

struct Component;

export!(Component);

impl Guest for Component {
    fn foo() {}
}
