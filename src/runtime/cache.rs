use std::collections::BTreeMap;

use super::refc::Refc;
use super::Grammar;
use super::{Match, ParseResult};

pub struct Cache<G: Grammar> {
    mappings: Box<[BTreeMap<u32, Entry<G>>]>,
}

impl<G: Grammar> Cache<G> {
    pub fn new(grammar: &G) -> Self {
        let mut mappings = Vec::with_capacity(grammar.cache_slots());

        for _ in 0..grammar.cache_slots() {
            mappings.push(BTreeMap::new());
        }

        Self {
            mappings: mappings.into_boxed_slice(),
        }
    }

    pub fn get(&self, slot: u32, position: u32) -> Option<ParseResult<G>> {
        let slot_mappings = unsafe { self.mappings.get_unchecked(slot as usize) };

        match slot_mappings.get(&position)? {
            Entry::Matched(value) => {
                let value = Match::unboxed(value);
                Some(ParseResult::Matched(value))
            }
            Entry::Unmatched {
                scan_distance,
                work,
            } => Some(ParseResult::Unmatched {
                scan_distance: *scan_distance,
                work: *work,
            }),
        }
    }

    pub fn insert(&mut self, slot: u32, position: u32, result: ParseResult<G>) -> ParseResult<G> {
        let (insertion, result) = match result {
            ParseResult::Matched(value) => {
                let boxed = Match::boxed(value);
                let result = Match::unboxed(&boxed);
                (Entry::Matched(boxed), ParseResult::Matched(result))
            }
            ParseResult::Unmatched {
                scan_distance,
                work,
            } => {
                let insertion = Entry::Unmatched {
                    scan_distance,
                    work,
                };
                (insertion, result)
            }
        };

        let slot_mappings = unsafe { self.mappings.get_unchecked_mut(slot as usize) };
        slot_mappings.insert(position, insertion);

        result
    }
}

enum Entry<G: Grammar> {
    Matched(Refc<Match<G>>),
    Unmatched { scan_distance: u32, work: u32 },
}
