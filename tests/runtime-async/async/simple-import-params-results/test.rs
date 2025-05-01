include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::a::b::i::Guest for Component {
    async fn one_argument(x: u32) {
        assert_eq!(x, 1);
    }
    async fn one_result() -> u32 {
        2
    }
    async fn one_argument_and_result(x: u32) -> u32 {
        assert_eq!(x, 3);
        4
    }
    async fn two_arguments(x: u32, y: u32) {
        assert_eq!(x, 5);
        assert_eq!(y, 6);
    }
    async fn two_arguments_and_result(x: u32, y: u32) -> u32 {
        assert_eq!(x, 7);
        assert_eq!(y, 8);
        9
    }
}
