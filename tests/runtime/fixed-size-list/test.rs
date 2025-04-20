include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl exports::test::fixed_size_lists::to_test::Guest for Component {
    fn list_param(_a: [u32; 4]) {}
    fn list_param2(_a: [[u32; 2]; 2]) {}
    // fn list_param3(_a: [i32; 20]) {}
}
