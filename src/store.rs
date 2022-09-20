use std::collections::BTreeMap;
use std::{fmt, mem};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

pub trait StoreKey: Copy + Eq + Ord + Hash {
    fn from_usize(value: usize) -> Self;
    fn into_usize(self) -> usize;
}

/// An ordered map from a key that is convertable to a `usize`, to any value
/// type. Insertion automatically generates a new key that has not yet been used
pub struct Store<K, V> {
    next_id: usize,
    map: BTreeMap<usize, V>,
    marker: PhantomData<K>,
}

impl<K: StoreKey, V> Store<K, V> {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            map: BTreeMap::new(),
            marker: PhantomData::default(),
        }
    }

    /// Generate a key for future insertion without currently inserting into
    /// the map
    pub fn reserve(&mut self) -> K {
        let id = self.next_id;
        self.next_id += 1;
        K::from_usize(id)
    }

    pub fn insert(&mut self, value: V) -> K {
        let id = self.reserve();
        self.set(id, value);
        id
    }

    pub fn set(&mut self, id: K, value: V) {
        let id = id.into_usize();
        self.next_id = self.next_id.max(id + 1);
        self.map.insert(id, value);
    }

    pub fn remove(&mut self, id: K) -> Option<V> {
        self.map.remove(&id.into_usize())
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (K, &V)> {
        self.map.iter().map(|(k, v)| (K::from_usize(*k), v))
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (K, &mut V)> {
        self.map.iter_mut().map(|(k, v)| (K::from_usize(*k), v))
    }

    pub fn drain(&mut self) -> impl DoubleEndedIterator<Item = (K, V)> {
        let store = mem::replace(self, Self::new());
        store.map.into_iter().map(|(k, v)| (K::from_usize(k), v))
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<K: StoreKey, V> Index<K> for Store<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &V {
        self.map.get(&index.into_usize()).unwrap()
    }
}

impl<K: StoreKey, V> IndexMut<K> for Store<K, V> {
    fn index_mut(&mut self, index: K) -> &mut V {
        self.map.get_mut(&index.into_usize()).unwrap()
    }
}

impl<K: StoreKey, V: Copy> Store<K, V> {
    pub fn iter_copied(&self) -> impl DoubleEndedIterator<Item = (K, V)> + '_ {
        self.iter().map(|(k, v)| (k, *v))
    }
}

impl<K: StoreKey, V: Eq + PartialEq> Eq for Store<K, V> {}

impl<K: StoreKey, V: PartialEq> PartialEq<Self> for Store<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

impl<K, V: Debug> Debug for Store<K, V> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.map, f)
    }
}

impl<K, V: Serialize> Serialize for Store<K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let maximum = self.map.keys().map(|value| value + 1).max().unwrap_or(0);

        let mut seq = serializer.serialize_seq(Some(maximum))?;

        for i in 0..maximum {
            let value = self.map.get(&i);
            seq.serialize_element(&value)?;
        }

        seq.end()
    }
}
