use serde::Serialize;

use crate::store::StoreKey;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct SeriesId(pub usize);

impl StoreKey for SeriesId {
    fn from_usize(value: usize) -> Self {
        Self(value)
    }

    fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Clone, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Series {
    classes: Vec<Class>,
}

impl Series {
    pub fn empty() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    pub fn concatenate(first: &Series, second: &Series) -> Series {
        let mut result = Self::empty();

        for class in first.classes.iter().chain(second.classes.iter()) {
            result.append(class.clone());
        }

        result
    }

    pub fn merge(first: &Series, second: &Series) -> Option<Series> {
        if first.is_never() {
            return Some(second.clone());
        }

        if second.is_never() {
            return Some(first.clone());
        }

        if first.classes.len() != second.classes.len() {
            return None;
        }

        let len = first.classes.len();
        if len == 0 {
            return Some(Self::empty());
        }

        if let Some(result) = Self::union_equivalent(first, second, len) {
            return Some(result);
        }

        if let Some(result) = Self::union_subset(first, second, len) {
            return Some(result);
        }

        if let Some(result) = Self::union_subset(second, first, len) {
            return Some(result);
        }

        None
    }

    fn union_equivalent(first: &Series, second: &Series, len: usize) -> Option<Series> {
        if first.classes[0..len - 1] != second.classes[0..len - 1] {
            return None;
        }

        let first_last = first.classes.last().unwrap();
        let second_last = second.classes.last().unwrap();
        let last = Class::union(first_last, second_last);

        let mut classes = first.classes[0..len - 1].to_vec();
        classes.push(last);

        Some(Self { classes })
    }

    fn union_subset(first: &Series, second: &Series, len: usize) -> Option<Series> {
        for i in 0..len {
            if !first.classes[i].contains(&second.classes[i]) {
                return None;
            }
        }

        Some(first.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    pub fn is_never(&self) -> bool {
        self.classes.iter().any(|class| class.is_never())
    }

    pub fn append(&mut self, class: Class) {
        self.classes.push(class);

        if self.is_never() {
            self.classes.clear();
            self.classes.push(Class::new(false));
        }
    }

    pub fn classes(&self) -> &[Class] {
        &self.classes
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Clone, Ord, PartialOrd)]
pub struct Class {
    negated: bool,
    ranges: Vec<(u8, u8)>,
}

impl Class {
    pub fn new(negated: bool) -> Self {
        Self {
            negated,
            ranges: vec![],
        }
    }

    pub fn union(first: &Self, second: &Self) -> Self {
        if first.negated == second.negated {
            let mut result = first.clone();

            for (start, end) in &second.ranges {
                result.insert(*start, *end);
            }

            result
        } else {
            let (negated, non_negated) = if first.negated {
                (first, second)
            } else {
                (second, first)
            };

            let mut result = negated.clone();

            for (start, end) in &non_negated.ranges {
                result.remove(*start, *end);
            }

            result
        }
    }

    pub fn insert<T: Into<u8>>(&mut self, start: T, end: T) {
        let start = start.into();
        let end = end.into();
        assert!(start <= end);

        self.ranges.push((start, end));
        self.normalize();
    }

    pub fn remove<T: Into<u8>>(&mut self, start: T, end: T) {
        let start = start.into();
        let end = end.into();
        assert!(start <= end);

        let mut new_ranges = Vec::new();

        for (old_start, old_end) in self.ranges.iter().copied() {
            if old_start < start {
                new_ranges.push((old_start, old_end.min(start - 1)));
            }

            if old_end > end {
                new_ranges.push((old_start.max(end + 1), old_end));
            }
        }

        self.ranges = new_ranges;
        self.normalize();
    }

    pub fn contains(&self, other: &Self) -> bool {
        let union = Self::union(self, other);
        self == &union
    }

    fn normalize(&mut self) {
        self.ranges.sort_unstable_by_key(|(start, _)| *start);

        let mut i = 0;
        while i + 1 < self.ranges.len() {
            let current = self.ranges[i];
            let next = &mut self.ranges[i + 1];

            if current.1 >= next.0 {
                next.0 = u8::min(current.0, next.0);
                next.1 = u8::max(current.1, next.1);
                self.ranges.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn is_never(&self) -> bool {
        if self.negated {
            self.ranges == [(u8::MIN, u8::MAX)]
        } else {
            self.ranges.is_empty()
        }
    }

    pub fn negated(&self) -> bool {
        self.negated
    }

    pub fn ranges(&self) -> &[(u8, u8)] {
        &self.ranges
    }
}
