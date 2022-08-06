use std::collections::HashMap;
use std::ops::Index;

use crate::core::{Instruction, InstructionId, Parser};
use crate::ordered_set::OrderedSet;

impl Parser {
    /// Solves an inductive function over the instruction graph by iterating
    /// until a fixed point is reached. That is, it allows evaluating functions
    /// that are defined recursively over an instruction's children even in the
    /// presence of cycles
    pub(super) fn solve_fixed_point<T: Eq>(
        &self,
        default: T,
        mut evaluate: impl FnMut(InstructionId, Instruction, &FixedPointStates<T>) -> T,
    ) -> HashMap<InstructionId, T> {
        let predecessors = self.compute_predecessors();
        let mut states = FixedPointStates::new(default);

        let mut queue = OrderedSet::new();

        for (id, _) in self.instructions() {
            queue.push(id);
        }

        while let Some(id) = queue.pop() {
            let instruction = self.instructions[id];

            let new_value = evaluate(id, instruction, &states);
            let updated = states[id] != new_value;
            states.set(id, new_value);

            if updated {
                for predecessor in &predecessors[&id] {
                    queue.push(*predecessor);
                }
            }
        }

        states.map
    }
}

pub struct FixedPointStates<T> {
    map: HashMap<InstructionId, T>,
    default: T,
}

impl<T> FixedPointStates<T> {
    fn new(default: T) -> Self {
        Self {
            default,
            map: HashMap::new(),
        }
    }

    fn set(&mut self, id: InstructionId, value: T) {
        self.map.insert(id, value);
    }
}

impl<T> Index<InstructionId> for FixedPointStates<T> {
    type Output = T;

    fn index(&self, index: InstructionId) -> &T {
        self.map.get(&index).unwrap_or(&self.default)
    }
}
