include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn unit_ok() -> Result<(), ()> {
        Ok(())
    }

    async fn unit_err() -> Result<(), String> {
        Err("unit".to_string())
    }

    async fn halve(x: u32) -> Result<u32, String> {
        if x % 2 != 0 {
            return Err("odd".to_string());
        }
        Ok(x / 2)
    }
}
