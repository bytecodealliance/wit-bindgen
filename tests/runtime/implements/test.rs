include!(env!("BINDINGS"));

use std::cell::RefCell;
use std::collections::HashMap;

export!(Test);

struct Test;

thread_local! {
    static STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

impl exports::test::implements::store::Guest for Test {
    fn get(key: String) -> Option<String> {
        STORE.with(|s| s.borrow().get(&key).cloned())
    }

    fn set(key: String, value: String) {
        STORE.with(|s| {
            s.borrow_mut().insert(key, value);
        });
    }
}
