include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Test creating a stream with u32 values
        let mut rx = create_stream_with_values(3).await;
        let mut total = 0u32;
        let mut count = 0u32;
        loop {
            let buf = Vec::<u32>::with_capacity(10);
            let (result, values) = rx.read(buf).await;
            match result {
                StreamResult::Complete(n) if n > 0 => {
                    // Only process the first n items that were actually read
                    for v in values.iter().take(n) {
                        total += *v;
                        count += 1;
                    }
                }
                // Complete(0) means end of stream, or Dropped/Cancelled
                _ => break,
            }
        }
        assert_eq!(count, 3);
        assert_eq!(total, 0 + 1 + 2); // 0, 1, 2

        // Test creating a unit stream
        let mut rx = create_unit_stream(5).await;
        let mut count = 0u32;
        loop {
            let buf = Vec::<()>::with_capacity(10);
            let (result, _values) = rx.read(buf).await;
            match result {
                StreamResult::Complete(n) if n > 0 => {
                    count += n as u32;
                }
                // Complete(0) means end of stream, or Dropped/Cancelled
                _ => break,
            }
        }
        assert_eq!(count, 5);
    }
}
