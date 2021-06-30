use std::io::{self, Read};

witx_bindgen_rust::import!("../markdown/markdown.witx");

fn main() {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();
    print!("{}", markdown::render(&buffer));
}
