witx_bindgen_rust::export!("../../../tests/resource.witx");

use resource::*;
use std::sync::{Arc, Mutex};
use witx_bindgen_rust::Handle;

struct Component(Arc<Mutex<u32>>);

struct X(String, Arc<Mutex<u32>>);

impl Drop for X {
    fn drop(&mut self) {
        *self.1.lock().unwrap() -= 1;
    }
}

impl Resource for Component {
    fn acquire_an_x(&self, s: String) -> Handle<X> {
        *self.0.lock().unwrap() += 1;
        X(s, self.0.clone()).into()
    }

    fn acquire_lots_of_x(&self, s: Vec<String>) -> Vec<Handle<X>> {
        *self.0.lock().unwrap() += s.len() as u32;

        s.into_iter().map(|s| X(s, self.0.clone()).into()).collect()
    }

    fn receive_an_x(&self, x: Handle<X>) -> String {
        x.0.clone()
    }

    fn receive_lots_of_x(&self, vals: Vec<Handle<X>>) -> Vec<String> {
        vals.into_iter().map(|x| x.0.clone()).collect()
    }

    fn all_dropped(&self) -> bool {
        *self.0.lock().unwrap() == 0
    }
}

lazy_static::lazy_static! {
    static ref INSTANCE: Component = Component(Arc::new(Mutex::new(0)));
}

fn resource() -> &'static impl Resource {
    &INSTANCE as &Component
}
