#[path = "../parser.rs"]
mod parser;

use std::io::{Read, stdin};
use std::time::Instant;

pub fn main() {
    let mut input = Vec::new();
    stdin().read_to_end(&mut input).expect("could not read input");

    let start = Instant::now();
    let result = parser::parse(input.as_slice());

    println!("Parsed in {:.1?}", start.elapsed());
    println!("{:?}", result);
}