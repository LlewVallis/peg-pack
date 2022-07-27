use std::collections::HashMap;

use super::{Match, ParseResult};
use super::refc::Refc;

use super::Grammar;

pub struct Cache<G: Grammar> {
    mappings: HashMap<Key, Entry<G>>,
}

impl<G: Grammar> Cache<G> {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new()
        }
    }

    pub fn get(&self, slot: usize, position: usize) -> Option<ParseResult<G>> {
        let key = Key {
            slot,
            position,
        };

        match self.mappings.get(&key)? {
            Entry::Matched(value) => {
                let value = Match::unboxed(value);
                Some(ParseResult::Matched(value))
            }
            Entry::Unmatched { scan_distance } => {
                Some(ParseResult::Unmatched { scan_distance: *scan_distance })
            }
        }
    }

    pub fn insert(&mut self, slot: usize, position: usize, result: ParseResult<G>) -> ParseResult<G> {
        let key = Key {
            slot,
            position,
        };

        let (insertion, result) = match result {
            ParseResult::Matched(value) => {
                let boxed = Match::boxed(value);
                let result = Match::unboxed(&boxed);
                (Entry::Matched(boxed), ParseResult::Matched(result))
            }
            ParseResult::Unmatched { scan_distance } => {
                let insertion = Entry::Unmatched { scan_distance };
                (insertion, result)
            }
        };

        self.mappings.insert(key, insertion);
        result
    }
}

#[derive(Hash, Eq, PartialEq)]
struct Key {
    slot: usize,
    position: usize,
}

enum Entry<G: Grammar> {
    Matched(Refc<Match<G>>),
    Unmatched { scan_distance: usize },
}