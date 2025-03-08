//@ args = '--generate-unused-types'

#[expect(unused_imports)]
use foo::bar::component::UnusedEnum as _;
#[expect(unused_imports)]
use foo::bar::component::UnusedRecord as _;
#[expect(unused_imports)]
use foo::bar::component::UnusedVariant as _;

include!(env!("BINDINGS"));

fn main() {
    foo::bar::component::foo();
}
