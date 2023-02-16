use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::core::character::Character;
use crate::core::series::Series;
use crate::core::{Instruction, InstructionId, Parser};
use crate::ordered_set::OrderedSet;

#[derive(Eq, PartialEq, Clone)]
struct State {
    implications: Rc<HashMap<InstructionId, Implications>>,
    does_match: Rc<HashSet<InstructionId>>,
    doesnt_match: Rc<HashSet<InstructionId>>,
}

impl State {
    pub fn empty(implications: Rc<HashMap<InstructionId, Implications>>) -> Self {
        Self {
            implications,
            does_match: Rc::new(HashSet::new()),
            doesnt_match: Rc::new(HashSet::new()),
        }
    }

    pub fn does(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.does_match).insert(id);
    }

    pub fn doesnt(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.doesnt_match).insert(id);
    }

    pub fn mandates(&self, id: InstructionId) -> bool {
        for id in self.implications[&id].match_implies_match.iter() {
            if self.does_match.contains(id) {
                return true;
            }
        }

        for id in self.implications[&id].fail_implies_match.iter() {
            if self.doesnt_match.contains(id) {
                return true;
            }
        }

        false
    }

    pub fn forbids(&self, id: InstructionId) -> bool {
        for id in self.implications[&id].match_implies_fail.iter() {
            if self.does_match.contains(id) {
                return true;
            }
        }

        for id in self.implications[&id].fail_implies_fail.iter() {
            if self.doesnt_match.contains(id) {
                return true;
            }
        }

        false
    }

    pub fn intersection(first: &State, second: &State) -> Self {
        assert!(Rc::ptr_eq(&first.implications, &second.implications));

        let does_match = HashSet::intersection(&first.does_match, &second.does_match)
            .copied()
            .collect();

        let doesnt_match = HashSet::intersection(&first.doesnt_match, &second.doesnt_match)
            .copied()
            .collect();

        Self {
            implications: first.implications.clone(),
            does_match: Rc::new(does_match),
            doesnt_match: Rc::new(doesnt_match),
        }
    }

    pub fn union(first: &State, second: &State) -> Self {
        assert!(Rc::ptr_eq(&first.implications, &second.implications));

        let does_match = HashSet::union(&first.does_match, &second.does_match)
            .copied()
            .collect();

        let doesnt_match = HashSet::union(&first.doesnt_match, &second.doesnt_match)
            .copied()
            .collect();

        Self {
            implications: first.implications.clone(),
            does_match: Rc::new(does_match),
            doesnt_match: Rc::new(doesnt_match),
        }
    }
}

struct Preconditions {
    base: State,
    contributors: HashMap<InstructionId, State>,
}

impl Preconditions {
    pub fn new(base: State) -> Self {
        Self {
            base,
            contributors: HashMap::new(),
        }
    }

    pub fn update(&mut self, id: InstructionId, state: State) -> bool {
        if self.contributors.get(&id) == Some(&state) {
            false
        } else {
            self.contributors.insert(id, state);
            true
        }
    }

    pub fn state(&self) -> State {
        if self.contributors.is_empty() {
            return self.base.clone();
        }

        let mut contributors = self.contributors.values();
        let mut state = contributors.next().unwrap().clone();

        for contribution in contributors {
            state = State::intersection(&state, contribution);
        }

        state
    }
}

#[derive(Eq, PartialEq, Clone)]
struct Postconditions {
    positive: State,
    negative: State,
}

struct Stack {
    propagate: OrderedSet<InstructionId>,
    resolve: OrderedSet<InstructionId>,
}

impl Stack {
    pub fn new(id: InstructionId) -> Self {
        let mut result = Self {
            propagate: OrderedSet::new(),
            resolve: OrderedSet::new(),
        };

        result.add(id);

        result
    }

    pub fn add(&mut self, id: InstructionId) {
        self.propagate.push(id);
        self.resolve.push(id);
    }
}

struct PropagateContext<'a> {
    id: InstructionId,
    preconditions: &'a mut HashMap<InstructionId, Preconditions>,
    postconditions: &'a HashMap<InstructionId, Postconditions>,
    characters: &'a HashMap<InstructionId, Character>,
    stack: &'a mut Stack,
}

impl<'a> PropagateContext<'a> {
    pub fn update(&mut self, id: InstructionId, state: State) {
        let changed = self
            .preconditions
            .get_mut(&id)
            .unwrap()
            .update(self.id, state);

        if changed {
            self.stack.add(id);
        }
    }

    pub fn postconditions(&self, id: InstructionId) -> &Postconditions {
        &self.postconditions[&id]
    }

    pub fn character(&self, id: InstructionId) -> &Character {
        &self.characters[&id]
    }
}

struct ResolveContext<'a> {
    base: &'a State,
    total: &'a State,
    postconditions: &'a HashMap<InstructionId, Postconditions>,
    characters: &'a HashMap<InstructionId, Character>,
}

impl<'a> ResolveContext<'a> {
    pub fn postconditions(&self, id: InstructionId) -> &Postconditions {
        &self.postconditions[&id]
    }

    pub fn character(&self, id: InstructionId) -> &Character {
        &self.characters[&id]
    }

    fn base(&self) -> State {
        self.base.clone()
    }

    fn total(&self) -> State {
        self.total.clone()
    }
}

#[derive(Eq, PartialEq, Clone, Default)]
struct Implications {
    match_implies_match: Rc<HashSet<InstructionId>>,
    fail_implies_match: Rc<HashSet<InstructionId>>,
    match_implies_fail: Rc<HashSet<InstructionId>>,
    fail_implies_fail: Rc<HashSet<InstructionId>>,
}

impl Implications {
    pub fn merge_match_implies_match(&mut self, other: &Implications) {
        Rc::make_mut(&mut self.match_implies_match)
            .extend(other.match_implies_match.iter().copied());
        Rc::make_mut(&mut self.fail_implies_match).extend(other.fail_implies_match.iter().copied());
    }

    pub fn merge_fail_implies_match(&mut self, other: &Implications) {
        Rc::make_mut(&mut self.match_implies_match)
            .extend(other.match_implies_fail.iter().copied());
        Rc::make_mut(&mut self.fail_implies_match).extend(other.fail_implies_fail.iter().copied());
    }

    pub fn merge_match_implies_fail(&mut self, other: &Implications) {
        Rc::make_mut(&mut self.match_implies_fail)
            .extend(other.match_implies_match.iter().copied());
        Rc::make_mut(&mut self.fail_implies_fail).extend(other.fail_implies_match.iter().copied());
    }

    pub fn merge_fail_implies_fail(&mut self, other: &Implications) {
        Rc::make_mut(&mut self.fail_implies_fail).extend(other.fail_implies_fail.iter().copied());
        Rc::make_mut(&mut self.match_implies_fail).extend(other.match_implies_fail.iter().copied());
    }

    pub fn match_implies_match(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.match_implies_match).insert(id);
    }

    pub fn fail_implies_match(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.fail_implies_match).insert(id);
    }

    pub fn match_implies_fail(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.match_implies_fail).insert(id);
    }

    pub fn fail_implies_fail(&mut self, id: InstructionId) {
        Rc::make_mut(&mut self.fail_implies_fail).insert(id);
    }

    pub fn referents(&self) -> impl Iterator<Item=InstructionId> + '_ {
        Iterator::chain(
            Iterator::chain(
                self.match_implies_match.iter(),
                self.fail_implies_match.iter(),
            ),
            Iterator::chain(
                self.match_implies_fail.iter(),
                self.fail_implies_fail.iter(),
            ),
        )
            .copied()
    }
}

impl Parser {
    pub fn state_optimize(&mut self) {
        let characters = self.characterize();
        let (all_preconditions, all_postconditions) = self.analyze_states(&characters);

        let empty = Instruction::Series(self.insert_series(Series::empty()));
        let never = Instruction::Series(self.insert_series(Series::never()));

        for (id, instruction) in self.instructions.iter_mut() {
            Self::optimize_instruction(
                id,
                instruction,
                &characters,
                &all_preconditions,
                &all_postconditions,
                empty,
                never,
            );
        }
    }

    fn optimize_instruction(
        id: InstructionId,
        instruction: &mut Instruction,
        characters: &HashMap<InstructionId, Character>,
        all_preconditions: &HashMap<InstructionId, State>,
        all_postconditions: &HashMap<InstructionId, Postconditions>,
        empty: Instruction,
        never: Instruction,
    ) {
        let character = characters[&id];
        let effect_free =
            !character.antitransparent && !character.label_prone && !character.error_prone;
        let preconditions = &all_preconditions[&id];

        if preconditions.mandates(id) && effect_free {
            *instruction = empty;
        }

        if preconditions.forbids(id) {
            *instruction = never;
        }

        if let Instruction::Seq(first, second) = *instruction {
            let first_character = characters[&first];
            let second_character = characters[&second];

            let middle_state = State::union(
                &all_preconditions[&second],
                &all_postconditions[&first].positive,
            );

            if preconditions.mandates(first) && !first_character.antitransparent &&
                !first_character.label_prone && !first_character.error_prone {
                *instruction = Instruction::Delegate(second);
            }

            if middle_state.mandates(second) && !second_character.antitransparent &&
                !second_character.label_prone && !second_character.error_prone {
                *instruction = Instruction::Delegate(first);
            }

            if middle_state.forbids(second) {
                *instruction = never;
            }
        }

        if let Instruction::FirstChoice(first, second) = *instruction {
            let first_character = characters[&first];

            if preconditions.mandates(first) && !first_character.error_prone {
                *instruction = Instruction::Delegate(first);
            }

            if preconditions.forbids(first) {
                *instruction = Instruction::Delegate(second);
            }

            if preconditions.forbids(second) {
                *instruction = Instruction::Delegate(first);
            }
        }

        if let Instruction::Choice(first, second) = *instruction {
            if preconditions.mandates(first) {
                *instruction = Instruction::Delegate(first);
            }

            if preconditions.forbids(first) {
                *instruction = Instruction::Delegate(second);
            }

            if preconditions.forbids(second) {
                *instruction = Instruction::Delegate(first);
            }
        }
    }

    fn compute_implications(
        &self,
        characters: &HashMap<InstructionId, Character>,
    ) -> HashMap<InstructionId, Implications> {
        let mut map = HashMap::<_, Implications>::new();

        for (id, instruction) in self.instructions() {
            let implications = map.entry(id).or_default();

            implications.match_implies_match(id);
            implications.fail_implies_fail(id);

            match instruction {
                Instruction::Seq(first, second) => {
                    implications.fail_implies_fail(first);

                    if !characters[&second].fallible {
                        implications.match_implies_match(first);
                    }

                    map.entry(first).or_default().match_implies_match(id);

                    if !characters[&first].antitransparent {
                        map.entry(second).or_default().match_implies_match(id);
                    }
                }
                Instruction::Choice(first, second) | Instruction::FirstChoice(first, second) => {
                    implications.match_implies_match(first);
                    implications.match_implies_match(second);
                }
                Instruction::NotAhead(target) => {
                    implications.fail_implies_match(target);
                    implications.match_implies_fail(target);

                    map.entry(target).or_default().match_implies_fail(id);
                    map.entry(target).or_default().fail_implies_match(id);
                }
                Instruction::Error(target, _)
                | Instruction::Label(target, _)
                | Instruction::Cache(target, _)
                | Instruction::Delegate(target) => {
                    implications.match_implies_match(target);
                    implications.fail_implies_fail(target);
                    map.entry(target).or_default().match_implies_match(id);
                    map.entry(target).or_default().fail_implies_fail(id);
                }
                Instruction::Series(_) => {}
            }
        }

        self.implication_transitive_closure(&mut map);
        map
    }

    fn implication_transitive_closure(&self, map: &mut HashMap<InstructionId, Implications>) {
        let mut dependents = HashMap::new();

        for id in map.keys() {
            dependents.insert(*id, HashSet::new());
        }

        let mut queue = OrderedSet::new();
        queue.extend(map.keys().copied());

        while let Some(id) = queue.pop() {
            let implications = &map[&id];
            let mut new_implications = implications.clone();

            for other in implications.referents() {
                dependents.get_mut(&other).unwrap().insert(id);
            }

            for other in implications.match_implies_match.iter() {
                new_implications.merge_match_implies_match(&map[other]);
            }

            for other in implications.fail_implies_match.iter() {
                new_implications.merge_fail_implies_match(&map[other]);
            }

            for other in implications.match_implies_fail.iter() {
                new_implications.merge_match_implies_fail(&map[other]);
            }

            for other in implications.fail_implies_fail.iter() {
                new_implications.merge_fail_implies_fail(&map[other]);
            }

            if *implications != new_implications {
                map.insert(id, new_implications);
                queue.extend(dependents[&id].iter().copied());
            }
        }
    }

    fn analyze_states(
        &self,
        characters: &HashMap<InstructionId, Character>,
    ) -> (
        HashMap<InstructionId, State>,
        HashMap<InstructionId, Postconditions>,
    ) {
        let predecessors = self.compute_predecessors();
        let implications = Rc::new(self.compute_implications(&characters));

        let mut stack = Stack::new(self.start);

        let base = self.derive_base(&characters, implications.clone());
        let total = self.derive_total(implications.clone());

        let mut preconditions = HashMap::new();
        let mut postconditions = HashMap::new();

        for (id, _) in self.instructions() {
            preconditions.insert(id, Preconditions::new(base.clone()));
            postconditions.insert(
                id,
                Postconditions {
                    positive: base.clone(),
                    negative: base.clone(),
                },
            );
        }

        while !stack.resolve.is_empty() {
            while !stack.propagate.is_empty() {
                self.propagate_next(&mut stack, &mut preconditions, &postconditions, &characters);
            }

            self.resolve_next(
                &mut stack,
                &preconditions,
                &mut postconditions,
                &predecessors,
                &characters,
                &base,
                &total,
            );
        }

        let preconditions = preconditions
            .into_iter()
            .map(|(k, v)| (k, v.state()))
            .collect();

        (preconditions, postconditions)
    }

    fn resolve_next(
        &self,
        stack: &mut Stack,
        preconditions: &HashMap<InstructionId, Preconditions>,
        postconditions: &mut HashMap<InstructionId, Postconditions>,
        predecessors: &HashMap<InstructionId, HashSet<InstructionId>>,
        characters: &HashMap<InstructionId, Character>,
        base: &State,
        total: &State,
    ) {
        let id = stack.resolve.pop().unwrap();

        let instruction = self.instructions[id];
        let instruction_preconditions = preconditions[&id].state();

        let mut new_postconditions = self.resolve(
            instruction,
            &instruction_preconditions,
            ResolveContext {
                base: &base,
                total: &total,
                postconditions: &postconditions,
                characters: &characters,
            },
        );

        self.modify_postconditions(
            id,
            &characters[&id],
            &instruction_preconditions,
            &total,
            &mut new_postconditions,
        );

        if new_postconditions != postconditions[&id] {
            postconditions.insert(id, new_postconditions);

            for predecessor in &predecessors[&id] {
                stack.add(*predecessor);
            }
        }
    }

    fn propagate_next(
        &self,
        stack: &mut Stack,
        preconditions: &mut HashMap<InstructionId, Preconditions>,
        postconditions: &HashMap<InstructionId, Postconditions>,
        characters: &HashMap<InstructionId, Character>,
    ) {
        let id = stack.propagate.pop().unwrap();

        let instruction = self.instructions[id];
        let instruction_preconditions = preconditions[&id].state();

        self.propagate(
            id,
            instruction,
            &instruction_preconditions,
            PropagateContext {
                id,
                preconditions,
                postconditions,
                stack,
                characters: &characters,
            },
        );
    }

    fn derive_base(
        &self,
        characters: &HashMap<InstructionId, Character>,
        implications: Rc<HashMap<InstructionId, Implications>>,
    ) -> State {
        let mut state = State::empty(implications);

        for (id, _) in self.instructions() {
            let character = characters[&id];

            if !character.fallible {
                state.does(id);
            }

            if !character.transparent && !character.antitransparent {
                state.doesnt(id);
            }
        }

        state
    }

    fn derive_total(&self, implications: Rc<HashMap<InstructionId, Implications>>) -> State {
        let mut state = State::empty(implications);

        for (id, _) in self.instructions() {
            state.does(id);
            state.doesnt(id);
        }

        state
    }

    fn propagate(
        &self,
        id: InstructionId,
        instruction: Instruction,
        preconditions: &State,
        mut ctx: PropagateContext,
    ) {
        match instruction {
            Instruction::Seq(first, second) => {
                ctx.update(first, preconditions.clone());

                let first_postconditions = ctx.postconditions(first);
                let first_character = ctx.character(first);

                let mut second_preconditions = if !first_character.antitransparent {
                    State::union(preconditions, &first_postconditions.positive)
                } else {
                    first_postconditions.positive.clone()
                };

                if preconditions.mandates(id) {
                    second_preconditions.does(second);
                }

                ctx.update(second, second_preconditions);
            }
            Instruction::Choice(first, second) => {
                ctx.update(first, preconditions.clone());
                ctx.update(second, preconditions.clone());
            }
            Instruction::FirstChoice(first, second) => {
                ctx.update(first, preconditions.clone());
                let first_postconditions = ctx.postconditions(first);
                ctx.update(second, first_postconditions.negative.clone());
            }
            Instruction::NotAhead(target)
            | Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Cache(target, _)
            | Instruction::Delegate(target) => {
                ctx.update(target, preconditions.clone());
            }
            Instruction::Series(_) => {}
        }
    }

    fn resolve(
        &self,
        instruction: Instruction,
        preconditions: &State,
        ctx: ResolveContext,
    ) -> Postconditions {
        match instruction {
            Instruction::Seq(first, second) => self.resolve_seq(first, second, preconditions, ctx),
            Instruction::Choice(first, second) | Instruction::FirstChoice(first, second) => {
                self.resolve_choice(first, second, preconditions, ctx)
            }
            Instruction::NotAhead(target) => self.resolve_not_ahead(target, preconditions, ctx),
            Instruction::Error(target, _)
            | Instruction::Label(target, _)
            | Instruction::Cache(target, _)
            | Instruction::Delegate(target) => {
                self.resolve_delegate_like(target, preconditions, ctx)
            }
            Instruction::Series(_) => Postconditions {
                positive: ctx.base(),
                negative: ctx.base(),
            },
        }
    }

    fn resolve_seq(
        &self,
        first: InstructionId,
        second: InstructionId,
        preconditions: &State,
        ctx: ResolveContext,
    ) -> Postconditions {
        let first_character = ctx.character(first);
        let second_character = ctx.character(second);
        let first_postconditions = ctx.postconditions(first);
        let second_postconditions = ctx.postconditions(second);

        let mut positive = second_postconditions.positive.clone();

        if !second_character.antitransparent {
            positive = State::union(&positive, &first_postconditions.positive);
        }

        if preconditions.forbids(first) || preconditions.forbids(second) {
            positive = ctx.total();
        }

        let mut negative = ctx.base();

        if preconditions.mandates(first) && !first_character.antitransparent {
            negative = State::union(&negative, &second_postconditions.negative);
        }

        if (preconditions.mandates(second) && !first_character.antitransparent)
            || first_postconditions.positive.mandates(second)
        {
            negative = State::union(&negative, &first_postconditions.negative);
        }

        Postconditions { positive, negative }
    }

    fn resolve_choice(
        &self,
        first: InstructionId,
        second: InstructionId,
        preconditions: &State,
        ctx: ResolveContext,
    ) -> Postconditions {
        let first_character = ctx.character(first);
        let second_character = ctx.character(second);
        let first_postconditions = ctx.postconditions(first);
        let second_postconditions = ctx.postconditions(second);

        let mut positive = State::intersection(
            &first_postconditions.positive,
            &second_postconditions.positive,
        );

        if preconditions.forbids(first) {
            positive = State::union(&positive, &second_postconditions.positive);

            if !second_character.antitransparent {
                positive = State::union(&positive, preconditions);
            }
        }

        if preconditions.forbids(second) {
            positive = State::union(&positive, &first_postconditions.positive);

            if !first_character.antitransparent {
                positive = State::union(&positive, preconditions);
            }
        }

        if preconditions.forbids(first) && preconditions.forbids(second) {
            positive = ctx.total();
        }

        let mut negative = State::union(
            &first_postconditions.negative,
            &second_postconditions.negative,
        );

        if preconditions.mandates(first) || preconditions.mandates(second) {
            negative = ctx.total();
        }

        Postconditions { positive, negative }
    }

    fn resolve_not_ahead(
        &self,
        target: InstructionId,
        preconditions: &State,
        ctx: ResolveContext,
    ) -> Postconditions {
        let mut positive = ctx.base();
        positive.doesnt(target);

        if preconditions.mandates(target) {
            positive = ctx.total();
        }

        let mut negative = ctx.base();
        negative.does(target);

        if preconditions.forbids(target) {
            negative = ctx.total();
        }

        Postconditions { positive, negative }
    }

    fn resolve_delegate_like(
        &self,
        target: InstructionId,
        _preconditions: &State,
        ctx: ResolveContext,
    ) -> Postconditions {
        ctx.postconditions(target).clone()
    }

    fn modify_postconditions(
        &self,
        id: InstructionId,
        character: &Character,
        preconditions: &State,
        total: &State,
        postconditions: &mut Postconditions,
    ) {
        if !character.antitransparent {
            postconditions.positive.does(id);
            postconditions.positive = State::union(&postconditions.positive, preconditions);
        }

        if !character.fallible {
            postconditions.negative = total.clone();
        }

        if !character.transparent && !character.antitransparent {
            postconditions.positive = total.clone();
        }

        postconditions.negative = State::union(&postconditions.negative, preconditions);
        postconditions.negative.doesnt(id);
    }
}
