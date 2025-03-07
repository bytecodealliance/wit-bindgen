//@ args = '--generate-unused-types'

use foo::bar::component::UnusedEnum as _;
use foo::bar::component::UnusedRecord as _;
use foo::bar::component::UnusedVariant as _;

include!(env!("BINDINGS"));

fn main() {
    foo::bar::component::foo();
}
