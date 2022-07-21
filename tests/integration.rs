extern crate core;

use peg_pack::core::{CompilerSettings, Parser};
use serde::Deserialize;
use serde_json::Value;
use std::fs;

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
    reorder_from_start,
    trim,
    eliminate_delegates,
    fold_never_series,
    merge_series,
    deduplicate_series,
    deduplicate_label,
    deduplicate_components,
    deduplicate_rotated_components,
    deduplicate_component_instructions,
    infer_expected,
);

#[derive(Deserialize)]
struct Input {
    #[serde(default)]
    settings: InputSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputSettings {
    #[serde(default = "return_true")]
    merge_series: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self { merge_series: true }
    }
}

fn return_true() -> bool {
    true
}

fn test(input: &[u8], expected: &[u8]) {
    let settings = serde_json::from_slice::<Input>(input).unwrap().settings;

    let settings = CompilerSettings {
        merge_series: settings.merge_series,
    };

    let parser = Parser::load(input, settings).unwrap();
    let output = parser.dump_json();

    let actual = serde_json::from_str::<Value>(&output).unwrap();
    let expected = serde_json::from_slice::<Value>(expected).unwrap();

    if actual != expected {
        let actual = serde_json::to_string_pretty(&actual)
            .unwrap()
            .replace("\n", "\n    ");
        let expected = serde_json::to_string_pretty(&expected)
            .unwrap()
            .replace("\n", "\n    ");

        panic!(
            "test case failed:\n    actual: {}\n  expected: {}",
            actual, expected
        );
    }
}

fn count_cases() -> usize {
    let count = fs::read_dir("tests/cases").unwrap().count();
    assert_eq!(count % 2, 0);
    count / 2
}
