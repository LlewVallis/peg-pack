use serde::Serialize;
use std::collections::BTreeSet;

use crate::store::{Store, StoreKey};

mod character;
mod fixed_point;
mod generation;
mod graphvis;
mod load;
mod structure;
mod transformation;
mod validation;

#[derive(Debug, Eq, PartialEq)]
pub struct Parser {
    start: InstructionId,
    instructions: Store<InstructionId, Instruction>,
    classes: Store<ClassId, Class>,
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
            classes: &'a Store<ClassId, Class>,
            labels: &'a Store<LabelId, String>,
        }

        let proxy = Proxy {
            start: &self.start,
            instructions: &self.instructions,
            classes: &self.classes,
            labels: &self.labels,
        };

        serde_json::to_string(&proxy).unwrap()
    }

    fn new() -> Self {
        Self {
            start: InstructionId(0),
            instructions: Store::new(),
            classes: Store::new(),
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

    fn insert_class(&mut self, class: Class) -> ClassId {
        self.classes.insert(class)
    }

    fn classes(&self) -> impl Iterator<Item = (ClassId, &Class)> + '_ {
        self.classes.iter()
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
struct ClassId(pub usize);

impl StoreKey for ClassId {
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
    Class(ClassId),
    Empty,
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
            Instruction::Class(_) | Instruction::Empty => (None, None),
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
            Instruction::Class(_) | Instruction::Empty => *self,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize)]
struct Class {
    negated: bool,
    ranges: Vec<(u8, u8)>,
}

impl Class {
    pub fn new(negated: bool) -> Self {
        Self {
            negated,
            ranges: vec![],
        }
    }

    pub fn insert<T: Into<u8>>(&mut self, start: T, end: T) {
        let start = start.into();
        let end = end.into();

        assert!(start <= end);
        self.ranges.push((start, end));

        self.ranges.sort_unstable_by_key(|(start, _)| *start);

        let mut i = 0;
        while i + 1 < self.ranges.len() {
            let current = self.ranges[i];
            let next = &mut self.ranges[i + 1];

            if current.1 >= next.0 {
                next.0 = u8::min(current.0, next.0);
                next.1 = u8::max(current.1, next.1);
                self.ranges.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn negated(&self) -> bool {
        self.negated
    }

    pub fn ranges(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.ranges.iter().copied()
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
