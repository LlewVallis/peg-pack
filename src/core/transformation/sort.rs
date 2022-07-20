use std::collections::HashMap;

use crate::core::{InstructionId, Parser};

impl Parser {
    /// Sort the instructions in the map by a depth first search. This is not actually necessary,
    /// but makes the visualizations nicer
    pub(super) fn sort(&mut self) {
        let mut mappings = HashMap::new();

        for (id, _) in self.walk() {
            mappings.insert(id, InstructionId(mappings.len()));
        }

        self.relabel(|id| mappings[&id]);
    }
}
