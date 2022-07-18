use crate::core::character::Character;
use crate::core::{Class, Instruction, InstructionId, Parser};
use std::collections::{BTreeSet, HashMap, HashSet};

impl Parser {
    pub(super) fn resolve_syncs(&mut self) {
        let mut visited = HashSet::new();
        let mut recoveries = BTreeSet::new();

        let characters = self.characterize();

        self.resolve_syncs_in(self.start, &mut visited, &mut recoveries, &characters);
    }

    fn resolve_syncs_in(
        &mut self,
        id: InstructionId,
        visited: &mut HashSet<InstructionId>,
        recoveries: &mut BTreeSet<InstructionId>,
        characters: &HashMap<InstructionId, Character>,
    ) {
        if !visited.insert(id) {
            return;
        }

        let instruction = self.instructions[id];

        match instruction {
            Instruction::Seq(first, second) => {
                let inserted = recoveries.insert(second);

                self.resolve_syncs_in(first, visited, recoveries, characters);

                if inserted {
                    recoveries.remove(&second);
                }

                self.resolve_syncs_in(second, visited, recoveries, characters);
            }
            Instruction::Sync => {
                self.resolve_sync(id, recoveries, characters);
            }
            _ => {
                for successor in instruction.successors() {
                    self.resolve_syncs_in(successor, visited, recoveries, characters);
                }
            }
        }

        visited.remove(&id);
    }

    fn resolve_sync(
        &mut self,
        id: InstructionId,
        recoveries: &BTreeSet<InstructionId>,
        characters: &HashMap<InstructionId, Character>,
    ) {
        let recoveries = recoveries
            .iter()
            .copied()
            .filter(|recovery| {
                let mut visited = HashSet::new();
                !self.left_reachable(id, *recovery, &mut visited, characters)
            })
            .collect::<Vec<_>>();

        if recoveries.is_empty() {
            let never = self.insert_class(Class::new(true));
            self.instructions[id] = Instruction::Class(never);
        } else {
            let mut disjunction = recoveries[0];

            for recovery in &recoveries[1..] {
                disjunction = self.insert(Instruction::Choice(disjunction, *recovery));
            }

            let not_ahead = self.insert(Instruction::NotAhead(disjunction));
            self.instructions[id] = Instruction::NotAhead(not_ahead);
        }
    }

    fn left_reachable(
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

        let instruction = self.instructions[id];

        let mut first_successor = None;
        let mut second_successor = None;

        match instruction {
            Instruction::Seq(first, second) => {
                first_successor = Some(first);

                if let Some(first_character) = characters.get(&first) {
                    if first_character.transparent {
                        second_successor = Some(second);
                    }
                } else {
                    second_successor = Some(second);
                }
            }
            Instruction::Choice(first, second) => {
                first_successor = Some(first);
                second_successor = Some(second);
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target)
            | Instruction::Commit(target)
            | Instruction::Label(target, _)
            | Instruction::Delegate(target) => {
                first_successor = Some(target);
            }
            Instruction::Class(_) | Instruction::Sync | Instruction::Empty => {}
        }

        let mut result = false;

        let successors = first_successor.into_iter().chain(second_successor);
        for successor in successors {
            if self.left_reachable(base, successor, visited, characters) {
                result = true;
                break;
            }
        }

        visited.remove(&id);
        result
    }
}
