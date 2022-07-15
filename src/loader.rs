use regex::Regex;
use std::collections::HashMap;

use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::core::{Class, Instruction, InstructionId, Parser};

/// Required IR file version
const VERSION: u32 = 0;

/// Load some IR into a parser and rule name map, or fail with an error message
pub fn load_ir(bytes: &[u8]) -> Result<(Parser, HashMap<InstructionId, String>), String> {
    let ir = match serde_json::from_slice::<Ir>(bytes) {
        Ok(ir) => ir,
        Err(err) => return Err(format!("Malformed internal representation ({})", err)),
    };

    let mut loader = Loader {
        parser: Parser::new(),
        rule_names: HashMap::new(),
        instruction_count: 0,
    };

    loader.load_ir(ir)?;

    Ok((loader.parser, loader.rule_names))
}

struct Loader {
    parser: Parser,
    rule_names: HashMap<InstructionId, String>,
    instruction_count: usize,
}

impl Loader {
    pub fn load_ir(&mut self, ir: Ir) -> Result<(), String> {
        let (start, instructions) = match ir {
            Ir::Success {
                start,
                instructions,
                ..
            } => (start, instructions),
            Ir::Error { message: error, .. } => return Err(error),
        };

        self.instruction_count = instructions.len();

        let start = self.load_reference(start)?;
        *self.parser.start_mut() = start;

        for instruction in instructions {
            self.load_instruction(instruction)?;
        }

        Ok(())
    }

    fn load_instruction(&mut self, ir: InstructionIr) -> Result<(), String> {
        let id = match &ir {
            InstructionIr::Seq { first, second, .. } => {
                let first = self.load_reference(*first)?;
                let second = self.load_reference(*second)?;
                self.parser.insert(Instruction::Seq(first, second))
            }
            InstructionIr::Choice { first, second, .. } => {
                let first = self.load_reference(*first)?;
                let second = self.load_reference(*second)?;
                self.parser.insert(Instruction::Choice(first, second))
            }
            InstructionIr::NotAhead { target, .. } => {
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::NotAhead(target))
            }
            InstructionIr::Error { target, .. } => {
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::Error(target))
            }
            InstructionIr::Label { target, label, .. } => {
                let label = self.parser.insert_label(label.clone());
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::Label(target, label))
            }
            InstructionIr::Delegate { target, .. } => {
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::Delegate(target))
            }
            InstructionIr::Class {
                negated, ranges, ..
            } => {
                let mut class = Class::new(*negated);

                for (start, end) in ranges {
                    class.insert(*start, *end);
                }

                let class = self.parser.insert_class(class);
                self.parser.insert(Instruction::Class(class))
            }
            InstructionIr::Empty { .. } => self.parser.insert(Instruction::Empty),
        };

        match ir {
            InstructionIr::Seq { rule_name, .. }
            | InstructionIr::Choice { rule_name, .. }
            | InstructionIr::NotAhead { rule_name, .. }
            | InstructionIr::Error { rule_name, .. }
            | InstructionIr::Label { rule_name, .. }
            | InstructionIr::Delegate { rule_name, .. }
            | InstructionIr::Class { rule_name, .. }
            | InstructionIr::Empty { rule_name } => {
                self.rule_names.insert(id, rule_name);
            }
        }

        Ok(())
    }

    fn load_reference(&self, id: usize) -> Result<InstructionId, String> {
        if id < self.instruction_count {
            Ok(InstructionId(id))
        } else {
            Err(format!("Invalid IR: Illegal instruction ID: {}", id))
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum Ir {
    Error {
        #[serde(rename = "version")]
        _version: VersionCheck,
        message: String,
    },
    Success {
        #[serde(rename = "version")]
        _version: VersionCheck,
        start: usize,
        instructions: Vec<InstructionIr>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
enum InstructionIr {
    #[serde(rename_all = "camelCase")]
    Seq {
        first: usize,
        second: usize,
        rule_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Choice {
        first: usize,
        second: usize,
        rule_name: String,
    },
    #[serde(rename_all = "camelCase")]
    NotAhead { target: usize, rule_name: String },
    #[serde(rename_all = "camelCase")]
    Error { target: usize, rule_name: String },
    #[serde(rename_all = "camelCase")]
    Label {
        target: usize,
        label: String,
        rule_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Delegate { target: usize, rule_name: String },
    #[serde(rename_all = "camelCase")]
    Class {
        negated: bool,
        ranges: Vec<(u8, u8)>,
        rule_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Empty { rule_name: String },
}

struct VersionCheck;

impl<'a> Deserialize<'a> for VersionCheck {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let version = u32::deserialize(deserializer)?;

        if version == VERSION {
            Ok(VersionCheck)
        } else {
            Err(D::Error::custom("invalid version"))
        }
    }
}

struct Label(String);

impl<'a> Deserialize<'a> for Label {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;

        let regex = Regex::new("[a-z]+(_[a-z]+)*").unwrap();
        if !regex.is_match(&value) {
            return Err(D::Error::custom("invalid label"));
        }

        Ok(Label(value))
    }
}
