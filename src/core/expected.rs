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
    series: BTreeSet<Series>,
}

impl Expected {
    pub fn labels(&self) -> impl Iterator<Item = &str> + '_ {
        self.labels.iter().map(|string| string.as_str())
    }

    pub fn series(&self) -> impl Iterator<Item = &Series> + '_ {
        self.series.iter()
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
            series: BTreeSet::new(),
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
                let series = self.series[series].clone();
                if !series.is_empty() && !series.is_never() {
                    result.series.insert(series);
                }
            }
            Instruction::NotAhead(_) => {}
        }

        visited.remove(&id);
    }
}
