use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::core::expected::ExpectedId;
use crate::core::series::{Class, Series};
use crate::core::{DebugSymbol, Instruction, InstructionId, Parser};

/// Required IR file version
const VERSION: u32 = 0;

impl Parser {
    /// Load some IR into a parser and rule name map, or fail with an error message
    pub(super) fn load_ir(bytes: &[u8]) -> Result<Self, String> {
        let ir = match serde_json::from_slice::<Ir>(bytes) {
            Ok(ir) => ir,
            Err(err) => return Err(format!("Malformed internal representation ({})", err)),
        };

        let mut loader = Loader {
            parser: Parser::new(),
            instruction_count: 0,
        };

        loader.load_ir(ir)?;

        Ok(loader.parser)
    }
}

struct Loader {
    parser: Parser,
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
        let rule_name = match &ir {
            InstructionIr::Seq { rule_name, .. }
            | InstructionIr::Choice { rule_name, .. }
            | InstructionIr::NotAhead { rule_name, .. }
            | InstructionIr::Error { rule_name, .. }
            | InstructionIr::Label { rule_name, .. }
            | InstructionIr::Delegate { rule_name, .. }
            | InstructionIr::Series { rule_name, .. } => rule_name,
        };

        let symbol = match rule_name {
            Some(name) => DebugSymbol::named(name.clone()),
            None => DebugSymbol::anonymous(),
        };

        match &ir {
            InstructionIr::Seq { first, second, .. } => {
                let first = self.load_reference(*first)?;
                let second = self.load_reference(*second)?;
                self.parser.insert(Instruction::Seq(first, second), symbol);
            }
            InstructionIr::Choice { first, second, .. } => {
                let first = self.load_reference(*first)?;
                let second = self.load_reference(*second)?;
                self.parser
                    .insert(Instruction::Choice(first, second), symbol);
            }
            InstructionIr::NotAhead { target, .. } => {
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::NotAhead(target), symbol);
            }
            InstructionIr::Error {
                target, expected, ..
            } => {
                let target = self.load_reference(*target)?;
                let expected = self.load_reference(*expected)?;
                self.parser
                    .insert(Instruction::Error(target, ExpectedId(expected.0)), symbol);
            }
            InstructionIr::Label { target, label, .. } => {
                let label = self.parser.insert_label(label.clone());
                let target = self.load_reference(*target)?;
                self.parser
                    .insert(Instruction::Label(target, label), symbol);
            }
            InstructionIr::Delegate { target, .. } => {
                let target = self.load_reference(*target)?;
                self.parser.insert(Instruction::Delegate(target), symbol);
            }
            InstructionIr::Series { classes, .. } => {
                let mut series = Series::empty();

                for class_ir in classes {
                    let mut class = Class::new(class_ir.negated);

                    for (lower, upper) in &class_ir.ranges {
                        class.insert(*lower, *upper);
                    }

                    series.append(class);
                }

                let series = self.parser.insert_series(series);
                self.parser.insert(Instruction::Series(series), symbol);
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
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Choice {
        first: usize,
        second: usize,
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    NotAhead {
        target: usize,
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        target: usize,
        expected: usize,
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Label {
        target: usize,
        label: String,
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Delegate {
        target: usize,
        rule_name: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Series {
        classes: Vec<ClassIr>,
        rule_name: Option<String>,
    },
}

#[derive(Deserialize)]
struct ClassIr {
    negated: bool,
    ranges: Vec<(u8, u8)>,
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
