use std::io::{self, Read};

wit_bindgen_rust::import!("../markdown/markdown.wit");

fn main() {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();
    print!("{}", markdown::render(&buffer));
}
