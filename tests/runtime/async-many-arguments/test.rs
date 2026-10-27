use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

#[path = "../lists/alloc.rs"]
mod alloc;

use crate::exports::my::test::i::BigRecord;

impl crate::exports::my::test::i::Guest for Component {
    async fn many_arguments(
        a: String,
        b: String,
        c: String,
        d: String,
        e: String,
        f: String,
        g: String,
        h: String,
        i: String,
    ) {
        assert_eq!(
            [a, b, c, d, e, f, g, h, i],
            ["a", "b", "c", "d", "e", "f", "g", "h", "i"]
        );
    }

    async fn many_arguments_pending(
        x: FutureReader<()>,
        a: String,
        b: String,
        c: String,
        d: String,
        e: String,
        f: String,
        g: String,
        h: String,
    ) {
        assert_eq!(
            [a, b, c, d, e, f, g, h],
            ["a", "b", "c", "d", "e", "f", "g", "h"]
        );
        x.await
    }

    async fn many_arguments_cases(
        a: Option<u32>,
        b: Result<u32, String>,
        c: String,
        d: String,
        e: String,
        f: String,
        g: String,
        h: String,
    ) {
        assert!(
            matches!((a, &b), (Some(42), Err(e)) if e == "error")
                || matches!((a, &b), (None, Ok(7)))
        );
        assert_eq!([c, d, e, f, g, h], ["c", "d", "e", "f", "g", "h"]);
    }

    async fn one_big_record(r: BigRecord) {
        assert_eq!(r.f0, 0);
        assert_eq!(r.f8, 8);
        assert_eq!(r.f16, 16);
    }

    async fn direct_values(
        a: String,
        b: String,
        c: String,
        d: String,
        e: String,
        f: String,
        g: String,
        h: u32,
    ) {
        assert_eq!(
            [a, b, c, d, e, f, g],
            ["a", "b", "c", "d", "e", "f", "g"]
        );
        assert_eq!(h, 8);
    }

    fn heap_mark() -> u32 {
        alloc::get().try_into().unwrap()
    }

    fn supports_cancel() -> bool {
        true
    }
}
