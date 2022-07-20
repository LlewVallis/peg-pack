use crate::core::series::Series;
use crate::core::{Instruction, InstructionId, Parser};
use std::collections::HashMap;

impl Parser {
    pub(super) fn merge_series(&mut self) {
        let mut mappings = HashMap::new();

        let walk = self.walk().collect::<Vec<_>>();
        for (id, instruction) in walk.into_iter().rev() {
            self.visit(id, instruction, &mut mappings);
        }

        self.remap(|id| Self::follow_mappings(id, &mappings));
        self.trim();
    }

    fn visit(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
        mappings: &mut HashMap<InstructionId, InstructionId>,
    ) {
        let (first_id, second_id) = match instruction {
            Instruction::Seq(first, second) => (first, second),
            _ => return,
        };

        let first_id = Self::follow_mappings(first_id, mappings);
        let second_id = Self::follow_mappings(second_id, mappings);

        let first = match self.instructions[first_id] {
            Instruction::Series(id) => &self.series[id],
            _ => return,
        };

        let second = match self.instructions[second_id] {
            Instruction::Series(id) => &self.series[id],
            _ => return,
        };

        let merged = Series::merge(first, second);

        let series = self.insert_series(merged);
        let new_id = self.insert(Instruction::Series(series));

        mappings.insert(id, new_id);
    }
}
