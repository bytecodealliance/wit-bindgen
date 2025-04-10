include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl exports::test::strings::to_test::Guest for Component {
    fn take_basic(s: String) {
        assert_eq!(s, "latin utf16");
    }

    fn return_unicode () -> String {
        "🚀🚀🚀 𠈄𓀀".to_string()
    }

    fn return_empty() -> String{
        "".to_string()
    }

    fn roundtrip(s: String) -> String {
        s.clone()
    }
}