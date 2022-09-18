use std::io::{Read, stdin};
use std::time::Instant;

#[path = "../parser.rs"]
mod parser;

use parser::*;

pub fn main() {
    let mut input = Vec::new();
    stdin().read_to_end(&mut input).expect("could not read input");

    let start = Instant::now();
    let result = parse(input.as_slice());

    match result {
        Parse::Matched(result) => {
            let errors = result.unmerged_errors().count();
            println!("Parsed in {:.1?} with {} error(s)", start.elapsed(), errors);
            println!("{:#?}", result);
        }
        Parse::Unmatched => {
            println!("Failed to parse in {:.1?}", start.elapsed());
        }
    }
}