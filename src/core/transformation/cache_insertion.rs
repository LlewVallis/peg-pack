use std::collections::HashSet;

use crate::core::{Instruction, InstructionId, Parser};

const CACHE_COMPLEXITY_THRESHOLD: usize = 32;

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
            let complexity = self.complexity(id, &mut visited);

            if complexity
                .map(|value| value < CACHE_COMPLEXITY_THRESHOLD)
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

    fn complexity(&self, id: InstructionId, visited: &mut HashSet<InstructionId>) -> Option<usize> {
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
    ) -> Option<usize> {
        let instruction = self.instructions[id];
        let inherent_complexity = self.inherent_complexity(instruction);

        match instruction {
            Instruction::Seq(first, second) | Instruction::Choice(first, second) => {
                let first = self.complexity(first, visited)?;
                let second = self.complexity(second, visited)?;
                Some(first + second + inherent_complexity)
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Delegate(target) => {
                let target = self.complexity(target, visited)?;
                Some(target + inherent_complexity)
            }
            Instruction::Cache(_, _) | Instruction::Series(_) => Some(inherent_complexity),
        }
    }

    fn inherent_complexity(&self, instruction: Instruction) -> usize {
        match instruction {
            Instruction::Seq(_, _)
            | Instruction::Choice(_, _)
            | Instruction::NotAhead(_)
            | Instruction::Delegate(_) => 4,
            Instruction::Cache(_, _) => 8,
            Instruction::Error(_, _) | Instruction::Label(_, _) => 16,
            Instruction::Series(id) => {
                let series = &self.series[id];
                series.classes().len() + 4
            }
        }
    }
}
