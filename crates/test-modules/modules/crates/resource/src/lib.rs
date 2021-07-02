witx_bindgen_rust::export!("../../../tests/resource.witx");

use resource::*;
use witx_bindgen_rust::Handle;

struct Component;

struct X;

impl Resource for Component {
    fn acquire_an_x(&self) -> Handle<X> {
        X {}.into()
    }
    fn receive_an_x(&self, x: Handle<X>) {}
}

fn resource() -> &'static impl Resource {
    static INSTANCE: Component = Component;
    &INSTANCE
}
