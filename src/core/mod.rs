use std::collections::BTreeSet;

use crate::core::expected::{Expected, ExpectedId};
use serde::Serialize;

use crate::core::series::{Series, SeriesId};
use crate::store::{Store, StoreKey};

mod character;
mod expected;
mod fixed_point;
mod generation;
mod graphvis;
mod load;
mod series;
mod structure;
mod transformation;
mod validation;
mod walk;

#[derive(Debug, Eq, PartialEq)]
pub struct Parser {
    start: InstructionId,
    instructions: Store<InstructionId, Instruction>,
    series: Store<SeriesId, Series>,
    labels: Store<LabelId, String>,
    expecteds: Store<ExpectedId, Expected>,
}

impl Parser {
    pub fn load(ir: &[u8], settings: CompilerSettings) -> Result<Parser, Error> {
        let (mut parser, rule_names) = match Self::load_ir(ir) {
            Ok(result) => result,
            Err(err) => return Err(Error::Load(err)),
        };

        let errors = parser.validate();

        if !errors.is_empty() {
            let mut left_recursive = BTreeSet::new();

            for error in errors {
                match error {
                    ValidationError::LeftRecursion(id) => {
                        left_recursive.insert(rule_names[&id].clone());
                    }
                }
            }

            return Err(Error::LeftRecursive(left_recursive));
        }

        parser.transform(settings);

        Ok(parser)
    }

    pub fn dump_json(&self) -> String {
        #[derive(Serialize)]
        struct Proxy<'a> {
            start: &'a InstructionId,
            instructions: &'a Store<InstructionId, Instruction>,
            series: &'a Store<SeriesId, Series>,
            labels: &'a Store<LabelId, String>,
            expecteds: &'a Store<ExpectedId, Expected>,
        }

        let proxy = Proxy {
            start: &self.start,
            instructions: &self.instructions,
            series: &self.series,
            labels: &self.labels,
            expecteds: &self.expecteds,
        };

        serde_json::to_string(&proxy).unwrap()
    }

    fn new() -> Self {
        Self {
            start: InstructionId(0),
            instructions: Store::new(),
            series: Store::new(),
            labels: Store::new(),
            expecteds: Store::new(),
        }
    }

    fn insert(&mut self, instruction: Instruction) -> InstructionId {
        let id = self.instructions.reserve();
        self.instructions.set(id, instruction);
        id
    }

    fn instructions(&self) -> impl DoubleEndedIterator<Item = (InstructionId, Instruction)> + '_ {
        self.instructions.iter_copied()
    }

    fn start(&self) -> InstructionId {
        self.start
    }

    fn start_mut(&mut self) -> &mut InstructionId {
        &mut self.start
    }

    fn insert_series(&mut self, series: Series) -> SeriesId {
        self.series.insert(series)
    }

    fn series(&self) -> impl DoubleEndedIterator<Item = (SeriesId, &Series)> + '_ {
        self.series.iter()
    }

    fn insert_label(&mut self, label: String) -> LabelId {
        self.labels.insert(label)
    }

    fn labels(&self) -> impl DoubleEndedIterator<Item = (LabelId, &str)> + '_ {
        self.labels.iter().map(|(id, label)| (id, label.as_str()))
    }

    fn expecteds(&self) -> impl DoubleEndedIterator<Item = (ExpectedId, &Expected)> + '_ {
        self.expecteds.iter()
    }

    fn remap(&mut self, mut mapper: impl FnMut(InstructionId) -> InstructionId) {
        for (_, instruction) in self.instructions.iter_mut() {
            *instruction = instruction.remapped(&mut mapper);
        }

        self.start = mapper(self.start);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CompilerSettings {
    pub merge_series: bool,
}

impl CompilerSettings {
    pub fn normal() -> Self {
        Self { merge_series: true }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
struct InstructionId(pub usize);

impl StoreKey for InstructionId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
struct LabelId(pub usize);

impl StoreKey for LabelId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
enum Instruction {
    Seq(InstructionId, InstructionId),
    Choice(InstructionId, InstructionId),
    NotAhead(InstructionId),
    Error(InstructionId, ExpectedId),
    Label(InstructionId, LabelId),
    Delegate(InstructionId),
    Series(SeriesId),
}

impl Instruction {
    fn successors(&self) -> impl DoubleEndedIterator<Item = InstructionId> {
        let (first, second) = match *self {
            Instruction::Seq(first, second) | Instruction::Choice(first, second) => {
                (Some(first), Some(second))
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Delegate(target) => (Some(target), None),
            Instruction::Series(_) => (None, None),
        };

        first.into_iter().chain(second)
    }

    fn remapped(&self, mut mapper: impl FnMut(InstructionId) -> InstructionId) -> Self {
        match *self {
            Instruction::Seq(first, second) => Instruction::Seq(mapper(first), mapper(second)),
            Instruction::Choice(first, second) => {
                Instruction::Choice(mapper(first), mapper(second))
            }
            Instruction::NotAhead(target) => Instruction::NotAhead(mapper(target)),
            Instruction::Error(target, expected) => Instruction::Error(mapper(target), expected),
            Instruction::Label(target, label) => Instruction::Label(mapper(target), label),
            Instruction::Delegate(target) => Instruction::Delegate(mapper(target)),
            Instruction::Series(_) => *self,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    LeftRecursive(BTreeSet<String>),
    Load(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ValidationError {
    LeftRecursion(InstructionId),
}
