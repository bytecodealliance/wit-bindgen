include!(env!("BINDINGS"));

use crate::test::many_arguments::to_test::many_arguments;

fn main() {
    many_arguments(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}
