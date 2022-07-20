use serde::Serialize;
use std::collections::BTreeSet;
use crate::core::series::{Series, SeriesId};

use crate::store::{Store, StoreKey};

mod character;
mod fixed_point;
mod generation;
mod graphvis;
mod load;
mod structure;
mod transformation;
mod validation;
mod series;

#[derive(Debug, Eq, PartialEq)]
pub struct Parser {
    start: InstructionId,
    instructions: Store<InstructionId, Instruction>,
    series: Store<SeriesId, Series>,
    labels: Store<LabelId, String>,
}

impl Parser {
    pub fn load(ir: &[u8]) -> Result<Parser, Error> {
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

        parser.transform();

        Ok(parser)
    }

    pub fn dump_json(&self) -> String {
        #[derive(Serialize)]
        struct Proxy<'a> {
            start: &'a InstructionId,
            instructions: &'a Store<InstructionId, Instruction>,
            series: &'a Store<SeriesId, Series>,
            labels: &'a Store<LabelId, String>,
        }

        let proxy = Proxy {
            start: &self.start,
            instructions: &self.instructions,
            series: &self.series,
            labels: &self.labels,
        };

        serde_json::to_string(&proxy).unwrap()
    }

    fn new() -> Self {
        Self {
            start: InstructionId(0),
            instructions: Store::new(),
            series: Store::new(),
            labels: Store::new(),
        }
    }

    fn insert(&mut self, instruction: Instruction) -> InstructionId {
        let id = self.instructions.reserve();
        self.instructions.set(id, instruction);
        id
    }

    fn instructions(&self) -> impl Iterator<Item = (InstructionId, Instruction)> + '_ {
        self.instructions.iter_copied()
    }

    fn start(&self) -> InstructionId {
        self.start
    }

    fn start_mut(&mut self) -> &mut InstructionId {
        &mut self.start
    }

    fn insert_series(&mut self, sequence: Series) -> SeriesId {
        self.series.insert(sequence)
    }

    fn series(&self) -> impl Iterator<Item = (SeriesId, &Series)> + '_ {
        self.series.iter()
    }

    fn insert_label(&mut self, label: String) -> LabelId {
        self.labels.insert(label)
    }

    fn labels(&self) -> impl Iterator<Item = (LabelId, &str)> + '_ {
        self.labels.iter().map(|(id, label)| (id, label.as_str()))
    }

    fn unwrap_label(&self, id: LabelId) -> &str {
        &self.labels[id]
    }

    fn relabel(&mut self, mut mapper: impl FnMut(InstructionId) -> InstructionId) {
        let mut new_instructions = Store::new();

        for (id, instruction) in self.instructions() {
            let new_id = mapper(id);
            let new_instruction = instruction.remapped(&mut mapper);
            new_instructions.set(new_id, new_instruction);
        }

        self.instructions = new_instructions;
        self.start = mapper(self.start);
    }

    fn remap(&mut self, mut mapper: impl FnMut(InstructionId) -> InstructionId) {
        for (_, instruction) in self.instructions.iter_mut() {
            *instruction = instruction.remapped(&mut mapper);
        }

        self.start = mapper(self.start);
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
    Error(InstructionId),
    Label(InstructionId, LabelId),
    Delegate(InstructionId),
    Series(SeriesId),
}

impl Instruction {
    fn successors(&self) -> impl Iterator<Item = InstructionId> {
        let (first, second) = match *self {
            Instruction::Seq(first, second) | Instruction::Choice(first, second) => {
                (Some(first), Some(second))
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target)
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
            Instruction::Error(target) => Instruction::Error(mapper(target)),
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
