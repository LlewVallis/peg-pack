use std::collections::HashMap;
use crate::core::{InstructionId, Parser};

impl Parser {
    /// Sort the instructions in the map by a depth first search. This is not actually necessary,
    /// but makes the visualizations nicer
    pub(super) fn sort(&mut self) {
        let mut mappings = HashMap::new();
        self.sort_visit(self.start, &mut mappings);
        self.relabel(|id| mappings[&id]);
    }

    fn sort_visit(&self, id: InstructionId, mappings: &mut HashMap<InstructionId, InstructionId>) {
        if mappings.contains_key(&id) {
            return;
        }

        mappings.insert(id, InstructionId(mappings.len()));

        let instruction = self.instructions[id];
        for successor in instruction.successors() {
            self.sort_visit(successor, mappings);
        }
    }
}