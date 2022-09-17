use std::fmt::Debug;
use std::hash::Hash;
use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::ops::Deref;

use super::array_vec::ArrayVec;
use super::grammar::{Expected, Grammar, Label};
use super::refc::Refc;
use super::stack::Stack;

pub enum ParseResult<G: Grammar> {
    Matched(Match<G>),
    Unmatched { scan_distance: usize, work: usize },
}

impl<G: Grammar> ParseResult<G> {
    pub fn is_match(&self) -> bool {
        match self {
            Self::Matched(_) => true,
            Self::Unmatched { .. } => false,
        }
    }

    pub fn error_distance(&self) -> Option<usize> {
        match self {
            ParseResult::Matched(value) => value.error_distance,
            ParseResult::Unmatched { .. } => Some(0),
        }
    }

    pub fn is_error_free(&self) -> bool {
        self.error_distance().is_none()
    }

    pub fn scan_distance(&self) -> usize {
        match self {
            Self::Matched(value) => value.scan_distance(),
            Self::Unmatched { scan_distance, .. } => *scan_distance,
        }
    }

    pub fn work(&self) -> usize {
        match self {
            ParseResult::Matched(value) => value.work(),
            ParseResult::Unmatched { work, .. } => *work,
        }
    }

    pub fn distance(&self) -> usize {
        match self {
            Self::Matched(value) => value.distance(),
            Self::Unmatched { .. } => 0,
        }
    }

    pub fn extend_scan_distance(self, amount: usize) -> Self {
        match self {
            Self::Matched(value) => Self::Matched(value.extend_scan_distance(amount)),
            Self::Unmatched {
                scan_distance,
                work,
            } => Self::Unmatched {
                scan_distance: scan_distance.max(amount),
                work,
            },
        }
    }

    pub fn with_work(self, amount: usize) -> Self {
        match self {
            Self::Matched(value) => Self::Matched(value.with_work(amount)),
            Self::Unmatched { scan_distance, .. } => Self::Unmatched {
                work: amount,
                scan_distance,
            },
        }
    }

    pub fn add_work(self, amount: usize) -> Self {
        match self {
            Self::Matched(value) => Self::Matched(value.add_work(amount)),
            Self::Unmatched {
                scan_distance,
                work,
            } => Self::Unmatched {
                work: work + amount,
                scan_distance,
            },
        }
    }

    pub unsafe fn unwrap_match_unchecked(self) -> Match<G> {
        match self {
            Self::Matched(value) => value,
            Self::Unmatched { .. } => unreachable_unchecked(),
        }
    }

    pub fn negate(self) -> Self {
        match self {
            Self::Matched(value) => Self::Unmatched {
                scan_distance: value.scan_distance(),
                work: value.work,
            },
            Self::Unmatched {
                scan_distance,
                work,
            } => Self::Matched(Match::empty(scan_distance, work)),
        }
    }

    pub fn mark_error(self, expected: G::Expected) -> Self {
        match self {
            Self::Matched(value) => {
                let new_value = if value.grouping.is_none() {
                    Match {
                        grouping: Grouping::Error(expected),
                        scan_distance: value.scan_distance,
                        work: value.work,
                        distance: value.distance,
                        error_distance: Some(0),
                        children: value.children,
                    }
                } else {
                    Match {
                        grouping: Grouping::Error(expected),
                        scan_distance: value.scan_distance,
                        work: value.work,
                        distance: value.distance,
                        error_distance: Some(0),
                        children: ArrayVec::of([(0, value.boxed())]),
                    }
                };

                Self::Matched(new_value)
            }
            Self::Unmatched { .. } => self,
        }
    }

    pub fn label(self, label: G::Label) -> Self {
        match self {
            Self::Matched(value) => {
                let new_value = if value.grouping.is_none() {
                    Match {
                        grouping: Grouping::Label(label),
                        scan_distance: value.scan_distance,
                        work: value.work,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: value.children,
                    }
                } else {
                    Match {
                        grouping: Grouping::Label(label),
                        scan_distance: value.scan_distance,
                        work: value.work,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: ArrayVec::of([(0, value.boxed())]),
                    }
                };

                Self::Matched(new_value)
            }
            Self::Unmatched { .. } => self,
        }
    }
}

const MATCH_CHILDREN: usize = 4;

pub struct Match<G: Grammar> {
    scan_distance: usize,
    work: usize,
    distance: usize,
    error_distance: Option<usize>,
    grouping: Grouping<G::Label, G::Expected>,
    children: ArrayVec<(usize, Refc<Self>), MATCH_CHILDREN>,
}

impl<G: Grammar> Match<G> {
    pub fn empty(scan_distance: usize, work: usize) -> Self {
        Self::error_free(0, scan_distance, work)
    }

    pub fn error_free(distance: usize, scan_distance: usize, work: usize) -> Self {
        Self {
            scan_distance,
            work,
            distance,
            grouping: Grouping::None,
            error_distance: None,
            children: ArrayVec::new(),
        }
    }

    pub fn combine(first: Self, second: Self) -> Self {
        let scan_distance = usize::max(first.scan_distance, first.distance + second.scan_distance);

        let work = first.work + second.work;

        let distance = first.distance + second.distance;

        let error_distance = first.error_distance.or_else(|| {
            second
                .error_distance
                .map(|distance| first.distance + distance)
        });

        let first_offset = 0;
        let second_offset = first.distance;

        if first.grouping == Grouping::None
            && second.grouping == Grouping::None
            && first.children.len() + second.children.len() <= MATCH_CHILDREN
        {
            let children = Self::merge_children(first, second);

            return Self {
                grouping: Grouping::None,
                scan_distance,
                work,
                distance,
                error_distance,
                children,
            };
        }

        let children = [
            (first_offset, first.boxed()),
            (second_offset, second.boxed()),
        ];

        Self {
            grouping: Grouping::None,
            children: ArrayVec::of(children),
            scan_distance,
            work,
            distance,
            error_distance,
        }
    }

    fn merge_children(first: Self, second: Self) -> ArrayVec<(usize, Refc<Self>), MATCH_CHILDREN> {
        let mut children = ArrayVec::new();

        for (offset, child) in first.children {
            unsafe {
                children.push_unchecked((offset, child));
            }
        }

        for (offset, child) in second.children {
            unsafe {
                children.push_unchecked((offset + first.distance, child));
            }
        }

        children
    }

    pub fn extend_scan_distance(mut self, amount: usize) -> Self {
        self.scan_distance = self.scan_distance.max(amount);
        self
    }

    pub fn with_work(mut self, amount: usize) -> Self {
        self.work = amount;
        self
    }

    pub fn add_work(mut self, amount: usize) -> Self {
        self.work += amount;
        self
    }

    pub fn boxed(self) -> Refc<Self> {
        Refc::new(self)
    }

    pub fn unboxed(boxed: &Refc<Self>) -> Self {
        Self {
            grouping: Grouping::None,
            scan_distance: boxed.scan_distance,
            work: boxed.work,
            distance: boxed.distance,
            error_distance: boxed.error_distance,
            children: ArrayVec::of([(0, boxed.clone())]),
        }
    }

    pub fn grouping(&self) -> Grouping<G::Label, G::Expected> {
        self.grouping
    }

    pub fn scan_distance(&self) -> usize {
        self.scan_distance
    }

    pub fn work(&self) -> usize {
        self.work
    }

    pub fn distance(&self) -> usize {
        self.distance
    }

    pub fn error_distance(&self) -> Option<usize> {
        self.error_distance
    }

    pub fn walk(&self) -> impl Iterator<Item = (usize, &Self, EnterExit)> {
        Walk {
            initialized: false,
            parents: Stack::of((0, self, 0)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Grouping<L: Label, E: Expected<L>> {
    None,
    Label(L),
    Error(E),
}

impl<L: Label, E: Expected<L>> Grouping<L, E> {
    fn is_none(&self) -> bool {
        match self {
            Grouping::None => true,
            Grouping::Label(_) | Grouping::Error(_) => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum EnterExit {
    Enter,
    Exit,
}

struct Walk<'a, G: Grammar> {
    initialized: bool,
    parents: Stack<(usize, &'a Match<G>, usize)>,
}

impl<'a, G: Grammar> Iterator for Walk<'a, G> {
    type Item = (usize, &'a Match<G>, EnterExit);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.initialized {
            let (position, node, _) = unsafe { self.parents.top().unwrap_unchecked() };

            self.initialized = true;
            return Some((*position, node, EnterExit::Enter));
        }

        let (base_position, node, child_index) = self.parents.top_mut()?;
        let base_position = *base_position;
        let node = *node;

        if node.children.len() == *child_index {
            self.parents.pop();
            return Some((base_position + node.distance, node, EnterExit::Exit));
        }

        let (offset, child) = unsafe { node.children.get_unchecked(*child_index) };
        let child = child.deref();

        *child_index += 1;

        let position = base_position + offset;
        self.parents.push((position, child, 0));
        Some((position, child, EnterExit::Enter))
    }
}

impl<'a, G: Grammar> FusedIterator for Walk<'a, G> {}
