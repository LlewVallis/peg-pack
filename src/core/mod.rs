mod character;
mod fixed_point;
mod graphvis;
mod optimization;
mod structure;
mod validation;

use crate::store::{Store, StoreKey};
use std::collections::HashSet;

#[derive(Debug, Eq, PartialEq)]
pub struct Parser {
    start: InstructionId,
    instructions: Store<InstructionId, Instruction>,
    classes: Store<ClassId, Class>,
    labels: Store<LabelId, String>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            start: InstructionId(0),
            instructions: Store::new(),
            classes: Store::new(),
            labels: Store::new(),
        }
    }

    pub fn reserve(&mut self) -> InstructionId {
        self.instructions.reserve()
    }

    pub fn insert(&mut self, instruction: Instruction) -> InstructionId {
        let id = self.reserve();
        self.set(id, instruction);
        id
    }

    pub fn set(&mut self, id: InstructionId, instruction: Instruction) {
        assert!(!self.instructions.contains(id));
        self.instructions.set(id, instruction);
    }

    pub fn instructions(&self) -> impl Iterator<Item = (InstructionId, Instruction)> + '_ {
        self.instructions.iter_copied()
    }

    pub fn start(&self) -> InstructionId {
        self.start
    }

    pub fn start_mut(&mut self) -> &mut InstructionId {
        &mut self.start
    }

    pub fn insert_class(&mut self, class: Class) -> ClassId {
        self.classes.insert(class)
    }

    pub fn classes(&self) -> impl Iterator<Item = (ClassId, &Class)> + '_ {
        self.classes.iter()
    }

    pub fn insert_label(&mut self, label: String) -> LabelId {
        self.labels.insert(label)
    }

    pub fn labels(&self) -> impl Iterator<Item = (LabelId, &str)> + '_ {
        self.labels.iter().map(|(id, label)| (id, label.as_str()))
    }

    pub fn unwrap_label(&self, id: LabelId) -> &str {
        &self.labels[id]
    }

    pub fn compile(mut self) -> Result<String, HashSet<Error>> {
        let errors = self.validate();

        if !errors.is_empty() {
            return Err(errors);
        }

        self.optimize();

        Ok(self.generate())
    }

    fn relabel(&mut self, mapper: impl Fn(InstructionId) -> InstructionId) {
        let mut new_instructions = Store::new();

        for (id, instruction) in self.instructions() {
            let new_id = mapper(id);
            let new_instruction = instruction.remapped(&mapper);
            new_instructions.set(new_id, new_instruction);
        }

        self.instructions = new_instructions;
        self.start = mapper(self.start);
    }

    fn remap(&mut self, mapper: impl Fn(InstructionId) -> InstructionId) {
        for (_, instruction) in self.instructions.iter_mut() {
            *instruction = instruction.remapped(&mapper);
        }

        self.start = mapper(self.start);
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct InstructionId(pub usize);

impl StoreKey for InstructionId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassId(pub usize);

impl StoreKey for ClassId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct LabelId(pub usize);

impl StoreKey for LabelId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Instruction {
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

    fn remapped(&self, mapper: impl Fn(InstructionId) -> InstructionId) -> Self {
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

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Class {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    LeftRecursion(InstructionId),
}
