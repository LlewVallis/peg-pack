use crate::core::character::Character;
use crate::core::expected::Expected;
use crate::core::series::{Class, Series};
use crate::core::{Instruction, Parser};
use std::collections::{HashMap, HashSet};

impl Parser {
    pub fn visualize(&self) -> String {
        let mut result = String::from("digraph {\n");

        self.visualize_instructions(&mut result);
        self.visualize_debug_symbols(&mut result);

        result.push_str("}");
        result
    }

    fn visualize_instructions(&self, result: &mut String) {
        let characters = self.characterize();

        for (id, instruction) in self.instructions() {
            let character = characters[&id];
            let name = self.instruction_name(instruction, character);
            let shape = self.instruction_shape(instruction);

            let header = format!(
                "    i{}[shape={}, label=\"{} #{}\"];\n",
                id.0, shape, name, id.0
            );
            result.push_str(&header);

            match instruction {
                Instruction::Seq(first, second)
                | Instruction::Choice(first, second)
                | Instruction::FirstChoice(first, second) => {
                    result.push_str(&format!("    i{}:w -> i{};\n", id.0, first.0));
                    result.push_str(&format!("    i{}:e -> i{};\n", id.0, second.0));
                }
                Instruction::NotAhead(target)
                | Instruction::Error(target, _)
                | Instruction::Label(target, _)
                | Instruction::Cache(target, _)
                | Instruction::Delegate(target) => {
                    result.push_str(&format!("    i{} -> i{};\n", id.0, target.0));
                }
                Instruction::Series(_) => {}
            };
        }

        result.push_str(&format!("    i{}[peripheries=2];\n", self.start.0));
    }

    fn instruction_shape(&self, instruction: Instruction) -> &str {
        match instruction {
            Instruction::Seq(_, _)
            | Instruction::Choice(_, _)
            | Instruction::FirstChoice(_, _)
            | Instruction::NotAhead(_)
            | Instruction::Error(_, _)
            | Instruction::Label(_, _)
            | Instruction::Cache(_, _)
            | Instruction::Delegate(_) => "oval",
            Instruction::Series(_) => "box",
        }
    }

    fn instruction_name(&self, instruction: Instruction, character: Character) -> String {
        let mut name = match instruction {
            Instruction::Seq(_, _) => String::from("Sequence"),
            Instruction::Choice(_, _) => String::from("Choice"),
            Instruction::FirstChoice(_, _) => String::from("First choice"),
            Instruction::NotAhead(_) => String::from("Not ahead"),
            Instruction::Error(_, expected) => {
                let expected = &self.expecteds[expected];
                format!("Error[{}]", self.expected_specifier(expected))
            }
            Instruction::Cache(_, id) => match id {
                Some(id) => format!("Cache[{}]", id),
                None => String::from("Cache[?]"),
            },
            Instruction::Delegate(_) => String::from("Delegate"),
            Instruction::Label(_, label) => {
                let label = &self.labels[label];
                format!("Label[{}]", label)
            }
            Instruction::Series(series) => {
                let series = &self.series[series];
                format!("Series[{}]", self.series_specifier(series))
            }
        };

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

    fn series_specifier(&self, series: &Series) -> String {
        let mut specifier = String::new();

        for (i, class) in series.classes().iter().enumerate() {
            if i != 0 {
                specifier.push_str(", ");
            }

            specifier.push_str(&self.class_specifier(class));
        }

        specifier
    }

    fn class_specifier(&self, class: &Class) -> String {
        let mut specifier = String::new();
        let brackets = class.negated() || class.ranges().len() != 1;

        if brackets {
            specifier.push_str("[");
        }

        if class.negated() {
            specifier.push_str("^");
        }

        for (i, (start, end)) in class.ranges().iter().enumerate() {
            if i != 0 {
                specifier.push_str(", ");
            }

            if start == end {
                specifier.push_str(&self.format_class_bound(*start));
            } else {
                specifier.push_str(&format!(
                    "{}-{}",
                    self.format_class_bound(*start),
                    self.format_class_bound(*end)
                ));
            }
        }

        if brackets {
            specifier.push_str("]");
        }

        specifier
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

    fn expected_specifier(&self, expected: &Expected) -> String {
        let mut parts = Vec::new();

        for label in expected.labels() {
            parts.push(String::from(label));
        }

        for literal in expected.literals() {
            let mut series = Series::empty();
            for char in literal {
                let mut class = Class::new(false);
                class.insert(*char, *char);
                series.append(class);
            }

            parts.push(self.series_specifier(&series));
        }

        format!("[{}]", parts.join(", "))
    }

    fn visualize_debug_symbols(&self, result: &mut String) {
        let mut groups = HashMap::<_, HashSet<_>>::new();

        for (instruction, symbol) in &self.debug_symbols {
            if let Some(set) = groups.get_mut(symbol) {
                set.insert(*instruction);
            } else {
                groups.insert(symbol.clone(), HashSet::from([*instruction]));
            }
        }

        for (i, (symbol, instructions)) in groups.into_iter().enumerate() {
            let names = if symbol.names.is_empty() {
                String::from("<anonymous>")
            } else {
                symbol.names.iter().cloned().collect::<Vec<_>>().join(", ")
            };

            result.push_str(&format!("    subgraph cluster_{} {{\n", i));
            result.push_str(&format!("        label=\"{}\";\n", names));

            for instruction in instructions {
                result.push_str(&format!("        i{};\n", instruction.0));
            }

            result.push_str("    }\n");
        }
    }
}
