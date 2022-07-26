//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

use std::fmt::{self, Debug, Formatter};

use buffered_iter::BufferedIter;
pub use context::Context;
pub use grammar::*;
pub use input::*;
use result::{EnterExit, Grouping};
pub use result::{Match, ParseResult};

mod array_vec;
mod buffered_iter;
mod context;
mod grammar;
mod input;
mod refc;
mod result;
mod stack;

pub struct GenParseMatch<G: Grammar>(pub Match<G>);

impl<G: Grammar> GenParseMatch<G> {
    fn write_node(&self, f: &mut Formatter, node: &Match<G>) -> fmt::Result {
        match node.grouping() {
            Grouping::Label(label) => write!(f, "{:?}[{}]", label, node.distance()),
            Grouping::Error(expected) => write!(f, "{:?}[{}]", expected, node.distance()),
            Grouping::None => Ok(()),
        }
    }

    fn next_is_enter<'b>(
        &self,
        iter: &mut BufferedIter<impl Iterator<Item = (&'b Match<G>, EnterExit)>>,
    ) -> bool
    where
        G: 'b,
    {
        iter.peek()
            .map(|(_, state)| *state == EnterExit::Enter)
            .unwrap_or(false)
    }

    fn delimit_normal<'b>(
        &self,
        f: &mut Formatter,
        iter: &mut BufferedIter<impl Iterator<Item = (&'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        if self.next_is_enter(iter) {
            write!(f, ", ")?;
        }

        Ok(())
    }

    fn delimit_pretty<'b>(
        &self,
        f: &mut Formatter,
        iter: &mut BufferedIter<impl Iterator<Item = (&'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        if iter.peek().is_some() {
            write!(f, ",")?;
        }

        Ok(())
    }

    fn newline_indent(&self, f: &mut Formatter, amount: usize) -> fmt::Result {
        write!(f, "\n")?;

        for _ in 0..amount {
            write!(f, "    ")?;
        }

        Ok(())
    }

    fn fmt_normal<'b>(
        &self,
        f: &mut Formatter,
        iter: &mut BufferedIter<impl Iterator<Item = (&'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        if iter.peek().is_none() {
            return write!(f, "Match");
        }

        while let Some((node, state)) = iter.next() {
            match state {
                EnterExit::Enter => {
                    self.write_node(f, node)?;

                    if self.next_is_enter(iter) {
                        write!(f, "(")?;
                    } else {
                        iter.next();
                        self.delimit_normal(f, iter)?;
                    }
                }
                EnterExit::Exit => {
                    write!(f, ")")?;
                    self.delimit_normal(f, iter)?;
                }
            }
        }

        Ok(())
    }

    fn fmt_pretty<'b>(
        &self,
        f: &mut Formatter,
        iter: &mut BufferedIter<impl Iterator<Item = (&'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        let mut indent = 0;
        let mut start = true;

        if iter.peek().is_none() {
            return write!(f, "Match");
        }

        while let Some((node, state)) = iter.next() {
            match state {
                EnterExit::Enter => {
                    if start {
                        start = false;
                    } else {
                        self.newline_indent(f, indent)?;
                    }

                    self.write_node(f, node)?;

                    if self.next_is_enter(iter) {
                        write!(f, "(")?;
                        indent += 1;
                    } else {
                        iter.next();
                        self.delimit_pretty(f, iter)?;
                    }
                }
                EnterExit::Exit => {
                    indent -= 1;
                    self.newline_indent(f, indent)?;
                    write!(f, ")")?;
                    self.delimit_pretty(f, iter)?;
                }
            }
        }

        Ok(())
    }
}

impl<G: Grammar> Debug for GenParseMatch<G> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let iter = self
            .0
            .walk()
            .filter(|(node, _)| node.grouping() != Grouping::None);

        let mut iter = BufferedIter::new(iter);

        if f.alternate() {
            self.fmt_pretty(f, &mut iter)
        } else {
            self.fmt_normal(f, &mut iter)
        }
    }
}

pub type State = u32;

const FINISH_STATE: State = 0;

#[allow(unused)]
macro_rules! generate {
    ($start:expr, $dispatch:ident) => {
        impl std::fmt::Debug for ExpectedImpl {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut tuple = f.debug_tuple("Error");

                for label in self.labels() {
                    tuple.field(label);
                }

                for literal in self.literals() {
                    if let Ok(string) = std::str::from_utf8(literal) {
                        tuple.field(&string);
                    } else {
                        tuple.field(literal);
                    }
                }

                tuple.finish()
            }
        }

        struct Impl;

        impl Grammar for Impl {
            type Label = LabelImpl;
            type Expected = ExpectedImpl;

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

        #[derive(Debug)]
        pub enum Parse {
            Matched(ParseMatch),
            Unmatched,
        }

        pub struct ParseMatch(GenParseMatch<Impl>);

        impl std::fmt::Debug for ParseMatch {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        pub fn parse<I: Input + ?Sized>(input: &I) -> Parse {
            let grammar = Impl;
            let result = Context::run(input, &grammar);
            match result {
                ParseResult::Matched(value) => Parse::Matched(ParseMatch(GenParseMatch(value))),
                ParseResult::Unmatched { .. } => Parse::Unmatched,
            }
        }
    };
}

#[allow(unused)]
pub(super) use generate;
