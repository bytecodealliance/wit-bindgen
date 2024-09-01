
mod w;

struct MyImpl;

impl w::exports::test::test::i::Guest for MyImpl {
    fn f(a: Vec::<String>,) -> Vec::<String> {
        a
    }
}

w::export!(MyImpl with_types_in w);
