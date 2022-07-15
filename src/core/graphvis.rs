use crate::core::character::Character;
use crate::core::{Instruction, Parser};

impl Parser {
    pub fn visualize(&self) -> String {
        let mut result = String::from("digraph {\n");

        self.visualize_instructions(&mut result);
        self.visualize_classes(&mut result);
        self.visualize_labels(&mut result);
        self.visualize_components(&mut result);

        result.push_str("}");
        result
    }

    fn visualize_instructions(&self, result: &mut String) {
        let characters = self.characterize();

        for (id, instruction) in self.instructions() {
            let character = characters[&id];
            let name = self.instruction_name(instruction, character);

            let header = format!("    i{}[label=\"{} #{}\"];\n", id.0, name, id.0);
            result.push_str(&header);

            match instruction {
                Instruction::Seq(first, second) | Instruction::Choice(first, second) => {
                    result.push_str(&format!("    i{}:w -> i{};\n", id.0, first.0));
                    result.push_str(&format!("    i{}:e -> i{};\n", id.0, second.0));
                }
                Instruction::NotAhead(target)
                | Instruction::Error(target)
                | Instruction::Delegate(target) => {
                    result.push_str(&format!("    i{} -> i{};\n", id.0, target.0));
                }
                Instruction::Label(target, label) => {
                    result.push_str(&format!("    i{} -> i{};\n", id.0, target.0));
                    result.push_str(&format!("    i{} -> l{};\n", id.0, label.0));
                }
                Instruction::Class(class) => {
                    result.push_str(&format!("    i{} -> c{};\n", id.0, class.0));
                }
                Instruction::Empty => {}
            };
        }

        result.push_str(&format!("    i{}[peripheries=2];\n", self.start.0));
    }

    fn instruction_name(&self, instruction: Instruction, character: Character) -> String {
        let name = match instruction {
            Instruction::Seq(_, _) => "Sequence",
            Instruction::Choice(_, _) => "Choice",
            Instruction::NotAhead(_) => "Not ahead",
            Instruction::Error(_) => "Error",
            Instruction::Delegate(_) => "Delegate",
            Instruction::Label(_, _) => "Label",
            Instruction::Class(_) => "Class",
            Instruction::Empty => "Empty",
        };

        let mut name = String::from(name);

        if character.antitransparent {
            name.push_str(" (AT)");
        }

        if character.transparent {
            name.push_str(" (T)");
        }

        if character.fallible {
            name.push_str(" (F)");
        }

        name
    }

    fn visualize_classes(&self, result: &mut String) {
        for (id, class) in self.classes() {
            let mut specifier = String::new();

            if class.negated() {
                specifier.push_str("^");
            }

            for (i, (start, end)) in class.ranges().enumerate() {
                if i != 0 {
                    specifier.push_str(", ");
                }

                if start == end {
                    specifier.push_str(&self.format_class_bound(start));
                } else {
                    specifier.push_str(&format!(
                        "{}-{}",
                        self.format_class_bound(start),
                        self.format_class_bound(end)
                    ));
                }
            }

            result.push_str(&format!(
                "    c{}[label=\"[{}] #{}\", shape=box];\n",
                id.0, specifier, id.0
            ));
        }
    }

    fn format_class_bound(&self, bound: u8) -> String {
        let format_char =
            bound == 0 || bound == 9 || bound == 10 || bound == 13 || bound >= 32 && bound <= 126;

        if format_char {
            format!("{:?}", bound as char)
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        } else {
            format!("0x{:x}", bound)
        }
    }

    fn visualize_labels(&self, result: &mut String) {
        for (id, label) in self.labels() {
            let specifier = format!("{:?}", label)
                .replace('\\', "\\\\")
                .replace('"', "\\\"");

            result.push_str(&format!(
                "    l{}[label=\"{} #{}\", shape=box];\n",
                id.0, specifier, id.0
            ));
        }
    }

    fn visualize_components(&self, result: &mut String) {
        let components = self.separate_components();

        for (i, (_, component)) in components.components.iter().enumerate() {
            if component.instructions.len() < 2 {
                continue;
            }

            result.push_str(&format!("    subgraph cluster_comp{} {{\n", i));
            result.push_str("        style=dotted;\n");
            for instruction in &component.instructions {
                result.push_str(&format!("        i{};\n", instruction.0));
            }
            result.push_str("    }\n");
        }
    }
}
