//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

use std::fmt::{self, Debug, Formatter};

use buffered_iter::BufferedIter;
pub use context::Context;
pub use grammar::*;
pub use input::*;
use result::{EnterExit, Grouping, Walk};
pub use result::{Match, ParseResult};

mod array_vec;
mod buffered_iter;
mod cache;
mod context;
mod grammar;
mod input;
mod refc;
mod result;
mod stack;

pub(super) const SERIES_WORK: u32 = 1;
pub(super) const CACHE_WORK: u32 = 25;
pub(super) const LABEL_WORK: u32 = 50;
pub(super) const MARK_ERROR_WORK: u32 = 50;
pub(super) const NOT_AHEAD_WORK: u32 = 1;
pub(super) const CHOICE_WORK: u32 = 1;
pub(super) const SEQ_WORK: u32 = 1;
pub(super) const MAX_UNCACHED_WORK: u32 = 250;

pub struct GenParseMatch<G: Grammar>(pub Match<G>);

impl<G: Grammar> GenParseMatch<G> {
    #[allow(unused)]
    pub fn visit<V: GenVisitor<G>>(&self, visitor: &mut V) {
        let mut walk = self.0.walk();

        while let Some((position, node, state)) = walk.next() {
            let result = match node.grouping() {
                Grouping::Label(label) => match state {
                    EnterExit::Enter => visitor.enter(label, position, node.distance()),
                    EnterExit::Exit => {
                        visitor.exit(label, position, node.distance());
                        continue;
                    }
                },
                Grouping::Error(error) => match state {
                    EnterExit::Enter => visitor.enter_error(
                        error.labels(),
                        error.literals(),
                        position,
                        node.distance(),
                    ),
                    EnterExit::Exit => {
                        visitor.exit_error(
                            error.labels(),
                            error.literals(),
                            position,
                            node.distance(),
                        );
                        continue;
                    }
                },
                _ => continue,
            };

            match result {
                VisitResult::Continue => {}
                VisitResult::Skip => unsafe { walk.skip_node() },
                VisitResult::Exit => return,
            }
        }
    }

    #[allow(unused)]
    pub fn unmerged_errors(&self) -> impl Iterator<Item = GenErrorInfo<G>> + '_ {
        ErrorIter {
            walk: self.0.walk(),
        }
    }

    fn write_node(&self, f: &mut Formatter, start: u32, node: &Match<G>) -> fmt::Result {
        let end = start + node.distance();

        match node.grouping() {
            Grouping::Label(label) => write!(f, "{:?}[{}-{}]", label, start, end),
            Grouping::Error(expected) => write!(f, "{:?}[{}-{}]", expected, start, end),
            Grouping::None => Ok(()),
        }
    }

    fn next_is_enter<'b>(
        &self,
        iter: &mut BufferedIter<impl Iterator<Item = (u32, &'b Match<G>, EnterExit)>>,
    ) -> bool
    where
        G: 'b,
    {
        iter.peek()
            .map(|(_, _, state)| *state == EnterExit::Enter)
            .unwrap_or(false)
    }

    fn delimit_normal<'b>(
        &self,
        f: &mut Formatter,
        iter: &mut BufferedIter<impl Iterator<Item = (u32, &'b Match<G>, EnterExit)>>,
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
        iter: &mut BufferedIter<impl Iterator<Item = (u32, &'b Match<G>, EnterExit)>>,
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
        iter: &mut BufferedIter<impl Iterator<Item = (u32, &'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        if iter.peek().is_none() {
            return write!(f, "Match");
        }

        while let Some((position, node, state)) = iter.next() {
            match state {
                EnterExit::Enter => {
                    self.write_node(f, position, node)?;

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
        iter: &mut BufferedIter<impl Iterator<Item = (u32, &'b Match<G>, EnterExit)>>,
    ) -> fmt::Result
    where
        G: 'b,
    {
        let mut indent = 0;
        let mut start = true;

        if iter.peek().is_none() {
            return write!(f, "Match");
        }

        while let Some((position, node, state)) = iter.next() {
            match state {
                EnterExit::Enter => {
                    if start {
                        start = false;
                    } else {
                        self.newline_indent(f, indent)?;
                    }

                    self.write_node(f, position, node)?;

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
            .filter(|(_, node, _)| node.grouping() != Grouping::None);

        let mut iter = BufferedIter::new(iter);

        if f.alternate() {
            self.fmt_pretty(f, &mut iter)
        } else {
            self.fmt_normal(f, &mut iter)
        }
    }
}

pub trait GenVisitor<G: Grammar> {
    fn enter(&mut self, label: G::Label, position: u32, length: u32) -> VisitResult;

    fn exit(&mut self, label: G::Label, position: u32, length: u32);

    fn enter_error(
        &mut self,
        expected_labels: &'static [G::Label],
        expected_literals: &'static [&'static [u8]],
        position: u32,
        length: u32,
    ) -> VisitResult;

    fn exit_error(
        &mut self,
        expected_labels: &'static [G::Label],
        expected_literals: &'static [&'static [u8]],
        position: u32,
        length: u32,
    );
}

pub struct GenErrorInfo<G: Grammar> {
    pub expected_labels: &'static [G::Label],
    pub expected_literals: &'static [&'static [u8]],
    pub position: u32,
    pub length: u32,
}

struct ErrorIter<'a, G: Grammar> {
    walk: Walk<'a, G>,
}

impl<'a, G: Grammar> Iterator for ErrorIter<'a, G> {
    type Item = GenErrorInfo<G>;

    fn next(&mut self) -> Option<GenErrorInfo<G>> {
        while let Some((position, node, state)) = self.walk.next() {
            let node: &'a Match<G> = node;

            if state == EnterExit::Enter {
                if let Grouping::Error(error) = node.grouping() {
                    return Some(GenErrorInfo {
                        position,
                        expected_labels: error.labels(),
                        expected_literals: error.literals(),
                        length: node.distance(),
                    });
                }

                if node.error_distance().is_none() {
                    unsafe {
                        self.walk.skip_node();
                    }
                }
            }
        }

        None
    }
}

#[allow(unused)]
#[non_exhaustive]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum VisitResult {
    Continue,
    Skip,
    Exit,
}

pub type State = u32;

const FINISH_STATE: State = 0;

#[allow(unused)]
macro_rules! generate {
    ($start:expr, $cache_slots:expr, $dispatch:ident) => {
        impl std::fmt::Debug for Expected {
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
            type Label = Label;
            type Expected = Expected;

            fn start_state(&self) -> State {
                $start
            }

            fn cache_slots(&self) -> usize {
                $cache_slots
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

        impl ParseMatch {
            pub fn visit<V: Visitor>(&self, visitor: &mut V) {
                self.0.visit(visitor)
            }

            pub fn unmerged_errors(&self) -> impl Iterator<Item = ErrorInfo> + '_ {
                return self.0.unmerged_errors().map(|info| ErrorInfo {
                    expected_labels: info.expected_labels,
                    expected_literals: info.expected_literals,
                    position: info.position,
                    length: info.length,
                });
            }
        }

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

        pub use runtime::VisitResult;

        pub trait Visitor {
            fn enter(&mut self, info: VisitorEnterInfo) -> VisitResult {
                VisitResult::Continue
            }

            fn exit(&mut self, info: VisitorExitInfo) {}

            fn enter_error(&mut self, info: VisitorEnterErrorInfo) -> VisitResult {
                VisitResult::Continue
            }

            fn exit_error(&mut self, info: VisitorExitErrorInfo) {}
        }

        pub struct ErrorInfo {
            expected_labels: &'static [Label],
            expected_literals: &'static [&'static [u8]],
            position: u32,
            length: u32,
        }

        impl<V: Visitor> GenVisitor<Impl> for V {
            fn enter(&mut self, label: Label, position: u32, length: u32) -> VisitResult {
                self.enter(VisitorEnterInfo {
                    label, position, length
                })
            }

            fn exit(&mut self, label: Label, position: u32, length: u32) {
                self.exit(VisitorExitInfo {
                    label, position, length
                })
            }

            fn enter_error(
                &mut self,
                expected_labels: &'static [Label],
                expected_literals: &'static [&'static [u8]],
                position: u32,
                length: u32
            ) -> VisitResult {
                self.enter_error(VisitorEnterErrorInfo {
                    expected_labels, expected_literals, position, length
                })
            }

            fn exit_error(
                &mut self,
                expected_labels: &'static [Label],
                expected_literals: &'static [&'static [u8]],
                position: u32,
                length: u32
            ) {
                self.exit_error(VisitorExitErrorInfo {
                    expected_labels, expected_literals, position, length
                })
            }
        }

        #[non_exhaustive]
        pub struct VisitorEnterInfo {
            pub label: Label,
            pub position: u32,
            pub length: u32,
        }

        #[non_exhaustive]
        pub struct VisitorExitInfo {
            pub label: Label,
            pub position: u32,
            pub length: u32,
        }

        #[non_exhaustive]
        pub struct VisitorEnterErrorInfo {
            pub expected_labels: &'static [Label],
            pub expected_literals: &'static [&'static [u8]],
            pub position: u32,
            pub length: u32,
        }

        #[non_exhaustive]
        pub struct VisitorExitErrorInfo {
            pub expected_labels: &'static [Label],
            pub expected_literals: &'static [&'static [u8]],
            pub position: u32,
            pub length: u32,
        }
    };
}

#[allow(unused)]
pub(super) use generate;
