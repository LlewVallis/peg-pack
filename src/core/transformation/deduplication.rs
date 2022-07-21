use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use seahash::SeaHasher;

use crate::core::structure::{Component, ComponentId, Components};
use crate::core::{Instruction, InstructionId, Parser};
use crate::store::{Store, StoreKey};

impl Parser {
    pub(super) fn deduplicate(&mut self) {
        self.deduplicate_series();
        self.deduplicate_labels();
        self.deduplicate_expecteds();
        self.deduplicate_components();
        self.trim();
    }

    fn deduplicate_series(&mut self) {
        self.deduplicate_resource(
            |parser| &parser.series,
            |instruction, mappings| {
                if let Instruction::Series(id) = instruction {
                    *id = mappings[id];
                }
            },
        );
    }

    fn deduplicate_labels(&mut self) {
        self.deduplicate_resource(
            |parser| &parser.labels,
            |instruction, mappings| {
                if let Instruction::Label(_, id) = instruction {
                    *id = mappings[id];
                }
            },
        );
    }

    fn deduplicate_expecteds(&mut self) {
        self.deduplicate_resource(
            |parser| &parser.expecteds,
            |instruction, mappings| {
                if let Instruction::Error(_, id) = instruction {
                    *id = mappings[id];
                }
            },
        );
    }

    fn deduplicate_resource<K: StoreKey, V: Eq + Hash>(
        &mut self,
        resources: impl FnOnce(&Self) -> &Store<K, V>,
        fix: impl Fn(&mut Instruction, &HashMap<K, K>),
    ) {
        let mut canonicals = HashMap::new();
        let mut mappings = HashMap::new();
        let mut removals = Vec::new();

        for (id, resource) in resources(self).iter() {
            if let Some(canonical_id) = canonicals.get(resource) {
                mappings.insert(id, *canonical_id);
                removals.push(id);
            } else {
                canonicals.insert(resource, id);
                mappings.insert(id, id);
            }
        }

        for (_, instruction) in self.instructions.iter_mut() {
            fix(instruction, &mappings);
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
    fn deduplicate_components(&mut self) {
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
            for start in &component.instructions {
                let hash = self.create_canonical_hash(*start, component, mappings);
                canonicals.insert(hash, *start);
            }
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
        const BACKREFERENCE_HASH: &[u8] = &[0];
        const INSTRUCTION_HASH: &[u8] = &[1];
        const OUTREFERENCE_HASH: &[u8] = &[2];

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
            Instruction::Error(_, expected) => {
                hasher.write_u8(3);
                hasher.write_usize(expected.0);
            }
            Instruction::Label(_, label) => {
                hasher.write_u8(4);
                hasher.write_usize(label.0);
            }
            Instruction::Delegate(_) => hasher.write_u8(5),
            Instruction::Series(series) => {
                hasher.write_u8(6);
                hasher.write_usize(series.0)
            }
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
}
