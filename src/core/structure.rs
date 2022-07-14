use std::collections::{BTreeSet, HashMap, HashSet};

use crate::core::InstructionId;
use crate::core::Parser;
use crate::store::{Store, StoreKey};

impl Parser {
    /// Computes the predecessors of each instruction
    pub(super) fn compute_predecessors(&self) -> HashMap<InstructionId, HashSet<InstructionId>> {
        let mut results = HashMap::new();

        for (id, instruction) in self.instructions() {
            results.entry(id).or_insert(HashSet::new());

            for successor in instruction.successors() {
                results
                    .entry(successor)
                    .or_insert(HashSet::new())
                    .insert(id);
            }
        }

        results
    }

    /// Identifies the strongly connected components in the instruction graph
    pub(super) fn separate_components(&self) -> Components {
        let roots = self.kosaraju();

        let mut root_component_ids = HashMap::new();
        let mut components = Components::new();

        for (id, root) in roots {
            let component_id = *root_component_ids
                .entry(root)
                .or_insert_with(|| components.components.insert(Component::new()));

            components.instruction_components.insert(id, component_id);
            components.components.set(component_id, Component::new());
        }

        for (instruction_id, component_id) in &components.instruction_components {
            let component = &mut components.components[*component_id];
            component.instructions.insert(*instruction_id);

            let instruction = self.instructions[*instruction_id];
            for successor in instruction.successors() {
                let successor_component_id = components.instruction_components[&successor];
                if successor_component_id != *component_id {
                    component.successors.insert(successor);
                }
            }
        }

        components
    }

    fn kosaraju(&self) -> HashMap<InstructionId, InstructionId> {
        let mut visited = HashSet::new();
        let mut queue = Vec::new();

        for (id, _) in self.instructions() {
            self.kosaraju_visit(id, &mut visited, &mut queue);
        }

        let predecessors = self.compute_predecessors();
        let mut roots = HashMap::new();

        for id in queue.into_iter().rev() {
            self.kosaraju_assign(id, id, &predecessors, &mut roots);
        }

        roots
    }

    fn kosaraju_visit(
        &self,
        id: InstructionId,
        visited: &mut HashSet<InstructionId>,
        queue: &mut Vec<InstructionId>,
    ) {
        if visited.insert(id) {
            let instruction = self.instructions[id];

            for successor in instruction.successors() {
                self.kosaraju_visit(successor, visited, queue);
            }

            queue.push(id);
        }
    }

    fn kosaraju_assign(
        &self,
        id: InstructionId,
        root: InstructionId,
        predecessors: &HashMap<InstructionId, HashSet<InstructionId>>,
        roots: &mut HashMap<InstructionId, InstructionId>,
    ) {
        if !roots.contains_key(&id) {
            roots.insert(id, root);

            for predecessor in &predecessors[&id] {
                self.kosaraju_assign(*predecessor, root, predecessors, roots);
            }
        }
    }
}

/// A list of strongly connected components in the instruction graph
pub struct Components {
    /// A map of each instruction to it's component's ID
    pub instruction_components: HashMap<InstructionId, ComponentId>,
    /// The set of strongly connected components
    pub components: Store<ComponentId, Component>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            instruction_components: HashMap::new(),
            components: Store::new(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ComponentId(usize);

impl StoreKey for ComponentId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

/// A strongly connected component in the instruction graph
pub struct Component {
    /// The set of instructions in the component
    pub instructions: BTreeSet<InstructionId>,
    /// The set of instructions in other components that are referenced by
    /// instructions in this component
    pub successors: BTreeSet<InstructionId>,
}

impl Component {
    pub fn new() -> Self {
        Self {
            instructions: BTreeSet::new(),
            successors: BTreeSet::new(),
        }
    }
}
