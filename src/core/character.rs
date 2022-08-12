use std::collections::HashMap;

use crate::core::fixed_point::FixedPointStates;
use crate::core::series::SeriesId;
use crate::core::InstructionId;
use crate::core::{Instruction, Parser};

impl Parser {
    /// Computes the instruction characters of the parser
    pub(super) fn characterize(&self) -> HashMap<InstructionId, Character> {
        self.patch_characters(HashMap::new(), self.instructions().map(|(id, _)| id))
    }

    pub(super) fn patch_characters(
        &self,
        characters: HashMap<InstructionId, Character>,
        instructions: impl IntoIterator<Item = InstructionId>,
    ) -> HashMap<InstructionId, Character> {
        let default = Character {
            transparent: false,
            antitransparent: false,
            fallible: false,
            label_prone: false,
            error_prone: false,
        };

        self.solve_fixed_point(
            characters,
            instructions,
            default,
            |_, instruction, states| match instruction {
                Instruction::Seq(first, second) => self.characterize_seq(first, second, states),
                Instruction::Choice(first, second) => {
                    self.characterize_choice(first, second, states)
                }
                Instruction::NotAhead(target) => self.characterize_not_ahead(target, states),
                Instruction::Error(target, _) => self.characterize_error(target, states),
                Instruction::Label(target, _) => self.characterize_label(target, states),
                Instruction::Cache(target, _)
                | Instruction::Delegate(target) => self.characterize_delegate_like(target, states),
                Instruction::Series(series) => self.characterize_series(series),
            },
        )
    }

    fn characterize_seq(
        &self,
        first: InstructionId,
        second: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let first = states[first];
        let second = states[second];

        let possible = first.possible() && second.possible();

        Character {
            transparent: (first.transparent && second.transparent) && possible,
            antitransparent: (first.antitransparent || second.antitransparent) && possible,
            fallible: first.fallible || second.fallible,
            label_prone: (first.label_prone || second.label_prone) && possible,
            error_prone: (first.error_prone || second.error_prone) && possible,
        }
    }

    fn characterize_choice(
        &self,
        first: InstructionId,
        second: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let first = states[first];
        let second = states[second];

        let second_executable = first.fallible || first.error_prone;

        Character {
            transparent: first.transparent || second_executable && second.transparent,
            antitransparent: first.antitransparent || second_executable && second.antitransparent,
            fallible: first.fallible && second.fallible,
            label_prone: first.label_prone || second_executable && second.label_prone,
            error_prone: first.error_prone || second_executable && second.error_prone,
        }
    }

    fn characterize_not_ahead(
        &self,
        target: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let target = states[target];

        Character {
            transparent: target.fallible,
            antitransparent: false,
            fallible: target.possible(),
            label_prone: false,
            error_prone: false,
        }
    }

    fn characterize_label(
        &self,
        target: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let target = states[target];

        Character {
            transparent: target.transparent,
            antitransparent: target.antitransparent,
            fallible: target.fallible,
            label_prone: target.possible(),
            error_prone: target.error_prone,
        }
    }

    fn characterize_error(
        &self,
        target: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let target = states[target];

        Character {
            transparent: target.transparent,
            antitransparent: target.antitransparent,
            fallible: target.fallible,
            label_prone: target.label_prone,
            error_prone: target.possible(),
        }
    }

    fn characterize_delegate_like(
        &self,
        target: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let target = states[target];

        Character {
            transparent: target.transparent,
            antitransparent: target.antitransparent,
            fallible: target.fallible,
            label_prone: target.label_prone,
            error_prone: target.error_prone,
        }
    }

    fn characterize_series(&self, series: SeriesId) -> Character {
        let series = &self.series[series];

        Character {
            transparent: series.is_empty(),
            antitransparent: !series.is_empty() && !series.is_never(),
            fallible: !series.is_empty(),
            label_prone: false,
            error_prone: false,
        }
    }
}

/// The character of an instruction implements a conservative analysis of the
/// conditions under which it can succeed and fail
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Character {
    /// An instruction is transparent if it can successfully match without
    /// consuming input
    pub transparent: bool,
    /// An instruction is antitransparent if it can successfully match while
    /// consuming input
    pub antitransparent: bool,
    /// An instruction is fallible if it can fail to match
    pub fallible: bool,
    /// An instruction is label prone if it can successfully match with a label
    pub label_prone: bool,
    /// An instruction is label prone if it can successfully match with an
    /// error
    pub error_prone: bool,
}

impl Character {
    /// An instruction is possible if it can successfully match
    pub fn possible(&self) -> bool {
        self.transparent || self.antitransparent
    }
}
