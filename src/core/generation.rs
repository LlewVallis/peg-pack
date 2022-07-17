use crate::core::InstructionId;
use crate::core::{Class, Instruction, Parser};
use crate::output::{Codegen, Statements};
use std::collections::HashSet;

#[derive(Copy, Clone)]
struct State {
    id: usize,
    stage: usize,
    instruction: Instruction,
}

impl State {
    pub fn const_name(&self) -> String {
        format!("STATE_{}_{}", self.id, self.stage)
    }

    pub fn function_name(&self) -> String {
        format!("state_{}_{}", self.id, self.stage)
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

        codegen.line("/*");
        for line in self.visualize().lines() {
            codegen.line(&format!("{}", line));
        }
        codegen.line("*/");

        codegen.newline();

        self.generate_labels(&mut codegen);
        self.generate_state_constants(&mut codegen);
        self.generate_state_functions(&mut codegen);
        self.generate_class_functions(&mut codegen);
        self.generate_dispatch_function(&mut codegen);
        self.generate_macro(&mut codegen);

        codegen.finish()
    }

    fn generate_labels(&self, codegen: &mut Codegen) {
        codegen.line("#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]");
        let mut enumeration = codegen.enumeration("Label");

        let labels = self
            .labels()
            .map(|(_, label)| label)
            .collect::<HashSet<_>>();

        for label in labels {
            let label = self.pascal_case(label);
            enumeration.variant(&label);
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

        match state.instruction {
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
            Instruction::Error(id) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_error_start",
                        state,
                        id,
                    );
                }
                1 => {
                    function.line("ctx.state_error_end();");
                }
                _ => unreachable!(),
            },
            Instruction::Commit(id) => match state.stage {
                0 => {
                    self.generate_unary_continuing_dispatch(
                        &mut function,
                        "state_commit_start",
                        state,
                        id,
                    );
                }
                1 => {
                    function.line("ctx.state_commit_end();");
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
                    let label = self.unwrap_label(label);
                    let label = self.pascal_case(label);
                    function.line(&format!("ctx.state_label_end(Label::{});", label));
                }
                _ => unreachable!(),
            },
            Instruction::Delegate(id) => {
                assert_eq!(state.stage, 0);
                self.generate_unary_consuming_dispatch(&mut function, "state_delegate", id);
            }
            Instruction::Class(class_id) => {
                assert_eq!(state.stage, 0);
                function.line(&format!("ctx.state_class(class_{});", class_id.0));
            }
            Instruction::Empty => {
                assert_eq!(state.stage, 0);
                function.line("ctx.state_empty();");
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
        let continuation_name = format!("STATE_{}_{}", state.id, state.stage + 1);
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

    fn generate_class_functions(&self, codegen: &mut Codegen) {
        for (id, class) in self.classes() {
            self.generate_class_function(codegen, id.0, class);
        }
    }

    fn generate_class_function(&self, codegen: &mut Codegen, id: usize, class: &Class) {
        let signature = format!("fn class_{}(char: u8) -> bool", id);
        let mut function = codegen.function(&signature);

        for range in class.ranges() {
            function.line("#[allow(unused_comparisons)]");
            let control = format!("{} <= char && char <= {}", range.0, range.1);
            let mut branch = function.if_statement(&control);

            branch.line(&format!("return {};", !class.negated()));
        }

        function.line(&format!("{}", class.negated()));
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
                Instruction::Seq(_, _) => 3,
                Instruction::Choice(_, _) => 3,
                Instruction::NotAhead(_) => 2,
                Instruction::Error(_) => 2,
                Instruction::Commit(_) => 2,
                Instruction::Label(_, _) => 2,
                Instruction::Delegate(_) => 1,
                Instruction::Class(_) => 1,
                Instruction::Empty => 1,
            };

            for stage in 0..stages {
                states.push(State {
                    stage,
                    instruction,
                    id: id.0,
                });
            }
        }

        states.into_iter()
    }
}
