use std::fmt::Debug;
use std::hash::Hash;

use super::context::Context;
use super::input::Input;
use super::State;

pub trait Grammar: Sized {
    type Label: LabelType + 'static;
    type Expected: ExpectedType<Self::Label> + 'static;

    fn start_state(&self) -> State;

    fn cache_slots(&self) -> usize;

    unsafe fn dispatch_state<I: Input + ?Sized>(&self, state: State, ctx: &mut Context<I, Self>);
}

pub trait LabelType: Debug + Copy + Eq + Hash {}

pub trait ExpectedType<L: LabelType>: Debug + Copy + Eq + Hash {
    fn literals(&self) -> &'static [&'static [u8]];

    fn labels(&self) -> &'static [L];
}
