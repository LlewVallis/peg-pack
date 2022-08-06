use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

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
    debug_symbols: HashMap<InstructionId, DebugSymbol>,
}

impl Parser {
    pub fn load(ir: &[u8], settings: CompilerSettings) -> Result<Parser, Error> {
        let mut parser = match Self::load_ir(ir) {
            Ok(result) => result,
            Err(err) => return Err(Error::Load(err)),
        };

        let errors = parser.validate();

        if !errors.is_empty() {
            let mut left_recursive = BTreeSet::new();

            for error in errors {
                match error {
                    ValidationError::LeftRecursion(id) => {
                        let symbol = parser.debug_symbols[&id].clone();
                        for name in symbol.names.iter() {
                            left_recursive.insert(name.clone());
                        }
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
            debug_symbols: HashMap::new(),
        }
    }

    fn insert(&mut self, instruction: Instruction, symbol: DebugSymbol) -> InstructionId {
        let id = self.instructions.reserve();
        self.instructions.set(id, instruction);
        self.debug_symbols.insert(id, symbol);
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
        for (id, _) in self.instructions.iter() {
            let new_id = mapper(id);

            let source_symbol = &self.debug_symbols[&id];
            let dest_symbol = &self.debug_symbols[&new_id];
            let new_symbol = DebugSymbol::merge(source_symbol, dest_symbol);

            self.debug_symbols.insert(new_id, new_symbol);
        }

        for (_, instruction) in self.instructions.iter_mut() {
            *instruction = instruction.remapped(&mut mapper);
        }

        self.start = mapper(self.start);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CompilerSettings {
    pub merge_series: bool,
    pub cache_insertion: bool,
}

impl CompilerSettings {
    pub fn normal() -> Self {
        Self {
            merge_series: true,
            cache_insertion: true,
        }
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
    Cache(InstructionId, Option<usize>),
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
            | Instruction::Cache(target, _)
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
            Instruction::Cache(target, id) => Instruction::Cache(mapper(target), id),
            Instruction::Series(_) => *self,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct DebugSymbol {
    names: Rc<BTreeSet<String>>,
}

impl DebugSymbol {
    pub fn named(name: String) -> Self {
        Self {
            names: Rc::new(BTreeSet::from([name])),
        }
    }

    pub fn anonymous() -> Self {
        Self {
            names: Rc::new(BTreeSet::new()),
        }
    }

    pub fn merge_many<'a>(values: impl IntoIterator<Item = &'a DebugSymbol>) -> Self {
        let mut values = values.into_iter().collect::<Vec<_>>();

        if values.is_empty() {
            return DebugSymbol::anonymous();
        }

        let mut result = values.pop().unwrap().clone();

        while let Some(other) = values.pop() {
            result = Self::merge(&result, other);
        }

        result
    }

    pub fn merge(first: &DebugSymbol, second: &DebugSymbol) -> Self {
        if first.names == second.names {
            return first.clone();
        }

        if first.names.is_empty() {
            return second.clone();
        }

        if second.names.is_empty() {
            return first.clone();
        }

        let mut new_names = BTreeSet::new();
        new_names.extend(first.names.iter().cloned());
        new_names.extend(second.names.iter().cloned());

        Self {
            names: Rc::new(new_names),
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
