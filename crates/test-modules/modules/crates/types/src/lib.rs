witx_bindgen_rust::export!("crates/types/types.witx");

struct Component;

impl types::Types for Component {
    fn a(&self) {}
    fn b(
        &self,
        p0: u8,
        p1: i8,
        p2: u16,
        p3: i16,
        p4: u32,
        p5: i32,
        p6: u64,
        p7: i64,
    ) -> (u8, i8, u16, i16, u32, i32, u64, i64) {
        (p0, p1, p2, p3, p4, p5, p6, p7)
    }
    fn c(&self, p0: f32, p1: f64) -> (f32, f64) {
        (p0, p1)
    }
    fn d(&self, p0: String) -> String {
        p0
    }
    fn e(&self) -> String {
        "hello world!".into()
    }
    fn f(&self) -> (u32, String, u64) {
        (13, "hi".into(), 37)
    }
}

fn types() -> &'static impl types::Types {
    static INSTANCE: Component = Component;
    &INSTANCE
}
