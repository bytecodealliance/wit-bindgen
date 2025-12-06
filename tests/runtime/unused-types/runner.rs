//@ args = '--generate-unused-types'

#[expect(unused_imports)]
use foo::bar::component::UnusedEnum as _;
#[expect(unused_imports)]
use foo::bar::component::UnusedRecord as _;
#[expect(unused_imports)]
use foo::bar::component::UnusedVariant as _;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        foo::bar::component::foo();
    }
}
