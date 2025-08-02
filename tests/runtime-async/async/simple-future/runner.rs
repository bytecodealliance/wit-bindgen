include!(env!("BINDINGS"));

use crate::my::test::i::*;

fn main() {
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new(|| unreachable!());
        let (res, ()) = futures::join!(tx.write(()), read_future(rx));
        assert!(res.is_ok());

        let (tx, rx) = wit_future::new(|| unreachable!());
        let (res, ()) = futures::join!(tx.write(()), drop_future(rx));
        assert!(res.is_err());
    });
}
