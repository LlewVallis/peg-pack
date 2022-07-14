//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

mod context;
mod grammar;
mod input;
mod result;

pub use context::match_length;
pub use context::Context;
pub use grammar::Grammar;
pub use input::*;

pub type State = u32;

const FINISH_STATE: State = 0;

#[allow(unused)]
macro_rules! generate {
    ($start:expr, $dispatch:ident) => {
        struct Impl;

        impl Grammar for Impl {
            fn start_state(&self) -> State {
                $start
            }

            unsafe fn dispatch_state<I: Input + ?Sized>(
                &self,
                state: State,
                ctx: &mut Context<I, Self>,
            ) {
                $dispatch(state, ctx)
            }
        }

        pub fn parse(input: &[u8]) -> Option<usize> {
            let grammar = Impl;
            match_length(input, &grammar)
        }
    };
}

#[allow(unused)]
pub(super) use generate;
