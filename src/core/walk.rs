use std::collections::HashSet;
use std::iter::FusedIterator;

use crate::core::{Instruction, InstructionId, Parser};

impl Parser {
    pub(super) fn walk(&self) -> impl Iterator<Item = (InstructionId, Instruction)> + '_ {
        Walk {
            parser: self,
            queue: vec![self.start],
            visited: HashSet::new(),
        }
    }
}

struct Walk<'a> {
    parser: &'a Parser,
    queue: Vec<InstructionId>,
    visited: HashSet<InstructionId>,
}

impl<'a> Iterator for Walk<'a> {
    type Item = (InstructionId, Instruction);

    fn next(&mut self) -> Option<Self::Item> {
        let next = loop {
            let candidate = self.queue.pop()?;
            if self.visited.insert(candidate) {
                break candidate;
            }
        };

        let instruction = self.parser.instructions[next];

        for successor in instruction.successors().rev() {
            self.queue.push(successor);
        }

        Some((next, instruction))
    }
}

impl<'a> FusedIterator for Walk<'a> {}
