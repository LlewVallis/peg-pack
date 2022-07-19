use crate::core::{InstructionId, Parser};
use std::collections::{HashMap, HashSet};

impl Parser {
    /// Duplicates branches in the instruction graph such that there is a single non-cyclic path
    /// from the start node to each instruction. Furthermore, if an instruction `A` references
    /// instruction `B`, then either the cycle free path to `B` goes through `A` or the cycle free
    /// path to `A` goes through `B`
    pub(super) fn linearize(&mut self) {
        let mut mappings = HashMap::new();
        let mut ancestors = HashSet::new();
        let mut touched = HashSet::new();

        let new_start =
            self.linearize_instruction(self.start, &mut mappings, &mut ancestors, &mut touched);

        assert_eq!(self.start, new_start);
    }

    fn linearize_instruction(
        &mut self,
        id: InstructionId,
        mappings: &mut HashMap<InstructionId, InstructionId>,
        ancestors: &mut HashSet<InstructionId>,
        touched: &mut HashSet<InstructionId>,
    ) -> InstructionId {
        if !ancestors.insert(id) {
            return Self::follow_mappings(id, mappings);
        }

        let new_id = if touched.contains(&id) {
            let new_id = self.insert(self.instructions[id]);
            assert!(mappings.insert(id, new_id).is_none());
            assert!(ancestors.insert(new_id));
            new_id
        } else {
            id
        };

        assert!(touched.insert(new_id));

        let instruction = self.instructions[new_id];
        let new_instruction =
            instruction.remapped(|id| self.linearize_instruction(id, mappings, ancestors, touched));

        self.instructions[new_id] = new_instruction;

        if new_id != id {
            assert!(mappings.remove(&id).is_some());
            assert!(ancestors.remove(&new_id));
        }

        assert!(ancestors.remove(&id));

        new_id
    }
}
