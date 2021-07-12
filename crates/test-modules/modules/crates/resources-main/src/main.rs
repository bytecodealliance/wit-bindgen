witx_bindgen_rust::import!("crates/resources/resources.witx");

use resources::*;

fn main() {
    {
        assert_eq!(
            receive_an_x(&acquire_an_x("I heart Wasm!")),
            "I heart Wasm!"
        );
        assert_eq!(
            receive_an_x(&acquire_an_x("I heart interface types!")),
            "I heart interface types!"
        );

        let x = acquire_lots_of_x(&["hello", "world", "!"]);
        let x: Vec<_> = x.iter().collect();
        let x = receive_lots_of_x(&x);

        assert_eq!(x, &["hello", "world", "!"]);

        let _x = acquire_an_x("");
        assert!(!all_dropped());
    }

    assert!(all_dropped());
}
