extern crate core;

use std::fs;
use serde_json::Value;
use peg_pack::core::Parser;

macro_rules! case {
    ($name:ident) => {
        #[test]
        pub fn $name() {
            test(
                include_bytes!(concat!("cases/", stringify!($name), ".input.json")),
                include_bytes!(concat!("cases/", stringify!($name), ".expected.json")),
            );
        }
    };
}

macro_rules! cases {
    ($($name:ident),* $(,)?) => {
        $(case!($name);)*

        #[test]
        fn all_cases_accounted_for() {
            let expected = 0 $(+ (1, stringify!($name)).0)*;
            let actual = count_cases();
            assert_eq!(expected, actual);
        }
    };
}

cases!(
    empty,
    reorder_from_start,
    eliminate_delegates,
    deduplicate_class,
    deduplicate_label,
    deduplicate_components,
    deduplicate_rotated_components,
    deduplicate_component_instructions,
);

fn test(input: &[u8], expected: &[u8]) {
    let parser = Parser::load(input).unwrap();
    let output = parser.dump_json();

    let actual = serde_json::from_str::<Value>(&output).unwrap();
    let expected = serde_json::from_slice::<Value>(expected).unwrap();

    if actual != expected {
        let actual = serde_json::to_string_pretty(&actual).unwrap().replace("\n", "\n    ");
        let expected = serde_json::to_string_pretty(&expected).unwrap().replace("\n", "\n    ");

        panic!("test case failed:\n    actual: {}\n  expected: {}", actual, expected);
    }
}

fn count_cases() -> usize {
    let count = fs::read_dir("tests/cases").unwrap().count();
    assert_eq!(count % 2, 0);
    count / 2
}