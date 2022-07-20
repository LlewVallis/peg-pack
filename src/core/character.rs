use std::collections::HashMap;

use crate::core::fixed_point::FixedPointStates;
use crate::core::series::SeriesId;
use crate::core::InstructionId;
use crate::core::{Instruction, Parser};

impl Parser {
    /// Computes the instruction characters of the parser
    pub(super) fn characterize(&self) -> HashMap<InstructionId, Character> {
        let default = Character {
            transparent: false,
            antitransparent: false,
            fallible: false,
        };

        self.solve_fixed_point(default, |_, instruction, states| match instruction {
            Instruction::Seq(first, second) => self.characterize_seq(first, second, states),
            Instruction::Choice(first, second) => self.characterize_choice(first, second, states),
            Instruction::NotAhead(target) => self.characterize_not_ahead(target, states),
            Instruction::Error(target)
            | Instruction::Label(target, _)
            | Instruction::Delegate(target) => self.characterize_delegate_like(target, states),
            Instruction::Series(series) => self.characterize_series(series),
        })
    }

    fn characterize_seq(
        &self,
        first: InstructionId,
        second: InstructionId,
        states: &FixedPointStates<Character>,
    ) -> Character {
        let first = states[first];
        let second = states[second];

        let possible = (first.transparent || first.antitransparent)
            && (second.transparent || second.antitransparent);

        Character {
            transparent: (first.transparent && second.transparent) && possible,
            antitransparent: (first.antitransparent || second.antitransparent) && possible,
            fallible: first.fallible || second.fallible,
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

        Character {
            transparent: first.transparent || second.transparent,
            antitransparent: first.antitransparent || second.antitransparent,
            fallible: first.fallible && second.fallible,
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
            fallible: target.transparent || target.antitransparent,
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
        }
    }

    fn characterize_series(&self, series: SeriesId) -> Character {
        let series = &self.series[series];

        Character {
            transparent: series.is_empty(),
            antitransparent: !series.is_empty() && !series.is_never(),
            fallible: !series.is_empty(),
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
}
