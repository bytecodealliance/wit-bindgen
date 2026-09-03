//! Guest side of `tests/native_e2e.rs`: a plugin built as a native `cdylib`.
//! The test provides its imports through the import resolver.

wit_bindgen::generate!({
    inline: r#"
        package test:native-e2e;

        interface host {
            log: func(msg: string);
            add: func(a: u32, b: u32) -> u32;

            resource counter {
                constructor(start: u32);
                bump: func() -> u32;
            }

            enum kind { small, large }
            fetch: async func(id: s64) -> result<tuple<kind, s64>, string>;
        }

        world plugin {
            use host.{kind};
            import host;

            export greet: func(name: string, times: u32) -> string;
            export sum: func(xs: list<u32>) -> u32;
            export produce: async func(n: s64) -> result<tuple<stream<u8>, kind, s64>, string>;
        }
    "#,
});

use test::native_e2e::host::{self, Counter};
use wit_bindgen::rt::async_support::StreamReader;

struct Plugin;

impl Guest for Plugin {
    fn greet(name: String, times: u32) -> String {
        host::log(&format!("greet({name}, {times})"));
        let counter = Counter::new(10);
        let mut total = 0;
        for _ in 0..times {
            total = host::add(total, counter.bump());
        }
        format!("hello {name}: {total}")
    }

    fn sum(xs: Vec<u32>) -> u32 {
        xs.iter().sum()
    }

    /// Awaits an async import, then returns a stream it has written to.
    async fn produce(n: i64) -> Result<(StreamReader<u8>, Kind, i64), String> {
        let (kind, count) = host::fetch(n).await?;
        let (mut tx, rx) = wit_stream::new::<u8>();
        let unwritten = tx.write_all((1..=count as u8).collect()).await;
        assert!(unwritten.is_empty());
        drop(tx);
        Ok((rx, kind, count * 2))
    }
}

export!(Plugin);
