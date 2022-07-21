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

    pub fn merge(first: &Series, second: &Series) -> Series {
        let mut result = Self::empty();

        for class in first.classes.iter().chain(second.classes.iter()) {
            result.append(class.clone());
        }

        result
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

    pub fn insert<T: Into<u8>>(&mut self, start: T, end: T) {
        let start = start.into();
        let end = end.into();

        assert!(start <= end);
        self.ranges.push((start, end));

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
