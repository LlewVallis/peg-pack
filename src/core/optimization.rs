use seahash::SeaHasher;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hasher;

use crate::core::structure::{Component, ComponentId, Components};
use crate::core::InstructionId;
use crate::core::{Instruction, Parser};

impl Parser {
    /// Optimize the parser, cannot be run on an ill-formed grammar
    pub(super) fn optimize(&mut self) {
        self.trim();
        self.sort();
        self.deduplicate_classes();
        self.deduplicate_labels();
        self.remove_delegates();
        self.deduplicate();
        self.sort();
    }

    /// Remove all unreachable instructions and classes
    fn trim(&mut self) {
        self.trim_instructions();
        self.trim_classes();
    }

    fn trim_instructions(&mut self) {
        let mut reachable = HashSet::new();

        let mut queue = vec![self.start];
        while let Some(id) = queue.pop() {
            if reachable.insert(id) {
                let instruction = self.instructions[id];
                queue.extend(instruction.successors());
            }
        }

        let removals = self
            .instructions()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            self.instructions.remove(removal);
        }
    }

    fn trim_classes(&mut self) {
        let mut reachable = HashSet::new();

        for (_, instruction) in self.instructions() {
            if let Instruction::Class(class) = instruction {
                reachable.insert(class);
            }
        }

        let removals = self
            .classes()
            .map(|(k, _)| k)
            .filter(|id| !reachable.contains(id))
            .collect::<Vec<_>>();

        for removal in removals {
            self.classes.remove(removal);
        }
    }

    /// Sort the instructions in the map by a depth first search. This is not actually necessary,
    /// but makes the visualizations nicer
    fn sort(&mut self) {
        let mut mappings = HashMap::new();
        self.sort_visit(self.start, &mut mappings);
        self.relabel(|id| mappings[&id]);
    }

    fn sort_visit(&self, id: InstructionId, mappings: &mut HashMap<InstructionId, InstructionId>) {
        if mappings.contains_key(&id) {
            return;
        }

        mappings.insert(id, InstructionId(mappings.len()));

        let instruction = self.instructions[id];
        for successor in instruction.successors() {
            self.sort_visit(successor, mappings);
        }
    }

    /// Elides all delegates in the graph
    fn remove_delegates(&mut self) {
        let mut mappings = HashMap::new();

        for (id, _) in self.instructions() {
            let resolved = self.resolve_delegates(id);

            if id != resolved {
                mappings.insert(id, self.resolve_delegates(id));
            }
        }

        self.remap(|id| Self::follow_mappings(id, &mappings));
        self.trim_instructions();
    }

    fn resolve_delegates(&self, id: InstructionId) -> InstructionId {
        match self.instructions[id] {
            Instruction::Delegate(target) => self.resolve_delegates(target),
            _ => id,
        }
    }

    /// Merge duplicate classes into one
    fn deduplicate_classes(&mut self) {
        let mut canonicals = HashMap::new();
        let mut mappings = HashMap::new();
        let mut removals = Vec::new();

        for (id, class) in self.classes() {
            if let Some(canonical_id) = canonicals.get(class) {
                mappings.insert(id, *canonical_id);
                removals.push(id);
            } else {
                canonicals.insert(class, id);
                mappings.insert(id, id);
            }
        }

        for (_, instruction) in self.instructions.iter_mut() {
            if let Instruction::Class(id) = instruction {
                *id = mappings[id];
            }
        }

        for removal in removals {
            self.classes.remove(removal);
        }
    }

    /// Merge duplicate labels into one
    fn deduplicate_labels(&mut self) {
        let mut canonicals = HashMap::new();
        let mut mappings = HashMap::new();
        let mut removals = Vec::new();

        for (id, label) in self.labels() {
            if let Some(canonical_id) = canonicals.get(label) {
                mappings.insert(id, *canonical_id);
                removals.push(id);
            } else {
                canonicals.insert(label, id);
                mappings.insert(id, id);
            }
        }

        for (_, instruction) in self.instructions.iter_mut() {
            if let Instruction::Label(_, id) = instruction {
                *id = mappings[id];
            }
        }

        for removal in removals {
            self.labels.remove(removal);
        }
    }

    /// Attempts to remove as much duplication in the graph as possible. This
    /// works by first reducing the graph into a DAG of strongly connected
    /// components, performing internal deduplication of those components, and
    /// then doing bottom up deduplication of the components themselves.
    ///
    /// In order to determine equality between two components, a high quality
    /// hash is used. This hash, however, depends on the starting instruction
    /// of the component
    fn deduplicate(&mut self) {
        let components = self.separate_components();

        let mut mappings = HashMap::new();
        let mut canonicals = HashMap::new();
        let mut visited = HashSet::new();

        self.deduplicate_component(
            self.start,
            &components,
            &mut mappings,
            &mut canonicals,
            &mut visited,
        );

        self.remap(|id| Self::follow_mappings(id, &mappings));
        self.trim_instructions();
    }

    /// Performs a depth first search of all components, remapping if a
    /// duplicate is found. If a component is encountered that is not a
    /// duplicate, it is added to the canonicals map
    fn deduplicate_component(
        &mut self,
        start: InstructionId,
        components: &Components,
        mappings: &mut HashMap<InstructionId, InstructionId>,
        canonicals: &mut HashMap<u64, InstructionId>,
        visited: &mut HashSet<ComponentId>,
    ) {
        let component_id = components.instruction_components[&start];

        if !visited.insert(component_id) {
            return;
        }

        let component = &components.components[component_id];

        for successor in &component.successors {
            self.deduplicate_component(*successor, components, mappings, canonicals, visited);
        }

        self.deduplicate_instructions(component.instructions.clone(), mappings);

        let component_hash = self.create_canonical_hash(start, component, mappings);

        if let Some(replacement) = canonicals.get(&component_hash) {
            let replacement_component_id = components.instruction_components[replacement];
            let replacement_component = &components.components[replacement_component_id];

            self.reassign_component(
                start,
                component,
                *replacement,
                replacement_component,
                mappings,
            );
        } else {
            canonicals.insert(component_hash, start);
        }
    }

    /// Remaps all the instructions in a component the the corresponding
    /// instructions in another component. The two components must be equal
    fn reassign_component(
        &self,
        source_root: InstructionId,
        source_component: &Component,
        dest_root: InstructionId,
        dest_component: &Component,
        mappings: &mut HashMap<InstructionId, InstructionId>,
    ) {
        let mut queue = vec![(
            Self::follow_mappings(source_root, mappings),
            Self::follow_mappings(dest_root, mappings),
        )];

        let mut visited = HashSet::new();
        let mut new_mappings = Vec::new();

        while let Some((source_id, dest_id)) = queue.pop() {
            let source_visited = !visited.insert(source_id);
            let dest_visited = !visited.insert(dest_id);
            assert_eq!(source_visited, dest_visited);

            if source_visited || dest_visited {
                continue;
            }

            let source = self.instructions[source_id];
            let dest = self.instructions[dest_id];

            let successors = source.successors().zip(dest.successors());
            for (source_successor, dest_successor) in successors {
                let source_successor = Self::follow_mappings(source_successor, mappings);
                let dest_successor = Self::follow_mappings(dest_successor, mappings);

                let source_internal = source_component.instructions.contains(&source_successor);
                let dest_internal = dest_component.instructions.contains(&dest_successor);
                assert_eq!(source_internal, dest_internal);

                if source_internal && dest_internal {
                    queue.push((source_successor, dest_successor));
                }
            }

            new_mappings.push((source_id, dest_id));
        }

        for mapping in new_mappings {
            mappings.insert(mapping.0, mapping.1);
        }
    }

    /// Reduces a component to a hash for deduplication purposes, these hashes
    /// must never collide for non-equal components
    fn create_canonical_hash(
        &self,
        start: InstructionId,
        component: &Component,
        mappings: &HashMap<InstructionId, InstructionId>,
    ) -> u64 {
        const BACKREFERENCE_HASH: &'static [u8] = &[0];
        const INSTRUCTION_HASH: &'static [u8] = &[1];
        const OUTREFERENCE_HASH: &'static [u8] = &[2];

        let mut hasher = SeaHasher::new();
        let mut backreferences = HashMap::new();

        let mut queue = vec![Self::follow_mappings(start, mappings)];

        while let Some(id) = queue.pop() {
            if let Some(internal) = backreferences.get(&id) {
                hasher.write(BACKREFERENCE_HASH);
                hasher.write_usize(*internal);
                continue;
            }

            backreferences.insert(id, backreferences.len());

            let instruction = self.instructions[id];
            hasher.write(INSTRUCTION_HASH);
            self.intrinsic_instruction_hash(instruction, &mut hasher);

            for successor in instruction.successors() {
                let successor = Self::follow_mappings(successor, mappings);

                if component.instructions.contains(&successor) {
                    queue.push(successor);
                } else {
                    hasher.write(OUTREFERENCE_HASH);
                    hasher.write_usize(successor.0);
                }
            }
        }

        hasher.finish()
    }

    fn intrinsic_instruction_hash(&self, instruction: Instruction, hasher: &mut impl Hasher) {
        match instruction {
            Instruction::Seq(_, _) => hasher.write_u8(0),
            Instruction::Choice(_, _) => hasher.write_usize(1),
            Instruction::NotAhead(_) => hasher.write_u8(2),
            Instruction::Error(_) => hasher.write_u8(3),
            Instruction::Label(_, label) => {
                hasher.write_u8(4);
                hasher.write_usize(label.0);
            }
            Instruction::Delegate(_) => hasher.write_u8(5),
            Instruction::Class(class) => {
                hasher.write_u8(6);
                hasher.write_usize(class.0)
            }
            Instruction::Empty => hasher.write_u8(7),
        }
    }

    /// Deduplicates instructions within a component. This works by a similar
    /// algorithm to component deduplication. Cycles are ignored when
    /// performing the depth first search
    fn deduplicate_instructions(
        &mut self,
        mut unvisited: BTreeSet<InstructionId>,
        mappings: &mut HashMap<InstructionId, InstructionId>,
    ) {
        let mut canonicals = HashMap::new();

        self.canonicalize_instruction(self.start, mappings, &mut canonicals, &mut unvisited);
    }

    fn canonicalize_instruction(
        &mut self,
        id: InstructionId,
        mappings: &mut HashMap<InstructionId, InstructionId>,
        canonicals: &mut HashMap<Instruction, InstructionId>,
        unvisited: &mut BTreeSet<InstructionId>,
    ) {
        if !unvisited.remove(&id) {
            return;
        }

        let instruction = self.instructions[id];
        for successor in instruction.successors() {
            self.canonicalize_instruction(successor, mappings, canonicals, unvisited);
        }

        let canonical = instruction.remapped(|id| Self::follow_mappings(id, mappings));

        if let Some(replacement) = canonicals.get(&canonical) {
            mappings.insert(id, *replacement);
        } else {
            canonicals.insert(canonical, id);
        }
    }

    /// Look up the mapped ID of an instruction, potentially following multiple
    /// mappings
    fn follow_mappings(
        mut id: InstructionId,
        mappings: &HashMap<InstructionId, InstructionId>,
    ) -> InstructionId {
        while let Some(new_id) = mappings.get(&id) {
            id = *new_id;
        }

        id
    }
}
