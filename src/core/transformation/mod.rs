use std::collections::HashMap;

use crate::core::Parser;
use crate::core::{InstructionId, OptimizerSettings};

mod deduplication;
mod delegate_elimination;
mod merge_series;
mod sort;
mod trim;

impl Parser {
    /// Transform and optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn transform(&mut self, settings: OptimizerSettings) {
        self.trim();
        self.sort();

        if settings.remove_delegates {
            self.remove_delegates();
        }

        if settings.merge_series {
            self.merge_series();
        }

        if settings.deduplicate {
            self.deduplicate();
        }

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
