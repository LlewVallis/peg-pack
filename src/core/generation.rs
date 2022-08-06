use std::collections::HashSet;
use std::mem;

use crate::core::series::{Class, Series};
use crate::core::{Instruction, InstructionId, Parser};
use crate::output::{Codegen, Statements};

#[derive(Copy, Clone)]
struct State {
    id: InstructionId,
    stage: usize,
}

impl State {
    pub fn const_name(&self) -> String {
        format!("STATE_{}_{}", self.id.0, self.stage)
    }

    pub fn function_name(&self) -> String {
        format!("state_{}_{}", self.id.0, self.stage)
    }
}

impl Parser {
    pub fn generate(self) -> String {
        let mut codegen = Codegen::new();

        codegen.line("// Generated");
        codegen.newline();

        codegen.line("#[path = \"build/runtime/mod.rs\"]");
        codegen.line("mod runtime;");
        codegen.line("use runtime::*;");
        codegen.newline();

        self.generate_labels(&mut codegen);
        self.generate_expecteds(&mut codegen);
        self.generate_visualization_comment(&mut codegen);
        self.generate_state_constants(&mut codegen);
        self.generate_state_functions(&mut codegen);
        self.generate_series_functions(&mut codegen);
        self.generate_dispatch_function(&mut codegen);
        self.generate_macro(&mut codegen);

        codegen.finish()
    }

    fn generate_visualization_comment(&self, codegen: &mut Codegen) {
        codegen.line("/*");
        for line in self.visualize().lines() {
            codegen.line(&format!("{}", line));
        }
        codegen.line("*/");
        codegen.newline();
    }

    fn generate_labels(&self, codegen: &mut Codegen) {
        codegen.line("#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]");
        let mut enumeration = codegen.enumeration("LabelImpl");

        let labels = self
            .labels()
            .map(|(_, label)| label)
            .collect::<HashSet<_>>();

        for label in labels {
            let label = self.pascal_case(label);
            enumeration.variant(&label);
        }

        mem::drop(enumeration);

        codegen.trait_impl("Label", "LabelImpl");
    }

    fn generate_expecteds(&self, codegen: &mut Codegen) {
        codegen.line("#[derive(Copy, Clone, Eq, PartialEq, Hash)]");
        let mut enumeration = codegen.enumeration("ExpectedImpl");

        for (id, _) in self.expecteds() {
            enumeration.variant(&format!("E{}", id.0));
        }

        mem::drop(enumeration);

        let mut trait_impl = codegen.trait_impl("Expected<LabelImpl>", "ExpectedImpl");

        {
            let mut literals_function = trait_impl.function("fn literals(&self) -> &[&[u8]]");
            self.generate_expected_literals(&mut literals_function);
        }

        {
            let mut labels_function = trait_impl.function("fn labels(&self) -> &[LabelImpl]");
            self.generate_expected_labels(&mut labels_function);
        }
    }

    fn generate_expected_literals(&self, block: &mut Statements) {
        let mut match_statement = block.match_statement("self");

        for (id, expected) in self.expecteds() {
            let case = format!("Self::E{}", id.0);

            let literals = expected
                .literals()
                .map(|literal| format!("&{:?}", literal))
                .collect::<Vec<_>>();

            let line = format!("&[{}]", literals.join(", "));

            match_statement.case_line(&case, &line);
        }

        if self.expecteds().count() == 0 {
            match_statement.case_line("_", "unsafe { std::hint::unreachable_unchecked() }");
        }
    }

    fn generate_expected_labels(&self, block: &mut Statements) {
        let mut match_statement = block.match_statement("self");

        for (id, expected) in self.expecteds() {
            let case = format!("Self::E{}", id.0);

            let labels = expected
                .labels()
                .map(|label| format!("LabelImpl::{}", self.pascal_case(label)))
                .collect::<Vec<_>>();

            let line = format!("&[{}]", labels.join(", "));

            match_statement.case_line(&case, &line);
        }

        if self.expecteds().count() == 0 {
            match_statement.case_line("_", "unsafe { std::hint::unreachable_unchecked() }");
        }
    }

    fn pascal_case(&self, value: &str) -> String {
        let mut result = String::new();

        for segment in value.split('_') {
            let mut chars = segment.chars();

            if let Some(char) = chars.next() {
                result.push(char.to_ascii_uppercase());
            }

            result.extend(chars);
        }

        result
    }

    fn generate_state_constants(&self, codegen: &mut Codegen) {
        for (i, state) in self.states().enumerate() {
            let instruction = self.instructions[state.id];
            let symbol = &self.debug_symbols[&state.id];

            if symbol.names.is_empty() {
                codegen.line(&format!("// Anonymous: {:?}", instruction));
            } else {
                let names = symbol.names.iter().cloned().collect::<Vec<_>>();

                codegen.line(&format!("// Rule {}: {:?}", names.join(", "), instruction));
            }

            codegen.line(&format!("const {}: State = {};", state.const_name(), i + 1));
        }

        codegen.newline();
    }

    fn generate_state_functions(&self, codegen: &mut Codegen) {
        for state in self.states() {
            self.generate_state_function(codegen, state);
        }
    }

    fn generate_state_function(&self, codegen: &mut Codegen, state: State) {
        let function_name = state.function_name();
        let function_signature = format!(
            "unsafe fn {}<I: Input + ?Sized>(ctx: &mut Context<I, Impl>)",
            function_name
        );
        let mut function = codegen.function(&function_signature);

        match self.instructions[state.id] {
            Instruction::Seq(first, second) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_seq_start",
                        state,
                        first,
                    );
                }
                1 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_seq_middle",
                        state,
                        second,
                    );
                }
                2 => {
                    function.line("ctx.state_seq_end();");
                }
                _ => unreachable!(),
            },
            Instruction::Choice(first, second) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_choice_start",
                        state,
                        first,
                    );
                }
                1 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_choice_middle",
                        state,
                        second,
                    );
                }
                2 => {
                    function.line("ctx.state_choice_end();");
                }
                _ => unreachable!(),
            },
            Instruction::NotAhead(id) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_not_ahead_start",
                        state,
                        id,
                    );
                }
                1 => {
                    function.line("ctx.state_not_ahead_end();");
                }
                _ => unreachable!(),
            },
            Instruction::Error(id, expected) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_error_start",
                        state,
                        id,
                    );
                }
                1 => {
                    function.line(&format!(
                        "ctx.state_error_end(ExpectedImpl::E{});",
                        expected.0
                    ));
                }
                _ => unreachable!(),
            },
            Instruction::Label(target, label) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_label_start",
                        state,
                        target,
                    );
                }
                1 => {
                    let label = &self.labels[label];
                    let label = self.pascal_case(label);
                    function.line(&format!("ctx.state_label_end(LabelImpl::{});", label));
                }
                _ => unreachable!(),
            },
            Instruction::Cache(target, id) => {
                function.line(&format!("let id = {};", id.unwrap()));

                match state.stage {
                    0 => {
                        let target_name = format!("STATE_{}_0", target.0);
                        let continuation_name = format!("STATE_{}_{}", state.id.0, state.stage + 1);
                        function.line(&format!(
                            "ctx.state_cache_start::<{}, {}>(id);",
                            target_name, continuation_name
                        ));
                    }
                    1 => {
                        function.line("ctx.state_cache_end(id);");
                    }
                    _ => unreachable!(),
                }
            }
            Instruction::Delegate(id) => {
                assert_eq!(state.stage, 0);
                self.generate_unary_consuming_dispatch(&mut function, "state_delegate", id);
            }
            Instruction::Series(series_id) => {
                assert_eq!(state.stage, 0);
                function.line(&format!("ctx.state_series(series_{});", series_id.0));
            }
        }
    }

    fn generate_unary_continuing_dispatch(
        &self,
        block: &mut Statements,
        name: &str,
        state: State,
        target: InstructionId,
    ) {
        let target_name = format!("STATE_{}_0", target.0);
        let continuation_name = format!("STATE_{}_{}", state.id.0, state.stage + 1);
        block.line(&format!(
            "ctx.{}::<{}, {}>();",
            name, target_name, continuation_name
        ));
    }

    fn generate_unary_consuming_dispatch(
        &self,
        block: &mut Statements,
        name: &str,
        target: InstructionId,
    ) {
        block.line(&format!("ctx.{}::<STATE_{}_0>();", name, target.0));
    }

    fn generate_series_functions(&self, codegen: &mut Codegen) {
        for (id, series) in self.series() {
            self.generate_series_function(codegen, id.0, series);
        }
    }

    fn generate_series_function(&self, codegen: &mut Codegen, id: usize, series: &Series) {
        let signature = format!(
            "fn series_{}<I: Input + ?Sized>(input: &I, position: usize) -> (bool, usize)",
            id
        );

        let mut function = codegen.function(&signature);

        if series.is_never() {
            function.line("(false, 0)");
            return;
        }

        function.line("let mut length = 0;");
        function.newline();

        for (i, _) in series.classes().iter().enumerate() {
            let mut char_match = function.match_statement("input.get(position + length)");

            let pattern = format!("Some(char) if class_{}_{}(char)", id, i);
            char_match.case_line(&pattern, "length += 1");
            char_match.case_line("_", "return (false, length + 1)");

            mem::drop(char_match);
            function.newline();
        }

        function.line("(true, length)");

        mem::drop(function);
        for (i, class) in series.classes().iter().enumerate() {
            self.generate_class_function(codegen, id, i, class);
        }
    }

    fn generate_class_function(
        &self,
        codegen: &mut Codegen,
        series: usize,
        index: usize,
        class: &Class,
    ) {
        let signature = format!("fn class_{}_{}(char: u8) -> bool", series, index);
        let mut function = codegen.function(&signature);

        self.generate_class_ranges(&mut function, class.ranges(), class.negated());

        function.line(&format!("{}", class.negated()));
    }

    fn generate_class_ranges(&self, block: &mut Statements, ranges: &[(u8, u8)], negated: bool) {
        if ranges.len() <= 3 {
            for range in ranges {
                block.line("#[allow(unused_comparisons)]");
                let control = format!("{} <= char && char <= {}", range.0, range.1);
                let mut branch = block.if_statement(&control);

                branch.line(&format!("return {};", !negated));
            }
        } else {
            let midpoint = ranges.len() / 2;
            let threshold = ranges[midpoint].0;

            {
                block.line("#[allow(unused_comparisons)]");
                let mut below = block.if_statement(&format!("char < {}", threshold));
                self.generate_class_ranges(&mut below, &ranges[..midpoint], negated);
            }

            {
                block.line("#[allow(unused_comparisons)]");
                let mut above = block.if_statement(&format!("char >= {}", threshold));
                self.generate_class_ranges(&mut above, &ranges[midpoint..], negated);
            }
        }
    }

    fn generate_dispatch_function(&self, codegen: &mut Codegen) {
        let mut function = codegen.function(
            "unsafe fn dispatch<I: Input + ?Sized>(state: State, ctx: &mut Context<I, Impl>)",
        );

        let mut state_switch = function.match_statement("state");

        for state in self.states() {
            let case_line = format!("{}(ctx)", state.function_name());
            state_switch.case_line(&state.const_name(), &case_line);
        }

        state_switch.case_line("_", "std::hint::unreachable_unchecked()");
    }

    fn generate_macro(&self, codegen: &mut Codegen) {
        codegen.line(&format!("generate!(STATE_{}_0, dispatch);", self.start().0));
    }

    fn states(&self) -> impl Iterator<Item = State> {
        let mut states = Vec::new();

        for (id, instruction) in self.instructions() {
            let stages = match instruction {
                Instruction::Seq(_, _) | Instruction::Choice(_, _) => 3,
                Instruction::NotAhead(_)
                | Instruction::Error(_, _)
                | Instruction::Label(_, _)
                | Instruction::Cache(_, _) => 2,
                Instruction::Delegate(_) | Instruction::Series(_) => 1,
            };

            for stage in 0..stages {
                states.push(State { id, stage });
            }
        }

        states.into_iter()
    }
}
