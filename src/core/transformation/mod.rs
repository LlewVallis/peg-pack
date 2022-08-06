use std::collections::HashMap;

use crate::core::Parser;
use crate::core::{CompilerSettings, InstructionId};

mod cache_assignment;
mod cache_insertion;
mod debug_symbol_inference;
mod deduplication;
mod expected_inference;
mod normalize;
mod sort;
mod trim;

impl Parser {
    /// Transform and optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn transform(&mut self, settings: CompilerSettings) {
        // Must be first since all ExpectedIds start out invalid
        self.infer_expecteds();

        self.trim();
        self.sort();

        self.normalize(settings);

        self.deduplicate();

        if settings.cache_insertion {
            self.insert_cache_points();
        }

        self.assign_cache_ids();

        self.infer_debug_symbols();
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
