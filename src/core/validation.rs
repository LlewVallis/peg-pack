use std::collections::{HashMap, HashSet};

use crate::core::character::Character;
use crate::core::{Instruction, Parser};
use crate::core::{InstructionId, ValidationError};

impl Parser {
    /// Finds errors in the grammar
    pub(super) fn validate(&self) -> HashSet<ValidationError> {
        let characters = self.characterize();

        let mut errors = HashSet::new();

        for (id, _) in self.instructions() {
            let mut visited = HashSet::new();
            if self.can_reach(id, id, &mut visited, &characters) {
                errors.insert(ValidationError::LeftRecursion(id));
            }
        }

        errors
    }

    /// Determines if an instruction can be reached from another
    fn can_reach(
        &self,
        base: InstructionId,
        id: InstructionId,
        visited: &mut HashSet<InstructionId>,
        characters: &HashMap<InstructionId, Character>,
    ) -> bool {
        if base == id && !visited.is_empty() {
            return true;
        }

        if !visited.insert(id) {
            return false;
        }

        let result = match self.instructions[id] {
            Instruction::Seq(first, second) => {
                let first_transparent = characters[&first].transparent;
                let first = self.can_reach(base, first, visited, characters);
                let second = first_transparent && self.can_reach(base, second, visited, characters);

                first || second
            }
            Instruction::Choice(first, second) => {
                let first_character = characters[&first];
                let second_executable = first_character.fallible || first_character.error_prone;
                let first = self.can_reach(base, first, visited, characters);
                let second = second_executable && self.can_reach(base, second, visited, characters);
                first || second
            }
            Instruction::FirstChoice(first, second) => {
                let second_executable = characters[&first].fallible;
                let first = self.can_reach(base, first, visited, characters);
                let second = second_executable && self.can_reach(base, second, visited, characters);
                first || second
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Cache(target, _)
            | Instruction::Delegate(target) => self.can_reach(base, target, visited, characters),
            Instruction::Series(_) => false,
        };

        visited.remove(&id);

        result
    }
}
