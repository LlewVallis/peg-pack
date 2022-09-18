use std::collections::HashSet;

use crate::core::{Instruction, InstructionId, Parser};
use crate::runtime::{
    CACHE_WORK, CHOICE_WORK, LABEL_WORK, MARK_ERROR_WORK, MAX_UNCACHED_WORK, NOT_AHEAD_WORK,
    SEQ_WORK, SERIES_WORK,
};

impl Parser {
    pub(super) fn insert_cache_points(&mut self) {
        let predecessors = self.compute_duplicated_predecessors();

        let mut instructions = self.walk().map(|(k, _)| k).collect::<Vec<_>>();

        instructions.reverse();

        for id in instructions {
            if let Instruction::Cache(_, _) = self.instructions[id] {
                continue;
            }

            if predecessors[&id].len() < 2 {
                continue;
            }

            let mut visited = HashSet::new();
            let work = self.work(id, &mut visited);

            if work
                .map(|value| value <= MAX_UNCACHED_WORK)
                .unwrap_or(false)
            {
                continue;
            }

            let symbol = self.debug_symbols[&id].clone();
            let new_id = self.insert(Instruction::Cache(id, None), symbol);

            for pred_id in &predecessors[&id] {
                let pred = self.instructions[*pred_id];

                self.instructions[*pred_id] =
                    pred.remapped(|old_id| if old_id == id { new_id } else { old_id });
            }
        }
    }

    fn work(&self, id: InstructionId, visited: &mut HashSet<InstructionId>) -> Option<u32> {
        if !visited.insert(id) {
            return None;
        }

        let result = self.complexity_unvisited(id, visited);

        visited.remove(&id);
        result
    }

    fn complexity_unvisited(
        &self,
        id: InstructionId,
        visited: &mut HashSet<InstructionId>,
    ) -> Option<u32> {
        let instruction = self.instructions[id];
        let inherent_complexity = self.inherent_complexity(instruction);

        match instruction {
            Instruction::Seq(first, second)
            | Instruction::Choice(first, second)
            | Instruction::FirstChoice(first, second) => {
                let first = self.work(first, visited)?;
                let second = self.work(second, visited)?;
                Some(first + second + inherent_complexity)
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Delegate(target) => {
                let target = self.work(target, visited)?;
                Some(target + inherent_complexity)
            }
            Instruction::Cache(_, _) | Instruction::Series(_) => Some(inherent_complexity),
        }
    }

    fn inherent_complexity(&self, instruction: Instruction) -> u32 {
        match instruction {
            Instruction::Seq(_, _) => SEQ_WORK,
            Instruction::Choice(_, _) | Instruction::FirstChoice(_, _) => CHOICE_WORK,
            Instruction::NotAhead(_) => NOT_AHEAD_WORK,
            Instruction::Delegate(_) => 0,
            Instruction::Cache(_, _) => CACHE_WORK,
            Instruction::Error(_, _) => MARK_ERROR_WORK,
            Instruction::Label(_, _) => LABEL_WORK,
            Instruction::Series(_) => SERIES_WORK,
        }
    }
}
