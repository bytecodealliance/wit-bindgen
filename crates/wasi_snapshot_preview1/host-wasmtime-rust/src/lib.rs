wit_bindgen_host_wasmtime_rust::generate!({
    import: "../testwasi.wit",
});

pub use testwasi::add_to_linker;

#[derive(Default)]
pub struct TestWasi;

impl testwasi::Testwasi for TestWasi {
    fn log(&mut self, bytes: Vec<u8>) {
        match std::str::from_utf8(&bytes) {
            Ok(s) => print!("{}", s),
            Err(_) => println!("\nbinary: {:?}", bytes),
        }
    }
}
