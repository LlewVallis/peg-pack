//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

mod buffered_iter;
mod context;
mod grammar;
mod input;
mod result;

pub use context::Context;
pub use grammar::Grammar;
pub use input::*;

use buffered_iter::BufferedIter;
use result::{EnterExit, Match, ParseResult};
use std::fmt::{self, Debug, Formatter};

#[derive(Debug)]
pub enum Parse {
    Unmatched,
    Matched(ParseMatch),
}

impl Parse {
    #[allow(unused)]
    pub(super) fn wrap(result: ParseResult) -> Self {
        match result {
            ParseResult::Matched(value) => Self::Matched(ParseMatch(value)),
            ParseResult::Unmatched { .. } => Self::Unmatched,
        }
    }
}

pub struct ParseMatch(Match);

impl Debug for ParseMatch {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut iter = BufferedIter::new(self.0.walk_labelled());

        fn next_is_enter<T>(iter: &mut BufferedIter<impl Iterator<Item = (T, EnterExit)>>) -> bool {
            iter.peek()
                .map(|(_, state)| *state == EnterExit::Enter)
                .unwrap_or(false)
        }

        fn add_delimiter<T>(
            f: &mut Formatter,
            iter: &mut BufferedIter<impl Iterator<Item = (T, EnterExit)>>,
        ) -> fmt::Result {
            if next_is_enter(iter) {
                write!(f, ", ")?;
            }

            Ok(())
        }

        while let Some((node, state)) = iter.next() {
            let label = unsafe { node.label().unwrap_unchecked() };

            match state {
                EnterExit::Enter => {
                    if next_is_enter(&mut iter) {
                        write!(f, "{} {{ ", label)?;
                    } else {
                        write!(f, "{}", label)?;
                        iter.next();
                        add_delimiter(f, &mut iter)?;
                    }
                }
                EnterExit::Exit => {
                    write!(f, " }}")?;
                    add_delimiter(f, &mut iter)?;
                }
            }
        }

        Ok(())
    }
}

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

        pub use runtime::{Input, Parse};

        pub fn parse<I: Input + ?Sized>(input: &I) -> Parse {
            let grammar = Impl;
            let result = Context::run(input, &grammar);
            Parse::wrap(result)
        }
    };
}

#[allow(unused)]
pub(super) use generate;
