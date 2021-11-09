wai_bindgen_rust::export!("crates/resources/resources.wai");

use std::sync::{Arc, Mutex};
use wai_bindgen_rust::Handle;

lazy_static::lazy_static! {
    static ref INSTANCE: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

pub struct X(String, Arc<Mutex<u32>>);

impl Drop for X {
    fn drop(&mut self) {
        *self.1.lock().unwrap() -= 1;
    }
}

struct Resources;

impl resources::Resources for Resources {
    fn acquire_an_x(s: String) -> Handle<X> {
        // Increment by two: decremented in `drop_x` and in the `Drop` impl
        *INSTANCE.lock().unwrap() += 2;
        X(s, INSTANCE.clone()).into()
    }

    fn acquire_lots_of_x(s: Vec<String>) -> Vec<Handle<X>> {
        // Increment by a factor of 2: decremented for each call to `drop_x` and the `Drop` impl
        *INSTANCE.lock().unwrap() += (s.len() as u32) * 2;

        s.into_iter()
            .map(|s| X(s, INSTANCE.clone()).into())
            .collect()
    }

    fn receive_an_x(x: Handle<X>) -> String {
        x.0.clone()
    }

    fn receive_lots_of_x(vals: Vec<Handle<X>>) -> Vec<String> {
        vals.into_iter().map(|x| x.0.clone()).collect()
    }

    fn all_dropped() -> bool {
        *INSTANCE.lock().unwrap() == 0
    }

    fn drop_x(_x: X) {
        *INSTANCE.lock().unwrap() -= 1;
    }
}
