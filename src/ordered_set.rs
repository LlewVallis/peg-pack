use std::collections::HashSet;
use std::hash::Hash;

pub struct OrderedSet<T> {
    list: Vec<T>,
    set: HashSet<T>,
}

impl<T> OrderedSet<T> {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            set: HashSet::new(),
        }
    }

    pub fn reverse(&mut self) {
        self.list.reverse();
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<T: Hash + Eq + Copy> OrderedSet<T> {
    pub fn push(&mut self, value: T) {
        if self.set.insert(value) {
            self.list.push(value);
        }
    }

    pub fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            self.push(value);
        }
    }
}

impl<T: Hash + Eq + Copy> OrderedSet<T> {
    pub fn pop(&mut self) -> Option<T> {
        let value = self.list.pop()?;
        assert!(self.set.remove(&value));
        Some(value)
    }
}

impl<T: Hash + Eq + Copy> FromIterator<T> for OrderedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = Self::new();

        for value in iter {
            result.push(value);
        }

        result
    }
}
