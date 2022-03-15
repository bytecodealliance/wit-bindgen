wit_bindgen_rust::import!("../resources/resources.wit");

use resources::*;

fn main() {
    {
        assert_eq!(
            receive_an_x(&acquire_an_x("I heart Wasm!")),
            "I heart Wasm!"
        );

        let x = acquire_an_x("I heart interface types!");
        let x_clone = x.clone();
        assert_eq!(receive_an_x(&x), "I heart interface types!");
        drop(x);
        assert!(!all_dropped());
        assert_eq!(receive_an_x(&x_clone), "I heart interface types!");
        drop(x_clone);
        assert!(all_dropped());

        let x = acquire_lots_of_x(&["hello", "world", "!"]);
        let x: Vec<_> = x.iter().collect();
        let x = receive_lots_of_x(&x);

        assert_eq!(x, &["hello", "world", "!"]);

        let _x = acquire_an_x("");
        assert!(!all_dropped());
    }

    assert!(all_dropped());
}
