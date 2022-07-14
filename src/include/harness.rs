#[path = "../parser.rs"]
mod parser;

use std::io::{Read, stdin};
use std::time::Instant;

pub fn main() {
    let mut input = Vec::new();
    stdin().read_to_end(&mut input).expect("could not read input");

    let start = Instant::now();
    let result = parser::parse(&input);
    let elapsed = start.elapsed();

    match result {
        Some(match_length) => println!("Matched {} byte(s) in {:.2?}", match_length, elapsed),
        None => println!("Did not match in {:.2?}", elapsed),
    }
}