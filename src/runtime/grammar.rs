use std::fmt::Debug;
use std::hash::Hash;

use super::{Input, State};

pub trait Grammar: Sized {
    type Label: LabelType + 'static;
    type Expected: ExpectedType<Self::Label> + 'static;

    fn start_state<I: Input + ?Sized>(&self) -> State<I, Self>;

    fn cache_slots(&self) -> usize;
}

pub trait LabelType: Debug + Copy + Eq + Hash {}

pub trait ExpectedType<L: LabelType>: Debug + Copy + Eq + Hash {
    fn literals(&self) -> &'static [&'static [u8]];

    fn labels(&self) -> &'static [L];
}
