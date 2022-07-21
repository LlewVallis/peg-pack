use std::collections::HashMap;

use crate::core::Parser;
use crate::core::{CompilerSettings, InstructionId};

mod deduplication;
mod delegate_elimination;
mod expected_inference;
mod merge_series;
mod sort;
mod trim;

impl Parser {
    /// Transform and optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn transform(&mut self, settings: CompilerSettings) {
        // Must be first since all ExpectedIds start out invalid
        self.infer_expecteds();

        self.trim();
        self.sort();

        self.remove_delegates();

        if settings.merge_series {
            self.merge_series();
        }

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
