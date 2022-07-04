use crate::core::InstructionId;
use crate::core::{Class, Instruction, Parser};
use crate::output::{Codegen, Statements};

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
    pub(super) fn generate(self) -> String {
        let mut codegen = Codegen::new();

        codegen.line("// Generated");
        codegen.newline();

        codegen.line("#[macro_use]");
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

        self.generate_state_constants(&mut codegen);
        self.generate_state_functions(&mut codegen);
        self.generate_class_functions(&mut codegen);
        self.generate_dispatch_function(&mut codegen);
        self.generate_main(&mut codegen);

        codegen.finish()
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
        let mut function =
            codegen.unsafe_function(&function_name, &[("ctx", "&mut Context")], None);

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
                    function.line("state_seq_end(ctx);");
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
                    function.line("state_choice_end(ctx);");
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
                    function.line("state_not_ahead_end(ctx);");
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
                    function.line("state_error_end(ctx);");
                }
                _ => unreachable!(),
            },
            Instruction::Delegate(id) => {
                assert_eq!(state.stage, 0);
                self.generate_unary_consuming_dispatch(&mut function, "state_delegate", id);
            }
            Instruction::Class(class_id) => {
                assert_eq!(state.stage, 0);
                function.line(&format!("state_class(ctx, class_{});", class_id.0));
            }
            Instruction::Empty => {
                assert_eq!(state.stage, 0);
                function.line("state_empty(ctx);");
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
        let continuation_name = format!("STATE_{}_{}", state.id, state.stage + 1);
        block.line(&format!("let target = STATE_{}_0;", target.0));
        block.line(&format!("let continuation = {};", continuation_name));
        block.line(&format!("{}(ctx, target, continuation);", name));
    }

    fn generate_unary_consuming_dispatch(
        &self,
        block: &mut Statements,
        name: &str,
        target: InstructionId,
    ) {
        block.line(&format!("let target = STATE_{}_0;", target.0));
        block.line(&format!("{}(ctx, target);", name));
    }

    fn generate_class_functions(&self, codegen: &mut Codegen) {
        for (id, class) in self.classes() {
            self.generate_class_function(codegen, id.0, class);
        }
    }

    fn generate_class_function(&self, codegen: &mut Codegen, id: usize, class: &Class) {
        let name = format!("class_{}", id);
        let mut function = codegen.function(&name, &[("char", "Char")], Some("bool"));

        for range in class.ranges() {
            function.line("#[allow(unused_comparisons)]");
            let control = format!("{} <= char as u32 && char as u32 <= {}", range.0, range.1);
            let mut branch = function.if_statement(&control);

            branch.line(&format!("return {};", !class.negated()));
        }

        function.line(&format!("{}", class.negated()));
    }

    fn generate_dispatch_function(&self, codegen: &mut Codegen) {
        let mut function = codegen.unsafe_function("dispatch", &[("ctx", "&mut Context")], None);

        let mut state_switch = function.match_statement("ctx.state()");

        for state in self.states() {
            let mut case = state_switch.case(&state.const_name());
            case.line(&format!("{}(ctx);", state.function_name()));
        }

        state_switch
            .case("_")
            .line("core::hint::unreachable_unchecked();");
    }

    fn generate_main(&self, codegen: &mut Codegen) {
        codegen.line(&format!(
            "generate_implementation!(STATE_{}_0, dispatch);",
            self.start().0
        ));

        codegen.line("generate_main!(Impl);");
    }

    fn states(&self) -> impl Iterator<Item = State> {
        let mut states = Vec::new();

        for (id, instruction) in self.instructions() {
            let stages = match instruction {
                Instruction::Seq(_, _) => 3,
                Instruction::Choice(_, _) => 3,
                Instruction::NotAhead(_) => 2,
                Instruction::Error(_) => 2,
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
