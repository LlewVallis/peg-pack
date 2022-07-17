use std::collections::HashMap;

use crate::core::Parser;
use crate::core::InstructionId;

mod deduplication;
mod trim;
mod sort;
mod delegate_elimination;

impl Parser {
    /// Transform and optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn transform(&mut self) {
        self.trim();
        self.sort();
        self.remove_delegates();
        self.deduplicate();
        self.sort();
    }

    /// Look up the mapped ID of an instruction, potentially following multiple
    /// mappings
    fn follow_mappings(
        mut id: InstructionId,
        mappings: &HashMap<InstructionId, InstructionId>,
    ) -> InstructionId {
        while let Some(new_id) = mappings.get(&id) {
            id = *new_id;
        }

        id
    }
}
