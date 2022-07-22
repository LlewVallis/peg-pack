use std::collections::HashSet;
use crate::core::{DebugSymbol, Parser};

impl Parser {
    pub(super) fn infer_debug_symbols(&mut self) {
        let candidates = self.walk()
            .map(|(k, _)| k)
            .filter(|id| self.debug_symbols[id].names.is_empty())
            .collect::<HashSet<_>>();

        let mut queue = candidates.iter().copied().collect::<Vec<_>>();

        let predecessors = self.compute_predecessors();

        while let Some(id) = queue.pop() {
            let predecessor_symbols = predecessors[&id].iter()
                .map(|id| &self.debug_symbols[id]);

            let new_symbol = DebugSymbol::merge_many(predecessor_symbols);
            let new_symbol = DebugSymbol::merge(&new_symbol, &self.debug_symbols[&id]);

            if self.debug_symbols[&id] != new_symbol {
                self.debug_symbols.insert(id, new_symbol);

                for successor in self.instructions[id].successors() {
                    if candidates.contains(&successor) {
                        queue.push(successor);
                    }
                }
            }
        }
    }
}