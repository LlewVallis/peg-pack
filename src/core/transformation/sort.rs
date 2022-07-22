use std::collections::HashMap;

use crate::core::{Instruction, Parser};
use crate::store::{Store, StoreKey};

impl Parser {
    /// Sort the instructions in the map by a depth first search. This is not actually necessary,
    /// but makes the visualizations nicer
    pub(super) fn sort(&mut self) {
        self.sort_instructions();
        self.sort_series();
        self.sort_labels();
        self.sort_expecteds();
    }

    fn sort_instructions(&mut self) {
        let mut new_instructions = Store::new();
        let mut new_debug_symbols = HashMap::new();
        let mut instruction_mappings = HashMap::new();

        let walk = self.walk().collect::<Vec<_>>();
        for (id, instruction) in walk {
            let new_id = new_instructions.insert(instruction);
            instruction_mappings.insert(id, new_id);

            let name = self.debug_symbols[&id].clone();
            new_debug_symbols.insert(new_id, name);
        }

        for (_, instruction) in new_instructions.iter_mut() {
            *instruction = instruction.remapped(|id| instruction_mappings[&id]);
        }

        self.start = instruction_mappings[&self.start];
        self.instructions = new_instructions;
        self.debug_symbols = new_debug_symbols;
    }

    fn sort_series(&mut self) {
        self.sort_resource(
            |parser| &mut parser.series,
            |instruction| match instruction {
                Instruction::Series(id) => Some(id),
                _ => None,
            },
            |instruction, mappings| {
                if let Instruction::Series(id) = instruction {
                    *id = mappings[&id];
                }
            },
        );
    }

    fn sort_labels(&mut self) {
        self.sort_resource(
            |parser| &mut parser.labels,
            |instruction| match instruction {
                Instruction::Label(_, id) => Some(id),
                _ => None,
            },
            |instruction, mappings| {
                if let Instruction::Label(_, id) = instruction {
                    *id = mappings[&id];
                }
            },
        );
    }

    fn sort_expecteds(&mut self) {
        self.sort_resource(
            |parser| &mut parser.expecteds,
            |instruction| match instruction {
                Instruction::Error(_, id) => Some(id),
                _ => None,
            },
            |instruction, mappings| {
                if let Instruction::Error(_, id) = instruction {
                    *id = mappings[&id];
                }
            },
        );
    }

    fn sort_resource<K: StoreKey, V>(
        &mut self,
        store: impl Fn(&mut Self) -> &mut Store<K, V>,
        extract: impl Fn(Instruction) -> Option<K>,
        fix: impl Fn(&mut Instruction, &HashMap<K, K>),
    ) {
        let mut new_store = Store::new();
        let mut mappings = HashMap::new();

        let walk = self.walk().collect::<Vec<_>>();
        for (_, instruction) in walk {
            if let Some(id) = extract(instruction) {
                if !mappings.contains_key(&id) {
                    let value = store(self).remove(id).unwrap();
                    let new_id = new_store.insert(value);
                    mappings.insert(id, new_id);
                }
            }
        }

        for (_, instruction) in self.instructions.iter_mut() {
            fix(instruction, &mappings);
        }

        *store(self) = new_store;
    }
}
