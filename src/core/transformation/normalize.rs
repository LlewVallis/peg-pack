use crate::core::character::Character;
use crate::core::series::{Series, SeriesId};
use crate::core::{CompilerSettings, DebugSymbol, Instruction, InstructionId, Parser};
use crate::ordered_set::OrderedSet;
use std::collections::{HashMap, HashSet};
use std::mem;

type Pass = fn(&mut State, InstructionId, Instruction) -> Option<Instruction>;

macro_rules! pass {
    ($name:ident) => {
        |state, id, instruction| State::$name(state, id, instruction)
    };
}

macro_rules! passes {
    ($($name:ident),* $(,)?) => {
        &[$(pass!($name)),*]
    };
}

const STAGES: &[&[Pass]] = &[
    passes!(
        resolve_delegate,
        lower_to_first_choice,
        lower_to_first_choice_without_seq
    ),
    passes!(
        replace_by_character,
        eliminate_redundant_seqs,
        eliminate_redundant_choices,
        translate_unnecessary_non_first_choice,
        eliminate_double_not_aheads,
        concatenate_series,
        merge_series,
    ),
    passes!(
        normalize_seq_order,
        normalize_choice_order,
        normalize_first_choice_order
    ),
];

struct State<'a> {
    parser: &'a mut Parser,
    settings: CompilerSettings,
    queue: OrderedSet<InstructionId>,
    predecessors: HashMap<InstructionId, HashSet<InstructionId>>,
    characters: HashMap<InstructionId, Character>,
}

impl Parser {
    pub(super) fn normalize(&mut self, settings: CompilerSettings) {
        'normalize: loop {
            for stage in STAGES {
                if self.run_passes(settings, stage) {
                    continue 'normalize;
                }
            }

            return;
        }
    }

    fn run_passes(&mut self, settings: CompilerSettings, passes: &[Pass]) -> bool {
        let mut modified = false;

        let mut queue = self.walk().map(|(id, _)| id).collect::<OrderedSet<_>>();
        queue.reverse();

        let predecessors = self.compute_predecessors();
        let characters = self.characterize();

        let mut state = State {
            settings,
            queue,
            predecessors,
            characters,
            parser: self,
        };

        while let Some(id) = state.queue.pop() {
            let instruction = state.parser.instructions[id];

            if let Some(new_instruction) = state.normalize_instruction(id, instruction, passes) {
                if instruction != new_instruction {
                    for predecessor in &state.predecessors[&id] {
                        state.queue.push(*predecessor);
                    }

                    for old_successor in instruction.successors() {
                        state
                            .predecessors
                            .get_mut(&old_successor)
                            .unwrap()
                            .remove(&id);
                    }

                    for new_successor in instruction.successors() {
                        state
                            .predecessors
                            .get_mut(&new_successor)
                            .unwrap()
                            .insert(id);
                    }

                    state.characters = state.parser.patch_characters(state.characters, [id]);

                    state.queue.push(id);
                    state.parser.instructions[id] = new_instruction;
                    modified = true;
                }
            }
        }

        self.trim();
        modified
    }
}

impl<'a> State<'a> {
    pub fn insert(
        &mut self,
        instruction: Instruction,
        debug_symbol: DebugSymbol,
        predecessors: impl IntoIterator<Item = InstructionId>,
    ) -> InstructionId {
        let id = self.parser.insert(instruction, debug_symbol);

        self.queue.push(id);

        let predecessors = HashSet::from_iter(predecessors);
        assert!(self.predecessors.insert(id, predecessors).is_none());

        for successor in instruction.successors() {
            self.predecessors.get_mut(&successor).unwrap().insert(id);
        }

        let characters = mem::replace(&mut self.characters, HashMap::new());

        self.characters = self.parser.patch_characters(characters, [id]);

        id
    }

    fn normalize_instruction(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
        passes: &[Pass],
    ) -> Option<Instruction> {
        for pass in passes {
            if let Some(instruction) = pass(self, id, instruction) {
                return Some(instruction);
            }
        }

        None
    }

    fn resolve_delegate(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (_, target) = self.as_delegate(instruction)?;
        Some(target)
    }

    fn lower_to_first_choice(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (first_id, _, _, second) = self.as_choice_like(instruction)?;
        let (_, second_first, second_second_id, _) = self.as_seq(second)?;
        let (second_first_target_id, _) = self.as_not_ahead(second_first)?;

        if first_id == second_first_target_id {
            return Some(Instruction::FirstChoice(first_id, second_second_id));
        }

        None
    }

    fn lower_to_first_choice_without_seq(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (first_id, _, _, second) = self.as_choice_like(instruction)?;
        let (second_target_id, _) = self.as_not_ahead(second)?;

        if first_id == second_target_id {
            let empty_series_id = self.parser.series.insert(Series::empty());
            let debug_symbol = self.parser.debug_symbols[&id].clone();
            let empty_id = self.insert(Instruction::Series(empty_series_id), debug_symbol, [id]);

            return Some(Instruction::FirstChoice(first_id, empty_id));
        }

        None
    }

    fn concatenate_series(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        if !self.settings.merge_series {
            return None;
        }

        let (_, first, _, second) = self.as_seq(instruction)?;
        let (_, first) = self.as_series(first)?;
        let (_, second) = self.as_series(second)?;

        let new_series = Series::concatenate(first, second);
        let new_series_id = self.parser.series.insert(new_series);
        Some(Instruction::Series(new_series_id))
    }

    fn merge_series(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        if !self.settings.merge_series {
            return None;
        }

        let (_, first, _, second) = self.as_choice_like(instruction)?;
        let (_, first) = self.as_series(first)?;
        let (_, second) = self.as_series(second)?;

        let new_series = Series::merge(first, second)?;
        let new_series_id = self.parser.series.insert(new_series);
        Some(Instruction::Series(new_series_id))
    }

    fn replace_by_character(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        if !self.settings.character_replacement {
            return None;
        }

        if let Instruction::Series(_) = instruction {
            return None;
        }

        let character = self.characters[&id];

        let series = if !character.fallible
            && !character.antitransparent
            && !character.error_prone
            && !character.label_prone
        {
            Series::empty()
        } else if !character.possible() {
            Series::never()
        } else {
            return None;
        };

        let series_id = self.parser.series.insert(series);
        Some(Instruction::Series(series_id))
    }

    fn eliminate_redundant_seqs(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        if !self.settings.redundant_junction_elimination {
            return None;
        }

        let (left_id, left, right_id, right) = self.as_seq(instruction)?;

        let left_char = self.characters[&left_id];
        let right_char = self.characters[&right_id];

        if !left_char.fallible
            && !left_char.antitransparent
            && !left_char.error_prone
            && !left_char.label_prone
        {
            return Some(right);
        }

        if !right_char.fallible
            && !right_char.antitransparent
            && !right_char.error_prone
            && !right_char.label_prone
        {
            return Some(left);
        }

        None
    }

    fn eliminate_redundant_choices(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        if !self.settings.redundant_junction_elimination {
            return None;
        }

        let (left_id, left, right_id, right) = self.as_choice_like(instruction)?;

        let left_char = self.characters[&left_id];
        let right_char = self.characters[&right_id];

        let right_reachable = match instruction {
            Instruction::Choice(_, _) => left_char.fallible || left_char.error_prone,
            Instruction::FirstChoice(_, _) => left_char.fallible,
            _ => unreachable!(),
        };

        if !right_reachable {
            return Some(left);
        }

        if !left_char.possible() {
            return Some(right);
        }

        if !right_char.possible() {
            return Some(left);
        }

        None
    }

    fn translate_unnecessary_non_first_choice(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (left_id, _, right_id, _) = self.as_choice(instruction)?;

        if !self.characters[&left_id].error_prone {
            return Some(Instruction::FirstChoice(left_id, right_id));
        }

        None
    }

    fn eliminate_double_not_aheads(
        &mut self,
        _id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (_, target) = self.as_not_ahead(instruction)?;
        let (second_target_id, second_target) = self.as_not_ahead(target)?;

        let character = self.characters[&second_target_id];

        if !character.antitransparent && !character.label_prone && !character.error_prone {
            Some(second_target)
        } else {
            None
        }
    }

    fn normalize_seq_order(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (old_junction, old_junction_instruction, third, _) = self.as_seq(instruction)?;
        let (first, first_instruction, second, second_instruction) =
            self.as_seq(old_junction_instruction)?;

        // Could result in exponential instruction blowup
        if old_junction == third {
            return None;
        }

        if let Instruction::Seq(_, _) = first_instruction {
            return None;
        }

        if let Instruction::Seq(_, _) = second_instruction {
            return None;
        }

        let debug_symbol = self.parser.debug_symbols[&id].clone();
        let new_junction = self.insert(Instruction::Seq(second, third), debug_symbol, [id]);

        Some(Instruction::Seq(first, new_junction))
    }

    fn normalize_choice_order(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (_, old_junction_instruction, third, _) = self.as_choice(instruction)?;
        let (first, first_instruction, second, second_instruction) =
            self.as_choice(old_junction_instruction)?;

        if let Instruction::Choice(_, _) = first_instruction {
            return None;
        }

        if let Instruction::Choice(_, _) = second_instruction {
            return None;
        }

        let debug_symbol = self.parser.debug_symbols[&id].clone();
        let new_junction = self.insert(Instruction::Choice(second, third), debug_symbol, [id]);

        Some(Instruction::Choice(first, new_junction))
    }

    fn normalize_first_choice_order(
        &mut self,
        id: InstructionId,
        instruction: Instruction,
    ) -> Option<Instruction> {
        let (_, old_junction_instruction, third, _) = self.as_first_choice(instruction)?;
        let (first, first_instruction, second, second_instruction) =
            self.as_first_choice(old_junction_instruction)?;

        if let Instruction::FirstChoice(_, _) = first_instruction {
            return None;
        }

        if let Instruction::FirstChoice(_, _) = second_instruction {
            return None;
        }

        let debug_symbol = self.parser.debug_symbols[&id].clone();
        let new_junction = self.insert(Instruction::FirstChoice(second, third), debug_symbol, [id]);

        Some(Instruction::FirstChoice(first, new_junction))
    }

    fn as_seq(
        &self,
        instruction: Instruction,
    ) -> Option<(InstructionId, Instruction, InstructionId, Instruction)> {
        match instruction {
            Instruction::Seq(first, second) => Some((
                first,
                self.parser.instructions[first],
                second,
                self.parser.instructions[second],
            )),
            _ => None,
        }
    }

    fn as_choice(
        &self,
        instruction: Instruction,
    ) -> Option<(InstructionId, Instruction, InstructionId, Instruction)> {
        match instruction {
            Instruction::Choice(first, second) => Some((
                first,
                self.parser.instructions[first],
                second,
                self.parser.instructions[second],
            )),
            _ => None,
        }
    }

    fn as_first_choice(
        &self,
        instruction: Instruction,
    ) -> Option<(InstructionId, Instruction, InstructionId, Instruction)> {
        match instruction {
            Instruction::FirstChoice(first, second) => Some((
                first,
                self.parser.instructions[first],
                second,
                self.parser.instructions[second],
            )),
            _ => None,
        }
    }

    fn as_choice_like(
        &self,
        instruction: Instruction,
    ) -> Option<(InstructionId, Instruction, InstructionId, Instruction)> {
        match instruction {
            Instruction::Choice(first, second) | Instruction::FirstChoice(first, second) => Some((
                first,
                self.parser.instructions[first],
                second,
                self.parser.instructions[second],
            )),
            _ => None,
        }
    }

    fn as_series(&self, instruction: Instruction) -> Option<(SeriesId, &Series)> {
        match instruction {
            Instruction::Series(id) => Some((id, &self.parser.series[id])),
            _ => None,
        }
    }

    fn as_not_ahead(&self, instruction: Instruction) -> Option<(InstructionId, Instruction)> {
        match instruction {
            Instruction::NotAhead(target) => Some((target, self.parser.instructions[target])),
            _ => None,
        }
    }

    fn as_delegate(&self, instruction: Instruction) -> Option<(InstructionId, Instruction)> {
        match instruction {
            Instruction::Delegate(target) => Some((target, self.parser.instructions[target])),
            _ => None,
        }
    }
}
