//! Runtime common to all generated parsers. Copied into the build directory
//! when generating a parser

use std::fmt::{self, Debug, Formatter};
use std::iter::FusedIterator;

use buffered_iter::BufferedIter;
pub use context::Context;
pub use grammar::*;
pub use input::*;
use result::{EnterExit, Walk};
pub use result::{Grouping as GenGrouping, Match, ParseResult};

mod array_vec;
mod buffered_iter;
mod cache;
mod context;
mod grammar;
mod input;
mod refc;
mod result;
mod small_vec;
mod stack;

pub(super) const SERIES_WORK: u32 = 1;
pub(super) const CACHE_WORK: u32 = 25;
pub(super) const LABEL_WORK: u32 = 50;
pub(super) const MARK_ERROR_WORK: u32 = 50;
pub(super) const NOT_AHEAD_WORK: u32 = 1;
pub(super) const CHOICE_WORK: u32 = 1;
pub(super) const SEQ_WORK: u32 = 1;
pub(super) const MAX_UNCACHED_WORK: u32 = 250;

// The match must always have no grouping
pub struct GenParseMatch<G: Grammar>(Match<G>);

impl<G: Grammar> GenParseMatch<G> {
    #[allow(unused)]
    pub fn new(mut node: Match<G>) -> Self {
        if node.grouping() != GenGrouping::None {
            node = node.wrap();
        }

        Self(node)
    }

    pub fn root(&self) -> GenCursor<G> {
        GenCursor {
            node: &self.0,
            position: 0,
        }
    }

    #[allow(unused)]
    pub fn visit<V: GenVisitor<G>>(&self, visitor: &mut V) {
        self.root().visit(visitor);
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
            GenGrouping::Label(label) => write!(f, "{:?}[{}-{}]", label, start, end),
            GenGrouping::Error(expected) => write!(f, "{:?}[{}-{}]", expected, start, end),
            GenGrouping::None => Ok(()),
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
        struct Inner<'a, G: Grammar>(&'a GenParseMatch<G>);

        impl<'a, G: Grammar> Debug for Inner<'a, G> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                let iter = self
                    .0
                     .0
                    .walk()
                    .filter(|(_, node, _)| node.grouping() != GenGrouping::None);

                let mut iter = BufferedIter::new(iter);

                if f.alternate() {
                    self.0.fmt_pretty(f, &mut iter)
                } else {
                    self.0.fmt_normal(f, &mut iter)
                }
            }
        }

        let has_elements = self
            .0
            .walk()
            .filter(|(_, node, _)| node.grouping() != GenGrouping::None)
            .next()
            .is_some();

        if has_elements {
            f.debug_tuple("ParseMatch").field(&Inner(self)).finish()
        } else {
            f.debug_tuple("ParseMatch").finish()
        }
    }
}

pub trait GenVisitor<G: Grammar> {
    fn enter(
        &mut self,
        label: G::Label,
        position: u32,
        length: u32,
        has_error: bool,
    ) -> VisitResult;

    fn exit(&mut self, label: G::Label, position: u32, length: u32, has_error: bool);

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
                if let GenGrouping::Error(error) = node.grouping() {
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

impl<'a, G: Grammar> FusedIterator for ErrorIter<'a, G> {}

/// Directs the control flow when visiting a node.
///
/// Can be used to skip over a sub-tree or exit entirely.
#[allow(unused)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum VisitResult {
    /// Descends into the sub-tree.
    Continue,
    /// Skips over the sub-tree, without calling `exit`.
    Skip,
    /// Immediately returns from without visiting anything else.
    Exit,
}

pub struct GenCursor<'a, G: Grammar> {
    node: &'a Match<G>,
    position: u32,
}

impl<'a, G: Grammar> GenCursor<'a, G> {
    #[allow(unused)]
    pub fn grouping(&self) -> GenGrouping<G::Label, G::Expected> {
        self.node.grouping()
    }

    #[allow(unused)]
    pub fn label(&self) -> Option<G::Label> {
        match self.grouping() {
            GenGrouping::Label(label) => Some(label),
            _ => None,
        }
    }

    #[allow(unused)]
    pub fn position(&self) -> u32 {
        self.position
    }

    #[allow(unused)]
    pub fn length(&self) -> u32 {
        self.node.distance()
    }

    #[allow(unused)]
    pub fn has_error(&self) -> bool {
        self.node.error_distance().is_some()
    }

    #[allow(unused)]
    pub fn search<F: FnMut(GenCursor<'a, G>) -> bool>(
        &self,
        filter: F,
    ) -> impl Iterator<Item = GenCursor<'a, G>> {
        let mut walk = self.node.walk_from(self.position);
        walk.next();

        FindIter { walk, filter }
    }

    pub fn visit<V: GenVisitor<G>>(&self, visitor: &mut V) {
        let mut walk = self.node.walk_from(self.position);

        while let Some((position, node, state)) = walk.next() {
            let result = match node.grouping() {
                GenGrouping::Label(label) => match state {
                    EnterExit::Enter => visitor.enter(
                        label,
                        position,
                        node.distance(),
                        node.error_distance().is_some(),
                    ),
                    EnterExit::Exit => {
                        visitor.exit(
                            label,
                            position,
                            node.distance(),
                            node.error_distance().is_some(),
                        );
                        continue;
                    }
                },
                GenGrouping::Error(error) => match state {
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
}

impl<'a, G: Grammar> Clone for GenCursor<'a, G> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            position: self.position,
        }
    }
}

struct FindIter<'a, G: Grammar, F: FnMut(GenCursor<'a, G>) -> bool> {
    walk: Walk<'a, G>,
    filter: F,
}

impl<'a, G: Grammar, F: FnMut(GenCursor<'a, G>) -> bool> Iterator for FindIter<'a, G, F> {
    type Item = GenCursor<'a, G>;

    fn next(&mut self) -> Option<GenCursor<'a, G>> {
        while let Some((position, node, state)) = self.walk.next() {
            if state == EnterExit::Enter && node.grouping() != GenGrouping::None {
                let cursor = GenCursor { node, position };

                if (self.filter)(cursor) {
                    unsafe {
                        self.walk.skip_node();
                    }

                    return Some(GenCursor { node, position });
                }
            }
        }

        None
    }
}

impl<'a, G: Grammar, F: FnMut(GenCursor<'a, G>) -> bool> FusedIterator for FindIter<'a, G, F> {}

pub type State<I, G> = unsafe fn(ctx: &mut Context<I, G>);

#[allow(unused)]
macro_rules! generate {
    ($start:expr, $cache_slots:expr) => {
        pub use runtime::Input;

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

            fn start_state<I: Input + ?Sized>(&self) -> State<I, Self> {
                $start
            }

            fn cache_slots(&self) -> usize {
                $cache_slots
            }
        }

        /// The result of a successful or unsuccessful parse.
        ///
        /// Match on the variants to garner more information about the parse.
        #[derive(Debug)]
        pub enum Parse {
            /// The result of a parse that successfully matched at least some of the input.
            Matched(ParseMatch),
            /// Indicates that the parse did not match the input.
            Unmatched,
        }

        impl Parse {
            /// Unwraps the [`Matched`](Parse::Matched) variant, panicking if the parse did not match.
            #[track_caller]
            #[allow(unused)]
            pub fn unwrap(self) -> ParseMatch {
                match self {
                    Self::Matched(result) => result,
                    Self::Unmatched => panic!("parse did not match"),
                }
            }
        }

        /// The result of a parse that successfully matched.
        ///
        /// Although this represents a parse that matched the input the result may still contain
        /// errors.
        pub struct ParseMatch(GenParseMatch<Impl>);

        #[allow(unused)]
        impl ParseMatch {
            /// Creates a cursor that points to the root of the parse tree.
            ///
            /// This cursor's grouping will always be [`Grouping::Root`]. See [`Cursor`] for more
            /// information.
            pub fn root(&self) -> Cursor {
                Cursor(self.0.root())
            }

            /// Walks over the parse tree invoking the appropriate methods in the visitor.
            ///
            /// See the [`Visitor`] trait for more details.
            pub fn visit<V: Visitor>(&self, visitor: &mut V) {
                self.0.visit(visitor)
            }

            /// Creates an iterator over the errors in the parse tree.
            ///
            /// No effort is made to coalesce adjacent errors into one.
            pub fn unmerged_errors(&self) -> impl Iterator<Item = ErrorInfo> + '_ {
                return self.0.unmerged_errors().map(|info| ErrorInfo {
                    expected_labels: info.expected_labels,
                    expected_literals: info.expected_literals,
                    position: info.position,
                    length: info.length,
                    _private: (),
                });
            }
        }

        impl std::fmt::Debug for ParseMatch {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        /// Attempts to parse some input, returning a [`Parse`] that represents the result.
        ///
        /// See [`Input`] for information on what can be passed to this function.
        #[allow(unused)]
        pub fn parse<I: Input + ?Sized>(input: &I) -> Parse {
            let grammar = Impl;
            let result = Context::run(input, &grammar);
            match result {
                ParseResult::Matched(value) => {
                    Parse::Matched(ParseMatch(GenParseMatch::new(value)))
                }
                ParseResult::Unmatched { .. } => Parse::Unmatched,
            }
        }

        pub use runtime::VisitResult;

        /// An interface for walking a [`ParseMatch`] using the
        /// [visitor pattern](https://en.wikipedia.org/wiki/Visitor_pattern).
        pub trait Visitor {
            /// Called when entering a labelled node.
            ///
            /// A [`VisitResult`] is returned to indicate whether this node's descendants should
            /// be skipped, whether the traversal should be halted, or whether the traversal should
            /// continue.
            ///
            /// The default implementation simply returns [`Continue`](VisitResult::Continue).
            fn enter(&mut self, info: VisitorEnterInfo) -> VisitResult {
                let _ = info;
                VisitResult::Continue
            }

            /// Called when all of a labelled node's descendants have been traversed.
            ///
            /// This is called even if the node has no descendants, but is not called if the
            /// corresponding `enter` returned [`Skip`](VisitResult::Skip).
            fn exit(&mut self, info: VisitorExitInfo) {
                let _ = info;
            }

            /// Called when entering an error node.
            ///
            /// A [`VisitResult`] is returned to indicate whether this node's descendants should
            /// be skipped, whether the traversal should be halted, or whether the traversal should
            /// continue.
            ///
            /// The default implementation simply returns [`Continue`](VisitResult::Continue).
            fn enter_error(&mut self, info: VisitorEnterErrorInfo) -> VisitResult {
                let _ = info;
                VisitResult::Continue
            }

            /// Called when all of an error node's descendants have been traversed.
            ///
            /// This is called even if the node has no descendants, but is not called if the
            /// corresponding `enter_error` returned [`Skip`](VisitResult::Skip).
            fn exit_error(&mut self, info: VisitorExitErrorInfo) {
                let _ = info;
            }
        }

        /// Information about an error yielded by [`ParseMatch::unmerged_errors`].
        #[allow(unused)]
        #[derive(Debug)]
        pub struct ErrorInfo {
            /// The set of labels that were excepted at the error's position in the input stream.
            pub expected_labels: &'static [Label],
            /// The set of literals that were excepted at the error's position in the input stream.
            pub expected_literals: &'static [&'static [u8]],
            /// The position at which the error occurred.
            pub position: u32,
            /// The length of the input covered by the error.
            pub length: u32,
            _private: (),
        }

        impl<V: Visitor> GenVisitor<Impl> for V {
            fn enter(
                &mut self,
                label: Label,
                position: u32,
                length: u32,
                has_error: bool,
            ) -> VisitResult {
                self.enter(VisitorEnterInfo {
                    label,
                    position,
                    length,
                    has_error,
                    _private: (),
                })
            }

            fn exit(&mut self, label: Label, position: u32, length: u32, has_error: bool) {
                self.exit(VisitorExitInfo {
                    label,
                    position,
                    length,
                    has_error,
                    _private: (),
                })
            }

            fn enter_error(
                &mut self,
                expected_labels: &'static [Label],
                expected_literals: &'static [&'static [u8]],
                position: u32,
                length: u32,
            ) -> VisitResult {
                self.enter_error(VisitorEnterErrorInfo {
                    expected_labels,
                    expected_literals,
                    position,
                    length,
                    _private: (),
                })
            }

            fn exit_error(
                &mut self,
                expected_labels: &'static [Label],
                expected_literals: &'static [&'static [u8]],
                position: u32,
                length: u32,
            ) {
                self.exit_error(VisitorExitErrorInfo {
                    expected_labels,
                    expected_literals,
                    position,
                    length,
                    _private: (),
                })
            }
        }

        /// Information about a labelled node passed to [`Visitor::enter`].
        #[derive(Debug)]
        pub struct VisitorEnterInfo {
            /// The label applied to the section of input.
            pub label: Label,
            /// The position at which the label was applied.
            pub position: u32,
            /// The length of input covered by the label.
            pub length: u32,
            /// Whether any descendants of the node contain an error
            pub has_error: bool,
            _private: (),
        }

        /// Information about a labelled node passed to [`Visitor::exit`].
        #[derive(Debug)]
        pub struct VisitorExitInfo {
            /// The label applied to the section of input.
            pub label: Label,
            /// The position at which the label was applied.
            pub position: u32,
            /// The length of input covered by the label.
            pub length: u32,
            /// Whether any descendants of the node contain an error
            pub has_error: bool,
            _private: (),
        }

        /// Information about an error node passed to [`Visitor::enter_error`].
        #[derive(Debug)]
        pub struct VisitorEnterErrorInfo {
            /// The set of labels that were excepted at the error's position in the input stream.
            pub expected_labels: &'static [Label],
            /// The set of literals that were excepted at the error's position in the input stream.
            pub expected_literals: &'static [&'static [u8]],
            /// The position at which the error occurred.
            pub position: u32,
            /// The length of the input covered by the error.
            pub length: u32,
            _private: (),
        }

        /// Information about an error node passed to [`Visitor::exit_error`].
        #[derive(Debug)]
        pub struct VisitorExitErrorInfo {
            /// The set of labels that were excepted at the error's position in the input stream.
            pub expected_labels: &'static [Label],
            /// The set of literals that were excepted at the error's position in the input stream.
            pub expected_literals: &'static [&'static [u8]],
            /// The position at which the error occurred.
            pub position: u32,
            /// The length of the input covered by the error.
            pub length: u32,
            _private: (),
        }

        /// Points to a node in a parse tree.
        ///
        /// A cursor can point to three different types of node: a label node, an error node, or the
        /// root node. Label and error nodes designate points in the parse tree where a label or
        /// error was produced respectively. The root node is a special node which contains all
        /// other nodes in the parse tree.
        ///
        /// Use the [`grouping`](Cursor::grouping) method to determine what the cursor points to.
        #[derive(Clone)]
        pub struct Cursor<'a>(GenCursor<'a, Impl>);

        #[allow(unused)]
        impl<'a> Cursor<'a> {
            /// Produces an enum describing the node the cursor points to.
            ///
            /// See [`Grouping`] for more information.
            pub fn grouping(&self) -> Grouping {
                match self.0.grouping() {
                    GenGrouping::Label(label) => Grouping::Label(label),
                    GenGrouping::Error(error) => Grouping::Error {
                        expected_labels: error.labels(),
                        expected_literals: error.literals(),
                    },
                    GenGrouping::None => Grouping::Root,
                }
            }

            /// The label corresponding to the node the cursor points to, or `None` if the cursor
            /// points to an error or the root node.
            pub fn label(&self) -> Option<Label> {
                self.0.label()
            }

            /// Iterates over the immediate children of this node that have the provided label.
            pub fn labelled(&self, label: Label) -> impl Iterator<Item = Cursor<'a>> {
                self.children()
                    .filter(move |child| child.label() == Some(label))
            }

            /// Finds the first immediate child of this node that has the provided label, if any.
            pub fn first(&self, label: Label) -> Option<Cursor<'a>> {
                self.labelled(label).next()
            }

            /// Determines the position of the node in the input stream.
            pub fn position(&self) -> u32 {
                self.0.position()
            }

            /// Determines the length of the node.
            pub fn length(&self) -> u32 {
                self.0.length()
            }

            /// Determines whether the node has any error node descendants, including the node
            /// itself.
            ///
            /// If the node is an error node, this will always return `true`.
            pub fn has_error(&self) -> bool {
                self.0.has_error()
            }

            /// Visits each node in the sub-tree below the node using the [`Visitor`] API.
            ///
            /// If the referenced node is not the root node, then the node itself is also visited.
            /// See [`ParseMatch::visit`] for more information.
            pub fn visit<V: Visitor>(&self, visitor: &mut V) {
                self.0.visit(visitor)
            }

            /// Searches the parse tree for matching descendants.
            ///
            /// Performs a depth first search over the descendants of the node, yielding a cursor to
            /// any descendants on which the predicate returns `true`. If the predicate returns
            /// `true` on a node, it's descendants are skipped for the remainder of the search.
            pub fn search<F>(&self, mut predicate: F) -> impl Iterator<Item = Cursor<'a>>
            where
                F: FnMut(Cursor) -> bool,
            {
                self.0
                    .search(move |cursor| predicate(Cursor(cursor)))
                    .map(Cursor)
            }

            /// Iterates over the immediate children of this node, yielding a cursor for each of
            /// them.
            pub fn children(&self) -> impl Iterator<Item = Cursor<'a>> {
                self.search(|_| true)
            }
        }

        impl<'a> std::fmt::Debug for Cursor<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                #[allow(unused)]
                #[derive(Debug)]
                struct Cursor {
                    grouping: Grouping,
                    position: u32,
                    length: u32,
                    has_error: bool,
                }

                write!(
                    f,
                    "{:?}",
                    Cursor {
                        grouping: self.grouping(),
                        position: self.position(),
                        length: self.length(),
                        has_error: self.has_error(),
                    }
                )
            }
        }

        /// The type of a node reference by a [`Cursor`].
        #[allow(unused)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub enum Grouping {
            /// Represents a labelled segment of the parse tree.
            Label(Label),
            /// Represents a soft-error in the parse tree.
            Error {
                /// The set of labels that were excepted at the error's position in the input stream.
                expected_labels: &'static [Label],
                /// The set of literals that were excepted at the error's position in the input stream.
                expected_literals: &'static [&'static [u8]],
            },
            /// Identifies the root node of parse tree.
            ///
            /// For any given parse tree, there is one cursor whose [`grouping`](Cursor::grouping)
            /// method returns this variant.
            Root,
        }
    };
}

#[allow(unused)]
pub(super) use generate;
