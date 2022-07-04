use crate::core::{Instruction, InstructionId, Parser};
use std::collections::{HashMap, HashSet};
use std::ops::Index;

impl Parser {
    /// Solves an inductive function over the instruction graph by iterating 
    /// until a fixed point is reached. That is, it allows evaluating functions
    /// that are defined recursively over an instruction's children even in the
    /// presence of cycles
    pub fn solve_fixed_point<T: Eq>(
        &self,
        default: T,
        mut evaluate: impl FnMut(InstructionId, Instruction, &FixedPointStates<T>) -> T,
    ) -> HashMap<InstructionId, T> {
        let predecessors = self.compute_predecessors();
        let mut states = FixedPointStates::new(default);

        let mut queue = FixedPointQueue::new();

        for (id, _) in self.instructions() {
            queue.insert(id);
        }

        while let Some(id) = queue.pop() {
            let instruction = self.instructions[id];

            let new_value = evaluate(id, instruction, &states);
            let updated = states[id] != new_value;
            states.set(id, new_value);

            if updated {
                for predecessor in &predecessors[&id] {
                    queue.insert(*predecessor);
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

struct FixedPointQueue {
    elements: Vec<InstructionId>,
    set: HashSet<InstructionId>,
}

impl FixedPointQueue {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            set: HashSet::new(),
        }
    }

    pub fn insert(&mut self, value: InstructionId) {
        if self.set.insert(value) {
            self.elements.push(value);
        }
    }

    pub fn pop(&mut self) -> Option<InstructionId> {
        let result = self.elements.pop();

        if let Some(id) = result {
            self.set.remove(&id);
        }

        result
    }
}
