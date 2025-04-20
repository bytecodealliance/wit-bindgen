include!(env!("BINDINGS"));

use test::fixed_size_lists::to_test::*;

fn main() {
    list_param([1,2,3,4]);
    list_param2([[1,2],[3,4]]);
}
