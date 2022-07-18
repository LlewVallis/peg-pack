use std::collections::HashMap;

use crate::core::InstructionId;
use crate::core::Parser;

mod deduplication;
mod delegate_elimination;
mod linearization;
mod sort;
mod sync_elimination;
mod trim;

impl Parser {
    /// Transform and optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn transform(&mut self) {
        self.trim();
        self.sort();
        self.remove_delegates();
        self.deduplicate();
        self.linearize();
        self.resolve_syncs();
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
