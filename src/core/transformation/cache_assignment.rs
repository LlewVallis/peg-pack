use crate::core::{Instruction, Parser};

impl Parser {
    pub(super) fn assign_cache_ids(&mut self) {
        let mut next_id = 0;

        for (_, instruction) in self.instructions.iter_mut() {
            if let Instruction::Cache(_, id) = instruction {
                *id = Some(next_id);
                next_id += 1;
            }
        }
    }
}
