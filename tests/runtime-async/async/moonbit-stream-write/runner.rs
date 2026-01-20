include!(env!("BINDINGS"));

use crate::my::test::i::*;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Test creating a stream with u32 values
        let rx = create_stream_with_values(3).await;
        let mut total = 0u32;
        let mut count = 0u32;
        loop {
            match rx.read(10).await {
                Some(values) => {
                    for v in values {
                        total += v;
                        count += 1;
                    }
                }
                None => break,
            }
        }
        assert_eq!(count, 3);
        assert_eq!(total, 0 + 1 + 2); // 0, 1, 2

        // Test creating a unit stream
        let rx = create_unit_stream(5).await;
        let mut count = 0u32;
        loop {
            match rx.read(10).await {
                Some(values) => {
                    count += values.len() as u32;
                }
                None => break,
            }
        }
        assert_eq!(count, 5);
    }
}
