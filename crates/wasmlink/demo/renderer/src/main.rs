use std::io::{self, Read};

wai_bindgen_rust::import!("../markdown/markdown.wai");

fn main() {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();
    print!("{}", markdown::render(&buffer));
}
