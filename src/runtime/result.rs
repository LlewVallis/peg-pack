use std::hint::unreachable_unchecked;
use std::iter::FusedIterator;
use super::Grammar;

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

    pub fn mark_error(mut self) -> Self {
        match &mut self {
            ParseResult::Matched(value) => {
                value.error_distance = Some(0);
            }
            ParseResult::Unmatched { .. } => {}
        }

        self
    }

    pub fn label(self, label: G::Label) -> Self {
        match self {
            ParseResult::Matched(value) => {
                let new_value = Match {
                    label: Some(label),
                    scan_distance: value.scan_distance,
                    distance: value.distance,
                    error_distance: value.error_distance,
                    children: vec![value],
                };

                ParseResult::Matched(new_value)
            }
            ParseResult::Unmatched { .. } => self,
        }
    }
}

pub struct Match<G: Grammar> {
    label: Option<G::Label>,
    scan_distance: usize,
    distance: usize,
    error_distance: Option<usize>,
    children: Vec<Self>,
}

impl<G: Grammar> Match<G> {
    pub fn empty(scan_distance: usize) -> Self {
        Self::error_free(0, scan_distance)
    }

    pub fn error_free(distance: usize, scan_distance: usize) -> Self {
        Self {
            scan_distance,
            distance,
            label: None,
            error_distance: None,
            children: Vec::new(),
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

        Self {
            label: None,
            scan_distance,
            distance,
            error_distance,
            children: vec![first, second],
        }
    }

    pub fn extend_scan_distance(mut self, amount: usize) -> Self {
        self.scan_distance = self.scan_distance.max(amount);
        self
    }

    pub fn label(&self) -> Option<G::Label> {
        self.label
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

    pub fn walk_labelled(&self) -> impl Iterator<Item = (&Self, EnterExit)> {
        self.walk().filter(|(node, _)| node.label().is_some())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
        let index = *child_index;

        if node.children.len() == index {
            self.parents.pop();
            return Some((node, EnterExit::Exit));
        }

        *child_index += 1;
        self.parents.push((&node.children[index], 0));
        Some((&node.children[index], EnterExit::Enter))
    }
}

impl<'a, G: Grammar> FusedIterator for Walk<'a, G> {}
