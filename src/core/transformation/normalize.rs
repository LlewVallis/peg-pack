use crate::core::series::Series;
use crate::core::{CompilerSettings, Instruction, Parser};
use crate::ordered_set::OrderedSet;

type Pass = fn(&mut Parser, Instruction, &mut State) -> Option<Instruction>;

const PASSES: &[Pass] = &[
    Parser::resolve_delegate,
    Parser::concatenate_series,
    Parser::merge_series,
];

struct State {
    settings: CompilerSettings,
}

impl Parser {
    pub(super) fn normalize(&mut self, settings: CompilerSettings) {
        let mut queue = self.walk().map(|(id, _)| id).collect::<OrderedSet<_>>();
        queue.reverse();

        let mut predecessors = self.compute_predecessors();

        let mut state = State { settings };

        while let Some(id) = queue.pop() {
            let instruction = self.instructions[id];

            if let Some(new_instruction) = self.normalize_instruction(instruction, &mut state) {
                if instruction != new_instruction {
                    for predecessor in &predecessors[&id] {
                        queue.push(*predecessor);
                    }

                    for old_successor in instruction.successors() {
                        predecessors.get_mut(&old_successor).unwrap().remove(&id);
                    }

                    for new_successor in instruction.successors() {
                        predecessors.get_mut(&new_successor).unwrap().insert(id);
                    }

                    queue.push(id);
                    self.instructions[id] = new_instruction;
                }
            }
        }

        self.trim();
    }

    fn normalize_instruction(
        &mut self,
        instruction: Instruction,
        state: &mut State,
    ) -> Option<Instruction> {
        for pass in PASSES {
            if let Some(instruction) = pass(self, instruction, state) {
                return Some(instruction);
            }
        }

        None
    }

    fn resolve_delegate(
        &mut self,
        instruction: Instruction,
        _state: &mut State,
    ) -> Option<Instruction> {
        self.as_delegate(instruction)
    }

    fn concatenate_series(
        &mut self,
        instruction: Instruction,
        state: &mut State,
    ) -> Option<Instruction> {
        if !state.settings.merge_series {
            return None;
        }

        let (first, second) = self.as_seq(instruction)?;
        let first = self.as_series(first)?;
        let second = self.as_series(second)?;

        let new_series = Series::concatenate(first, second);
        let new_series_id = self.series.insert(new_series);
        Some(Instruction::Series(new_series_id))
    }

    fn merge_series(&mut self, instruction: Instruction, state: &mut State) -> Option<Instruction> {
        if !state.settings.merge_series {
            return None;
        }

        let (first, second) = self.as_choice(instruction)?;
        let first = self.as_series(first)?;
        let second = self.as_series(second)?;

        let new_series = Series::merge(first, second)?;
        let new_series_id = self.series.insert(new_series);
        Some(Instruction::Series(new_series_id))
    }

    fn as_seq(&self, instruction: Instruction) -> Option<(Instruction, Instruction)> {
        match instruction {
            Instruction::Seq(first, second) => {
                Some((self.instructions[first], self.instructions[second]))
            }
            _ => None,
        }
    }

    fn as_choice(&self, instruction: Instruction) -> Option<(Instruction, Instruction)> {
        match instruction {
            Instruction::Choice(first, second) => {
                Some((self.instructions[first], self.instructions[second]))
            }
            _ => None,
        }
    }

    fn as_series(&self, instruction: Instruction) -> Option<&Series> {
        match instruction {
            Instruction::Series(id) => Some(&self.series[id]),
            _ => None,
        }
    }

    fn as_delegate(&self, instruction: Instruction) -> Option<Instruction> {
        match instruction {
            Instruction::Delegate(target) => Some(self.instructions[target]),
            _ => None,
        }
    }
}
