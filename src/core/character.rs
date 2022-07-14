use std::collections::HashMap;

use crate::core::fixed_point::FixedPointStates;
use crate::core::{ClassId, InstructionId};
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
            Instruction::Error(target) => self.characterize_error(target, states),
            Instruction::Delegate(target) => self.characterize_delegate(target, states),
            Instruction::Class(class) => self.characterize_class(class),
            Instruction::Empty => self.characterize_empty(),
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
        }
    }

    fn characterize_delegate(
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

    fn characterize_class(&self, class: ClassId) -> Character {
        let class = &self.classes[class];

        let empty = class.ranges().count() == 0 && !class.negated();

        Character {
            transparent: false,
            antitransparent: !empty,
            fallible: true,
        }
    }

    fn characterize_empty(&self) -> Character {
        Character {
            transparent: true,
            antitransparent: false,
            fallible: false,
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
