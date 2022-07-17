use std::collections::HashSet;
use crate::core::{Instruction, Parser};

impl Parser {
    /// Remove all unreachable instructions, classes and labels
    pub(super) fn trim(&mut self) {
        self.trim_instructions();
        self.trim_classes();
        self.trim_labels();
    }

    fn trim_instructions(&mut self) {
        let mut reachable = HashSet::new();

        let mut queue = vec![self.start];
        while let Some(id) = queue.pop() {
            if reachable.insert(id) {
                let instruction = self.instructions[id];
                queue.extend(instruction.successors());
            }
        }

        let removals = self
            .instructions()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            self.instructions.remove(removal);
        }
    }

    fn trim_classes(&mut self) {
        let mut reachable = HashSet::new();

        for (_, instruction) in self.instructions() {
            if let Instruction::Class(class) = instruction {
                reachable.insert(class);
            }
        }

        let removals = self
            .classes()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            self.classes.remove(removal);
        }
    }

    fn trim_labels(&mut self) {
        let mut reachable = HashSet::new();

        for (_, instruction) in self.instructions() {
            if let Instruction::Label(_, label) = instruction {
                reachable.insert(label);
            }
        }

        let removals = self
            .labels()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            self.labels.remove(removal);
        }
    }
}