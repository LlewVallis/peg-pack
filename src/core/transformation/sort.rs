use std::collections::HashMap;

use crate::core::{Instruction, Parser};
use crate::store::Store;

impl Parser {
    /// Sort the instructions in the map by a depth first search. This is not actually necessary,
    /// but makes the visualizations nicer
    pub(super) fn sort(&mut self) {
        let mut new_instructions = Store::new();
        let mut instruction_mappings = HashMap::new();

        let mut new_series = Store::new();
        let mut series_mappings = HashMap::new();

        let mut new_labels = Store::new();
        let mut label_mappings = HashMap::new();

        let walk = self.walk().collect::<Vec<_>>();
        for (id, instruction) in walk {
            let new_id = new_instructions.insert(instruction);
            instruction_mappings.insert(id, new_id);

            match instruction {
                Instruction::Series(id) => {
                    if !series_mappings.contains_key(&id) {
                        let series = self.series.remove(id).unwrap();
                        let new_id = new_series.insert(series);
                        series_mappings.insert(id, new_id);
                    }
                }
                Instruction::Label(_, id) => {
                    if !label_mappings.contains_key(&id) {
                        let label = self.labels.remove(id).unwrap();
                        let new_id = new_labels.insert(label);
                        label_mappings.insert(id, new_id);
                    }
                }
                _ => {}
            }
        }

        for (_, instruction) in new_instructions.iter_mut() {
            *instruction = instruction.remapped(|id| instruction_mappings[&id]);

            match instruction {
                Instruction::Series(id) => {
                    *id = series_mappings[id];
                }
                Instruction::Label(_, id) => {
                    *id = label_mappings[id];
                }
                _ => {}
            }
        }

        self.start = instruction_mappings[&self.start];
        self.instructions = new_instructions;
        self.series = new_series;
        self.labels = new_labels;
    }
}
