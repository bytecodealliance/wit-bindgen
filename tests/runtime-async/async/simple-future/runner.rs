include!(env!("BINDINGS"));

use crate::my::test::i::*;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (tx, rx) = wit_future::new(|| unreachable!());
        let (res, ()) = futures::join!(tx.write(()), read_future(rx));
        assert!(res.is_ok());

        let (tx, rx) = wit_future::new(|| unreachable!());
        let (res, ()) = futures::join!(tx.write(()), drop_future(rx));
        assert!(res.is_err());
    }
}
