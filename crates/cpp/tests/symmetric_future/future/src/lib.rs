mod future_world;

use future_world::test::test::future_source::create;

future_world::export!(MyStruct with_types_in future_world);

struct MyStruct;

impl future_world::exports::test::test::future_source::Guest for MyStruct {
    async fn create() -> async_support::FutureReader<u32> {
        let (mut write, read) = async_support::future_support::new_future();
        async_support::spawn(async move {
            let input = create().await;
            write.assign(input * 2);
        });
        read
    }
}
