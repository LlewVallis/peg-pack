use crate::core::{Instruction, InstructionId, Parser};

impl Parser {
    pub(super) fn infer_expecteds(&mut self) {
        let characters = self.characterize();

        let instruction_ids = self.instructions().map(|(k, _)| k).collect::<Vec<_>>();

        for id in instruction_ids {
            let new_instruction = match self.instructions[id] {
                Instruction::Error(target, expected) => {
                    let expected = InstructionId(expected.0);
                    let expected = self.compute_expected(expected, &characters);
                    let expected = self.expecteds.insert(expected);
                    Instruction::Error(target, expected)
                }
                instruction => instruction,
            };

            self.instructions[id] = new_instruction;
        }
    }
}
