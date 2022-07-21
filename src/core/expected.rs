use std::collections::{BTreeSet, HashMap, HashSet};

use serde::Serialize;

use crate::core::character::Character;
use crate::core::series::Series;
use crate::core::{Instruction, InstructionId, Parser};
use crate::store::StoreKey;

/// Before expecteds are computer for all error rules, these actually point to
/// instructions
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExpectedId(pub usize);

impl StoreKey for ExpectedId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize)]
pub struct Expected {
    labels: BTreeSet<String>,
    literals: BTreeSet<Vec<u8>>,
}

impl Expected {
    fn append_series(&mut self, series: &Series) {
        let literal = self.linear_series_prefix(series);
        if !literal.is_empty() {
            self.literals.insert(literal);
            return;
        }

        if let Some(class) = series.classes().get(0) {
            if !class.negated() {
                for (lower, upper) in class.ranges() {
                    for char in *lower..=*upper {
                        self.literals.insert(vec![char]);
                    }
                }
            }
        }
    }

    fn linear_series_prefix(&self, series: &Series) -> Vec<u8> {
        let mut buffer = Vec::new();

        for class in series.classes() {
            if class.negated() || class.ranges().len() != 1 {
                return buffer;
            }

            let (lower, upper) = class.ranges()[0];
            if lower != upper {
                return buffer;
            }

            buffer.push(lower);
        }

        buffer
    }

    pub fn labels(&self) -> impl Iterator<Item = &str> + '_ {
        self.labels.iter().map(|string| string.as_str())
    }

    pub fn literals(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.literals.iter().map(|buffer| buffer.as_slice())
    }
}

impl Parser {
    pub(super) fn compute_expected(
        &self,
        id: InstructionId,
        characters: &HashMap<InstructionId, Character>,
    ) -> Expected {
        let mut result = Expected {
            labels: BTreeSet::new(),
            literals: BTreeSet::new(),
        };

        let mut visited = HashSet::new();

        self.expected_at(id, &mut result, characters, &mut visited);

        result
    }

    fn expected_at(
        &self,
        id: InstructionId,
        result: &mut Expected,
        characters: &HashMap<InstructionId, Character>,
        visited: &mut HashSet<InstructionId>,
    ) {
        if !visited.insert(id) {
            return;
        }

        let instruction = self.instructions[id];

        match instruction {
            Instruction::Seq(first, second) => {
                self.expected_at(first, result, characters, visited);

                if characters[&first].transparent {
                    self.expected_at(second, result, characters, visited);
                }
            }
            Instruction::Choice(first, second) => {
                self.expected_at(first, result, characters, visited);
                self.expected_at(second, result, characters, visited);
            }
            Instruction::Error(target, _) | Instruction::Delegate(target) => {
                self.expected_at(target, result, characters, visited);
            }
            Instruction::Label(_, label) => {
                let label = self.labels[label].clone();
                result.labels.insert(label);
            }
            Instruction::Series(series) => {
                let series = &self.series[series];
                result.append_series(series);
            }
            Instruction::NotAhead(_) => {}
        }

        visited.remove(&id);
    }
}
