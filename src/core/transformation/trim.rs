use std::collections::HashSet;

use crate::core::{Instruction, Parser};
use crate::store::{Store, StoreKey};

impl Parser {
    /// Remove all unreachable instructions, classes and labels
    pub(super) fn trim(&mut self) {
        self.trim_instructions();
        self.trim_series();
        self.trim_labels();
        self.trim_expecteds();
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
            assert!(self.instructions.remove(removal).is_some());
        }

        let symbol_removals = self
            .debug_symbols
            .keys()
            .copied()
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in symbol_removals {
            assert!(self.debug_symbols.remove(&removal).is_some());
        }
    }

    fn trim_series(&mut self) {
        self.trim_resource(
            |parser| &mut parser.series,
            |instruction| match instruction {
                Instruction::Series(id) => Some(id),
                _ => None,
            },
        );
    }

    fn trim_labels(&mut self) {
        self.trim_resource(
            |parser| &mut parser.labels,
            |instruction| match instruction {
                Instruction::Label(_, id) => Some(id),
                _ => None,
            },
        );
    }

    fn trim_expecteds(&mut self) {
        self.trim_resource(
            |parser| &mut parser.expecteds,
            |instruction| match instruction {
                Instruction::Error(_, id) => Some(id),
                _ => None,
            },
        );
    }

    fn trim_resource<K: StoreKey, V>(
        &mut self,
        store: impl FnOnce(&mut Self) -> &mut Store<K, V>,
        extract: impl Fn(Instruction) -> Option<K>,
    ) {
        let mut reachable = HashSet::new();

        for (_, instruction) in self.instructions() {
            if let Some(id) = extract(instruction) {
                reachable.insert(id);
            }
        }

        let store = store(self);

        let removals = store
            .iter()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            store.remove(removal);
        }
    }
}
