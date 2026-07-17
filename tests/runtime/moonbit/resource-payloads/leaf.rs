include!(env!("BINDINGS"));

use crate::exports::my::test::leaf_interface::{
    Body, Fields, Guest, GuestBody, GuestFields, GuestLeafThing, GuestResponse,
};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use wit_bindgen::{FutureReader, StreamReader};

struct Component;

export!(Component);

static LEAF_DROP_COUNT: AtomicU32 = AtomicU32::new(0);
static LEAF_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);
static FIELDS_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);
static BODY_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);
static RESPONSE_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);

struct MyLeafThing {
    value: String,
}

struct MyFields {
    value: String,
}

struct MyBody {
    contents: RefCell<Option<StreamReader<u8>>>,
    trailers: RefCell<Option<FutureReader<Fields>>>,
}

struct MyResponse {
    body: RefCell<Option<Body>>,
}

impl Guest for Component {
    type LeafThing = MyLeafThing;
    type Fields = MyFields;
    type Body = MyBody;
    type Response = MyResponse;

    fn leaf_drop_count() -> u32 {
        LEAF_DROP_COUNT.load(Ordering::SeqCst)
    }

    fn leaf_live_count() -> u32 {
        LEAF_LIVE_COUNT.load(Ordering::SeqCst)
    }

    fn fields_live_count() -> u32 {
        FIELDS_LIVE_COUNT.load(Ordering::SeqCst)
    }

    fn body_live_count() -> u32 {
        BODY_LIVE_COUNT.load(Ordering::SeqCst)
    }

    fn response_live_count() -> u32 {
        RESPONSE_LIVE_COUNT.load(Ordering::SeqCst)
    }
}

impl GuestLeafThing for MyLeafThing {
    fn new(s: String) -> Self {
        LEAF_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self { value: s }
    }

    fn get(&self) -> String {
        self.value.clone()
    }
}

impl Drop for MyLeafThing {
    fn drop(&mut self) {
        LEAF_DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        LEAF_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl GuestFields for MyFields {
    fn new(s: String) -> Self {
        FIELDS_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self { value: s }
    }

    fn get(&self) -> String {
        self.value.clone()
    }
}

impl Drop for MyFields {
    fn drop(&mut self) {
        FIELDS_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl GuestBody for MyBody {
    fn new(contents: StreamReader<u8>, trailers: Option<FutureReader<Fields>>) -> Self {
        BODY_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            contents: RefCell::new(Some(contents)),
            trailers: RefCell::new(trailers),
        }
    }

    async fn collect(&self) -> (String, Option<String>) {
        let contents = self.contents.borrow_mut().take().unwrap();
        let bytes = contents.collect().await;
        let body = String::from_utf8(bytes).unwrap();
        let trailers = match self.trailers.borrow_mut().take() {
            Some(trailers) => Some(trailers.await.get::<MyFields>().get()),
            None => None,
        };
        (body, trailers)
    }
}

impl Drop for MyBody {
    fn drop(&mut self) {
        BODY_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl GuestResponse for MyResponse {
    fn new(body: Body) -> Self {
        RESPONSE_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self {
            body: RefCell::new(Some(body)),
        }
    }

    async fn collect(&self) -> (String, Option<String>) {
        let body = self.body.borrow_mut().take().unwrap();
        body.get::<MyBody>().collect().await
    }
}

impl Drop for MyResponse {
    fn drop(&mut self) {
        RESPONSE_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}
