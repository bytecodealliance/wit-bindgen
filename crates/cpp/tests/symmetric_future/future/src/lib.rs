mod future_world;

use future_world::test::test::future_source;
use wit_bindgen::rt::async_support;

future_world::export!(MyStruct with_types_in future_world);

struct MyStruct;

impl future_world::exports::test::test::future_test::Guest for MyStruct {
    fn create() -> async_support::FutureReader<u32> {
        let (write, read) = future_world::wit_future::new(u32::default);
        let input = future_source::create();
        async_support::spawn(async move {
            let input = input.await;
            let _ = write.write(input * 2).await;
        });
        read
    }
}
