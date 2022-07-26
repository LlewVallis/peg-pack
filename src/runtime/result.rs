use std::fmt::Debug;
use std::hash::Hash;
use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use std::ops::Deref;

use super::array_vec::ArrayVec;
use super::grammar::{Expected, Grammar, Label};
use super::refc::Refc;

pub enum ParseResult<G: Grammar> {
    Matched(Match<G>),
    Unmatched { scan_distance: usize },
}

impl<G: Grammar> ParseResult<G> {
    pub fn is_match(&self) -> bool {
        match self {
            Self::Matched(_) => true,
            Self::Unmatched { .. } => false,
        }
    }

    pub fn is_error_free(&self) -> bool {
        match self {
            Self::Matched(value) => value.is_error_free(),
            Self::Unmatched { .. } => false,
        }
    }

    pub fn scan_distance(&self) -> usize {
        match self {
            Self::Matched(value) => value.scan_distance(),
            Self::Unmatched { scan_distance } => *scan_distance,
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
            Self::Unmatched { scan_distance } => Self::Unmatched {
                scan_distance: scan_distance.max(amount),
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
            },
            Self::Unmatched { scan_distance } => Self::Matched(Match::empty(scan_distance)),
        }
    }

    pub fn mark_error(self, expected: G::Expected) -> Self {
        match self {
            ParseResult::Matched(value) => {
                let replace =
                    value.grouping.is_none() || value.grouping == Grouping::Error(expected);

                let new_value = if replace {
                    Match {
                        grouping: Grouping::Error(expected),
                        scan_distance: value.scan_distance,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: value.children,
                    }
                } else {
                    Match {
                        grouping: Grouping::Error(expected),
                        scan_distance: value.scan_distance,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: ArrayVec::of([Refc::new(value)]),
                    }
                };

                ParseResult::Matched(new_value)
            }
            ParseResult::Unmatched { .. } => self,
        }
    }

    pub fn label(self, label: G::Label) -> Self {
        match self {
            ParseResult::Matched(value) => {
                let new_value = if value.grouping.is_none() {
                    Match {
                        grouping: Grouping::Label(label),
                        scan_distance: value.scan_distance,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: value.children,
                    }
                } else {
                    Match {
                        grouping: Grouping::Label(label),
                        scan_distance: value.scan_distance,
                        distance: value.distance,
                        error_distance: value.error_distance,
                        children: ArrayVec::of([Refc::new(value)]),
                    }
                };

                ParseResult::Matched(new_value)
            }
            ParseResult::Unmatched { .. } => self,
        }
    }
}

const MATCH_CHILDREN: usize = 4;

pub struct Match<G: Grammar> {
    grouping: Grouping<G::Label, G::Expected>,
    scan_distance: usize,
    distance: usize,
    error_distance: Option<usize>,
    children: ArrayVec<Refc<Self>, MATCH_CHILDREN>,
}

impl<G: Grammar> Match<G> {
    pub fn empty(scan_distance: usize) -> Self {
        Self::error_free(0, scan_distance)
    }

    pub fn error_free(distance: usize, scan_distance: usize) -> Self {
        Self {
            scan_distance,
            distance,
            grouping: Grouping::None,
            error_distance: None,
            children: ArrayVec::new(),
        }
    }

    pub fn combine(first: Self, second: Self) -> Self {
        let scan_distance = usize::max(first.scan_distance, first.distance + second.scan_distance);

        let distance = first.distance + second.distance;

        let error_distance = first.error_distance.or_else(|| {
            second
                .error_distance
                .map(|distance| first.distance + distance)
        });

        let first_grouping = first.grouping;
        let second_grouping = second.grouping;

        let concatenation_would_overflow =
            (first.children.len() + second.children.len()) > MATCH_CHILDREN;
        let both_have_no_label = first_grouping.is_none() && second_grouping.is_none();
        let can_concatenate = both_have_no_label && !concatenation_would_overflow;

        if can_concatenate {
            let children = unsafe { ArrayVec::concat_unchecked(first.children, second.children) };

            return Self {
                grouping: first.grouping,
                scan_distance,
                distance,
                error_distance,
                children,
            };
        }

        if first.is_insignificant() {
            return Self {
                grouping: second.grouping,
                scan_distance,
                distance,
                error_distance,
                children: second.children,
            };
        }

        if second.is_insignificant() {
            return Self {
                grouping: first.grouping,
                scan_distance,
                distance,
                error_distance,
                children: first.children,
            };
        }

        Self {
            grouping: Grouping::None,
            scan_distance,
            distance,
            error_distance,
            children: ArrayVec::of([Refc::new(first), Refc::new(second)]),
        }
    }

    fn is_insignificant(&self) -> bool {
        self.distance == 0 && self.grouping.is_none() && self.children.is_empty()
    }

    pub fn extend_scan_distance(mut self, amount: usize) -> Self {
        self.scan_distance = self.scan_distance.max(amount);
        self
    }

    pub fn grouping(&self) -> Grouping<G::Label, G::Expected> {
        self.grouping
    }

    pub fn scan_distance(&self) -> usize {
        self.scan_distance
    }

    pub fn distance(&self) -> usize {
        self.distance
    }

    pub fn error_distance(&self) -> Option<usize> {
        self.error_distance
    }

    pub fn is_error_free(&self) -> bool {
        self.error_distance.is_none()
    }

    pub fn walk(&self) -> impl Iterator<Item = (&Self, EnterExit)> {
        Walk {
            initial: false,
            parents: vec![(self, 0)],
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
    initial: bool,
    parents: Vec<(&'a Match<G>, usize)>,
}

impl<'a, G: Grammar> Iterator for Walk<'a, G> {
    type Item = (&'a Match<G>, EnterExit);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.initial {
            let (node, _) = unsafe { self.parents.first().unwrap_unchecked() };

            self.initial = true;
            return Some((node, EnterExit::Enter));
        }

        let (node, child_index) = self.parents.last_mut()?;
        let node = *node;

        if node.children.len() == *child_index {
            self.parents.pop();
            return Some((node, EnterExit::Exit));
        }

        let child = unsafe { node.children.get_unchecked(*child_index).deref() };
        *child_index += 1;

        self.parents.push((child, 0));
        Some((child, EnterExit::Enter))
    }
}

impl<'a, G: Grammar> FusedIterator for Walk<'a, G> {}
