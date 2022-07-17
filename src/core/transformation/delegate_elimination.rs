use std::collections::HashMap;
use crate::core::{Instruction, InstructionId, Parser};

impl Parser {
    /// Elides all delegates in the graph
    pub(super) fn remove_delegates(&mut self) {
        let mut mappings = HashMap::new();

        for (id, _) in self.instructions() {
            let resolved = self.resolve_delegates(id);

            if id != resolved {
                mappings.insert(id, self.resolve_delegates(id));
            }
        }

        self.remap(|id| Self::follow_mappings(id, &mappings));
        self.trim();
    }

    fn resolve_delegates(&self, id: InstructionId) -> InstructionId {
        match self.instructions[id] {
            Instruction::Delegate(target) => self.resolve_delegates(target),
            _ => id,
        }
    }
}
