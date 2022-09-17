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
    concatenate_series,
    deduplicate_series,
    deduplicate_label,
    deduplicate_components,
    deduplicate_rotated_components,
    deduplicate_component_instructions,
    infer_expected,
    cache_insertion_low_cost,
    cache_insertion_high_cost,
    reorder_seqs,
    reorder_seqs_loopback,
    reorder_seqs_blowup,
    reorder_choices,
    reorder_choices_loopback,
    reduce_infallible_not_ahead,
    reduce_never_not_ahead,
    double_not_ahead_elimination,
    double_not_ahead_elimination_irreducible,
    character_replacement_reachable_annotations,
    character_replacement_unreachable_annotations,
    eliminate_redundant_seq,
    eliminate_redundant_choice,
    lower_to_first_choice
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
    #[serde(default = "return_true")]
    character_replacement: bool,
    #[serde(default = "return_true")]
    cache_insertion: bool,
    #[serde(default = "return_true")]
    redundant_junction_elimination: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        serde_json::from_str("{}").unwrap()
    }
}

fn return_true() -> bool {
    true
}

fn test(input: &[u8], expected: &[u8]) {
    let settings = serde_json::from_slice::<Input>(input).unwrap().settings;

    let settings = CompilerSettings {
        merge_series: settings.merge_series,
        character_replacement: settings.character_replacement,
        cache_insertion: settings.cache_insertion,
        redundant_junction_elimination: settings.redundant_junction_elimination,
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
